//! Canonical ports — hexagonal architecture boundary (wave 1).
//!
//! These four traits define the primary interfaces between the application
//! layer (services) and the infrastructure layer (repositories, state).
//!
//! Each port is implemented by a concrete adapter that delegates to the
//! existing `ProjectRepository` or command-level state. The adapters are
//! additive wrappers: they do NOT refactor the internal structure of
//! `queries.rs` or split it into smaller files.
//!
//! # Ports
//! - [`ScanRepository`] — scan result persistence and project lookup
//! - [`GraphRepository`] — graph cache persistence and file search
//! - [`WorkspaceRepository`] — workspace CRUD and project attachment
//! - [`AppStatePort`] — transient in-memory state (scan status, AI config, project root)
//!
//! # Adapters
//! - [`ScanRepositoryAdapter`] — implements `ScanRepository` via `ProjectRepository`
//! - [`GraphRepositoryAdapter`] — implements `GraphRepository` via `ProjectRepository`
//! - [`WorkspaceRepositoryAdapter`] — implements `WorkspaceRepository` via `ProjectRepository`
//! - [`AppStatePortAdapter`] — implements `AppStatePort` via in-memory `Mutex<...>` fields

use crate::db::DbPool;
use crate::models::{FileInfo, ImportInfo, OutlineItem, ProjectMeta, ScanResult, ScanStatus};
use crate::Result;
use std::sync::{Arc, Mutex};

// ─────────────────────────────────────────────────────────────────────────────
// ScanRepository — scan result persistence and project lookup
// ─────────────────────────────────────────────────────────────────────────────

/// Port for scan operations.
///
/// Abstracts the persistence of scan results and project metadata so that
/// application services (e.g., `ScanService`) are decoupled from the concrete
/// `ProjectRepository` implementation.
pub trait ScanRepository: Send + Sync {
    /// Persist a scan result (project metadata + files + symbols).
    fn save_scan_result(&self, result: &ScanResult) -> Result<()>;

    /// Look up a project by its root path. Returns full metadata without files.
    fn get_project_by_path(&self, root_path: &str) -> Result<Option<ProjectMeta>>;

    /// Get project metadata by ID.
    fn get_project(&self, project_id: &str) -> Result<Option<(String, String, i64)>>;

    /// Retrieve all files with hydrated symbols for a project.
    fn get_files(&self, project_id: &str) -> Result<Vec<FileInfo>>;

    /// Retrieve all import edges for a project.
    fn get_imports(&self, project_id: &str) -> Result<Vec<ImportInfo>>;

    /// Persist an import edge.
    fn save_import(&self, import: &ImportInfo) -> Result<()>;

    /// Get file info by ID (with symbols hydrated).
    fn get_file_by_id(&self, file_id: &str) -> Result<Option<FileInfo>>;

    /// Persist outline items for a file. Uses INSERT OR REPLACE so rescan
    /// of the same file updates the outline JSON.
    fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> Result<()>;

    /// Retrieve outline items for a file.
    fn get_outline_items(&self, file_id: &str) -> Result<Vec<OutlineItem>>;

    /// Get the current scan status for a project (by root path or project id).
    fn get_scan_status(&self, project_id: &str) -> Result<Option<ScanStatus>>;

    /// Cancel an in-progress scan for a project. Sets status to `Cancelled`.
    /// Returns `Ok(())` if the scan was found and cancelled (or was already completed).
    /// Returns `Err(AppError::NotFound)` if no scan exists for this project.
    fn cancel(&self, project_id: &str) -> Result<()>;
}

/// Adapter that implements `ScanRepository` by delegating to `ProjectRepository`.
///
/// This adapter is additive: it wraps the existing `ProjectRepository` without
/// modifying its internal structure. `queries.rs` stays intact.
///
/// Two constructors are provided:
/// - `new(&pool)`: lifetime-tied, used by internal engine tests that hold a
///   stack-local `DbPool`.
/// - `from_arc(Arc<DbPool>)`: produces a `'static` adapter that owns its
///   own `Arc<DbPool>` clone. This is the form consumed by the
///   presentation layer's `Arc<dyn ScanRepository>` (PR-B Task B.5).
pub struct ScanRepositoryAdapter {
    inner: crate::db::queries::ProjectRepository<'static>,
    _pool: std::sync::Arc<DbPool>,
}

impl ScanRepositoryAdapter {
    /// Internal-test constructor. The returned adapter borrows the pool;
    /// it cannot outlive it.
    ///
    /// Internally the pool is cloned into an `Arc<DbPool>` and held as
    /// a `'static` `ProjectRepository`, so this constructor is
    /// equivalent to `from_arc(Arc::new(pool.clone()))`. The simpler
    /// signature is preserved so existing tests do not need to be
    /// rewritten.
    pub fn new(pool: &DbPool) -> Self {
        let pool = std::sync::Arc::new(pool.clone());
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            _pool: pool,
        }
    }

    /// Production constructor. Owns the `Arc<DbPool>` so the adapter
    /// can be stored in `Arc<dyn ScanRepository>`.
    pub fn from_arc(pool: std::sync::Arc<DbPool>) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            _pool: pool,
        }
    }
}

impl ScanRepository for ScanRepositoryAdapter {
    fn save_scan_result(&self, result: &ScanResult) -> Result<()> {
        self.inner
            .save_scan_result(result)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_project_by_path(&self, root_path: &str) -> Result<Option<ProjectMeta>> {
        self.inner
            .get_project_by_path(root_path)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_project(&self, project_id: &str) -> Result<Option<(String, String, i64)>> {
        self.inner
            .get_project(project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_files(&self, project_id: &str) -> Result<Vec<FileInfo>> {
        self.inner
            .get_files(project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_imports(&self, project_id: &str) -> Result<Vec<ImportInfo>> {
        self.inner
            .get_imports(project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn save_import(&self, import: &ImportInfo) -> Result<()> {
        self.inner
            .save_import(import)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_file_by_id(&self, file_id: &str) -> Result<Option<FileInfo>> {
        self.inner
            .get_file_by_id(file_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> Result<()> {
        self.inner
            .save_outline_items(file_id, items)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_outline_items(&self, file_id: &str) -> Result<Vec<OutlineItem>> {
        self.inner
            .get_outline_items(file_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_scan_status(&self, project_id: &str) -> Result<Option<ScanStatus>> {
        self.inner
            .get_scan_status(project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn cancel(&self, project_id: &str) -> Result<()> {
        self.inner
            .mark_scan_cancelled(project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GraphRepository — graph cache and file search
// ─────────────────────────────────────────────────────────────────────────────

/// Port for graph operations.
///
/// Abstracts graph cache persistence and file search so that application
/// services (e.g., `GraphService`) are decoupled from the concrete
/// `ProjectRepository` implementation.
pub trait GraphRepository: Send + Sync {
    /// Save a serialized graph JSON for a project.
    fn save_graph_cache(&self, project_id: &str, graph_json: &str) -> Result<()>;

    /// Retrieve cached graph JSON for a project.
    fn get_graph_cache(&self, project_id: &str) -> Result<Option<String>>;

    /// Search files by name substring (case-insensitive).
    fn search_files(&self, project_id: &str, query: &str, limit: usize) -> Result<Vec<FileInfo>>;

    /// Get project root path for a file ID (used for on-demand outline fallback).
    fn get_project_root_for_file(&self, file_id: &str) -> Result<Option<String>>;

    /// Save outline items for a file.
    fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> Result<()>;

    /// Get outline items for a file.
    fn get_outline_items(&self, file_id: &str) -> Result<Vec<OutlineItem>>;

    /// Get all nodes that the given node depends on (outgoing import edges).
    /// Returns empty Vec for known nodes with no dependencies.
    /// Returns `Err(AppError::NotFound)` for unknown nodes.
    fn get_dependencies(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>>;

    /// Get all nodes that depend on the given node (incoming import edges).
    /// Returns empty Vec for known nodes with no dependents.
    /// Returns `Err(AppError::NotFound)` for unknown nodes.
    fn get_dependents(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>>;
}

/// Adapter that implements `GraphRepository` by delegating to `ProjectRepository`.
///
/// Additive wrapper: does not refactor the internal structure of `queries.rs`.
///
/// Two constructors: `new(&pool)` for internal tests, `from_arc(Arc<DbPool>)`
/// for production (stores the adapter inside `Arc<dyn GraphRepository>`).
pub struct GraphRepositoryAdapter {
    inner: crate::db::queries::ProjectRepository<'static>,
    _pool: std::sync::Arc<DbPool>,
}

impl GraphRepositoryAdapter {
    pub fn new(pool: &DbPool) -> Self {
        let pool = std::sync::Arc::new(pool.clone());
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            _pool: pool,
        }
    }

    pub fn from_arc(pool: std::sync::Arc<DbPool>) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            _pool: pool,
        }
    }
}

impl GraphRepository for GraphRepositoryAdapter {
    fn save_graph_cache(&self, project_id: &str, graph_json: &str) -> Result<()> {
        self.inner
            .save_graph_cache(project_id, graph_json)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_graph_cache(&self, project_id: &str) -> Result<Option<String>> {
        self.inner
            .get_graph_cache(project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn search_files(&self, project_id: &str, query: &str, limit: usize) -> Result<Vec<FileInfo>> {
        self.inner
            .search_files(project_id, query, limit)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_project_root_for_file(&self, file_id: &str) -> Result<Option<String>> {
        self.inner
            .get_project_root_for_file(file_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> Result<()> {
        self.inner
            .save_outline_items(file_id, items)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_outline_items(&self, file_id: &str) -> Result<Vec<OutlineItem>> {
        self.inner
            .get_outline_items(file_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_dependencies(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>> {
        self.inner
            .get_node_outgoing_edges(node_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_dependents(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>> {
        self.inner
            .get_node_incoming_edges(node_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WorkspaceRepository — workspace CRUD and project attachment
// ─────────────────────────────────────────────────────────────────────────────

/// Port for workspace operations.
///
/// Abstracts workspace lifecycle and project attachment so that application
/// services (e.g., `WorkspaceService`) are decoupled from the concrete
/// `ProjectRepository` implementation.
///
/// All methods are implemented by `WorkspaceRepositoryAdapter` which delegates
/// to `ProjectRepository`. This port enables full testability with mock doubles.
pub trait WorkspaceRepository: Send + Sync {
    /// Create a new workspace. Returns (id, name, created_at).
    fn create_workspace(&self, name: &str) -> Result<(String, String, String)>;

    /// List all workspaces ordered by creation date (newest first).
    fn list_workspaces(&self) -> Result<Vec<(String, String, String)>>;

    /// Attach a project to a workspace.
    fn attach_project_to_workspace(&self, workspace_id: &str, project_id: &str) -> Result<()>;

    /// List projects attached to a workspace.
    fn list_workspace_projects(&self, workspace_id: &str) -> Result<Vec<(String, String)>>;

    /// Create a snapshot. Returns (id, project_id, workspace_id, label, created_at, payload_json).
    #[allow(clippy::type_complexity)]
    fn create_snapshot(
        &self,
        project_id: &str,
        label: &str,
        workspace_id: Option<&str>,
    ) -> Result<(
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
    )>;

    /// Get a snapshot by ID.
    #[allow(clippy::type_complexity)]
    fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<
        Option<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    >;

    /// List snapshots for a project, optionally filtered by workspace.
    #[allow(clippy::type_complexity)]
    fn list_snapshots(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<
        Vec<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    >;

    /// Add a comment/annotation to a node.
    /// Returns (id, project_id, node_id, author, kind, text, created_at).
    #[allow(clippy::type_complexity)]
    fn add_comment(
        &self,
        project_id: &str,
        node_id: &str,
        author: &str,
        text: &str,
        kind: Option<&str>,
    ) -> Result<(String, String, String, String, String, String, String)>;

    /// List comments for a project, optionally filtered by node.
    #[allow(clippy::type_complexity)]
    fn list_comments(
        &self,
        project_id: &str,
        node_id: Option<&str>,
    ) -> Result<Vec<(String, String, String, String, String, String, String)>>;

    /// Get health timeline for a project within a date range.
    #[allow(clippy::type_complexity)]
    fn get_health_timeline(
        &self,
        project_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<(String, String, f64, f64, f64, i64, i64)>>;

    /// Compute executive summary for a workspace.
    fn compute_executive_summary(
        &self,
        workspace_id: &str,
    ) -> Result<crate::db::queries::ExecutiveSummary>;

    /// Compare two snapshots and return diff.
    fn compare_snapshots(
        &self,
        base_snapshot_id: &str,
        target_snapshot_id: &str,
    ) -> Result<crate::db::queries::SnapshotDiff>;

    /// Get C4 view for a project at a given level.
    fn get_c4_view(&self, project_id: &str, level: u8) -> Result<crate::db::queries::C4View>;
}

/// Adapter that implements `WorkspaceRepository` by delegating to `ProjectRepository`.
///
/// Additive wrapper: does not refactor the internal structure of `queries.rs`.
///
/// Two constructors: `new(&pool)` for internal tests, `from_arc(Arc<DbPool>)`
/// for production (stores the adapter inside `Arc<dyn WorkspaceRepository>`).
pub struct WorkspaceRepositoryAdapter {
    inner: crate::db::queries::ProjectRepository<'static>,
    _pool: std::sync::Arc<DbPool>,
}

impl WorkspaceRepositoryAdapter {
    pub fn new(pool: &DbPool) -> Self {
        let pool = std::sync::Arc::new(pool.clone());
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            _pool: pool,
        }
    }

    pub fn from_arc(pool: std::sync::Arc<DbPool>) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            _pool: pool,
        }
    }
}

impl WorkspaceRepository for WorkspaceRepositoryAdapter {
    fn create_workspace(&self, name: &str) -> Result<(String, String, String)> {
        self.inner
            .create_workspace(name)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn list_workspaces(&self) -> Result<Vec<(String, String, String)>> {
        self.inner
            .list_workspaces()
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn attach_project_to_workspace(&self, workspace_id: &str, project_id: &str) -> Result<()> {
        self.inner
            .attach_project_to_workspace(workspace_id, project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn list_workspace_projects(&self, workspace_id: &str) -> Result<Vec<(String, String)>> {
        self.inner
            .list_workspace_projects(workspace_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn create_snapshot(
        &self,
        project_id: &str,
        label: &str,
        workspace_id: Option<&str>,
    ) -> Result<(
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
    )> {
        self.inner
            .create_snapshot(project_id, label, workspace_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<
        Option<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    > {
        self.inner
            .get_snapshot(snapshot_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn list_snapshots(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<
        Vec<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    > {
        self.inner
            .list_snapshots(project_id, workspace_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    #[allow(clippy::type_complexity)]
    fn add_comment(
        &self,
        project_id: &str,
        node_id: &str,
        author: &str,
        text: &str,
        kind: Option<&str>,
    ) -> Result<(String, String, String, String, String, String, String)> {
        self.inner
            .add_comment(project_id, node_id, author, text, kind)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    #[allow(clippy::type_complexity)]
    fn list_comments(
        &self,
        project_id: &str,
        node_id: Option<&str>,
    ) -> Result<Vec<(String, String, String, String, String, String, String)>> {
        self.inner
            .list_comments(project_id, node_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    #[allow(clippy::type_complexity)]
    fn get_health_timeline(
        &self,
        project_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<(String, String, f64, f64, f64, i64, i64)>> {
        self.inner
            .get_health_timeline(project_id, from, to)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn compute_executive_summary(
        &self,
        workspace_id: &str,
    ) -> Result<crate::db::queries::ExecutiveSummary> {
        self.inner
            .compute_executive_summary(workspace_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn compare_snapshots(
        &self,
        base_snapshot_id: &str,
        target_snapshot_id: &str,
    ) -> Result<crate::db::queries::SnapshotDiff> {
        self.inner
            .compare_snapshots(base_snapshot_id, target_snapshot_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn get_c4_view(&self, project_id: &str, level: u8) -> Result<crate::db::queries::C4View> {
        self.inner
            .get_c4_view(project_id, level)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AnalysisDataSource — analysis persistence
// ─────────────────────────────────────────────────────────────────────────────

/// Port for analysis persistence operations.
///
/// Abstracts the persistence of architecture detection results and graph
/// insights so that `AnalysisService` is decoupled from the concrete
/// `ProjectRepository` implementation.
pub trait AnalysisDataSource: Send + Sync {
    /// Returns a reference to the underlying database pool.
    /// Used by `AnalysisService` to pass to pure analysis functions that
    /// take `&DbPool` directly.
    fn pool(&self) -> &DbPool;

    /// Persist an architecture detection result for a project.
    fn save_architecture_detection(
        &self,
        project_id: &str,
        pattern: &str,
        confidence: f64,
        evidence_json: &str,
    ) -> Result<()>;

    /// Save graph insights for a project (upsert).
    fn save_graph_insights(
        &self,
        project_id: &str,
        cycles_json: &str,
        hotspots_json: &str,
        avg_coupling: Option<f64>,
        density: Option<f64>,
    ) -> Result<()>;

    /// Get cached graph insights for a project.
    #[allow(clippy::type_complexity)]
    fn get_cached_graph_insights(
        &self,
        project_id: &str,
    ) -> Result<Option<(String, String, f64, f64, String)>>;
}

/// Adapter that implements `AnalysisDataSource` by delegating to `ProjectRepository`.
///
/// Two constructors: `new(&pool)` for internal tests, `from_arc(Arc<DbPool>)`
/// for production (stores the adapter inside `Arc<dyn AnalysisDataSource>`).
pub struct AnalysisDataSourceAdapter {
    pool: std::sync::Arc<DbPool>,
    inner: crate::db::queries::ProjectRepository<'static>,
}

impl AnalysisDataSourceAdapter {
    pub fn new(pool: &crate::db::DbPool) -> Self {
        let pool = std::sync::Arc::new(pool.clone());
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            pool,
        }
    }

    pub fn from_arc(pool: std::sync::Arc<crate::db::DbPool>) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::from_arc(pool.clone()),
            pool,
        }
    }
}

impl AnalysisDataSource for AnalysisDataSourceAdapter {
    fn pool(&self) -> &crate::db::DbPool {
        &self.pool
    }

    fn save_architecture_detection(
        &self,
        project_id: &str,
        pattern: &str,
        confidence: f64,
        evidence_json: &str,
    ) -> Result<()> {
        self.inner
            .save_architecture_detection(project_id, pattern, confidence, evidence_json)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    fn save_graph_insights(
        &self,
        project_id: &str,
        cycles_json: &str,
        hotspots_json: &str,
        avg_coupling: Option<f64>,
        density: Option<f64>,
    ) -> Result<()> {
        self.inner
            .save_graph_insights(
                project_id,
                cycles_json,
                hotspots_json,
                avg_coupling,
                density,
            )
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }

    #[allow(clippy::type_complexity)]
    fn get_cached_graph_insights(
        &self,
        project_id: &str,
    ) -> Result<Option<(String, String, f64, f64, String)>> {
        self.inner
            .get_cached_graph_insights(project_id)
            .map_err(|e| crate::AppError::Database(e.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AppStatePort — transient in-memory state
// ─────────────────────────────────────────────────────────────────────────────

/// Port for transient application state.
///
/// Abstracts the in-memory state that lives for the duration of the Tauri
/// process (scan status, AI configuration, currently open project root).
///
/// The concrete state is owned by Tauri's `State<AppState>` and accessed
/// via `Mutex<...>` fields. This port wraps those fields so that application
/// services can read/write state without depending on the concrete `AppState`
/// struct directly.
pub trait AppStatePort: Send + Sync {
    /// Read the current scan status.
    fn get_scan_status(&self) -> Result<crate::models::ScanStatus>;

    /// Write a new scan status.
    fn set_scan_status(&self, status: crate::models::ScanStatus) -> Result<()>;

    /// Read the AI configuration.
    fn get_ai_config(&self) -> Result<Option<crate::models::AIConfig>>;

    /// Write the AI configuration.
    fn set_ai_config(&self, config: crate::models::AIConfig) -> Result<()>;

    /// Read the project root path of the currently open project.
    fn get_project_root(&self) -> Result<String>;

    /// Write the project root path.
    fn set_project_root(&self, path: &str) -> Result<()>;
}

/// Adapter that implements `AppStatePort` by wrapping three `Arc<Mutex<...>>` handles.
///
/// These handles point to the SAME mutexes that live in Tauri's `AppState` struct
/// (`src-tauri/src/commands.rs`). Using `Arc` allows the adapter to share ownership
/// with the real `AppState` so that mutations through the adapter are immediately
/// visible in the original state — no copies, no dead state.
///
/// All fields are `Arc<Mutex<T>>` where `T: Send`; the compiler auto-derives
/// `Send + Sync`, so no explicit unsafe impl is needed.
pub struct AppStatePortAdapter {
    scan_status: Arc<Mutex<crate::models::ScanStatus>>,
    ai_config: Arc<Mutex<Option<crate::models::AIConfig>>>,
    project_root: Arc<Mutex<String>>,
}

impl AppStatePortAdapter {
    pub fn new(
        scan_status: Mutex<crate::models::ScanStatus>,
        ai_config: Mutex<Option<crate::models::AIConfig>>,
        project_root: Mutex<String>,
    ) -> Self {
        Self {
            scan_status: Arc::new(scan_status),
            ai_config: Arc::new(ai_config),
            project_root: Arc::new(project_root),
        }
    }

    /// Construct from `Arc<Mutex<T>>` references to the real `AppState` mutexes.
    ///
    /// This constructor is intended for use in Tauri commands where the
    /// `Mutex` fields live in `State<AppState>` (shared access). The adapter
    /// clones the `Arc` handles so that both the adapter AND the original
    /// `AppState` point to the same inner mutex data. Mutations through the
    /// adapter mutate the real `AppState`.
    ///
    /// **Critical invariant**: the `Arc` handles must reference the SAME mutexes
    /// that live in `AppState`. If independent `Arc<Mutex<T>>` values are passed,
    /// the adapter mutates copies that are invisible to `AppState`.
    pub fn from_arc_refs(
        scan_status: &Arc<Mutex<crate::models::ScanStatus>>,
        ai_config: &Arc<Mutex<Option<crate::models::AIConfig>>>,
        project_root: &Arc<Mutex<String>>,
    ) -> Self {
        Self {
            scan_status: Arc::clone(scan_status),
            ai_config: Arc::clone(ai_config),
            project_root: Arc::clone(project_root),
        }
    }
}

impl AppStatePort for AppStatePortAdapter {
    fn get_scan_status(&self) -> Result<crate::models::ScanStatus> {
        Ok(*self.scan_status.lock().unwrap())
    }

    fn set_scan_status(&self, status: crate::models::ScanStatus) -> Result<()> {
        *self.scan_status.lock().unwrap() = status;
        Ok(())
    }

    fn get_ai_config(&self) -> Result<Option<crate::models::AIConfig>> {
        Ok(self.ai_config.lock().unwrap().clone())
    }

    fn set_ai_config(&self, config: crate::models::AIConfig) -> Result<()> {
        *self.ai_config.lock().unwrap() = Some(config);
        Ok(())
    }

    fn get_project_root(&self) -> Result<String> {
        Ok(self.project_root.lock().unwrap().clone())
    }

    fn set_project_root(&self, path: &str) -> Result<()> {
        *self.project_root.lock().unwrap() = path.to_string();
        Ok(())
    }
}

// ─── from_arc tests ─────────────────────────────────────────────────────────
//
// These tests verify that the production `from_arc(Arc<DbPool>)` constructor
// returns a `'static` adapter that holds the pool alive, callable from
// the trait methods. The tests are minimal: they construct the adapter,
// call one cheap method on it, and assert the pool is still queryable.

// ─────────────────────────────────────────────────────────────────────────────
// Arc<dyn Trait> impls — enables Arc<dyn Trait> to satisfy S: Trait bounds
// in service constructors. The blanket impl `impl<T: Trait> Trait for Arc<T>`
// fails for `T = dyn Trait` because `dyn Trait: ?Sized` and type parameters
// default to `Sized`. We add explicit impls for `Arc<dyn Trait>` which works
// because Arc<dyn Trait> is Sized and the impl delegates via Deref.
// ─────────────────────────────────────────────────────────────────────────────

impl ScanRepository for std::sync::Arc<dyn ScanRepository> {
    fn save_scan_result(&self, result: &ScanResult) -> Result<()> {
        (**self).save_scan_result(result)
    }
    fn get_project_by_path(&self, root_path: &str) -> Result<Option<ProjectMeta>> {
        (**self).get_project_by_path(root_path)
    }
    fn get_project(&self, project_id: &str) -> Result<Option<(String, String, i64)>> {
        (**self).get_project(project_id)
    }
    fn get_files(&self, project_id: &str) -> Result<Vec<FileInfo>> {
        (**self).get_files(project_id)
    }
    fn get_imports(&self, project_id: &str) -> Result<Vec<ImportInfo>> {
        (**self).get_imports(project_id)
    }
    fn save_import(&self, import: &ImportInfo) -> Result<()> {
        (**self).save_import(import)
    }
    fn get_file_by_id(&self, file_id: &str) -> Result<Option<FileInfo>> {
        (**self).get_file_by_id(file_id)
    }
    fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> Result<()> {
        (**self).save_outline_items(file_id, items)
    }
    fn get_outline_items(&self, file_id: &str) -> Result<Vec<OutlineItem>> {
        (**self).get_outline_items(file_id)
    }
    fn get_scan_status(&self, project_id: &str) -> Result<Option<ScanStatus>> {
        (**self).get_scan_status(project_id)
    }
    fn cancel(&self, project_id: &str) -> Result<()> {
        (**self).cancel(project_id)
    }
}

impl GraphRepository for std::sync::Arc<dyn GraphRepository> {
    fn save_graph_cache(&self, project_id: &str, graph_json: &str) -> Result<()> {
        (**self).save_graph_cache(project_id, graph_json)
    }
    fn get_graph_cache(&self, project_id: &str) -> Result<Option<String>> {
        (**self).get_graph_cache(project_id)
    }
    fn search_files(&self, project_id: &str, query: &str, limit: usize) -> Result<Vec<FileInfo>> {
        (**self).search_files(project_id, query, limit)
    }
    fn get_project_root_for_file(&self, file_id: &str) -> Result<Option<String>> {
        (**self).get_project_root_for_file(file_id)
    }
    fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> Result<()> {
        (**self).save_outline_items(file_id, items)
    }
    fn get_outline_items(&self, file_id: &str) -> Result<Vec<OutlineItem>> {
        (**self).get_outline_items(file_id)
    }
    fn get_dependencies(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>> {
        (**self).get_dependencies(node_id)
    }
    fn get_dependents(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>> {
        (**self).get_dependents(node_id)
    }
}

impl AnalysisDataSource for std::sync::Arc<dyn AnalysisDataSource> {
    fn pool(&self) -> &DbPool {
        (**self).pool()
    }
    fn save_architecture_detection(
        &self,
        project_id: &str,
        pattern: &str,
        confidence: f64,
        evidence_json: &str,
    ) -> Result<()> {
        (**self).save_architecture_detection(project_id, pattern, confidence, evidence_json)
    }
    fn save_graph_insights(
        &self,
        project_id: &str,
        cycles_json: &str,
        hotspots_json: &str,
        avg_coupling: Option<f64>,
        density: Option<f64>,
    ) -> Result<()> {
        (**self).save_graph_insights(
            project_id,
            cycles_json,
            hotspots_json,
            avg_coupling,
            density,
        )
    }
    #[allow(clippy::type_complexity)]
    fn get_cached_graph_insights(
        &self,
        project_id: &str,
    ) -> Result<Option<(String, String, f64, f64, String)>> {
        (**self).get_cached_graph_insights(project_id)
    }
}

impl WorkspaceRepository for std::sync::Arc<dyn WorkspaceRepository> {
    fn create_workspace(&self, name: &str) -> Result<(String, String, String)> {
        (**self).create_workspace(name)
    }

    fn list_workspaces(&self) -> Result<Vec<(String, String, String)>> {
        (**self).list_workspaces()
    }

    fn attach_project_to_workspace(&self, workspace_id: &str, project_id: &str) -> Result<()> {
        (**self).attach_project_to_workspace(workspace_id, project_id)
    }

    fn list_workspace_projects(&self, workspace_id: &str) -> Result<Vec<(String, String)>> {
        (**self).list_workspace_projects(workspace_id)
    }

    #[allow(clippy::type_complexity)]
    fn create_snapshot(
        &self,
        project_id: &str,
        label: &str,
        workspace_id: Option<&str>,
    ) -> Result<(
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
    )> {
        (**self).create_snapshot(project_id, label, workspace_id)
    }

    #[allow(clippy::type_complexity)]
    fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<
        Option<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    > {
        (**self).get_snapshot(snapshot_id)
    }

    #[allow(clippy::type_complexity)]
    fn list_snapshots(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<
        Vec<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    > {
        (**self).list_snapshots(project_id, workspace_id)
    }

    #[allow(clippy::type_complexity)]
    fn add_comment(
        &self,
        project_id: &str,
        node_id: &str,
        author: &str,
        text: &str,
        kind: Option<&str>,
    ) -> Result<(String, String, String, String, String, String, String)> {
        (**self).add_comment(project_id, node_id, author, text, kind)
    }

    #[allow(clippy::type_complexity)]
    fn list_comments(
        &self,
        project_id: &str,
        node_id: Option<&str>,
    ) -> Result<Vec<(String, String, String, String, String, String, String)>> {
        (**self).list_comments(project_id, node_id)
    }

    fn get_health_timeline(
        &self,
        project_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<(String, String, f64, f64, f64, i64, i64)>> {
        (**self).get_health_timeline(project_id, from, to)
    }

    fn compute_executive_summary(
        &self,
        workspace_id: &str,
    ) -> Result<crate::db::queries::ExecutiveSummary> {
        (**self).compute_executive_summary(workspace_id)
    }

    fn compare_snapshots(
        &self,
        base_snapshot_id: &str,
        target_snapshot_id: &str,
    ) -> Result<crate::db::queries::SnapshotDiff> {
        (**self).compare_snapshots(base_snapshot_id, target_snapshot_id)
    }

    fn get_c4_view(&self, project_id: &str, level: u8) -> Result<crate::db::queries::C4View> {
        (**self).get_c4_view(project_id, level)
    }
}

#[cfg(test)]
mod from_arc_tests {
    use super::*;
    use crate::db::DbPool;

    fn make_pool() -> DbPool {
        let pool = DbPool::in_memory().expect("in-memory pool");
        pool.init_schema().expect("schema init");
        pool
    }

    #[test]
    fn scan_repository_adapter_from_arc_keeps_pool_alive() {
        let pool = std::sync::Arc::new(make_pool());
        let adapter = ScanRepositoryAdapter::from_arc(pool.clone());
        // Round-trip a project metadata to prove the inner ProjectRepository
        // can actually talk to the pool.
        let result = adapter.get_project_by_path("/no/such/path");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn graph_repository_adapter_from_arc_keeps_pool_alive() {
        let pool = std::sync::Arc::new(make_pool());
        let adapter = GraphRepositoryAdapter::from_arc(pool.clone());
        let result = adapter.get_graph_cache("unknown-project");
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_repository_adapter_from_arc_keeps_pool_alive() {
        let pool = std::sync::Arc::new(make_pool());
        let adapter = WorkspaceRepositoryAdapter::from_arc(pool.clone());
        let result = adapter.list_workspaces();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn analysis_repository_adapter_from_arc_keeps_pool_alive() {
        let pool = std::sync::Arc::new(make_pool());
        let weak = std::sync::Arc::downgrade(&pool);
        let adapter = AnalysisDataSourceAdapter::from_arc(pool.clone());
        drop(pool);
        // The adapter's internal Arc still holds the pool alive; the
        // Weak from the caller can still upgrade.
        assert!(weak.upgrade().is_some());
        // And the pool() method returns a usable DbPool.
        let pool_ref = adapter.pool();
        let result = pool_ref.with_connection(|conn| conn.execute_batch("SELECT 1"));
        assert!(result.is_ok());
    }

    #[test]
    fn new_constructor_still_works_for_internal_tests() {
        // Regression guard: `new(&pool)` must still work because the
        // internal engine tests (in `engine/src/services/`) and the
        // existing integration tests in `engine/tests/` use it.
        let pool = make_pool();
        let _adapter = ScanRepositoryAdapter::new(&pool);
        let _adapter = GraphRepositoryAdapter::new(&pool);
        let _adapter = WorkspaceRepositoryAdapter::new(&pool);
        let _adapter = AnalysisDataSourceAdapter::new(&pool);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hexagonal ports — Clock, IdGenerator, Stopwatch (wave 2)
// ─────────────────────────────────────────────────────────────────────────────

pub mod hexagonal;

// Re-export the three port traits and their adapters at the ports module level
// so callers can use `engine::ports::{Clock, IdGenerator, Stopwatch}` without
// knowing the inner module name.
pub use hexagonal::{
    Clock, SystemClock, MockClock,
    IdGenerator, RandomIdGen, MockIdGen,
    Stopwatch, SystemStopwatch, MockStopwatch, StopwatchHandle,
};
