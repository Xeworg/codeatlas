// Export JSON Benchmark
// Run: cargo bench --package engine -- bench_export
// H1 Gate 1 — NFR Evidence

#![feature(test)]
extern crate test;

#[cfg(test)]
mod export_bench {
    use engine::db::Database;
    use engine::export_view::ExportService;
    use engine::models::{FileInfo, ImportInfo, ProjectInfo, Symbol};
    use tempfile::TempDir;
    use test::Bencher;

    fn setup_export_project(db: &Database, project_id: &str, node_count: usize) {
        let project = ProjectInfo {
            id: project_id.to_string(),
            name: "bench-export".to_string(),
            root_path: ".".to_string(),
            status: "ready".to_string(),
            files_count: node_count,
            symbols_count: node_count * 3,
            imports_count: node_count,
            scan_duration_ms: 0,
            last_scanned_at: chrono::Utc::now().to_rfc3339(),
            error: None,
        };
        db.save_project(&project).unwrap();

        let mut files = Vec::new();
        let mut imports = Vec::new();

        for i in 0..node_count {
            files.push(FileInfo {
                id: format!("n-{:04}", i),
                path: format!("src/file{:04}.ts", i),
                name: format!("file{:04}.ts", i),
                extension: "ts".to_string(),
                symbols: vec![
                    Symbol {
                        id: format!("s-{:04}", i * 2),
                        name: format!("fn{}", i),
                        kind: "function".to_string(),
                        line_start: 1,
                        line_end: 25,
                        is_exported: true,
                    },
                    Symbol {
                        id: format!("s-{:04}", i * 2 + 1),
                        name: format!("cls{}", i),
                        kind: "class".to_string(),
                        line_start: 26,
                        line_end: 50,
                        is_exported: false,
                    },
                ],
                lines: 50,
            });

            let target = (i + 7) % node_count;
            imports.push(ImportInfo {
                id: format!("i-{:04}", i),
                source_file_id: format!("n-{:04}", i),
                target_file_id: Some(format!("n-{:04}", target)),
                target_module: None,
                imports: vec![],
                is_default: false,
                is_type: false,
            });
        }

        db.save_files(project_id, &files).unwrap();
        db.save_imports(&imports).unwrap();
    }

    #[bench]
    fn bench_export_json_500_nodes(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();
        setup_export_project(&db, "proj-export-500", 500);

        b.iter(|| {
            let svc = ExportService::new();
            let _ = svc.export_json("proj-export-500", &db);
        });
    }

    #[bench]
    fn bench_export_json_2000_nodes(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();
        setup_export_project(&db, "proj-export-2k", 2000);

        b.iter(|| {
            let svc = ExportService::new();
            let _ = svc.export_json("proj-export-2k", &db);
        });
    }

    #[bench]
    fn bench_export_json_5000_nodes(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();
        setup_export_project(&db, "proj-export-5k", 5000);

        b.iter(|| {
            let svc = ExportService::new();
            let _ = svc.export_json("proj-export-5k", &db);
        });
    }
}
