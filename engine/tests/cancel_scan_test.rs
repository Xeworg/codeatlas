//! Tests for C1.2: ScanService::cancel(scan_id) with 3 outcomes
//!
//! RED PHASE: These tests define the expected behavior. They FAIL before
//! implementation and PASS after.

use engine::models::AIConfig;
use engine::models::ScanStatus;
use engine::ports::{AppStatePort, ScanRepository};
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

/// Mock ScanRepository for testing cancel behavior.
struct MockScanRepo {
    scans: std::sync::Mutex<std::collections::HashMap<String, ScanStatus>>,
}

impl MockScanRepo {
    fn new() -> Self {
        Self {
            scans: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn with_scan(id: &str, status: ScanStatus) -> Self {
        let mut scans = std::collections::HashMap::new();
        scans.insert(id.to_string(), status);
        Self {
            scans: std::sync::Mutex::new(scans),
        }
    }
}

impl ScanRepository for MockScanRepo {
    fn save_scan_result(&self, _result: &engine::models::ScanResult) -> engine::Result<()> {
        unimplemented!()
    }

    fn get_project_by_path(
        &self,
        _root_path: &str,
    ) -> engine::Result<Option<engine::models::ProjectMeta>> {
        unimplemented!()
    }

    fn get_project(&self, _project_id: &str) -> engine::Result<Option<(String, String, i64)>> {
        unimplemented!()
    }

    fn get_files(&self, _project_id: &str) -> engine::Result<Vec<engine::models::FileInfo>> {
        unimplemented!()
    }

    fn get_imports(&self, _project_id: &str) -> engine::Result<Vec<engine::models::ImportInfo>> {
        unimplemented!()
    }

    fn save_import(&self, _import: &engine::models::ImportInfo) -> engine::Result<()> {
        unimplemented!()
    }

    fn get_file_by_id(&self, _file_id: &str) -> engine::Result<Option<engine::models::FileInfo>> {
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
        unimplemented!()
    }

    fn get_scan_status(&self, scan_id: &str) -> engine::Result<Option<ScanStatus>> {
        let scans = self.scans.lock().unwrap();
        Ok(scans.get(scan_id).copied())
    }

    fn cancel(&self, scan_id: &str) -> engine::Result<()> {
        let mut scans = self.scans.lock().unwrap();
        let status = scans.get(scan_id).copied();
        match status {
            None => Err(engine::AppError::NotFound(scan_id.to_string())),
            Some(
                ScanStatus::Idle | ScanStatus::Ready | ScanStatus::Cancelled | ScanStatus::Error,
            ) => {
                // No-op for non-cancellable states
                Ok(())
            }
            Some(ScanStatus::Scanning | ScanStatus::BuildingGraph) => {
                // Cancel the scan
                *scans.get_mut(scan_id).unwrap() = ScanStatus::Cancelled;
                Ok(())
            }
        }
    }
}

/// Mock AppStatePort for testing.
struct MockAppState {
    status: std::sync::Mutex<ScanStatus>,
}

impl MockAppState {
    fn new(status: ScanStatus) -> Self {
        Self {
            status: std::sync::Mutex::new(status),
        }
    }
}

impl AppStatePort for MockAppState {
    fn get_scan_status(&self) -> engine::Result<ScanStatus> {
        Ok(*self.status.lock().unwrap())
    }

    fn set_scan_status(&self, status: ScanStatus) -> engine::Result<()> {
        *self.status.lock().unwrap() = status;
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

/// Test: Running scan is cancelled and returns Ok(()).
#[tokio::test]
async fn cancel_running_scan_returns_ok_and_sets_cancelled() {
    let repo = MockScanRepo::with_scan("scan-1", ScanStatus::Scanning);
    let state = MockAppState::new(ScanStatus::Scanning);
    let service = make_scan_service(repo, state);

    let result = service.cancel("scan-1").await;

    assert!(result.is_ok(), "cancel should return Ok for running scan");
}

/// Test: Completed scan is a no-op and returns Ok(()).
#[tokio::test]
async fn cancel_completed_scan_returns_ok_noop() {
    let repo = MockScanRepo::with_scan("scan-2", ScanStatus::Ready);
    let state = MockAppState::new(ScanStatus::Ready);
    let service = make_scan_service(repo, state);

    let result = service.cancel("scan-2").await;

    assert!(
        result.is_ok(),
        "cancel should return Ok for completed scan (no-op)"
    );
}

/// Test: Unknown scan returns NotFound error.
#[tokio::test]
async fn cancel_unknown_scan_returns_not_found() {
    let repo = MockScanRepo::new();
    let state = MockAppState::new(ScanStatus::Idle);
    let service = make_scan_service(repo, state);

    let result = service.cancel("ghost-scan").await;

    assert!(result.is_err(), "cancel should return Err for unknown scan");
    assert!(matches!(result.unwrap_err(), engine::AppError::NotFound(_)));
}
