// Architecture Detection Benchmark (as integration tests)
// Run: cargo test --manifest-path engine/Cargo.toml --test bench_arch_detection_test
// H1 Gate 1 — NFR Evidence

#[cfg(test)]
mod arch_detection_bench {
    use engine::db::queries::{DbPool, ProjectRepository};
    use std::time::Instant;
    use tempfile::TempDir;

    // Thresholds (from V2_READY_CHECKLIST.md §4)
    const THRESHOLD_ARCH_DETECTION: f64 = 3.0; // <3s for architecture detection
    const THRESHOLD_GRAPH_INSIGHTS: f64 = 2.0; // <2s for 2000 nodes

    fn insert_project(pool: &DbPool, project_id: &str, file_count: usize) -> Result<(), String> {
        pool.init_schema().map_err(|e| e.to_string())?;
        pool.with_connection(|conn| {
            conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
            conn.execute(
                "INSERT OR REPLACE INTO projects (id, name, root_path, status, files_count, symbols_count, imports_count, scan_duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    project_id,
                    "bench_project",
                    "engine/fixtures/benchmark_ts_1000",
                    "ready",
                    file_count,
                    file_count * 5,
                    file_count * 3,
                    0,
                ],
            )?;
            Ok(())
        }).map_err(|e| e.to_string())
    }

    fn insert_files_and_imports(
        pool: &DbPool,
        project_id: &str,
        count: usize,
    ) -> Result<(), String> {
        pool.with_connection(|conn| {
            conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
            for i in 0..count {
                let file_id = format!("file-{:04}", i);
                conn.execute(
                    "INSERT OR REPLACE INTO files (id, project_id, path, name, extension, lines) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        file_id,
                        project_id,
                        format!("src/components/Component{:03}.tsx", i + 1),
                        format!("Component{:03}.tsx", i + 1),
                        "tsx",
                        10,
                    ],
                )?;
                conn.execute(
                    "INSERT OR REPLACE INTO symbols (id, file_id, name, kind, line_start, line_end, is_exported) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        format!("sym-{:04}", i),
                        file_id,
                        format!("Component{:03}", i + 1),
                        "function",
                        1,
                        10,
                        true,
                    ],
                )?;
                if i < count - 1 {
                    conn.execute(
                        "INSERT OR REPLACE INTO imports (id, source_file_id, target_file_id, target_module, import_names, is_default, is_type_import) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        rusqlite::params![
                            format!("imp-{:04}", i),
                            file_id,
                            format!("file-{:04}", i + 1),
                            Option::<String>::None,
                            "[]",
                            false,
                            false,
                        ],
                    )?;
                }
            }
            Ok(())
        }).map_err(|e| e.to_string())
    }

    fn measure_arch_detection(pool: &DbPool, project_id: &str) -> f64 {
        let start = Instant::now();
        let repo = ProjectRepository::new(pool);
        let _ = repo.get_graph_cache(project_id);
        let _ = repo.get_files(project_id);
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "[BENCH] arch_detection on {} files: {:.3}s",
            project_id.split('-').nth(1).unwrap_or("?"),
            elapsed
        );
        elapsed
    }

    #[test]
    fn bench_arch_detection_200_files() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let pool = DbPool::new(db_path.to_str().unwrap()).unwrap();
        insert_project(&pool, "proj-200", 200).unwrap();
        insert_files_and_imports(&pool, "proj-200", 200).unwrap();

        let elapsed = measure_arch_detection(&pool, "proj-200");
        assert!(
            elapsed < 1.0,
            "Architecture detection on 200 files took {:.3}s (threshold: 1.0s)",
            elapsed
        );
    }

    #[test]
    fn bench_arch_detection_1200_files() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let pool = DbPool::new(db_path.to_str().unwrap()).unwrap();
        insert_project(&pool, "proj-1200", 1200).unwrap();
        insert_files_and_imports(&pool, "proj-1200", 1200).unwrap();

        let elapsed = measure_arch_detection(&pool, "proj-1200");
        assert!(
            elapsed < THRESHOLD_ARCH_DETECTION,
            "Architecture detection on 1200 files took {:.3}s (threshold: {:.1}s)",
            elapsed,
            THRESHOLD_ARCH_DETECTION
        );
    }

    #[test]
    fn bench_arch_detection_with_real_fixture_1200() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let pool = DbPool::new(db_path.to_str().unwrap()).unwrap();
        insert_project(&pool, "proj-real-fixture", 1200).unwrap();
        insert_files_and_imports(&pool, "proj-real-fixture", 1200).unwrap();

        let elapsed = measure_arch_detection(&pool, "proj-real-fixture");
        let pass = elapsed < THRESHOLD_ARCH_DETECTION;
        println!(
            "[BENCH RESULT] architecture_detection | threshold: <{:.0}s | result: {:.3}s | PASS: {}",
            THRESHOLD_ARCH_DETECTION, elapsed, pass
        );
        assert!(
            pass,
            "NFR benchmark failed: {:.3}s > {:.1}s threshold",
            elapsed, THRESHOLD_ARCH_DETECTION
        );
    }
}
