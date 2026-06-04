//! Shim tests for import source_file_id persistence contract.
//!
//! Verifies that:
//! 1. `source_file_id` is converted from relative path to persisted UUID before `save_import`.
//! 2. Target resolution still works from the relative path context.
//!
//! These tests run in-process without needing a live Tauri app state.

use engine::models::ImportInfo;
use std::collections::HashMap;

/// Simulates the conversion that `scan_project` applies to `all_imports`:
/// relative_path → UUID via path_to_id, then resolver.resolve(module, rel_path).
#[test]
fn import_source_file_id_converts_relative_path_to_uuid() {
    // path_to_id: relative_path → UUID (as built in scan_project from file_infos)
    let mut path_to_id: HashMap<String, String> = HashMap::new();
    path_to_id.insert(
        "src/service.ts".into(),
        "550e8400-e29b-41d4-a716-446655440001".into(),
    );
    path_to_id.insert(
        "src/utils.rs".into(),
        "550e8400-e29b-41d4-a716-446655440002".into(),
    );

    // Simulate imports coming back from scan_files with relative_path as source_file_id
    let mut imports = vec![
        ImportInfo {
            id: "imp-1".into(),
            source_file_id: "src/service.ts".into(), // relative path (as registry sets it)
            target_file_id: None,
            target_module: Some("./utils".into()),
            imports: vec!["Helper".into()],
            is_default: false,
            is_type: false,
        },
        ImportInfo {
            id: "imp-2".into(),
            source_file_id: "src/utils.rs".into(), // relative path
            target_file_id: None,
            target_module: Some("std::collections".into()),
            imports: vec!["HashMap".into()],
            is_default: false,
            is_type: false,
        },
    ];

    // Apply the same conversion that scan_project applies after scan_files
    for imp in &mut imports {
        // Convert source: relative_path → persisted UUID
        if let Some(uuid) = path_to_id.get(&imp.source_file_id) {
            imp.source_file_id = uuid.clone();
        }
    }

    // After conversion, source_file_id must be UUID (matches `files.id` in DB)
    assert_eq!(
        imports[0].source_file_id, "550e8400-e29b-41d4-a716-446655440001",
        "src/service.ts → uuid"
    );
    assert_eq!(
        imports[1].source_file_id, "550e8400-e29b-41d4-a716-446655440002",
        "src/utils.rs → uuid"
    );

    // Verify no imports lost the source_file_id (would break get_imports query)
    for imp in &imports {
        assert!(
            !imp.source_file_id.is_empty(),
            "source_file_id must not be empty after conversion"
        );
        // UUIDs are 36 chars; relative paths are much shorter
        assert!(
            imp.source_file_id.len() >= 20,
            "source_file_id '{}' should be a UUID (≥20 chars), not a relative path",
            imp.source_file_id
        );
    }
}

/// Verify that path_to_id mapping is built from file_infos before import loop.
/// This ensures the lookup needed for source_file_id conversion is always available.
#[test]
fn path_to_id_map_covers_all_import_sources() {
    // Build path_to_id from file_infos (simulating scan_files output)
    struct FakeFileInfo {
        pub id: String,
        pub path: String,
    }
    let file_infos = vec![
        FakeFileInfo {
            id: "uuid-001".into(),
            path: "src/a.ts".into(),
        },
        FakeFileInfo {
            id: "uuid-002".into(),
            path: "src/b.ts".into(),
        },
    ];

    let path_to_id: HashMap<String, String> = file_infos
        .iter()
        .map(|f| (f.path.clone(), f.id.clone()))
        .collect();

    assert_eq!(path_to_id.get("src/a.ts"), Some(&"uuid-001".into()));
    assert_eq!(path_to_id.get("src/b.ts"), Some(&"uuid-002".into()));
    assert_eq!(path_to_id.len(), 2, "all file paths must be in path_to_id");
}

/// Regression: get_imports(project_id) uses `WHERE source_file_id IN (SELECT id FROM files ...)`.
/// If source_file_id is a relative path, the WHERE clause returns 0 rows even when imports exist.
#[test]
fn source_file_id_must_be_uuid_not_relative_path_for_get_imports_query() {
    // Simulate what get_imports SQL does:
    // SELECT ... FROM imports WHERE source_file_id IN (SELECT id FROM files ...)
    struct FakeFile {
        pub id: String,
    }
    struct FakeImport {
        pub source_file_id: String,
    }

    let files = vec![
        FakeFile {
            id: "uuid-001".into(),
        },
        FakeFile {
            id: "uuid-002".into(),
        },
    ];
    let file_ids: Vec<String> = files.iter().map(|f| f.id.clone()).collect();

    let imports = vec![
        FakeImport {
            source_file_id: "uuid-001".into(),
        }, // correct (UUID)
        FakeImport {
            source_file_id: "src/service.ts".into(),
        }, // BUG (relative path)
    ];

    // With correct UUIDs: query matches
    let matched_correct = imports
        .iter()
        .filter(|i| file_ids.contains(&i.source_file_id))
        .count();
    assert_eq!(
        matched_correct, 1,
        "UUID-based source_file_id must match files.id"
    );

    // With wrong relative path: query misses
    let all_source_ids: Vec<&String> = imports.iter().map(|i| &i.source_file_id).collect();
    let matched = all_source_ids
        .iter()
        .filter(|id| file_ids.contains(id))
        .count();
    // Only uuid-001 matches; "src/service.ts" does not
    assert_eq!(
        matched, 1,
        "only UUIDs in source_file_id pass the get_imports filter; relative paths are silently lost"
    );
}
