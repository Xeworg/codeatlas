//! ScanService — application service for scan orchestration.
//!
//! Orchestrates the scan lifecycle using canonical ports:
//! - [`ScanRepository`] — persists scan results, files, imports, and outlines
//! - [`AppStatePort`] — tracks scan status and project root in-memory
//!
//! The service owns the infrastructure for file discovery, parsing, and import
//! resolution. It does NOT instantiate concrete repositories or database pools;
//! those are injected via the ports at construction time from the composition root
//! (`src-tauri/src/lib.rs`).
//!
//! # Workflow
//!
//! ```text
//! scan_project(path)
//!   -> AppStatePort.set(Scanning)
//!   -> FileWalker.discover()
//!   -> ParserRegistry.parse_file() [via scan_files, single dispatch]
//!   -> PathResolver.resolve() for each import
//!   -> ScanRepository.save_scan_result()
//!   -> AppStatePort.set(BuildingGraph)
//!   -> ScanRepository.save_import() for each import
//!   -> ScanRepository.save_outline_items() for each file
//!   -> ScanRepository.save_scan_result() [final authoritative count]
//!   -> AppStatePort.set(Ready|Error)
//!   -> AppStatePort.set_project_root()
//!
//! open_project_by_path(path)
//!   -> ScanRepository.get_project_by_path()
//!   -> ScanRepository.get_files()
//!   -> AppStatePort.set(Ready)
//!   -> AppStatePort.set_project_root()
//!
//! get_scan_status()
//!   -> AppStatePort.get_scan_status()
//! ```

use crate::commands::{scan_files, ScanFilesOutput};
use crate::graph::PathResolver;
use crate::models::{ImportInfo, ScanResult, ScanStatus};
use crate::ports::{AppStatePort, ScanRepository};
use crate::scanner::parser::ParserRegistry;
use crate::scanner::FileWalker;
use crate::AppError;
use crate::Result;
use std::collections::HashMap;

/// Application service for scan orchestration.
///
/// Generic over `S: ScanRepository` and `A: AppStatePort` so tests can inject
/// doubles without touching the database.
pub struct ScanService<S, A> {
    scan_repo: S,
    state: A,
}

impl<S, A> ScanService<S, A> {
    /// Construct a new ScanService.
    ///
    /// `scan_repo` and `state` are injected from the composition root.
    /// They are kept separate so this service is fully testable with mocks.
    pub fn new(scan_repo: S, state: A) -> Self {
        Self { scan_repo, state }
    }

    /// Access the state port (read-only view for testing).
    ///
    /// Prefer this over exposing internal fields. Used by tests to verify
    /// state transitions without duplicating the `AppStatePort` trait.
    pub fn state(&self) -> &A
    where
        A: AppStatePort,
    {
        &self.state
    }
}

impl<S: ScanRepository, A: AppStatePort> ScanService<S, A> {
    /// Cancel an in-progress scan.
    ///
    /// Three outcomes:
    /// - Running scan (Scanning | BuildingGraph) → cancelled, returns `Ok(())`
    /// - Completed/terminal scan (Ready | Idle | Cancelled | Error) → no-op, returns `Ok(())`
    /// - Unknown scan → `Err(AppError::NotFound(scan_id))`
    pub async fn cancel(&self, scan_id: &str) -> Result<()> {
        let status = self.scan_repo.get_scan_status(scan_id)?;
        match status {
            None => Err(AppError::NotFound(scan_id.to_string())),
            Some(ScanStatus::Idle | ScanStatus::Ready | ScanStatus::Cancelled | ScanStatus::Error) => {
                // Non-cancellable state — no-op
                Ok(())
            }
            Some(ScanStatus::Scanning | ScanStatus::BuildingGraph) => {
                // Running — cancel it
                self.scan_repo.cancel(scan_id)?;
                Ok(())
            }
        }
    }

    /// Scan a project directory: discover files, parse, resolve imports, persist.
    ///
    /// Orchestrates the full scan lifecycle:
    /// 1. Transitions `AppStatePort` to `Scanning`
    /// 2. Discovers files via `FileWalker`
    /// 3. Parses each file once via `ParserRegistry` (single-dispatch via `scan_files`)
    /// 4. Resolves import targets via `PathResolver`
    /// 5. Persists scan result, imports, and outline items via `ScanRepository`
    /// 6. Transitions `AppStatePort` through `BuildingGraph` → `Ready|Error`
    /// 7. Sets `project_root` in `AppStatePort`
    ///
    /// # Errors
    ///
    /// Returns `AppError::ProjectNotFound` on UNIQUE constraint conflict
    /// (already-existing project at this path). Returns `AppError::Database`
    /// on other persistence failures. Propagates parsing/discovery errors.
    pub fn scan_project(&self, path: &str) -> Result<ScanResult> {
        let root_for_state = path.to_string();

        // Phase 0: Transition to Scanning
        self.state.set_scan_status(ScanStatus::Scanning)?;

        // Phase 1: Discover files (with timing)
        let walker = FileWalker::new(path);
        let discover_start = std::time::Instant::now();
        let discovered = walker.discover();
        let discover_ms = discover_start.elapsed().as_millis() as u64;

        // Phase 2: Parse all files (single dispatch — registry called exactly once per file)
        let registry = ParserRegistry::new();
        let scan_output: ScanFilesOutput =
            scan_files(&registry, &discovered, std::path::Path::new(path))
                .with_discover_ms(discover_ms);

        // Phase 3: Build path → UUID lookup for import resolution
        let path_to_id: HashMap<String, String> = scan_output
            .file_infos
            .iter()
            .map(|f| (f.path.clone(), f.id.clone()))
            .collect();

        // Phase 4: Resolve import targets using PathResolver
        let resolver = PathResolver::new(path);
        let mut all_imports: Vec<ImportInfo> = Vec::new();
        for mut imp in scan_output.all_imports {
            // Convert source: relative_path → persisted UUID
            if let Some(uuid) = path_to_id.get(&imp.source_file_id) {
                imp.source_file_id = uuid.clone();
            }
            if let Some(ref module) = imp.target_module {
                // For target resolution, we need the original relative_path.
                // Look up reverse mapping: UUID → relative_path.
                let rel_path = path_to_id
                    .iter()
                    .find(|(_, v)| *v == &imp.source_file_id)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_else(|| imp.source_file_id.clone());

                let res = resolver.resolve(module, &rel_path);
                match res {
                    crate::graph::resolver::Resolution::Internal(p) => {
                        imp.target_file_id = path_to_id.get(&p).cloned();
                    }
                    crate::graph::resolver::Resolution::External(_) => {}
                    crate::graph::resolver::Resolution::Unresolved(_) => {}
                }
            }
            all_imports.push(imp);
        }

        let file_infos = scan_output.file_infos;
        let total_ms = scan_output.discover_ms + scan_output.parse_ms;

        // Build the initial scan result
        let symbols_count: usize = file_infos.iter().map(|f| f.symbols.len()).sum();
        let mut result = ScanResult {
            project_id: uuid::Uuid::new_v4().to_string(),
            project_name: path.split('/').next_back().unwrap_or("Project").to_string(),
            root_path: path.to_string(),
            files_count: file_infos.len(),
            symbols_count,
            imports_count: 0,
            files: file_infos,
            scan_duration_ms: total_ms,
            status: ScanStatus::Ready,
            error: None,
        };

        // Phase 5: Persist initial scan result (without import count)
        if let Err(e) = self.scan_repo.save_scan_result(&result) {
            let err_str = e.to_string();
            // Map UNIQUE constraint on root_path to user-friendly error
            if is_root_path_conflict(&err_str) {
                self.state.set_scan_status(ScanStatus::Error)?;
                return Err(map_save_scan_result_error(
                    &err_str,
                    path,
                    &result.project_id,
                ));
            } else {
                self.state.set_scan_status(ScanStatus::Error)?;
                return Err(crate::AppError::Database(err_str));
            }
        }

        // Phase 6: Transition to BuildingGraph while import edges are persisted
        self.state.set_scan_status(ScanStatus::BuildingGraph)?;

        // Phase 7: Persist import edges
        let parsed_count = all_imports.len();
        let mut skipped_empty = 0usize;
        let mut persist_errors = 0usize;
        let mut persisted_count = 0usize;

        for imp in &all_imports {
            if imp.source_file_id.is_empty() {
                skipped_empty += 1;
                continue;
            }
            match self.scan_repo.save_import(imp) {
                Ok(()) => {
                    persisted_count += 1;
                }
                Err(_) => {
                    persist_errors += 1;
                }
            }
        }

        // Phase 8: Persist outline items (already parsed in scan_output.outlines)
        for file in &discovered {
            let file_id = match path_to_id.get(&file.relative_path) {
                Some(id) => id,
                None => continue,
            };
            let outline = match scan_output.outlines.get(&file.relative_path) {
                Some(o) => o,
                None => continue,
            };
            if !outline.is_empty() {
                // Outline persistence errors are non-fatal (logged only)
                let _ = self.scan_repo.save_outline_items(file_id, outline);
            }
        }

        // Phase 9: Determine degraded state
        let non_empty_total = parsed_count.saturating_sub(skipped_empty);
        let failure_count = skipped_empty + persist_errors;
        let degraded = non_empty_total > 0 && failure_count > non_empty_total / 2;

        result.imports_count = persisted_count;
        if degraded {
            result.status = ScanStatus::Error;
            result.error = Some(format!(
                "Import persistence degraded: {} parsed, {} skipped (empty source), {} DB errors, {} persisted",
                parsed_count, skipped_empty, persist_errors, persisted_count
            ));
        }

        // Phase 10: Persist final authoritative scan result
        if let Err(e) = self.scan_repo.save_scan_result(&result) {
            let err_str = e.to_string();
            if is_root_path_conflict(&err_str) {
                self.state.set_scan_status(ScanStatus::Error)?;
                return Err(map_save_scan_result_error(
                    &err_str,
                    path,
                    &result.project_id,
                ));
            } else {
                self.state.set_scan_status(ScanStatus::Error)?;
                return Err(crate::AppError::Database(err_str));
            }
        }

        // Phase 11: Final status and project root
        let final_status = if degraded {
            ScanStatus::Error
        } else {
            ScanStatus::Ready
        };
        self.state.set_scan_status(final_status)?;
        self.state.set_project_root(&root_for_state)?;

        Ok(result)
    }

    /// Reopen a previously scanned project by its root path.
    ///
    /// Loads project metadata and hydrates files (with symbols) from the database
    /// via `ScanRepository`. Updates `AppStatePort` with `Ready` status and the
    /// project root path so subsequent graph commands can run.
    ///
    /// Does NOT re-scan or rebuild the graph. The caller should fetch the graph
    /// via `get_graph` after this command.
    ///
    /// # Errors
    ///
    /// Returns `AppError::ProjectNotFound` if no project exists at `path`.
    /// Propagates database errors from `ScanRepository`.
    pub fn open_project_by_path(&self, path: &str) -> Result<ScanResult> {
        let meta = self
            .scan_repo
            .get_project_by_path(path)?
            .ok_or_else(|| AppError::ProjectNotFound(path.to_string()))?;

        let files = self.scan_repo.get_files(&meta.project_id)?;

        // Update in-memory state
        self.state.set_scan_status(meta.status)?;
        self.state.set_project_root(path)?;

        Ok(ScanResult {
            project_id: meta.project_id,
            project_name: meta.project_name,
            root_path: meta.root_path,
            files_count: meta.files_count,
            symbols_count: meta.symbols_count,
            imports_count: meta.imports_count,
            files,
            scan_duration_ms: meta.scan_duration_ms,
            status: meta.status,
            error: meta.error,
        })
    }

    /// Read the current scan status from `AppStatePort`.
    ///
    /// Used by the `get_scan_status` Tauri command to surface progress to the frontend.
    pub fn get_scan_status(&self) -> Result<ScanStatus> {
        self.state.get_scan_status()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Error helpers — same logic as src-tauri/src/commands.rs but owned by the service
// so the service can return typed AppError instead of String.
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if the given error string indicates a SQLite UNIQUE constraint
/// violation on the `projects.root_path` column.
fn is_root_path_conflict(err: &str) -> bool {
    err.contains("UNIQUE constraint failed: projects.root_path")
}

/// Maps a `save_scan_result` error string to an `AppError`.
fn map_save_scan_result_error(err: &str, root_path: &str, project_id: &str) -> AppError {
    if is_root_path_conflict(err) {
        tracing::warn!(
            project_id = %project_id,
            root_path = %root_path,
            "projects.root_path UNIQUE constraint conflict"
        );
        AppError::ProjectNotFound(root_path.to_string())
    } else {
        AppError::Database(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FileInfo, ImportInfo, ProjectMeta};
    use crate::scanner::OutlineItem;
    use std::sync::Mutex;

    /// No-op ScanRepository for tests that only call get_scan_status (no DB needed).
    struct NoOpScanRepo;
    impl ScanRepository for NoOpScanRepo {
        fn save_scan_result(&self, _result: &ScanResult) -> Result<()> {
            unreachable!()
        }
        fn get_project_by_path(&self, _root_path: &str) -> Result<Option<ProjectMeta>> {
            unreachable!()
        }
        fn get_project(&self, _project_id: &str) -> Result<Option<(String, String, i64)>> {
            unreachable!()
        }
        fn get_files(&self, _project_id: &str) -> Result<Vec<FileInfo>> {
            unreachable!()
        }
        fn get_imports(&self, _project_id: &str) -> Result<Vec<ImportInfo>> {
            unreachable!()
        }
        fn save_import(&self, _import: &ImportInfo) -> Result<()> {
            unreachable!()
        }
        fn get_file_by_id(&self, _file_id: &str) -> Result<Option<FileInfo>> {
            unreachable!()
        }
        fn save_outline_items(&self, _file_id: &str, _items: &[OutlineItem]) -> Result<()> {
            unreachable!()
        }
        fn get_outline_items(&self, _file_id: &str) -> Result<Vec<OutlineItem>> {
            unreachable!()
        }
        fn get_scan_status(&self, _project_id: &str) -> Result<Option<ScanStatus>> {
            unreachable!()
        }
        fn cancel(&self, _project_id: &str) -> Result<()> {
            unreachable!()
        }
    }

    /// T8.4 helper: verify status transitions during scan.
    #[test]
    fn scan_project_transitions_status_through_scanning_and_building_graph() {
        let pool = crate::db::DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();

        let scan_repo = crate::ports::ScanRepositoryAdapter::new(&pool);
        let app_state = crate::ports::AppStatePortAdapter::new(
            Mutex::new(ScanStatus::Idle),
            Mutex::new(None),
            Mutex::new(String::new()),
        );

        let service = ScanService::new(scan_repo, app_state);

        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("index.ts"), "export const x = 1;").ok();

        let result = service.scan_project(tmp.path().to_string_lossy().as_ref());

        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.status, ScanStatus::Ready);
        assert_eq!(r.files_count, 1);

        // Verify final status
        let final_status = service.state().get_scan_status().unwrap();
        assert_eq!(final_status, ScanStatus::Ready);
    }

    /// T8.5 helper: project_root is set after scan.
    #[test]
    fn scan_project_sets_project_root() {
        let pool = crate::db::DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();

        let scan_repo = crate::ports::ScanRepositoryAdapter::new(&pool);
        let app_state = crate::ports::AppStatePortAdapter::new(
            Mutex::new(ScanStatus::Idle),
            Mutex::new(None),
            Mutex::new(String::new()),
        );

        let service = ScanService::new(scan_repo, app_state);

        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.ts"), "const x = 1;").ok();
        let root = tmp.path().to_string_lossy().as_ref().to_string();

        let result = service.scan_project(&root);
        assert!(result.is_ok());
        assert_eq!(service.state().get_project_root().unwrap(), root);
    }

    /// T8.7 helper: open_project_by_path loads project and sets state.
    #[test]
    fn open_project_by_path_loads_and_sets_state() {
        let pool = crate::db::DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();

        let scan_repo = crate::ports::ScanRepositoryAdapter::new(&pool);
        let app_state = crate::ports::AppStatePortAdapter::new(
            Mutex::new(ScanStatus::Idle),
            Mutex::new(None),
            Mutex::new(String::new()),
        );

        let service = ScanService::new(scan_repo, app_state);

        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("test.ts"), "export const y = 2;").ok();
        let root = tmp.path().to_string_lossy().as_ref().to_string();

        // First scan
        let scan_result = service.scan_project(&root).unwrap();
        let project_id = scan_result.project_id.clone();

        // Reopen
        let reopened = service.open_project_by_path(&root).unwrap();
        assert_eq!(reopened.project_id, project_id);
        assert_eq!(reopened.files_count, 1);
        assert_eq!(
            service.state().get_scan_status().unwrap(),
            ScanStatus::Ready
        );
        assert_eq!(service.state().get_project_root().unwrap(), root);
    }

    /// T8.8 helper: open_project_by_path returns error for missing project.
    #[test]
    fn open_project_by_path_not_found() {
        let pool = crate::db::DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();

        let scan_repo = crate::ports::ScanRepositoryAdapter::new(&pool);
        let app_state = crate::ports::AppStatePortAdapter::new(
            Mutex::new(ScanStatus::Idle),
            Mutex::new(None),
            Mutex::new(String::new()),
        );

        let service = ScanService::new(scan_repo, app_state);

        let result = service.open_project_by_path("/nonexistent/path");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ProjectNotFound(_)));
    }

    /// T8.9 helper: get_scan_status returns current status.
    #[test]
    fn get_scan_status_returns_current() {
        // get_scan_status doesn't call scan_repo at all, so use a NoOp mock
        let scan_repo = NoOpScanRepo;
        let app_state = crate::ports::AppStatePortAdapter::new(
            Mutex::new(ScanStatus::Scanning),
            Mutex::new(None),
            Mutex::new("/some/path".to_string()),
        );

        let service = ScanService::new(scan_repo, app_state);
        assert_eq!(service.get_scan_status().unwrap(), ScanStatus::Scanning);
    }
}
