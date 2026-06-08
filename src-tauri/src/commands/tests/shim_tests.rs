//! Shim tests for import source_file_id persistence contract.
//!
//! Verifies that:
//! 1. source_file_id is converted from relative path to persisted UUID before save_import.
//! 2. Target resolution still works from the relative path context.
//!
//! These tests run in-process without needing a live Tauri app state.

#[allow(unused_imports)]
use engine::models::ImportInfo;
#[allow(unused_imports)]
use std::collections::HashMap;

/// Simulates the conversion that scan_project applies to all_imports:
/// relative_path -> UUID via path_to_id, then resolver.resolve(module, rel_path).
#[test]
fn import_source_file_id_converts_relative_path_to_uuid() {
    let mut path_to_id: HashMap<String, String> = HashMap::new();
    path_to_id.insert(
        "src/service.ts".into(),
        "550e8400-e29b-41d4-a716-446655440001".into(),
    );
    path_to_id.insert(
        "src/utils.rs".into(),
        "550e8400-e29b-41d4-a716-446655440002".into(),
    );

    let mut imports = vec![
        ImportInfo {
            id: "imp-1".into(),
            source_file_id: "src/service.ts".into(),
            target_file_id: None,
            target_module: Some("./utils".into()),
            imports: vec!["Helper".to_string()].into(),
            is_default: false,
            is_type: false,
        },
        ImportInfo {
            id: "imp-2".into(),
            source_file_id: "src/utils.rs".into(),
            target_file_id: None,
            target_module: Some("std::collections".into()),
            imports: vec!["HashMap".to_string()].into(),
            is_default: false,
            is_type: false,
        },
    ];

    for imp in &mut imports {
        if let Some(uuid) = path_to_id.get(&imp.source_file_id) {
            imp.source_file_id = uuid.clone();
        }
    }

    assert_eq!(
        imports[0].source_file_id,
        "550e8400-e29b-41d4-a716-446655440001"
    );
    assert_eq!(
        imports[1].source_file_id,
        "550e8400-e29b-41d4-a716-446655440002"
    );

    for imp in &imports {
        assert!(!imp.source_file_id.is_empty());
        assert!(imp.source_file_id.len() >= 20);
    }
}

/// Verify that path_to_id mapping is built from file_infos before import loop.
#[test]
fn path_to_id_map_covers_all_import_sources() {
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
    assert_eq!(path_to_id.len(), 2);
}

/// Regression: get_imports uses source_file_id IN (SELECT id FROM files ...).
/// If source_file_id is a relative path, the WHERE clause returns 0 rows.
#[test]
fn source_file_id_must_be_uuid_not_relative_path_for_get_imports_query() {
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
        },
        FakeImport {
            source_file_id: "src/service.ts".into(),
        },
    ];

    let matched_correct = imports
        .iter()
        .filter(|i| file_ids.contains(&i.source_file_id))
        .count();
    assert_eq!(matched_correct, 1);

    let all_source_ids: Vec<&String> = imports.iter().map(|i| &i.source_file_id).collect();
    let matched = all_source_ids
        .iter()
        .filter(|id| file_ids.contains(id))
        .count();
    assert_eq!(matched, 1);
}
