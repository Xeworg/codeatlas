// Scan benchmarks — PR6 Hardening
// Run: cargo bench --package engine
// These are informative (not gatekeeping).

#![feature(test)]
extern crate test;

#[cfg(test)]
mod benchmarks {
    use test::Bencher;
    use engine::{scanner::FileWalker, graph::GraphBuilder};
    use engine::models::{FileInfo, NodeType};
    use engine::models::ImportInfo;

    fn fixture_path() -> &'static str {
        // Point to the engine/fixtures test directory
        "engine/fixtures/small_ts"
    }

    #[bench]
    fn bench_file_discovery_100_files(b: &mut Bencher) {
        // Fixture: engine/fixtures/scan_100/
        // Run with: cargo bench --package engine -- bench_file_discovery_100_files
        b.iter(|| {
            let walker = FileWalker::new(".");
            walker.discover()
        });
    }

    #[bench]
    fn bench_graph_build_50_nodes(b: &mut Bencher) {
        let files: Vec<FileInfo> = (0..50)
            .map(|i| FileInfo {
                id: format!("file-{}", i),
                path: format!("src/file{}.ts", i),
                name: format!("file{}.ts", i),
                extension: "ts".to_string(),
                symbols: vec![],
                lines: 100,
            })
            .collect();

        let imports: Vec<ImportInfo> = (0..49)
            .map(|i| ImportInfo {
                id: format!("imp-{}", i),
                source_file_id: format!("file-{}", i),
                target_file_id: Some(format!("file-{}", i + 1)),
                target_module: None,
                imports: vec![],
                is_default: false,
                is_type: false,
            })
            .collect();

        b.iter(|| {
            let builder = GraphBuilder::new("./");
            builder.build(&files, &imports).unwrap()
        });
    }
}
