//! Tauri commands — Presentation layer
//! Exposes engine functionality to the frontend via invoke().
//! All commands return timing metadata where applicable.

use engine::{
    models::{
        ChatMessage, FileInfo, GraphData, NodeExplanation, OutlineItem, ScanResult, ScanStatus,
    },
    ports::AppStatePortAdapter,
    services::{AnalysisService, GraphService, ScanService},
};

use crate::ipc_error::to_ipc_error;

use std::sync::{Arc, Mutex};
use tauri::State;

// ─── Global state ──────────────────────────────────────────────────────────

/// Tauri-managed global state, accessible to every command via
/// `tauri::State<'_, AppState>`.
///
/// All mutable fields use `Arc<Mutex<T>>` rather than plain `Mutex<T>` so
/// that the `AppStatePortAdapter` (which lives in the `engine` crate) can
/// hold `Arc::clone()` references to the SAME mutexes this struct owns.
/// Mutations through the adapter are therefore immediately visible to
/// the original `AppState` — no copies, no dead state. If these fields
/// were plain `Mutex<T>`, the adapter would receive independent copies
/// that mutate unreachable state and reads would silently diverge.
///
/// Lifetime contract:
/// - The `Arc` instances are created exactly once in `lib.rs::run()` and
///   live for the entire application lifetime.
/// - The `AppState` itself is moved into Tauri's `State` and shared with
///   every command invocation; commands MUST hold the lock only for the
///   duration of their work and MUST NOT store the lock guard.
/// - `Send + Sync` is auto-derived: every `T` stored in a `Mutex<T>` is
///   required to be `Send`, and `Arc<Mutex<T>>: Send + Sync` whenever
///   `T: Send`. No `unsafe impl` is needed and none is used.
///
/// `ai_service_port` is the port-trait reference through which the
/// presentation layer consumes AI functionality. Commands delegate to
/// it without depending on the concrete `AIService` struct.
pub struct AppState {
    pub scan_status: Arc<Mutex<ScanStatus>>,
    pub ai_config: Arc<Mutex<Option<engine::models::AIConfig>>>,
    /// Root path of the currently open project (set on scan).
    pub project_root: Arc<Mutex<String>>,
    /// AI service exposed through the `AIServicePort` port trait.
    pub ai_service_port: Arc<dyn engine::ai::AIServicePort>,
    /// Graph repository port — consumed by graph commands in B.6.
    pub scan_repo: Arc<dyn engine::ports::ScanRepository>,
    /// Graph repository port — consumed by graph commands in B.6.
    pub graph_repo: Arc<dyn engine::ports::GraphRepository>,
    /// Analysis repository port — consumed by analysis commands in B.8.
    pub analysis_repo: Arc<dyn engine::ports::AnalysisDataSource>,
    /// Workspace repository port — consumed by 13 workspace commands in B.7.
    pub workspace_repo: Arc<dyn engine::ports::WorkspaceRepository>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanStatusResponse {
    pub status: String,
    pub progress: f32,
}

// MARK: Project & Scanning Commands
// Thin shims — all orchestration delegated to ScanService (engine::services::ScanService).
// State transitions, persistence, and error mapping are owned by the service.

/// Scan a project directory.
///
/// Thin shim: constructs `ScanService` from `AppState` fields and delegates to
/// `ScanService::scan_project`. The service owns the scan lifecycle:
/// - transitions `AppStatePort` through Scanning → BuildingGraph → Ready|Error
/// - owns FileWalker, ParserRegistry, PathResolver
/// - persists via `ScanRepository`
///
/// Uses `Arc<Mutex<T>>` handles so the adapter shares ownership with the real
/// `AppState` mutexes — mutations through the adapter mutate the real state.
#[tauri::command]
pub async fn scan_project(path: String, state: State<'_, AppState>) -> Result<ScanResult, String> {
    let scan_repo = state.scan_repo.clone();
    // Wrap the real AppState mutexes in Arc so the adapter and AppState share
    // the same inner data. Arc::clone() creates a new handle to the same mutex.
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = ScanService::new(scan_repo, app_state_adapter);
    service.scan_project(&path).map_err(to_ipc_error)
}

/// Reopen a previously indexed project by root path.
///
/// Thin shim: delegates to `ScanService::open_project_by_path`. The service
/// loads metadata from `ScanRepository`, hydrates files, and updates
/// `AppStatePort` with Ready status and project root.
#[tauri::command]
pub async fn open_project_by_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = ScanService::new(scan_repo, app_state_adapter);
    service.open_project_by_path(&path).map_err(to_ipc_error)
}

/// Read current scan status.
///
/// Thin shim: delegates to `ScanService::get_scan_status` and maps the
/// `ScanStatus` enum to a human-readable string response.
#[tauri::command]
pub async fn get_scan_status(state: State<'_, AppState>) -> Result<ScanStatusResponse, String> {
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = ScanService::new(scan_repo, app_state_adapter);
    let status = service.get_scan_status().map_err(to_ipc_error)?;
    let status_str = match status {
        engine::models::ScanStatus::Idle => "idle",
        engine::models::ScanStatus::Scanning => "scanning",
        engine::models::ScanStatus::BuildingGraph => "building_graph",
        engine::models::ScanStatus::Ready => "ready",
        engine::models::ScanStatus::Cancelled => "cancelled",
        engine::models::ScanStatus::Error => "error",
    };
    Ok(ScanStatusResponse {
        status: status_str.to_string(),
        progress: if matches!(status, engine::models::ScanStatus::Ready) {
            1.0
        } else {
            0.5
        },
    })
}

/// Cancel an in-progress scan.
///
/// Thin shim: constructs `ScanService` from `AppState` fields and delegates to
/// `ScanService::cancel`. Three outcomes: running scan → cancelled, completed
/// scan → no-op, unknown scan → NotFound error.
#[tauri::command]
pub async fn cancel_scan(scan_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = ScanService::new(scan_repo, app_state_adapter);
    service.cancel(&scan_id).await.map_err(to_ipc_error)
}

// MARK: Graph Commands
// Thin shims — all orchestration delegated to GraphService (engine::services::GraphService).
// Cache management, graph building, outline parsing, and state transitions are owned by the service.

/// Get the dependency graph for a project.
///
/// Thin shim: constructs `GraphService` from `AppState` fields and delegates to
/// `GraphService::get_graph`. The service owns:
/// - state transitions (BuildingGraph → Ready|Error)
/// - cache hit/miss logic
/// - GraphBuilder orchestration
/// - result caching
#[tauri::command]
pub async fn get_graph(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<GraphData, String> {
    let graph_repo = state.graph_repo.clone();
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = GraphService::new(graph_repo, scan_repo, app_state_adapter);
    service.get_graph(&project_id).map_err(to_ipc_error)
}

/// Get file metadata for a node.
///
/// Thin shim: delegates to `GraphService::get_node_details`.
#[tauri::command]
pub fn get_node_details(node_id: String, state: State<'_, AppState>) -> Result<FileInfo, String> {
    let graph_repo = state.graph_repo.clone();
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = GraphService::new(graph_repo, scan_repo, app_state_adapter);
    service.get_node_details(&node_id).map_err(to_ipc_error)
}

/// Get outline items for a node.
///
/// Thin shim: delegates to `GraphService::get_node_outline`. The service owns:
/// - cached outline fast path
/// - on-demand outline generation with source file parsing
/// - outline persistence
#[tauri::command]
pub fn get_node_outline(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<OutlineItem>, String> {
    let graph_repo = state.graph_repo.clone();
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = GraphService::new(graph_repo, scan_repo, app_state_adapter);
    service
        .get_node_outline(&node_id, None)
        .map_err(to_ipc_error)
}

/// Search files by name substring (case-insensitive).
///
/// Thin shim: delegates to `GraphService::search_nodes`.
#[tauri::command]
pub fn search_nodes(
    project_id: String,
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<engine::models::GraphNode>, String> {
    let graph_repo = state.graph_repo.clone();
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = GraphService::new(graph_repo, scan_repo, app_state_adapter);
    service
        .search_nodes(&project_id, &query, limit)
        .map_err(to_ipc_error)
}

/// Get all nodes that the given node depends on (outgoing import edges).
///
/// Thin shim: delegates to `GraphService::get_dependencies`.
#[tauri::command]
pub async fn get_dependencies(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<engine::models::NodeRef>, String> {
    let graph_repo = state.graph_repo.clone();
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = GraphService::new(graph_repo, scan_repo, app_state_adapter);
    service.get_dependencies(&node_id).await.map_err(to_ipc_error)
}

/// Get all nodes that depend on the given node (incoming import edges).
///
/// Thin shim: delegates to `GraphService::get_dependents`.
#[tauri::command]
pub async fn get_dependents(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<engine::models::NodeRef>, String> {
    let graph_repo = state.graph_repo.clone();
    let scan_repo = state.scan_repo.clone();
    let app_state_adapter = AppStatePortAdapter::from_arc_refs(
        &state.scan_status,
        &state.ai_config,
        &state.project_root,
    );
    let service = GraphService::new(graph_repo, scan_repo, app_state_adapter);
    service.get_dependents(&node_id).await.map_err(to_ipc_error)
}

// MARK: AI Commands

#[tauri::command]
pub fn configure_ai(
    config: engine::models::AIConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut ai_config = state.ai_config.lock().map_err(to_ipc_error)?;
    *ai_config = Some(config);
    Ok(())
}

#[tauri::command]
pub fn get_ai_config(
    state: State<'_, AppState>,
) -> Result<Option<engine::models::AIConfig>, String> {
    let config = state.ai_config.lock().map_err(to_ipc_error)?;
    Ok(config.clone())
}

#[tauri::command]
pub async fn explain_node(
    node_id: String,
    project_id: String,
    state: State<'_, AppState>,
) -> Result<NodeExplanation, String> {
    use std::path::Path;

    let (config, root_path) = {
        let cfg = state.ai_config.lock().map_err(to_ipc_error)?.clone();
        let root = state.project_root.lock().map_err(to_ipc_error)?.clone();
        (cfg, root)
    };

    let cfg = config.ok_or_else(|| "AI not configured".to_string())?;

    // Fetch file metadata from DB via scan_repo
    let file_info = state
        .scan_repo
        .get_file_by_id(&node_id)
        .map_err(to_ipc_error)?
        .ok_or_else(|| format!("File not found: {}", node_id))?;

    // Read actual file content from disk
    let file_path = Path::new(&root_path).join(&file_info.path);
    let file_content = if file_path.exists() {
        std::fs::read_to_string(&file_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Get cached graph for context (or build minimal one)
    let graph = state
        .graph_repo
        .get_graph_cache(&project_id)
        .map_err(to_ipc_error)?
        .and_then(|json| serde_json::from_str::<GraphData>(&json).ok())
        .unwrap_or_else(|| GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: project_id.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        });

    // Load outline items for this file (non-blocking; empty outline is fine)
    let outline = state
        .scan_repo
        .get_outline_items(&node_id)
        .unwrap_or_default();

    state
        .ai_service_port
        .explain_node_with_context(&cfg, &file_info, &file_content, &graph, &outline)
        .await
        .map_err(to_ipc_error)
}

#[tauri::command]
pub async fn chat(
    project_id: String,
    message: String,
    history: Vec<ChatMessage>,
    state: State<'_, AppState>,
) -> Result<engine::models::ChatResponse, String> {
    use std::path::Path;

    let (config, root_path) = {
        let cfg = state.ai_config.lock().map_err(to_ipc_error)?.clone();
        let root = state.project_root.lock().map_err(to_ipc_error)?.clone();
        (cfg, root)
    };

    let cfg = config.ok_or_else(|| "AI not configured".to_string())?;

    // Get project root from DB (fall back to state.project_root if not persisted)
    let root = state
        .scan_repo
        .get_project(&project_id)
        .map_err(to_ipc_error)?
        .and_then(|(_, r, _)| if r.is_empty() { None } else { Some(r) })
        .unwrap_or(root_path);

    // Fetch project files from DB
    let files = state
        .scan_repo
        .get_files(&project_id)
        .map_err(to_ipc_error)?;

    // Read file contents for context (limit to first 10 files to avoid overhead)
    let file_contents: Vec<(String, String)> = files
        .iter()
        .take(10)
        .filter_map(|f| {
            let path = Path::new(&root).join(&f.path);
            std::fs::read_to_string(&path)
                .ok()
                .map(|content| (f.path.clone(), content))
        })
        .collect();

    // Get graph for structure context
    let graph = state
        .graph_repo
        .get_graph_cache(&project_id)
        .map_err(to_ipc_error)?
        .and_then(|json| serde_json::from_str::<GraphData>(&json).ok())
        .unwrap_or_else(|| GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: project_id.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        });

    // Add user message to history
    let mut full_history = history;
    full_history.push(ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role: engine::models::ChatRole::User,
        content: message.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    state
        .ai_service_port
        .chat_with_context(
            &cfg,
            &project_id,
            &root,
            &file_contents,
            &graph,
            &full_history,
            &message,
        )
        .await
        .map_err(to_ipc_error)
}

// MARK: Analysis Commands
// Thin shims — all orchestration delegated to AnalysisService (engine::services::AnalysisService).
// Response DTOs imported from engine::services to avoid duplication.
use engine::services::{
    ArchitectureDetectionResponse, ExportPayloadResponse, GraphInsightsResponse,
    ImpactAnalysisResponse,
};

/// Thin shim: constructs AnalysisService and delegates to
/// `AnalysisService::get_architecture_detection`.
#[tauri::command]
pub fn get_architecture_detection(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<ArchitectureDetectionResponse, String> {
    let analysis_repo = state.analysis_repo.clone();
    let graph_repo = state.graph_repo.clone();
    let service = AnalysisService::new(analysis_repo, graph_repo);
    service
        .get_architecture_detection(&project_id)
        .map_err(to_ipc_error)
}

/// Thin shim: constructs AnalysisService and delegates to
/// `AnalysisService::get_impact_analysis`.
#[tauri::command]
pub fn get_impact_analysis(
    project_id: String,
    node_id: String,
    state: State<'_, AppState>,
) -> Result<ImpactAnalysisResponse, String> {
    let analysis_repo = state.analysis_repo.clone();
    let graph_repo = state.graph_repo.clone();
    let service = AnalysisService::new(analysis_repo, graph_repo);
    service
        .get_impact_analysis(&project_id, &node_id)
        .map_err(to_ipc_error)
}

/// Thin shim: constructs AnalysisService and delegates to
/// `AnalysisService::get_graph_insights`.
#[tauri::command]
pub fn get_graph_insights(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<GraphInsightsResponse, String> {
    let analysis_repo = state.analysis_repo.clone();
    let graph_repo = state.graph_repo.clone();
    let service = AnalysisService::new(analysis_repo, graph_repo);
    service
        .get_graph_insights(&project_id)
        .map_err(to_ipc_error)
}

/// Thin shim: constructs AnalysisService and delegates to
/// `AnalysisService::export_view`.
#[tauri::command]
pub fn export_view(
    project_id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<ExportPayloadResponse, String> {
    let analysis_repo = state.analysis_repo.clone();
    let graph_repo = state.graph_repo.clone();
    let service = AnalysisService::new(analysis_repo, graph_repo);
    service
        .export_view(&project_id, format)
        .map_err(to_ipc_error)
}

// ─── Observability helpers ──────────────────────────────────────────────────

// SQLite UNIQUE-constraint error parsing lives in `engine::db::error_mapping`.
// Both the service layer and the presentation layer call those helpers; do
// not re-implement the string parsing here.

mod tests;

// MARK: v3 Workspace Commands
// Thin shims — all orchestration delegated to WorkspaceService (engine::services::WorkspaceService).
// State extraction, service delegation, and error mapping are owned by the service.
// Response DTOs imported directly from engine::services to avoid duplication.

use engine::services::{
    AnnotationResponse, C4ViewResponse, ExecutiveSummaryResponse, HealthTimelineResponse,
    SnapshotDiffResponse, SnapshotResponse, WorkspaceProjectResponse, WorkspaceResponse,
    WorkspaceService,
};

/// Thin shim: constructs WorkspaceService with the trait-object port from AppState
/// and delegates. After B.7 the port is `state.workspace_repo: Arc<dyn WorkspaceRepository>`.
macro_rules! workspace_service {
    ($state:expr) => {{
        WorkspaceService::new($state.workspace_repo.clone())
    }};
}

#[tauri::command]
pub fn create_workspace(
    name: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceResponse, String> {
    workspace_service!(state)
        .create_workspace(&name)
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<WorkspaceResponse>, String> {
    workspace_service!(state)
        .list_workspaces()
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn attach_project_to_workspace(
    workspace_id: String,
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    workspace_service!(state)
        .attach_project_to_workspace(&workspace_id, &project_id)
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn list_workspace_projects(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceProjectResponse>, String> {
    workspace_service!(state)
        .list_workspace_projects(&workspace_id)
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn create_snapshot(
    project_id: String,
    label: String,
    workspace_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<SnapshotResponse, String> {
    workspace_service!(state)
        .create_snapshot(&project_id, &label, workspace_id.as_deref())
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn get_snapshot(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<Option<SnapshotResponse>, String> {
    workspace_service!(state)
        .get_snapshot(&snapshot_id)
        .map_err(to_ipc_error)
}

// ─── Annotation commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn add_comment(
    project_id: String,
    node_id: String,
    author: String,
    text: String,
    kind: Option<String>,
    state: State<'_, AppState>,
) -> Result<AnnotationResponse, String> {
    workspace_service!(state)
        .add_comment(&project_id, &node_id, &author, &text, kind.as_deref())
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn list_comments(
    project_id: String,
    node_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AnnotationResponse>, String> {
    workspace_service!(state)
        .list_comments(&project_id, node_id.as_deref())
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn list_snapshots(
    project_id: String,
    workspace_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SnapshotResponse>, String> {
    workspace_service!(state)
        .list_snapshots(&project_id, workspace_id.as_deref())
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn get_health_timeline(
    project_id: String,
    from: String,
    to: String,
    state: State<'_, AppState>,
) -> Result<HealthTimelineResponse, String> {
    workspace_service!(state)
        .get_health_timeline(&project_id, &from, &to)
        .map_err(to_ipc_error)
}

// ========================================================================
// H3 — Executive Summary + Diff + C4 Views
// ========================================================================

#[tauri::command]
pub fn get_executive_summary(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<ExecutiveSummaryResponse, String> {
    workspace_service!(state)
        .get_executive_summary(&workspace_id)
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn compare_snapshots(
    base_snapshot_id: String,
    target_snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<SnapshotDiffResponse, String> {
    workspace_service!(state)
        .compare_snapshots(&base_snapshot_id, &target_snapshot_id)
        .map_err(to_ipc_error)
}

#[tauri::command]
pub fn get_c4_view(
    project_id: String,
    level: u8,
    state: State<'_, AppState>,
) -> Result<C4ViewResponse, String> {
    workspace_service!(state)
        .get_c4_view(&project_id, level)
        .map_err(to_ipc_error)
}
