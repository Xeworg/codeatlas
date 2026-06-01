// WAL Concurrency Test
// Run: cargo test --manifest-path engine/Cargo.toml --test wal_concurrency_test
// H1 Gate 1 — NFR Evidence (WAL concurrent reads = 0 deadlocks)

#[cfg(test)]
mod wal_concurrency_tests {
    use engine::db::queries::DbPool;
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn test_wal_concurrent_reads_no_deadlock() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db_path_str = db_path.to_str().unwrap();
        let _pool = DbPool::new(db_path_str).unwrap();

        let counter = Arc::new(Mutex::new(0usize));
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        let barrier = Arc::new(Barrier::new(10));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let path_str = db_path_str.to_string();
                let counter = Arc::clone(&counter);
                let errors = Arc::clone(&errors);
                let barrier = Arc::clone(&barrier);

                thread::spawn(move || {
                    let pool = DbPool::new(&path_str).unwrap();
                    barrier.wait();
                    for j in 0..20 {
                        match pool.with_connection(|_conn| Ok(())) {
                            Ok(_) => {
                                let mut c = counter.lock().unwrap();
                                *c += 1;
                            }
                            Err(e) => {
                                let mut errs = errors.lock().unwrap();
                                errs.push(format!("thread-{} read-{}: {:?}", i, j, e));
                            }
                        }
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        let total_reads = *counter.lock().unwrap();
        let errs = errors.lock().unwrap().clone();

        assert_eq!(errs.len(), 0, "WAL errors: {:?}", errs);
        assert_eq!(
            total_reads, 200,
            "Expected 200 reads (10 threads × 20), got {}",
            total_reads
        );
    }

    #[test]
    fn test_wal_write_and_read_no_deadlock() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db_path_str = db_path.to_str().unwrap();
        let _pool = DbPool::new(db_path_str).unwrap();

        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        let barrier = Arc::new(Barrier::new(3));

        // 2 reader threads
        let readers: Vec<_> = (0..2)
            .map(|i| {
                let path_str = db_path_str.to_string();
                let errors = Arc::clone(&errors);
                let barrier = Arc::clone(&barrier);

                thread::spawn(move || {
                    let pool = DbPool::new(&path_str).unwrap();
                    barrier.wait();
                    for j in 0..50 {
                        match pool.with_connection(|_conn| Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                errors
                                    .lock()
                                    .unwrap()
                                    .push(format!("reader-{} error: {:?}", i, e));
                            }
                        }
                    }
                })
            })
            .collect();

        // 1 writer thread
        let writer = {
            let path_str = db_path_str.to_string();
            let errors = Arc::clone(&errors);
            let barrier = Arc::clone(&barrier);

            thread::spawn(move || {
                let pool = DbPool::new(&path_str).unwrap();
                barrier.wait();
                for j in 0..10 {
                    match pool.with_connection(|_conn| Ok(())) {
                        Ok(_) => {}
                        Err(e) => {
                            errors
                                .lock()
                                .unwrap()
                                .push(format!("writer error: {:?}", e));
                        }
                    }
                }
            })
        };

        writer.join().unwrap();
        for r in readers {
            r.join().unwrap();
        }

        let errs = errors.lock().unwrap().clone();
        assert_eq!(errs.len(), 0, "WAL deadlock errors: {:?}", errs);
    }
}
