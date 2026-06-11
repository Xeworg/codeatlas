//! ScanService integration tests — PR-3: Verify orchestration of scan commands via ports.
//!
//! These tests confirm that `ScanService` orchestrates `scan_project`,
//! `open_project_by_path`, and `get_scan_status` correctly through the
//! canonical ports (`ScanRepository`, `AppStatePort`).
//!
//! T8 RED phase: these tests MUST fail until ScanService is implemented.
//! T9 GREEN phase: ScanService created, commands thinned to shims.

use std::sync::Mutex;

use engine::models::{FileInfo, ImportInfo, OutlineItem, ProjectMeta, ScanResult, ScanStatus};
use engine::ports::{AppStatePort, ScanRepository, ScanRepositoryAdapter};
use engine::services::ScanService;

fn make_scan_service<S: ScanRepository, A: AppStatePort>(
    scan_repo: S,
    app_state: A,
) -> ScanService<S, A, engine::SystemClock, engine::RandomIdGen, engine::SystemStopwatch> {
    ScanService::new(
        scan_repo,
        app_state,
        engine::SystemClock,
        engine::RandomIdGen,
        engine::SystemStopwatch,
    )
}

/// T8.1 — Verify ScanService struct exists and has a scan_project method.
#[test]
fn scan_service_exists() {
    // This test is a compile-time assertion: if ScanService exists and
    // is generic over ScanRepository + AppStatePort, this compiles.
    // If ScanService doesn't exist, compilation fails — RED confirms the gap.
    fn _assert_scan_service_exists<
        S: ScanRepository,
        A: AppStatePort,
        C: engine::Clock,
        I: engine::IdGenerator,
        W: engine::Stopwatch,
    >(
        _: ScanService<S, A, C, I, W>,
    ) {
    }
}

/// T8.2 — Verify ScanService has open_project_by_path method.
#[test]
fn scan_service_has_open_project_by_path_method() {
    fn _assert_signature_exists<
        S: ScanRepository,
        A: AppStatePort,
        C: engine::Clock,
        I: engine::IdGenerator,
        W: engine::Stopwatch,
    >(
        _: fn(ScanService<S, A, C, I, W>, &str) -> engine::Result<ScanResult>,
    ) {
    }
}

/// T8.3 — Verify ScanService has get_scan_status method.
#[test]
fn scan_service_has_get_scan_status_method() {
    fn _assert_signature_exists<
        S: ScanRepository,
        A: AppStatePort,
        C: engine::Clock,
        I: engine::IdGenerator,
        W: engine::Stopwatch,
    >(
        _: fn(ScanService<S, A, C, I, W>) -> engine::Result<ScanStatus>,
    ) {
    }
}

/// T8.4 — scan_project: sets ScanStatus to Scanning at start, then Ready on success.
#[test]
fn scan_project_transitions_status_correctly() {
    use engine::db::DbPool;

    // Set up in-memory DB
    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_repo = ScanRepositoryAdapter::new(&pool);

    // Capture status transitions in a thread-safe vec
    let statuses: std::sync::Arc<Mutex<Vec<ScanStatus>>> =
        std::sync::Arc::new(Mutex::new(Vec::new()));

    // Create a wrapper that tracks status changes
    struct TrackingAppState {
        inner: engine::ports::AppStatePortAdapter,
        statuses: std::sync::Arc<Mutex<Vec<ScanStatus>>>,
    }

    impl TrackingAppState {
        fn new(statuses: std::sync::Arc<Mutex<Vec<ScanStatus>>>) -> Self {
            Self {
                inner: engine::ports::AppStatePortAdapter::new(
                    Mutex::new(ScanStatus::Idle),
                    Mutex::new(None),
                    Mutex::new(String::new()),
                ),
                statuses,
            }
        }
    }

    impl AppStatePort for TrackingAppState {
        fn get_scan_status(&self) -> engine::Result<ScanStatus> {
            // Don't track reads — only track writes to see state transitions.
            self.inner.get_scan_status()
        }
        fn set_scan_status(&self, status: ScanStatus) -> engine::Result<()> {
            // Track each state transition so we can verify the lifecycle.
            self.statuses.lock().unwrap().push(status);
            self.inner.set_scan_status(status)
        }
        fn get_ai_config(&self) -> engine::Result<Option<engine::models::AIConfig>> {
            self.inner.get_ai_config()
        }
        fn set_ai_config(&self, config: engine::models::AIConfig) -> engine::Result<()> {
            self.inner.set_ai_config(config)
        }
        fn get_project_root(&self) -> engine::Result<String> {
            self.inner.get_project_root()
        }
        fn set_project_root(&self, path: &str) -> engine::Result<()> {
            self.inner.set_project_root(path)
        }
    }

    let statuses_clone = statuses.clone();
    let app_state = TrackingAppState::new(statuses_clone);

    let service = make_scan_service(scan_repo, app_state);

    // Use a real temp directory so the walker finds actual files
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("index.ts"), "export const x = 1;").ok();

    let result = service.scan_project(tmp.path().to_string_lossy().as_ref());

    // Assert: scan succeeded
    assert!(
        result.is_ok(),
        "scan_project should succeed, got: {:?}",
        result
    );
    let scan_result = result.unwrap();
    assert_eq!(scan_result.status, ScanStatus::Ready);
    assert_eq!(scan_result.files_count, 1);

    // Assert: status was transitioned through Scanning
    let observed: Vec<ScanStatus> = statuses.lock().unwrap().clone();
    assert!(
        observed.contains(&ScanStatus::Scanning),
        "status should transition to Scanning; observed: {:?}",
        observed
    );
    assert!(
        observed.contains(&ScanStatus::BuildingGraph),
        "status should transition to BuildingGraph; observed: {:?}",
        observed
    );
    // Final status should be Ready
    assert_eq!(
        observed.last().copied(),
        Some(ScanStatus::Ready),
        "final status should be Ready; observed: {:?}",
        observed
    );
}

/// T8.5 — scan_project: sets project_root in AppStatePort on success.
#[test]
fn scan_project_sets_project_root() {
    use engine::db::DbPool;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_repo = ScanRepositoryAdapter::new(&pool);
    let app_state = engine::ports::AppStatePortAdapter::new(
        Mutex::new(ScanStatus::Idle),
        Mutex::new(None),
        Mutex::new(String::new()),
    );

    let service = make_scan_service(scan_repo, app_state);

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("main.ts"), "const x = 1;").ok();

    let result = service.scan_project(tmp.path().to_string_lossy().as_ref());

    assert!(result.is_ok());
    let root = service.state().get_project_root().unwrap();
    assert_eq!(root, tmp.path().to_string_lossy().as_ref());
}

/// T8.6 — scan_project: propagates save error as AppError and sets status to Error.
#[test]
fn scan_project_propagates_save_error() {
    // Create a repo that always fails on save
    struct FailingScanRepo;
    impl ScanRepository for FailingScanRepo {
        fn save_scan_result(&self, _result: &ScanResult) -> engine::Result<()> {
            Err(engine::AppError::Database("simulated failure".into()))
        }
        fn get_project_by_path(&self, _root_path: &str) -> engine::Result<Option<ProjectMeta>> {
            Ok(None)
        }
        fn get_project(&self, _project_id: &str) -> engine::Result<Option<(String, String, i64)>> {
            Ok(None)
        }
        fn get_files(&self, _project_id: &str) -> engine::Result<Vec<FileInfo>> {
            Ok(vec![])
        }
        fn get_imports(&self, _project_id: &str) -> engine::Result<Vec<ImportInfo>> {
            Ok(vec![])
        }
        fn save_import(&self, _import: &ImportInfo) -> engine::Result<()> {
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

    let app_state = engine::ports::AppStatePortAdapter::new(
        Mutex::new(ScanStatus::Idle),
        Mutex::new(None),
        Mutex::new(String::new()),
    );

    let service = make_scan_service(FailingScanRepo, app_state);
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("failing.ts"), "export const x = 1;").ok();

    let result = service.scan_project(tmp.path().to_string_lossy().as_ref());

    // On DB error, status should be Error and result should be Err
    assert!(result.is_err(), "scan_project should propagate DB error");
    let err = result.unwrap_err();
    assert!(matches!(err, engine::AppError::Database(_)));
    // Status should be Error
    let status = service.state().get_scan_status().unwrap();
    assert_eq!(status, ScanStatus::Error);
}

/// T8.7 — open_project_by_path: loads project from ScanRepository and sets app state.
#[test]
fn open_project_by_path_loads_project_and_sets_state() {
    use engine::db::DbPool;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    // First: create a project in the DB by running a real scan
    let scan_repo = ScanRepositoryAdapter::new(&pool);
    let app_state = engine::ports::AppStatePortAdapter::new(
        Mutex::new(ScanStatus::Idle),
        Mutex::new(None),
        Mutex::new(String::new()),
    );

    let service = make_scan_service(scan_repo, app_state);

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("test.ts"), "export const y = 2;").ok();
    let root_path = tmp.path().to_string_lossy().as_ref().to_string();

    let scan_result = service.scan_project(&root_path).unwrap();
    let project_id = scan_result.project_id.clone();

    // Now reopen the project via open_project_by_path
    let reopen_result = service.open_project_by_path(&root_path);

    assert!(
        reopen_result.is_ok(),
        "open_project_by_path should succeed, got: {:?}",
        reopen_result
    );
    let reopened = reopen_result.unwrap();
    assert_eq!(reopened.project_id, project_id);
    assert_eq!(reopened.files_count, 1);

    // Status should be Ready after reopen
    let status = service.state().get_scan_status().unwrap();
    assert_eq!(status, ScanStatus::Ready);

    // Project root should be set
    let root = service.state().get_project_root().unwrap();
    assert_eq!(root, root_path);
}

/// T8.8 — open_project_by_path: returns error if project not found.
#[test]
fn open_project_by_path_not_found_error() {
    use engine::db::DbPool;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_repo = ScanRepositoryAdapter::new(&pool);
    let app_state = engine::ports::AppStatePortAdapter::new(
        Mutex::new(ScanStatus::Idle),
        Mutex::new(None),
        Mutex::new(String::new()),
    );

    let service = make_scan_service(scan_repo, app_state);

    let result = service.open_project_by_path("/nonexistent/path");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, engine::AppError::ProjectNotFound(_)));
}

/// T8.9 — get_scan_status: returns current scan status from AppStatePort.
#[test]
fn get_scan_status_returns_current_status() {
    use engine::db::DbPool;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_repo = ScanRepositoryAdapter::new(&pool);
    let app_state = engine::ports::AppStatePortAdapter::new(
        Mutex::new(ScanStatus::Scanning),
        Mutex::new(None),
        Mutex::new("/some/path".to_string()),
    );

    let service = make_scan_service(scan_repo, app_state);

    let status = service.get_scan_status().unwrap();
    assert_eq!(status, ScanStatus::Scanning);
}

/// T8.10 — scan_project: sets project_root even on successful scan.
#[test]
fn scan_project_sets_project_root_on_success() {
    use engine::db::DbPool;

    let pool = DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let scan_repo = ScanRepositoryAdapter::new(&pool);
    let app_state = engine::ports::AppStatePortAdapter::new(
        Mutex::new(ScanStatus::Idle),
        Mutex::new(None),
        Mutex::new(String::new()),
    );

    let service = make_scan_service(scan_repo, app_state);

    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("root_test.ts"), "export const z = 3;").ok();
    let root_path = tmp.path().to_string_lossy().as_ref().to_string();

    let result = service.scan_project(&root_path).unwrap();

    // On success, project_root must be set
    let root = service.state().get_project_root().unwrap();
    assert_eq!(root, root_path);
    // Status should be Ready
    assert_eq!(result.status, ScanStatus::Ready);
}
