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
use crate::models::{FileInfo, ImportInfo, OutlineItem, ProjectMeta, ScanResult};
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
pub trait ScanRepository {
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
}

/// Adapter that implements `ScanRepository` by delegating to `ProjectRepository`.
///
/// This adapter is additive: it wraps the existing `ProjectRepository` without
/// modifying its internal structure. `queries.rs` stays intact.
pub struct ScanRepositoryAdapter<'pool> {
    inner: crate::db::queries::ProjectRepository<'pool>,
}

impl<'pool> ScanRepositoryAdapter<'pool> {
    pub fn new(pool: &'pool DbPool) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::new(pool),
        }
    }
}

impl<'pool> ScanRepository for ScanRepositoryAdapter<'pool> {
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
}

// ─────────────────────────────────────────────────────────────────────────────
// GraphRepository — graph cache and file search
// ─────────────────────────────────────────────────────────────────────────────

/// Port for graph operations.
///
/// Abstracts graph cache persistence and file search so that application
/// services (e.g., `GraphService`) are decoupled from the concrete
/// `ProjectRepository` implementation.
pub trait GraphRepository {
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
}

/// Adapter that implements `GraphRepository` by delegating to `ProjectRepository`.
///
/// Additive wrapper: does not refactor the internal structure of `queries.rs`.
pub struct GraphRepositoryAdapter<'pool> {
    inner: crate::db::queries::ProjectRepository<'pool>,
}

impl<'pool> GraphRepositoryAdapter<'pool> {
    pub fn new(pool: &'pool DbPool) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::new(pool),
        }
    }
}

impl<'pool> GraphRepository for GraphRepositoryAdapter<'pool> {
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
pub trait WorkspaceRepository {
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
pub struct WorkspaceRepositoryAdapter<'pool> {
    inner: crate::db::queries::ProjectRepository<'pool>,
}

impl<'pool> WorkspaceRepositoryAdapter<'pool> {
    pub fn new(pool: &'pool DbPool) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::new(pool),
        }
    }
}

impl<'pool> WorkspaceRepository for WorkspaceRepositoryAdapter<'pool> {
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
// AnalysisRepository — analysis persistence
// ─────────────────────────────────────────────────────────────────────────────

/// Port for analysis persistence operations.
///
/// Abstracts the persistence of architecture detection results and graph
/// insights so that `AnalysisService` is decoupled from the concrete
/// `ProjectRepository` implementation.
pub trait AnalysisRepository {
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

/// Adapter that implements `AnalysisRepository` by delegating to `ProjectRepository`.
pub struct AnalysisRepositoryAdapter<'pool> {
    pool: &'pool crate::db::DbPool,
    inner: crate::db::queries::ProjectRepository<'pool>,
}

impl<'pool> AnalysisRepositoryAdapter<'pool> {
    pub fn new(pool: &'pool crate::db::DbPool) -> Self {
        Self {
            pool,
            inner: crate::db::queries::ProjectRepository::new(pool),
        }
    }
}

impl<'pool> AnalysisRepository for AnalysisRepositoryAdapter<'pool> {
    fn pool(&self) -> &crate::db::DbPool {
        self.pool
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
