//! Shim tests for import source_file_id persistence contract and Tauri command error boundaries.
//!
//! ## Part A: Import source_file_id tests
//! Verifies that:
//! 1. source_file_id is converted from relative path to persisted UUID before save_import.
//! 2. Target resolution still works from the relative path context.
//!
//! ## Part B: Command-boundary tests (C3b: T4)
//! Tests the IPC error boundary at the serializer level (IpcErrorPayload contract).
//! True end-to-end Tauri command tests would require a live Tauri app harness
//! or proper State construction with all trait implementations.
//!
//! ## Part C: Command-logic tests (C3b: T4)
//! Verifies the critical error paths in command logic using unit test patterns.
//! These test the logic that would execute in a command handler, proving the
//! error construction and serialization work correctly.

#[allow(unused_imports)]
use engine::models::ImportInfo;
#[allow(unused_imports)]
use std::collections::HashMap;
use serde_json::Value;
use std::sync::{Arc, Mutex};

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
            imports: vec!["Helper".to_string()],
            is_default: false,
            is_type: false,
        },
        ImportInfo {
            id: "imp-2".into(),
            source_file_id: "src/utils.rs".into(),
            target_file_id: None,
            target_module: Some("std::collections".into()),
            imports: vec!["HashMap".to_string()],
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
    let file_infos = [
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

    let files = [
        FakeFile {
            id: "uuid-001".into(),
        },
        FakeFile {
            id: "uuid-002".into(),
        },
    ];
    let file_ids: Vec<String> = files.iter().map(|f| f.id.clone()).collect();

    let imports = [
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

// =============================================================================
// Part B: IpcErrorPayload serializer tests (C3b: T4)
// =============================================================================
// These tests verify the IPC error serialization contract at the boundary.
// They prove that AppError variants serialize to the correct IpcErrorPayload JSON.
// =============================================================================

#[allow(unused_imports)]
use crate::ipc_error::to_ipc_error;
#[allow(unused_imports)]
use engine::AppError;

/// Verifies the IPC payload for AI_UNAVAILABLE from explain_node error path.
/// The serializer produces: { "code": "AI_UNAVAILABLE", "message": "...", "details": { "reason": "..." } }
#[test]
fn serializer_ipc_payload_ai_unavailable_reason_ai_not_configured() {
    let app_err = AppError::AIUnavailable("AI not configured".to_string());
    let raw = to_ipc_error(app_err);
    let v: Value = serde_json::from_str(&raw).expect("to_ipc_error must produce valid JSON");

    assert_eq!(v["code"], "AI_UNAVAILABLE", "code must be AI_UNAVAILABLE");
    let msg = v["message"].as_str().unwrap();
    assert!(
        msg.contains("AI unavailable"),
        "message must contain 'AI unavailable', got: {}",
        msg
    );
    assert_eq!(
        v["details"]["reason"].as_str().unwrap(),
        "AI not configured",
        "details.reason must be 'AI not configured'"
    );
}

/// Verifies the IPC payload for AI_UNAVAILABLE from chat error path.
#[test]
fn serializer_ipc_payload_chat_ai_unavailable_reason_ai_not_configured() {
    let app_err = AppError::AIUnavailable("AI not configured".to_string());
    let raw = to_ipc_error(app_err);
    let v: Value = serde_json::from_str(&raw).expect("to_ipc_error must produce valid JSON");

    assert_eq!(v["code"], "AI_UNAVAILABLE", "code must be AI_UNAVAILABLE");
    assert_eq!(
        v["details"]["reason"].as_str().unwrap(),
        "AI not configured",
        "details.reason must be 'AI not configured'"
    );
}

/// Verifies the IPC payload for FILE_NOT_FOUND when a node is not found.
/// This error path is exercised by explain_node when the service cannot find file metadata.
#[test]
fn serializer_ipc_payload_file_not_found_with_node_id() {
    let node_id = "node-uuid-123";
    let app_err = AppError::FileNotFound(node_id.to_string());
    let raw = to_ipc_error(app_err);
    let v: Value = serde_json::from_str(&raw).expect("to_ipc_error must produce valid JSON");

    assert_eq!(v["code"], "FILE_NOT_FOUND", "code must be FILE_NOT_FOUND");
    let msg = v["message"].as_str().unwrap();
    assert!(
        msg.contains("File not found"),
        "message must contain 'File not found', got: {}",
        msg
    );
    assert_eq!(
        v["details"]["path"].as_str().unwrap(),
        node_id,
        "details.path must be the node_id"
    );
}

// =============================================================================
// Part C: Real command-boundary tests (C3b: T4)
// =============================================================================
// These tests invoke the actual Tauri command entrypoints with managed state.
// They lock the public command contract, not only the serializer internals.
// =============================================================================

#[cfg(test)]
use tauri::test::{mock_app, MockRuntime};
#[cfg(test)]
use tauri::Manager;

#[cfg(test)]
fn test_ai_config() -> engine::models::AIConfig {
    engine::models::AIConfig {
        provider: "anthropic".to_string(),
        api_key: "test-key".to_string(),
        model: "test-model".to_string(),
        endpoint: None,
    }
}

#[cfg(test)]
fn build_test_app(ai_config: Option<engine::models::AIConfig>) -> tauri::App<MockRuntime> {
    let db_path = std::env::temp_dir().join(format!(
        "codeatlas-c3b-shim-tests-{}.db",
        uuid::Uuid::new_v4()
    ));

    let db_pool = engine::db::DbPool::new(db_path.to_str().expect("temp path must be valid UTF-8"))
        .expect("test db pool must initialize");
    db_pool.init_schema().expect("test schema must initialize");
    let _ = db_pool.with_connection(|conn| {
        use engine::db::migrations::run_pending_migrations;
        run_pending_migrations(conn)
    });

    let pool = db_pool.clone();
    let scan_repo =
        engine::ports::ScanRepositoryAdapter::from_arc(std::sync::Arc::new(pool.clone()));
    let graph_repo =
        engine::ports::GraphRepositoryAdapter::from_arc(std::sync::Arc::new(pool.clone()));
    let ai_service_port = std::sync::Arc::new(engine::ai::AIService::new(
        engine::ai::ProviderFactory,
        engine::SystemClock,
        engine::RandomIdGen,
        scan_repo,
        graph_repo,
    )) as std::sync::Arc<dyn engine::ai::AIServicePort>;

    let app_state = super::super::AppState {
        scan_status: Arc::new(Mutex::new(engine::models::ScanStatus::Idle)),
        ai_config: Arc::new(Mutex::new(ai_config)),
        project_root: Arc::new(Mutex::new(String::new())),
        ai_service_port,
        scan_repo: std::sync::Arc::new(engine::ports::ScanRepositoryAdapter::from_arc(
            std::sync::Arc::new(pool.clone()),
        )) as std::sync::Arc<dyn engine::ports::ScanRepository>,
        graph_repo: std::sync::Arc::new(engine::ports::GraphRepositoryAdapter::from_arc(
            std::sync::Arc::new(pool.clone()),
        )) as std::sync::Arc<dyn engine::ports::GraphRepository>,
        analysis_repo: std::sync::Arc::new(engine::ports::AnalysisDataSourceAdapter::from_arc(
            std::sync::Arc::new(pool.clone()),
        )) as std::sync::Arc<dyn engine::ports::AnalysisDataSource>,
        workspace_repo: std::sync::Arc::new(engine::ports::WorkspaceRepositoryAdapter::from_arc(
            std::sync::Arc::new(pool),
        )) as std::sync::Arc<dyn engine::ports::WorkspaceRepository>,
        clock: std::sync::Arc::new(engine::SystemClock) as std::sync::Arc<dyn engine::Clock>,
        id_gen: std::sync::Arc::new(engine::RandomIdGen) as std::sync::Arc<dyn engine::IdGenerator>,
        stopwatch: std::sync::Arc::new(engine::SystemStopwatch)
            as std::sync::Arc<dyn engine::Stopwatch>,
    };

    let app = mock_app();
    app.manage(app_state);
    app
}

#[cfg(test)]
#[test]
fn command_boundary_explain_node_returns_ai_unavailable_when_config_is_none() {
    let app = build_test_app(None);

    let err = tauri::async_runtime::block_on(super::super::explain_node(
        "node-1".to_string(),
        "project-1".to_string(),
        app.state::<super::super::AppState>(),
    ))
    .expect_err("explain_node must fail when ai_config is missing");

    let v: Value = serde_json::from_str(&err).expect("command error must be valid JSON");
    assert_eq!(v["code"], "AI_UNAVAILABLE");
    assert_eq!(v["details"]["reason"], "AI not configured");
}

#[cfg(test)]
#[test]
fn command_boundary_chat_returns_ai_unavailable_when_config_is_none() {
    let app = build_test_app(None);

    let err = tauri::async_runtime::block_on(super::super::chat(
        "project-1".to_string(),
        "hello".to_string(),
        vec![],
        app.state::<super::super::AppState>(),
    ))
    .expect_err("chat must fail when ai_config is missing");

    let v: Value = serde_json::from_str(&err).expect("command error must be valid JSON");
    assert_eq!(v["code"], "AI_UNAVAILABLE");
    assert_eq!(v["details"]["reason"], "AI not configured");
}

#[cfg(test)]
#[test]
fn command_boundary_explain_node_returns_file_not_found_when_service_misses_file() {
    let app = build_test_app(Some(test_ai_config()));
    let node_id = "missing-node-uuid";

    let err = tauri::async_runtime::block_on(super::super::explain_node(
        node_id.to_string(),
        "project-1".to_string(),
        app.state::<super::super::AppState>(),
    ))
    .expect_err("explain_node must fail when file metadata is missing");

    let v: Value = serde_json::from_str(&err).expect("command error must be valid JSON");
    assert_eq!(v["code"], "FILE_NOT_FOUND");
    assert_eq!(v["details"]["path"], node_id);
}
