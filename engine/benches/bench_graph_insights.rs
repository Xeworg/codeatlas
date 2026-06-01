// Graph Insights Benchmark
// Run: cargo bench --package engine -- bench_graph_insights
// H1 Gate 1 — NFR Evidence

#![feature(test)]
extern crate test;

#[cfg(test)]
mod graph_insights_bench {
    use engine::analysis::graph_insights::GraphInsightsEngine;
    use engine::db::Database;
    use engine::models::{FileInfo, ImportInfo, ProjectInfo, Symbol};
    use tempfile::TempDir;
    use test::Bencher;

    fn insert_2000_nodes(db: &Database, project_id: &str) {
        let project = ProjectInfo {
            id: project_id.to_string(),
            name: "bench-insights".to_string(),
            root_path: ".".to_string(),
            status: "ready".to_string(),
            files_count: 2000,
            symbols_count: 10000,
            imports_count: 5000,
            scan_duration_ms: 0,
            last_scanned_at: chrono::Utc::now().to_rfc3339(),
            error: None,
        };
        db.save_project(&project).unwrap();

        let mut files = Vec::new();
        let mut imports = Vec::new();

        // 2000 files: 5 groups of 400 (like services, components, etc.)
        for i in 0..2000 {
            files.push(FileInfo {
                id: format!("n-{:04}", i),
                path: format!("src/mod{}/file{:04}.ts", i / 400, i),
                name: format!("file{:04}.ts", i),
                extension: "ts".to_string(),
                symbols: vec![Symbol {
                    id: format!("s-{:04}", i),
                    name: format!("fn{:04}", i),
                    kind: "function".to_string(),
                    line_start: 1,
                    line_end: 20,
                    is_exported: i % 3 == 0,
                }],
                lines: 20,
            });

            // Each file imports 2-3 others (creates graph density)
            let target = (i + 100) % 2000;
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
    fn bench_graph_insights_2000_nodes(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();
        insert_2000_nodes(&db, "proj-insights-2k");

        b.iter(|| {
            let engine = GraphInsightsEngine::new();
            let _ = engine.compute("proj-insights-2k", &db);
        });
    }

    #[bench]
    fn bench_graph_insights_500_nodes(b: &mut Bencher) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("codeatlas.db");
        let db = Database::new(&db_path).unwrap();

        let project = ProjectInfo {
            id: "proj-insights-500".to_string(),
            name: "bench-small".to_string(),
            root_path: ".".to_string(),
            status: "ready".to_string(),
            files_count: 500,
            symbols_count: 2500,
            imports_count: 1200,
            scan_duration_ms: 0,
            last_scanned_at: chrono::Utc::now().to_rfc3339(),
            error: None,
        };
        db.save_project(&project).unwrap();

        let mut files = Vec::new();
        let mut imports = Vec::new();
        for i in 0..500 {
            files.push(FileInfo {
                id: format!("n-{:03}", i),
                path: format!("src/f{:03}.ts", i),
                name: format!("f{:03}.ts", i),
                extension: "ts".to_string(),
                symbols: vec![],
                lines: 15,
            });
            if i > 0 {
                imports.push(ImportInfo {
                    id: format!("i-{:03}", i),
                    source_file_id: format!("n-{:03}", i),
                    target_file_id: Some(format!("n-{:03}", (i + 1) % 500)),
                    target_module: None,
                    imports: vec![],
                    is_default: false,
                    is_type: false,
                });
            }
        }
        db.save_files("proj-insights-500", &files).unwrap();
        db.save_imports(&imports).unwrap();

        b.iter(|| {
            let engine = GraphInsightsEngine::new();
            let _ = engine.compute("proj-insights-500", &db);
        });
    }
}
