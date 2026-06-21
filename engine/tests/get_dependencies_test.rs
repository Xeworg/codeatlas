//! Tests for C1.3: GraphService::get_dependencies with 3 outcomes.
//!
//! RED PHASE: These tests define the expected behavior.

use engine::models::AIConfig;
use engine::models::{FileInfo, NodeRef, ScanStatus};
use engine::ports::hexagonal::FileSourceReader;
use engine::ports::{AppStatePort, GraphRepository, ScanRepository};
use engine::services::GraphService;
use std::collections::HashMap;
use std::path::Path;

/// Mock file reader that returns empty content on every read.
struct MockFileSrc(&'static str);

impl FileSourceReader for MockFileSrc {
    fn read_source(&self, _path: &Path) -> std::io::Result<String> {
        Ok(self.0.to_string())
    }
}

fn mock_file_reader() -> impl FileSourceReader {
    MockFileSrc("")
}

/// Mock GraphRepository for testing get_dependencies and get_dependents.
struct MockGraphRepo {
    /// Maps node_id -> list of NodeRef dependencies (outgoing edges).
    dependencies: std::sync::Mutex<HashMap<String, Vec<NodeRef>>>,
    /// Maps node_id -> list of NodeRef dependents (incoming edges).
    dependents: std::sync::Mutex<HashMap<String, Vec<NodeRef>>>,
}

impl MockGraphRepo {
    fn new() -> Self {
        Self {
            dependencies: std::sync::Mutex::new(HashMap::new()),
            dependents: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn with_dependencies(node_id: &str, deps: Vec<NodeRef>) -> Self {
        let mut map = HashMap::new();
        map.insert(node_id.to_string(), deps);
        Self {
            dependencies: std::sync::Mutex::new(map),
            dependents: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn with_dependents(node_id: &str, deps: Vec<NodeRef>) -> Self {
        let mut map = HashMap::new();
        map.insert(node_id.to_string(), deps);
        Self {
            dependencies: std::sync::Mutex::new(HashMap::new()),
            dependents: std::sync::Mutex::new(map),
        }
    }
}

impl GraphRepository for MockGraphRepo {
    fn save_graph_cache(&self, _project_id: &str, _graph_json: &str) -> engine::Result<()> {
        unimplemented!()
    }

    fn get_graph_cache(&self, _project_id: &str) -> engine::Result<Option<String>> {
        unimplemented!()
    }

    fn search_files(
        &self,
        _project_id: &str,
        _query: &str,
        _limit: usize,
    ) -> engine::Result<Vec<FileInfo>> {
        unimplemented!()
    }

    fn get_project_root_for_file(&self, _file_id: &str) -> engine::Result<Option<String>> {
        unimplemented!()
    }

    fn save_outline_items(
        &self,
        _file_id: &str,
        _items: &[engine::models::OutlineItem],
    ) -> engine::Result<()> {
        unimplemented!()
    }

    fn get_outline_items(
        &self,
        _file_id: &str,
    ) -> engine::Result<Vec<engine::models::OutlineItem>> {
        Ok(vec![])
    }

    fn get_dependencies(&self, node_id: &str) -> engine::Result<Vec<NodeRef>> {
        let deps = self.dependencies.lock().unwrap();
        Ok(deps.get(node_id).cloned().unwrap_or_else(Vec::new))
    }

    fn get_dependents(&self, node_id: &str) -> engine::Result<Vec<NodeRef>> {
        let deps = self.dependents.lock().unwrap();
        Ok(deps.get(node_id).cloned().unwrap_or_else(Vec::new))
    }
}

/// Mock ScanRepository that tracks which nodes exist.
struct MockScanRepo {
    /// Set of known node IDs (files).
    known_nodes: std::sync::Mutex<HashMap<String, bool>>,
}

impl MockScanRepo {
    fn new() -> Self {
        Self {
            known_nodes: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn with_node(node_id: &str) -> Self {
        let mut map = HashMap::new();
        map.insert(node_id.to_string(), true);
        Self {
            known_nodes: std::sync::Mutex::new(map),
        }
    }
}

impl ScanRepository for MockScanRepo {
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
    fn get_file_by_id(&self, file_id: &str) -> engine::Result<Option<FileInfo>> {
        let known = self.known_nodes.lock().unwrap();
        if known.contains_key(file_id) {
            Ok(Some(FileInfo {
                id: file_id.to_string(),
                path: format!("/src/{}.ts", file_id),
                name: format!("{}.ts", file_id),
                extension: "ts".to_string(),
                symbols: vec![],
                lines: 10,
            }))
        } else {
            Ok(None)
        }
    }
    fn save_outline_items(
        &self,
        _file_id: &str,
        _items: &[engine::models::OutlineItem],
    ) -> engine::Result<()> {
        Ok(())
    }
    fn get_outline_items(
        &self,
        _file_id: &str,
    ) -> engine::Result<Vec<engine::models::OutlineItem>> {
        Ok(vec![])
    }
    fn get_scan_status(&self, _project_id: &str) -> engine::Result<Option<ScanStatus>> {
        Ok(None)
    }
    fn cancel(&self, _project_id: &str) -> engine::Result<()> {
        Ok(())
    }
}

/// Mock AppStatePort for tests.
struct MockAppState;

impl AppStatePort for MockAppState {
    fn get_scan_status(&self) -> engine::Result<ScanStatus> {
        Ok(ScanStatus::Idle)
    }
    fn set_scan_status(&self, _status: ScanStatus) -> engine::Result<()> {
        Ok(())
    }
    fn get_ai_config(&self) -> engine::Result<Option<AIConfig>> {
        Ok(None)
    }
    fn set_ai_config(&self, _config: AIConfig) -> engine::Result<()> {
        Ok(())
    }
    fn get_project_root(&self) -> engine::Result<String> {
        Ok(String::new())
    }
    fn set_project_root(&self, _root: &str) -> engine::Result<()> {
        Ok(())
    }
}

/// Test: Known node with dependencies returns non-empty list.
#[tokio::test]
async fn get_dependencies_returns_deps_for_known_node() {
    let deps = vec![
        NodeRef::new("b-id".into(), "/src/b.ts".into(), "./b".into(), vec![]),
        NodeRef::new("c-id".into(), "/src/c.ts".into(), "./c".into(), vec![]),
    ];
    let graph_repo = MockGraphRepo::with_dependencies("a-id", deps);
    let scan_repo = MockScanRepo::with_node("a-id");
    let state = MockAppState;
    let service = GraphService::new(graph_repo, scan_repo, state, mock_file_reader());

    let result = service.get_dependencies("a-id").await;

    assert!(
        result.is_ok(),
        "get_dependencies should return Ok for known node"
    );
    let deps = result.unwrap();
    assert_eq!(deps.len(), 2, "node A should have 2 dependencies");
    assert_eq!(deps[0].id, "b-id");
    assert_eq!(deps[1].id, "c-id");
}

/// Test: Known node with no dependencies returns empty list.
#[tokio::test]
async fn get_dependencies_returns_empty_for_node_with_no_deps() {
    let graph_repo = MockGraphRepo::new(); // no deps for any node
    let scan_repo = MockScanRepo::with_node("x-id"); // but x-id is known
    let state = MockAppState;
    let service = GraphService::new(graph_repo, scan_repo, state, mock_file_reader());

    let result = service.get_dependencies("x-id").await;

    assert!(
        result.is_ok(),
        "get_dependencies should return Ok for known node"
    );
    assert!(
        result.unwrap().is_empty(),
        "node with no deps should return empty list"
    );
}

/// Test: Unknown node returns NotFound error.
#[tokio::test]
async fn get_dependencies_returns_not_found_for_unknown_node() {
    let graph_repo = MockGraphRepo::new();
    let scan_repo = MockScanRepo::new(); // no known nodes
    let state = MockAppState;
    let service = GraphService::new(graph_repo, scan_repo, state, mock_file_reader());

    let result = service.get_dependencies("ghost-node").await;

    assert!(
        result.is_err(),
        "get_dependencies should return Err for unknown node"
    );
    assert!(matches!(result.unwrap_err(), engine::AppError::NotFound(_)));
}

/// Test: Known node with dependents returns non-empty list.
#[tokio::test]
async fn get_dependents_returns_deps_for_known_node() {
    let deps = vec![
        NodeRef::new("a-id".into(), "/src/a.ts".into(), "./a".into(), vec![]),
        NodeRef::new("b-id".into(), "/src/b.ts".into(), "./b".into(), vec![]),
    ];
    let graph_repo = MockGraphRepo::with_dependents("c-id", deps);
    let scan_repo = MockScanRepo::with_node("c-id");
    let state = MockAppState;
    let service = GraphService::new(graph_repo, scan_repo, state, mock_file_reader());

    let result = service.get_dependents("c-id").await;

    assert!(
        result.is_ok(),
        "get_dependents should return Ok for known node"
    );
    let deps = result.unwrap();
    assert_eq!(deps.len(), 2, "node C should have 2 dependents");
    assert_eq!(deps[0].id, "a-id");
    assert_eq!(deps[1].id, "b-id");
}

/// Test: Known node with no dependents returns empty list.
#[tokio::test]
async fn get_dependents_returns_empty_for_node_with_no_deps() {
    let graph_repo = MockGraphRepo::new(); // no dependents for any node
    let scan_repo = MockScanRepo::with_node("x-id"); // but x-id is known
    let state = MockAppState;
    let service = GraphService::new(graph_repo, scan_repo, state, mock_file_reader());

    let result = service.get_dependents("x-id").await;

    assert!(
        result.is_ok(),
        "get_dependents should return Ok for known node"
    );
    assert!(
        result.unwrap().is_empty(),
        "node with no dependents should return empty list"
    );
}

/// Test: Unknown node returns NotFound error.
#[tokio::test]
async fn get_dependents_returns_not_found_for_unknown_node() {
    let graph_repo = MockGraphRepo::new();
    let scan_repo = MockScanRepo::new(); // no known nodes
    let state = MockAppState;
    let service = GraphService::new(graph_repo, scan_repo, state, mock_file_reader());

    let result = service.get_dependents("ghost-node").await;

    assert!(
        result.is_err(),
        "get_dependents should return Err for unknown node"
    );
    assert!(matches!(result.unwrap_err(), engine::AppError::NotFound(_)));
}
