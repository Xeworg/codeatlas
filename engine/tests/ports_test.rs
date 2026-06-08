//! Ports integration tests — PR-2: Verify canonical ports exist and are usable.
//!
//! These tests confirm that the four wave-1 ports are available and can be
//! implemented by concrete adapters derived from the existing `ProjectRepository`.
//!
//! T5 RED phase: these tests MUST fail until ports.rs is implemented.
//! T6 GREEN phase: ports.rs created with trait definitions.
//! T7 GREEN phase: adapter impls wired up so these tests pass.

use std::sync::Mutex;

use engine::db::DbPool;
use engine::models::{FileInfo, ScanResult, ScanStatus, SymbolInfo, SymbolKind};

// Re-export the canonical ports for compile-time verification.
// If these don't exist, compilation fails — RED test confirms the gap.
use engine::ports::{AppStatePort, GraphRepository, ScanRepository, WorkspaceRepository};

/// T5.1 — Verify ScanRepository trait exists and can be implemented by ProjectRepository adapter.
#[test]
fn scan_repository_trait_is_defined() {
    // Trait must exist. If `use engine::ports::*` above compiled, the trait exists.
    // This test is a compile-time assertion; if it compiles, the trait is defined.
    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    // Verify we can create the adapter via the trait object or explicit impl.
    // The concrete adapter type `ScanRepositoryAdapter` must exist in ports module.
    use engine::ports::ScanRepositoryAdapter;
    let _ = ScanRepositoryAdapter::new(&pool);
}

/// T5.2 — Verify GraphRepository trait exists.
#[test]
fn graph_repository_trait_is_defined() {
    // Compile-time assertion: trait must exist.
    // If this compiles, the trait is defined with at least the canonical methods.
    use engine::ports::GraphRepositoryAdapter;
    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    let _ = GraphRepositoryAdapter::new(&pool);
}

/// T5.3 — Verify WorkspaceRepository trait exists.
#[test]
fn workspace_repository_trait_is_defined() {
    use engine::ports::WorkspaceRepositoryAdapter;
    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    let _ = WorkspaceRepositoryAdapter::new(&pool);
}

/// T5.4 — Verify AppStatePort trait exists and can wrap the concrete AppState.
#[test]
fn app_state_port_trait_is_defined() {
    use engine::ports::AppStatePortAdapter;
    use std::sync::Mutex;

    let scan_status = Mutex::new(engine::models::ScanStatus::Idle);
    let ai_config = Mutex::new(None::<engine::models::AIConfig>);
    let project_root = Mutex::new(String::new());

    let _ = AppStatePortAdapter::new(scan_status, ai_config, project_root);
}

/// T5.5 — Verify ScanRepository::save_scan_result and ::get_project_by_path.
/// These methods must exist on the trait — failure confirms the gap.
#[test]
fn scan_repository_save_and_retrieve() {
    use engine::ports::ScanRepositoryAdapter;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    let repo = ScanRepositoryAdapter::new(&pool);

    let result = ScanResult {
        project_id: "port-test-proj".into(),
        project_name: "PortTest".into(),
        root_path: "/tmp/port-test".into(),
        files_count: 1,
        symbols_count: 1,
        imports_count: 0,
        files: vec![FileInfo {
            id: "port-file-1".into(),
            path: "src/main.ts".into(),
            name: "main.ts".into(),
            extension: "ts".into(),
            symbols: vec![SymbolInfo {
                id: "sym-port-1".into(),
                name: "mainFn".into(),
                kind: SymbolKind::Function,
                file_id: "port-file-1".into(),
                line_start: 1,
                line_end: 5,
                exports: true,
            }],
            lines: 5,
        }],
        scan_duration_ms: 100,
        status: ScanStatus::Ready,
        error: None,
    };

    repo.save_scan_result(&result).unwrap();

    // Verify we can retrieve by path
    let meta = repo.get_project_by_path("/tmp/port-test").unwrap();
    assert!(meta.is_some());
    assert_eq!(meta.unwrap().project_id, "port-test-proj");
}

/// T5.6 — Verify GraphRepository::get_graph_cache and ::save_graph_cache.
#[test]
fn graph_repository_cache_operations() {
    use engine::ports::GraphRepositoryAdapter;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    let repo = GraphRepositoryAdapter::new(&pool);

    repo.save_graph_cache("graph-test-proj", r#"{"nodes":[],"edges":[]}"#)
        .unwrap();

    let cached = repo.get_graph_cache("graph-test-proj").unwrap();
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), r#"{"nodes":[],"edges":[]}"#);
}

/// T5.7 — Verify WorkspaceRepository::create_workspace and ::list_workspaces.
#[test]
fn workspace_repository_create_and_list() {
    use engine::ports::WorkspaceRepositoryAdapter;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    let repo = WorkspaceRepositoryAdapter::new(&pool);

    let (id, name, created) = repo.create_workspace("PortWorkspace").unwrap();
    assert!(!id.is_empty());
    assert_eq!(name, "PortWorkspace");
    assert!(!created.is_empty());

    let workspaces = repo.list_workspaces().unwrap();
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0].1, "PortWorkspace");
}

/// T5.8 — Verify AppStatePort can read and write scan status.
#[test]
fn app_state_port_scan_status() {
    use engine::ports::AppStatePortAdapter;
    use std::sync::Mutex;

    let scan_status = Mutex::new(engine::models::ScanStatus::Idle);
    let ai_config = Mutex::new(None::<engine::models::AIConfig>);
    let project_root = Mutex::new(String::new());

    let state = AppStatePortAdapter::new(scan_status, ai_config, project_root);

    state
        .set_scan_status(engine::models::ScanStatus::Scanning)
        .unwrap();
    let status = state.get_scan_status().unwrap();
    assert!(matches!(status, engine::models::ScanStatus::Scanning));

    state
        .set_scan_status(engine::models::ScanStatus::Ready)
        .unwrap();
    let status = state.get_scan_status().unwrap();
    assert!(matches!(status, engine::models::ScanStatus::Ready));
}

/// T5.9 — Verify AppStatePort can store and retrieve project_root.
#[test]
fn app_state_port_project_root() {
    use engine::ports::AppStatePortAdapter;
    use std::sync::Mutex;

    let scan_status = Mutex::new(engine::models::ScanStatus::Idle);
    let ai_config = Mutex::new(None::<engine::models::AIConfig>);
    let project_root = Mutex::new(String::new());

    let state = AppStatePortAdapter::new(scan_status, ai_config, project_root);

    state.set_project_root("/tmp/test-root").unwrap();
    let root = state.get_project_root().unwrap();
    assert_eq!(root, "/tmp/test-root");
}

/// T5.10 — Verify adapters implement the Send + Sync safety guarantees required
/// for Tauri's multi-threaded runtime.
#[test]
fn adapters_are_send_and_sync() {
    use engine::ports::{
        AppStatePortAdapter, GraphRepositoryAdapter, ScanRepositoryAdapter,
        WorkspaceRepositoryAdapter,
    };

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_repo = ScanRepositoryAdapter::new(&pool);
    assert_send_sync(&scan_repo);

    let graph_repo = GraphRepositoryAdapter::new(&pool);
    assert_send_sync(&graph_repo);

    let ws_repo = WorkspaceRepositoryAdapter::new(&pool);
    assert_send_sync(&ws_repo);

    let scan_status = Mutex::new(engine::models::ScanStatus::Idle);
    let ai_config = Mutex::new(None::<engine::models::AIConfig>);
    let project_root = Mutex::new(String::new());
    let app_state = AppStatePortAdapter::new(scan_status, ai_config, project_root);
    assert_send_sync(&app_state);
}

/// T5.11 — Verify AppStatePortAdapter::from_guards mutates the REAL AppState,
/// not dead copies.
///
/// This is the canonical shared-state regression test: mutations through the
/// adapter must be visible in the original mutexes that AppState owns.
/// If from_guards clones into new independent mutexes (the original bug),
/// this test FAILS because the original mutexes are never touched.
#[test]
fn app_state_port_adapter_from_guards_mutates_real_state() {
    use engine::ports::AppStatePortAdapter;
    use std::sync::{Arc, Mutex};

    // Simulate the real AppState mutexes (owned by State<AppState>)
    let real_scan_status = Arc::new(Mutex::new(engine::models::ScanStatus::Idle));
    let real_ai_config = Arc::new(Mutex::new(None::<engine::models::AIConfig>));
    let real_project_root = Arc::new(Mutex::new(String::new()));

    // Create adapter via from_guards (takes Arc refs to the real mutexes)
    let adapter =
        AppStatePortAdapter::from_arc_refs(&real_scan_status, &real_ai_config, &real_project_root);

    // Mutate state through the adapter
    adapter
        .set_scan_status(engine::models::ScanStatus::Scanning)
        .unwrap();
    adapter.set_project_root("/real/path").unwrap();

    // RED assertion: mutations MUST be visible in the original Arc<Mutex> handles.
    // If from_arc_refs creates independent copies, these assertions FAIL.
    let status_val = *real_scan_status.lock().unwrap();
    assert!(
        matches!(status_val, engine::models::ScanStatus::Scanning),
        "from_arc_refs must mutate the real scan_status, not a copy"
    );

    let root_val = real_project_root.lock().unwrap().clone();
    assert_eq!(
        root_val, "/real/path",
        "from_arc_refs must mutate the real project_root, not a copy"
    );
}

fn assert_send_sync<T: Send + Sync>(_val: &T) {}
