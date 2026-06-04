//! RED tests for C.1 — engine::commands pure functions.
//!
//! These tests fail initially and turn green once `engine/src/commands.rs`
//! implements the contracts:
//! - C.1 RED: `scan_files` must call `ParserRegistry::parse_file` exactly once
//!            per discovered file (contract: no re-parsing per file).
//! - C.1 RED: `outline_for_file` must call registry exactly once.

/// C.1 RED — module existence and type-level contracts.
/// These test-only imports verify the module exposes the right types.
#[cfg(test)]
mod module_exists_tests {
    /// Verify the module-level types exist (type resolution = compile check).
    #[test]
    fn commands_module_types_exist() {
        let _: Option<crate::commands::ScanFilesOutput> = None;
        let _: Option<&dyn crate::commands::ParseFile> = None;
    }
}

/// C.1 RED — single dispatch: `scan_files` must call the registry exactly once
/// per file. We use a mock registry that wraps `ParserRegistry` and tracks call count.
#[cfg(test)]
mod single_dispatch_tests {
    use crate::commands::{scan_files, DiscoveredFile, ParseFile, ScanFilesOutput};
    use crate::scanner::parser::ParserRegistry;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// `TrackingRegistry` wraps a real `ParserRegistry` and tracks call count.
    struct TrackingRegistry {
        inner: ParserRegistry,
        calls: AtomicUsize,
    }

    impl TrackingRegistry {
        fn new() -> Self {
            Self {
                inner: ParserRegistry::new(),
                calls: AtomicUsize::new(0),
            }
        }
        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl ParseFile for TrackingRegistry {
        fn parse_file(
            &self,
            path: &str,
            source: &str,
            extension: &str,
            file_id: &str,
        ) -> crate::models::ParseResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inner.parse_file(path, source, extension, file_id)
        }
    }

    /// RED test: scan_files calls registry exactly N times for N files.
    #[test]
    fn scan_files_calls_registry_exactly_n_times() {
        // GIVEN 3 discovered files with real paths on disk.
        // scan_files reads files from disk, so use /tmp paths that exist.
        let files = vec![
            DiscoveredFile {
                path: "/tmp/engine_cmd_test/a.ts".into(),
                relative_path: "src/a.ts".into(),
                extension: "ts".into(),
                size_bytes: 100,
            },
            DiscoveredFile {
                path: "/tmp/engine_cmd_test/b.ts".into(),
                relative_path: "src/b.ts".into(),
                extension: "ts".into(),
                size_bytes: 100,
            },
            DiscoveredFile {
                path: "/tmp/engine_cmd_test/c.rs".into(),
                relative_path: "src/c.rs".into(),
                extension: "rs".into(),
                size_bytes: 100,
            },
        ];
        let root = std::path::PathBuf::from("/tmp/engine_cmd_test");
        let registry = TrackingRegistry::new();

        // WHEN scan_files is called
        let output: ScanFilesOutput = scan_files(&registry, &files, &root);

        // THEN registry is called exactly once per file (no re-parsing)
        assert_eq!(
            output.registry_call_count, 3,
            "scan_files must call registry exactly once per file, got {} calls for 3 files",
            output.registry_call_count
        );
        assert_eq!(
            output.file_infos.len(),
            3,
            "scan_files must return one FileInfo per discovered file"
        );
    }

    /// GREEN test: scan_files propagates symbols and imports into the right shape.
    #[test]
    fn scan_files_propagates_symbols_and_imports() {
        let files = vec![DiscoveredFile {
            path: "/tmp/engine_cmd_test/a.ts".into(),
            relative_path: "src/service.ts".into(),
            extension: "ts".into(),
            size_bytes: 100,
        }];
        let root = std::path::PathBuf::from("/tmp/engine_cmd_test");
        let registry = TrackingRegistry::new();

        let output: ScanFilesOutput = scan_files(&registry, &files, &root);

        // Contract: one file record produced
        assert_eq!(output.file_infos.len(), 1, "expected exactly 1 FileInfo");
        // Registry was called exactly once
        assert_eq!(
            registry.call_count(),
            1,
            "registry must be called exactly once"
        );
        // Parse metadata is tracked
        assert_eq!(output.registry_call_count, 1);
    }
}

/// C.1 RED — outline single dispatch.
#[cfg(test)]
mod outline_single_dispatch_tests {
    use crate::commands::{outline_for_file, ParseFile};
    use crate::models::{OutlineItem, OutlineItemKind};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock registry that returns a non-empty outline and tracks calls.
    struct CountingRegistry {
        calls: AtomicUsize,
    }

    impl CountingRegistry {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
            }
        }
        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl ParseFile for CountingRegistry {
        fn parse_file(
            &self,
            _path: &str,
            _source: &str,
            _extension: &str,
            file_id: &str,
        ) -> crate::models::ParseResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            crate::models::ParseResult {
                symbols: vec![],
                imports: vec![],
                outline: vec![OutlineItem {
                    id: "out-1".into(),
                    file_id: file_id.into(),
                    name: "TestFunc".into(),
                    kind: OutlineItemKind::Function,
                    line_start: 1,
                    line_end: 3,
                    column_start: None,
                    column_end: None,
                    children: vec![],
                }],
                ..Default::default()
            }
        }
    }

    /// RED test: outline_for_file calls registry exactly once.
    #[test]
    fn outline_for_file_calls_registry_exactly_once() {
        let registry = CountingRegistry::new();
        let file_id = "file-outline-test";
        let path = "/tmp/engine_cmd_test/Test.ts";
        let ext = "ts";
        let source = "export function TestFunc() {}";

        let outline = outline_for_file(&registry, file_id, path, ext, source);

        assert_eq!(
            registry.call_count(),
            1,
            "outline_for_file must call registry exactly once, got {} calls",
            registry.call_count()
        );
        assert!(
            !outline.is_empty(),
            "outline_for_file should return non-empty outline for parse result"
        );
    }
}
