//! GraphService integration tests — TDD RED phase for PR-4.
//!
//! These tests define the contracts for `GraphService` before implementation.
//! All tests should FAIL on the first run and turn GREEN once GraphService
//! is implemented correctly.
//!
//! # Contracts (from design.md AD-3)
//!
//! - `get_graph`: cache hit returns cached JSON; miss builds fresh graph from DB
//! - `get_node_details`: returns FileInfo by node_id
//! - `get_node_outline`: returns cached outline; on-demand fallback if empty
//! - `search_nodes`: searches files by name, returns GraphNode list

use engine::models::{FileInfo, OutlineItem, ScanStatus};
use engine::ports::AppStatePort;
use engine::services::GraphService;
use std::sync::Mutex;

// ─────────────────────────────────────────────────────────────────────────────
// Mock implementations — no DB, pure test doubles
// ─────────────────────────────────────────────────────────────────────────────

/// Mock ScanRepository for tests that don't need file lookups.
/// Returns Ok(None) for get_file_by_id so GraphService returns FileNotFound.
struct NoOpScanRepo;
impl engine::ports::ScanRepository for NoOpScanRepo {
    fn save_scan_result(&self, _result: &engine::models::ScanResult) -> engine::Result<()> {
        Ok(())
    }
    fn get_project_by_path(
        &self,
        _root_path: &str,
    ) -> engine::Result<Option<engine::models::ProjectMeta>> {
        Ok(None)
    }
    fn get_project(&self, _project_id: &str) -> engine::Result<Option<(String, String, i64)>> {
        Ok(None)
    }
    fn get_files(&self, _project_id: &str) -> engine::Result<Vec<FileInfo>> {
        Ok(vec![])
    }
    fn get_imports(&self, _project_id: &str) -> engine::Result<Vec<engine::models::ImportInfo>> {
        Ok(vec![])
    }
    fn save_import(&self, _import: &engine::models::ImportInfo) -> engine::Result<()> {
        Ok(())
    }
    fn get_file_by_id(&self, _file_id: &str) -> engine::Result<Option<FileInfo>> {
        Ok(None)
    }
    fn save_outline_items(&self, _file_id: &str, _items: &[OutlineItem]) -> engine::Result<()> {
        Ok(())
    }
    fn get_outline_items(&self, _file_id: &str) -> engine::Result<Vec<OutlineItem>> {
        Ok(vec![])
    }
    fn get_scan_status(&self, _project_id: &str) -> engine::Result<Option<ScanStatus>> {
        Ok(None)
    }
    fn cancel(&self, _project_id: &str) -> engine::Result<()> {
        Ok(())
    }
}

/// Mock AppStatePort for tests that don't need to verify state transitions.
struct NoOpAppState;
impl AppStatePort for NoOpAppState {
    fn get_scan_status(&self) -> engine::Result<ScanStatus> {
        Ok(ScanStatus::Ready)
    }
    fn set_scan_status(&self, _status: ScanStatus) -> engine::Result<()> {
        Ok(())
    }
    fn get_ai_config(&self) -> engine::Result<Option<engine::models::AIConfig>> {
        Ok(None)
    }
    fn set_ai_config(&self, _config: engine::models::AIConfig) -> engine::Result<()> {
        Ok(())
    }
    fn get_project_root(&self) -> engine::Result<String> {
        Ok(String::new())
    }
    fn set_project_root(&self, _path: &str) -> engine::Result<()> {
        Ok(())
    }
}

/// Test double for GraphRepository that captures calls and returns controlled data.
struct RecordingGraphRepo {
    pub graph_cache: Mutex<Option<String>>,
    pub save_graph_calls: Mutex<Vec<(String, String)>>,
    pub get_outline_calls: Mutex<Vec<String>>,
    pub outline_items: Mutex<Vec<OutlineItem>>,
    pub files: Mutex<Vec<FileInfo>>,
    pub project_root: Mutex<Option<String>>,
}

impl RecordingGraphRepo {
    fn new() -> Self {
        Self {
            graph_cache: Mutex::new(None),
            save_graph_calls: Mutex::new(Vec::new()),
            get_outline_calls: Mutex::new(Vec::new()),
            outline_items: Mutex::new(Vec::new()),
            files: Mutex::new(Vec::new()),
            project_root: Mutex::new(None),
        }
    }
}

impl engine::ports::GraphRepository for RecordingGraphRepo {
    fn save_graph_cache(&self, project_id: &str, graph_json: &str) -> engine::Result<()> {
        self.save_graph_calls
            .lock()
            .unwrap()
            .push((project_id.to_string(), graph_json.to_string()));
        Ok(())
    }
    fn get_graph_cache(&self, project_id: &str) -> engine::Result<Option<String>> {
        let cache = self.graph_cache.lock().unwrap().clone();
        tracing::debug!("get_graph_cache({}) = {:?}", project_id, cache.is_some());
        Ok(cache)
    }
    fn search_files(
        &self,
        project_id: &str,
        query: &str,
        limit: usize,
    ) -> engine::Result<Vec<FileInfo>> {
        let all = self.files.lock().unwrap().clone();
        let results: Vec<FileInfo> = all
            .into_iter()
            .filter(|f| f.name.to_lowercase().contains(&query.to_lowercase()))
            .take(limit)
            .collect();
        tracing::debug!(
            "search_files({project_id}, {query}, {limit}) = {} results",
            results.len()
        );
        Ok(results)
    }
    fn get_project_root_for_file(&self, _file_id: &str) -> engine::Result<Option<String>> {
        Ok(self.project_root.lock().unwrap().clone())
    }
    fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> engine::Result<()> {
        tracing::debug!("save_outline_items({file_id}, {} items)", items.len());
        Ok(())
    }
    fn get_outline_items(&self, file_id: &str) -> engine::Result<Vec<OutlineItem>> {
        self.get_outline_calls
            .lock()
            .unwrap()
            .push(file_id.to_string());
        Ok(self.outline_items.lock().unwrap().clone())
    }
    fn get_dependencies(&self, _node_id: &str) -> engine::Result<Vec<engine::models::NodeRef>> {
        Ok(vec![])
    }
    fn get_dependents(&self, _node_id: &str) -> engine::Result<Vec<engine::models::NodeRef>> {
        Ok(vec![])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.1 — get_graph returns cached graph JSON when cache hit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_graph_returns_cached_graph_when_cache_hit() {
    let repo = RecordingGraphRepo::new();
    // GraphData uses camelCase serialization: projectId, generatedAt
    let cached_json = r#"{"nodes": [{"id":"n1","label":"A.ts","path":"src/A.ts","type":"unknown","symbolCount":1,"position":null}],"edges": [],"projectId":"p1","generatedAt":"2026-06-01T00:00:00Z"}"#;
    repo.graph_cache
        .lock()
        .unwrap()
        .replace(cached_json.to_string());

    let scan_repo = NoOpScanRepo;
    let state = NoOpAppState;
    let service = GraphService::new(repo, scan_repo, state);

    let result = service.get_graph("p1");

    assert!(
        result.is_ok(),
        "get_graph should succeed, got: {:?}",
        result
    );
    let graph = result.unwrap();
    assert_eq!(graph.nodes.len(), 1, "cached graph should have 1 node");
    assert_eq!(graph.nodes[0].id, "n1");
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.2 — get_graph builds fresh graph when cache miss (integration)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_graph_builds_fresh_graph_on_cache_miss() {
    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_status = std::sync::Arc::new(Mutex::new(ScanStatus::Idle));
    let ai_config = std::sync::Arc::new(Mutex::new(None));
    let project_root = std::sync::Arc::new(Mutex::new(String::new()));

    let scan_repo_adapter = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state_scan =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let scan_service = engine::services::ScanService::new(scan_repo_adapter, app_state_scan);

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("A.ts"), "export const a = 1;").ok();
    std::fs::write(tmp.path().join("B.ts"), "export const b = 2;").ok();
    let root = tmp.path().to_string_lossy().as_ref().to_string();

    let scan_result = scan_service
        .scan_project(&root)
        .expect("scan should succeed");
    let project_id = scan_result.project_id.clone();

    let graph_repo = engine::ports::GraphRepositoryAdapter::new(&pool);
    let scan_repo = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state_graph =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let service = GraphService::new(graph_repo, scan_repo, app_state_graph);

    let result = service.get_graph(&project_id);
    assert!(
        result.is_ok(),
        "get_graph should build on cache miss, got: {:?}",
        result
    );
    let graph = result.unwrap();
    assert!(
        !graph.nodes.is_empty(),
        "fresh graph should have nodes from scan"
    );
    assert!(
        graph.nodes.iter().all(|n| !n.id.is_empty()),
        "all nodes should have ids"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.3 — get_graph returns error when project has no files
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_graph_returns_error_when_no_files() {
    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let graph_repo = engine::ports::GraphRepositoryAdapter::new(&pool);
    let scan_repo = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state = engine::ports::AppStatePortAdapter::new(
        Mutex::new(ScanStatus::Idle),
        Mutex::new(None),
        Mutex::new(String::new()),
    );

    let service = GraphService::new(graph_repo, scan_repo, app_state);

    let result = service.get_graph("nonexistent-project-id");
    assert!(result.is_err(), "get_graph should error on missing project");
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.4 — get_node_details returns FileInfo for a node_id
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_node_details_returns_file_info() {
    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_status = std::sync::Arc::new(Mutex::new(ScanStatus::Idle));
    let ai_config = std::sync::Arc::new(Mutex::new(None));
    let project_root = std::sync::Arc::new(Mutex::new(String::new()));

    let scan_repo_adapter = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state_scan =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let scan_service = engine::services::ScanService::new(scan_repo_adapter, app_state_scan);

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("Main.ts"), "export const x = 1;").ok();
    let root = tmp.path().to_string_lossy().as_ref().to_string();
    let scan_result = scan_service
        .scan_project(&root)
        .expect("scan should succeed");

    let file_id = scan_result
        .files
        .first()
        .expect("at least one file")
        .id
        .clone();

    let graph_repo = engine::ports::GraphRepositoryAdapter::new(&pool);
    let scan_repo = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state_graph =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let service = GraphService::new(graph_repo, scan_repo, app_state_graph);

    let result = service.get_node_details(&file_id);
    assert!(
        result.is_ok(),
        "get_node_details should succeed, got: {:?}",
        result
    );
    let file_info = result.unwrap();
    assert_eq!(file_info.name, "Main.ts");
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.5 — get_node_details returns error for unknown node_id
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_node_details_returns_error_for_unknown_node() {
    let repo = RecordingGraphRepo::new();
    let scan_repo = NoOpScanRepo;
    let state = NoOpAppState;
    let service = GraphService::new(repo, scan_repo, state);

    let result = service.get_node_details("nonexistent-node-id");
    assert!(
        result.is_err(),
        "get_node_details should error on missing node"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.6 — get_node_outline returns cached outline
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_node_outline_returns_cached_outline() {
    let repo = RecordingGraphRepo::new();
    let scan_repo = NoOpScanRepo;
    let state = NoOpAppState;
    let service = GraphService::new(repo, scan_repo, state);

    let result = service.get_node_outline("some-file-id", None);
    assert!(
        result.is_ok(),
        "get_node_outline should succeed, got: {:?}",
        result
    );
    let outline = result.unwrap();
    // RecordingGraphRepo has no cached outline items
    assert!(
        outline.is_empty(),
        "RecordingGraphRepo has no cached outline"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.7 — search_nodes returns matching files as GraphNode list
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn search_nodes_returns_matching_files() {
    let repo = RecordingGraphRepo::new();
    let scan_repo = NoOpScanRepo;
    let state = NoOpAppState;
    let service = GraphService::new(repo, scan_repo, state);

    let result = service.search_nodes("p1", "Main", None);
    assert!(
        result.is_ok(),
        "search_nodes should succeed, got: {:?}",
        result
    );
    let nodes = result.unwrap();
    assert!(
        nodes.is_empty(),
        "RecordingGraphRepo has no files, so empty result"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.8 — search_nodes with limit respects the limit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn search_nodes_respects_limit() {
    let repo = RecordingGraphRepo::new();
    repo.files.lock().unwrap().extend(vec![
        FileInfo {
            id: "f1".into(),
            path: "src/Component1.ts".into(),
            name: "Component1.ts".into(),
            extension: "ts".into(),
            symbols: vec![],
            lines: 10,
        },
        FileInfo {
            id: "f2".into(),
            path: "src/Component2.ts".into(),
            name: "Component2.ts".into(),
            extension: "ts".into(),
            symbols: vec![],
            lines: 10,
        },
        FileInfo {
            id: "f3".into(),
            path: "src/Component3.ts".into(),
            name: "Component3.ts".into(),
            extension: "ts".into(),
            symbols: vec![],
            lines: 10,
        },
    ]);

    let scan_repo = NoOpScanRepo;
    let state = NoOpAppState;
    let service = GraphService::new(repo, scan_repo, state);

    let result = service.search_nodes("p1", "Component", Some(2));
    assert!(result.is_ok());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 2, "search should respect limit of 2");
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.9 — get_graph transitions AppStatePort through BuildingGraph → Ready
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_graph_transitions_status_through_building_graph_to_ready() {
    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_status = std::sync::Arc::new(Mutex::new(ScanStatus::Idle));
    let ai_config = std::sync::Arc::new(Mutex::new(None));
    let project_root = std::sync::Arc::new(Mutex::new(String::new()));

    let scan_repo_adapter = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state_scan =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let scan_service = engine::services::ScanService::new(scan_repo_adapter, app_state_scan);

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("Test.ts"), "export const test = 1;").ok();
    let root = tmp.path().to_string_lossy().as_ref().to_string();
    let scan_result = scan_service
        .scan_project(&root)
        .expect("scan should succeed");
    let project_id = scan_result.project_id.clone();

    // Reset status to simulate starting from idle
    let app_state_reset =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    app_state_reset.set_scan_status(ScanStatus::Idle).unwrap();

    let graph_repo = engine::ports::GraphRepositoryAdapter::new(&pool);
    let scan_repo = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state_graph =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let service = GraphService::new(graph_repo, scan_repo, app_state_graph);

    let result = service.get_graph(&project_id);
    assert!(result.is_ok());

    let app_state_check =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let final_status = app_state_check.get_scan_status().unwrap();
    assert_eq!(
        final_status,
        ScanStatus::Ready,
        "status should be Ready after get_graph"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T10.10 — get_graph sets error status when no files in DB
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_graph_sets_error_status_when_no_files() {
    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_status = std::sync::Arc::new(Mutex::new(ScanStatus::Idle));
    let ai_config = std::sync::Arc::new(Mutex::new(None));
    let project_root = std::sync::Arc::new(Mutex::new(String::new()));

    let graph_repo = engine::ports::GraphRepositoryAdapter::new(&pool);
    let scan_repo = engine::ports::ScanRepositoryAdapter::new(&pool);
    let app_state =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let service = GraphService::new(graph_repo, scan_repo, app_state);

    let result = service.get_graph("totally-nonexistent-project");
    assert!(result.is_err());

    let app_state_check =
        engine::ports::AppStatePortAdapter::from_arc_refs(&scan_status, &ai_config, &project_root);
    let final_status = app_state_check.get_scan_status().unwrap();
    assert_eq!(
        final_status,
        ScanStatus::Error,
        "status should be Error when no files found"
    );
}
