// Architecture Detection Benchmark
// Run: cargo bench --package engine -- bench_arch_detection
// H1 Gate 1 — NFR Evidence

#![feature(test)]
extern crate test;

#[cfg(test)]
mod arch_detection_bench {
    use engine::analysis::architecture_detector::ArchitectureDetector;
    use engine::db::Database;
    use engine::models::{FileInfo, ImportInfo, ProjectInfo, Symbol};
    use std::sync::Mutex;
    use tempfile::TempDir;
    use test::Bencher;

    fn make_fixture_db(tmp: &TempDir) -> Database {
        let db_path = tmp.path().join("codeatlas.db");
        Database::new(&db_path).unwrap()
    }

    fn insert_1200_files(db: &Database, project_id: &str) {
        let project = ProjectInfo {
            id: project_id.to_string(),
            name: "benchmark_project".to_string(),
            root_path: "engine/fixtures/benchmark_ts_1000".to_string(),
            status: "ready".to_string(),
            files_count: 1200,
            symbols_count: 6000,
            imports_count: 3600,
            scan_duration_ms: 0,
            last_scanned_at: chrono::Utc::now().to_rfc3339(),
            error: None,
        };
        db.save_project(&project).unwrap();

        let mut files = Vec::new();
        let mut imports = Vec::new();

        for i in 0..1200 {
            let file = FileInfo {
                id: format!("file-{:04}", i),
                path: format!("src/components/Component{:03}.tsx", i + 1),
                name: format!("Component{:03}.tsx", i + 1),
                extension: "tsx".to_string(),
                symbols: vec![Symbol {
                    id: format!("sym-{:04}", i),
                    name: format!("Component{:03}", i + 1),
                    kind: "function".to_string(),
                    line_start: 1,
                    line_end: 10,
                    is_exported: true,
                }],
                lines: 10,
            };
            files.push(file);

            if i < 1199 {
                imports.push(ImportInfo {
                    id: format!("imp-{:04}", i),
                    source_file_id: format!("file-{:04}", i),
                    target_file_id: Some(format!("file-{:04}", i + 1)),
                    target_module: None,
                    imports: vec![],
                    is_default: false,
                    is_type: false,
                });
            }
        }
        db.save_files(project_id, &files).unwrap();
        db.save_imports(&imports).unwrap();
    }

    #[bench]
    fn bench_arch_detection_1200_files(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db = make_fixture_db(&tmp);
        insert_1200_files(&db, "proj-bench-001");

        b.iter(|| {
            let detector = ArchitectureDetector::new();
            let _ = detector.detect("proj-bench-001", &db);
        });
    }

    #[bench]
    fn bench_arch_detection_200_files(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db = make_fixture_db(&tmp);

        let project = ProjectInfo {
            id: "proj-bench-200".to_string(),
            name: "small".to_string(),
            root_path: ".".to_string(),
            status: "ready".to_string(),
            files_count: 200,
            symbols_count: 1000,
            imports_count: 600,
            scan_duration_ms: 0,
            last_scanned_at: chrono::Utc::now().to_rfc3339(),
            error: None,
        };
        db.save_project(&project).unwrap();

        let mut files = Vec::new();
        for i in 0..200 {
            files.push(FileInfo {
                id: format!("f-{:03}", i),
                path: format!("src/f{}.ts", i),
                name: format!("f{}.ts", i),
                extension: "ts".to_string(),
                symbols: vec![],
                lines: 20,
            });
        }
        db.save_files("proj-bench-200", &files).unwrap();

        b.iter(|| {
            let detector = ArchitectureDetector::new();
            let _ = detector.detect("proj-bench-200", &db);
        });
    }
}
