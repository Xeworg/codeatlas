// Impact Analysis Benchmark
// Run: cargo bench --package engine -- bench_impact
// H1 Gate 1 — NFR Evidence

#![feature(test)]
extern crate test;

#[cfg(test)]
mod impact_bench {
    use engine::analysis::impact_engine::ImpactEngine;
    use engine::db::Database;
    use engine::models::{FileInfo, ImportInfo, ProjectInfo, Symbol};
    use tempfile::TempDir;
    use test::Bencher;

    fn setup_project(db: &Database, project_id: &str, node_count: usize) {
        let project = ProjectInfo {
            id: project_id.to_string(),
            name: "bench-impact".to_string(),
            root_path: ".".to_string(),
            status: "ready".to_string(),
            files_count: node_count,
            symbols_count: node_count * 5,
            imports_count: node_count * 2,
            scan_duration_ms: 0,
            last_scanned_at: chrono::Utc::now().to_rfc3339(),
            error: None,
        };
        db.save_project(&project).unwrap();

        let mut files = Vec::new();
        let mut imports = Vec::new();

        for i in 0..node_count {
            files.push(FileInfo {
                id: format!("node-{:04}", i),
                path: format!("src/file{:04}.ts", i),
                name: format!("file{:04}.ts", i),
                extension: "ts".to_string(),
                symbols: vec![
                    Symbol {
                        id: format!("sym-fn-{:04}", i),
                        name: format!("func{:04}", i),
                        kind: "function".to_string(),
                        line_start: 1,
                        line_end: 30,
                        is_exported: true,
                    },
                    Symbol {
                        id: format!("sym-cls-{:04}", i),
                        name: format!("Class{:04}", i),
                        kind: "class".to_string(),
                        line_start: 31,
                        line_end: 60,
                        is_exported: true,
                    },
                ],
                lines: 60,
            });

            // Create a chain: node i imports node i+1 and node i+2
            // This creates a graph with meaningful impact radius
            if i + 1 < node_count {
                imports.push(ImportInfo {
                    id: format!("imp-a-{:04}", i),
                    source_file_id: format!("node-{:04}", i),
                    target_file_id: Some(format!("node-{:04}", (i + 1) % node_count)),
                    target_module: None,
                    imports: vec![],
                    is_default: false,
                    is_type: false,
                });
            }
            if i + 2 < node_count {
                imports.push(ImportInfo {
                    id: format!("imp-b-{:04}", i),
                    source_file_id: format!("node-{:04}", i),
                    target_file_id: Some(format!("node-{:04}", (i + 2) % node_count)),
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
    fn bench_impact_single_node_500(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();
        setup_project(&db, "proj-impact-500", 500);

        b.iter(|| {
            let engine = ImpactEngine::new();
            let _ = engine.analyze("proj-impact-500", "node-0000", &db);
        });
    }

    #[bench]
    fn bench_impact_single_node_1000(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();
        setup_project(&db, "proj-impact-1k", 1000);

        b.iter(|| {
            let engine = ImpactEngine::new();
            let _ = engine.analyze("proj-impact-1k", "node-0000", &db);
        });
    }

    #[bench]
    fn bench_impact_central_node(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();
        setup_project(&db, "proj-impact-central", 800);

        b.iter(|| {
            let engine = ImpactEngine::new();
            // Central node: node-0400 is in the middle of the graph
            let _ = engine.analyze("proj-impact-central", "node-0400", &db);
        });
    }
}
