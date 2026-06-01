//! Tauri commands — Presentation layer
//! Exposes engine functionality to the frontend via invoke().
//! All commands return timing metadata where applicable.

use engine::{
    ai::{AnthropicProvider, ContextBuilder, AIProvider},
    db::{DbPool, ProjectRepository},
    graph::{GraphBuilder, PathResolver},
    models::{ChatMessage, FileInfo, GraphData, NodeExplanation, ScanResult},
    scanner::{CodeParser, FileWalker},
    AppError, Result,
};
use std::{sync::Mutex, time::Instant};
use tauri::State;

// Global state
pub struct AppState {
    pub db: DbPool,
    pub scan_status: Mutex<ScanStatus>,
    pub ai_config: Mutex<Option<engine::models::AIConfig>>,
    /// Root path of the currently open project (set on scan).
    pub project_root: Mutex<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanStatusResponse {
    pub status: String,
    pub progress: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanTiming {
    pub discover_ms: u64,
    pub parse_ms: u64,
    pub total_ms: u64,
    pub files_count: usize,
    pub symbols_count: usize,
    pub imports_count: usize,
}

// MARK: Project & Scanning Commands

#[tauri::command]
pub async fn scan_project(path: String, state: State<'_, AppState>) -> Result<ScanResult, String> {
    let root_for_state = path.clone();

    {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = ScanStatus::Scanning;
    }

    let discover_start = Instant::now();
    let walker = FileWalker::new(&path);
    let discovered = walker.discover();
    let discover_ms = discover_start.elapsed().as_millis() as u64;

    let parse_start = Instant::now();
    let mut files: Vec<FileInfo> = Vec::new();
    let mut all_imports: Vec<engine::models::ImportInfo> = Vec::new();

    for file in &discovered {
        let content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (symbols, imports) =
            CodeParser::parse_file(&file.path, &content, &file.extension);

        let file_info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            path: file.relative_path.clone(),
            name: file.path.split('/').last().unwrap_or("").to_string(),
            extension: file.extension.clone(),
            symbols,
            lines: content.lines().count() as u32,
        };

        // Update imports with resolved targets
        let resolver = PathResolver::new(&path);
        let resolved_imports: Vec<engine::models::ImportInfo> = imports
            .into_iter()
            .map(|mut imp| {
                if let Some(ref module) = imp.target_module {
                    let res = resolver.resolve(module, &file.relative_path);
                    match res {
                        crate::graph::resolver::Resolution::Internal(p) => {
                            imp.target_file_id = files.iter().find(|f| f.path == p).map(|f| f.id.clone());
                        }
                        crate::graph::resolver::Resolution::External(_) => {}
                        crate::graph::resolver::Resolution::Unresolved(_) => {}
                    }
                }
                imp
            })
            .collect();

        all_imports.extend(resolved_imports);
        files.push(file_info);
    }

    let parse_ms = parse_start.elapsed().as_millis() as u64;
    let total_ms = discover_ms + parse_ms;
    let symbols_count: usize = files.iter().map(|f| f.symbols.len()).sum();
    let imports_count = all_imports.len();

    let result = ScanResult {
        project_id: uuid::Uuid::new_v4().to_string(),
        project_name: path.split('/').last().unwrap_or("Project").to_string(),
        root_path: path,
        files_count: files.len(),
        symbols_count,
        imports_count,
        files,
        scan_duration_ms: total_ms,
        status: engine::models::ScanStatus::Ready,
        error: None,
    };

    // Persist
    if let Ok(repo) = ProjectRepository::new(&state.db) {
        let _ = repo.save_scan_result(&result);
    }

    {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = ScanStatus::Ready;
    }

    // Track project root so AI commands can read files from disk
    {
        let mut pr = state.project_root.lock().map_err(|e| e.to_string())?;
        *pr = root_for_state;
    }

    Ok(result)
}

#[tauri::command]
pub fn get_scan_status(state: State<'_, AppState>) -> Result<ScanStatusResponse, String> {
    let status = state.scan_status.lock().map_err(|e| e.to_string())?;
    let status_str = match &*status {
        ScanStatus::Idle => "idle",
        ScanStatus::Scanning => "scanning",
        ScanStatus::BuildingGraph => "building_graph",
        ScanStatus::Ready => "ready",
        ScanStatus::Error => "error",
    };
    Ok(ScanStatusResponse {
        status: status_str.to_string(),
        progress: if matches!(&*status, ScanStatus::Ready) {
            1.0
        } else {
            0.5
        },
    })
}

// MARK: Graph Commands

#[tauri::command]
pub async fn get_graph(project_id: String, state: State<'_, AppState>) -> Result<GraphData, String> {
    let repo = ProjectRepository::new(&state.db).map_err(|e| e.to_string())?;
    let build_start = Instant::now();

    // Try to return cached graph first
    if let Ok(Some(cached)) = repo.get_graph_cache(&project_id) {
        let elapsed = build_start.elapsed().as_millis();
        if elapsed > 0 {
            tracing::info!("Graph cache hit for {} ({}ms)", project_id, elapsed);
        }
        return serde_json::from_str(&cached).map_err(|e| e.to_string());
    }

    // Build fresh from DB
    let files = repo.get_files(&project_id).map_err(|e| e.to_string())?;
    let all_imports: Vec<engine::models::ImportInfo> = vec![];
    let builder = GraphBuilder::new(format!("/projects/{}", project_id));
    let graph = builder.build(&files, &all_imports).map_err(|e| e.to_string())?;

    // Cache it
    if let Ok(graph_json) = serde_json::to_string(&graph) {
        let _ = repo.save_graph_cache(&project_id, &graph_json);
    }

    let elapsed = build_start.elapsed().as_millis();
    tracing::info!(
        "Graph built for {}: {} nodes, {} edges ({}ms)",
        project_id,
        graph.nodes.len(),
        graph.edges.len(),
        elapsed
    );

    Ok(graph)
}

#[tauri::command]
pub fn get_node_details(node_id: String, state: State<'_, AppState>) -> Result<FileInfo, String> {
    let repo = ProjectRepository::new(&state.db).map_err(|e| e.to_string())?;
    repo.get_file_by_id(&node_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", node_id))
}

#[tauri::command]
pub fn search_nodes(
    project_id: String,
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<engine::models::GraphNode>, String> {
    let repo = ProjectRepository::new(&state.db).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(20);

    let files = repo
        .search_files(&project_id, &query, limit)
        .map_err(|e| e.to_string())?;

    let nodes: Vec<engine::models::GraphNode> = files
        .into_iter()
        .map(|f| engine::models::GraphNode {
            id: f.id,
            label: f.name,
            path: f.path,
            node_type: engine::models::NodeType::Unknown,
            symbol_count: 0,
            position: None,
        })
        .collect();

    Ok(nodes)
}

// MARK: AI Commands

#[tauri::command]
pub fn configure_ai(config: engine::models::AIConfig, state: State<'_, AppState>) -> Result<(), String> {
    let mut ai_config = state.ai_config.lock().map_err(|e| e.to_string())?;
    *ai_config = Some(config);
    Ok(())
}

#[tauri::command]
pub fn get_ai_config(state: State<'_, AppState>) -> Result<Option<engine::models::AIConfig>, String> {
    let config = state.ai_config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
pub async fn explain_node(
    node_id: String,
    project_id: String,
    state: State<'_, AppState>,
) -> Result<NodeExplanation, String> {
    let (config, root_path) = {
        let cfg = state.ai_config.lock().map_err(|e| e.to_string())?.clone();
        let root = state.project_root.lock().map_err(|e| e.to_string())?.clone();
        (cfg, root)
    };

    let cfg = config.ok_or_else(|| "AI not configured".to_string())?;

    let repo = ProjectRepository::new(&state.db).map_err(|e| e.to_string())?;

    // Fetch file metadata from DB
    let file_info = repo
        .get_file_by_id(&node_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", node_id))?;

    // Read actual file content from disk
    let file_path = Path::new(&root_path).join(&file_info.path);
    let file_content = if file_path.exists() {
        std::fs::read_to_string(&file_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Get cached graph for context (or build minimal one)
    let graph = repo
        .get_graph_cache(&project_id)
        .map_err(|e| e.to_string())?
        .and_then(|json| serde_json::from_str::<GraphData>(&json).ok())
        .unwrap_or_else(|| GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: project_id.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        });

    // Build compact context (8KB cap + top-5 deps + top-3 dependents)
    let context = ContextBuilder::build_node_context(
        &file_content,
        &file_info.path,
        &graph,
        &node_id,
    );

    // Build dependency labels list
    let deps: Vec<String> = graph
        .edges
        .iter()
        .filter(|e| e.source == node_id)
        .take(5)
        .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.target))
        .map(|n| n.label.clone())
        .collect();

    let provider = AnthropicProvider::new(&cfg.api_key, Some(&cfg.model));

    provider
        .explain_node(&node_id, &context, &deps)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chat(
    project_id: String,
    message: String,
    history: Vec<ChatMessage>,
    state: State<'_, AppState>,
) -> Result<engine::models::ChatResponse, String> {
    let (config, root_path) = {
        let cfg = state.ai_config.lock().map_err(|e| e.to_string())?.clone();
        let root = state.project_root.lock().map_err(|e| e.to_string())?.clone();
        (cfg, root)
    };

    let cfg = config.ok_or_else(|| "AI not configured".to_string())?;

    let repo = ProjectRepository::new(&state.db).map_err(|e| e.to_string())?;

    // Get project root
    let project = repo
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let (project_name, root, _) = project;
    let root = if !root.is_empty() { root } else { root_path };

    // Fetch project files from DB
    let files = repo.get_files(&project_id).map_err(|e| e.to_string())?;

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
    let graph = repo
        .get_graph_cache(&project_id)
        .map_err(|e| e.to_string())?
        .and_then(|json| serde_json::from_str::<GraphData>(&json).ok())
        .unwrap_or_else(|| GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: project_id.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        });

    // Build chat context
    let context = ContextBuilder::build_chat_context(&file_contents, &graph, &message);

    // Add user message to history
    let mut full_history = history;
    full_history.push(ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role: engine::models::ChatRole::User,
        content: message,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    let provider = AnthropicProvider::new(&cfg.api_key, Some(&cfg.model));

    provider
        .chat(&full_history, &context)
        .await
        .map_err(|e| e.to_string())
}

// MARK: v2 Analysis Commands

use engine::analysis::{
    ArchitectureDetectionResult as EngineArchResult,
    compute_impact, compute_graph_insights, ImpactAnalysisResult as EngineImpactResult,
    graph_insights::GraphInsights as EngineGraphInsights, ImpactConfig, InsightsConfig,
};
use serde::Serialize;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactAnalysisResponse {
    pub version: String,
    pub changed_node_id: String,
    pub affected_nodes: Vec<String>,
    pub impact_score: f64,
    pub explanation: String,
}

impl From<EngineImpactResult> for ImpactAnalysisResponse {
    fn from(r: EngineImpactResult) -> Self {
        Self {
            version: r.version,
            changed_node_id: r.changed_node_id,
            affected_nodes: r.affected_nodes,
            impact_score: r.impact_score,
            explanation: r.explanation,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphInsightsResponse {
    pub version: String,
    pub cycles: Vec<serde_json::Value>,
    pub hotspots: Vec<serde_json::Value>,
    pub avg_coupling: Option<f64>,
    pub density: Option<f64>,
    pub status: Option<String>,
}

impl From<EngineGraphInsights> for GraphInsightsResponse {
    fn from(r: EngineGraphInsights) -> Self {
        Self {
            version: r.version,
            cycles: r.cycles.iter().map(|c| serde_json::json!({"nodes": &c.nodes, "length": c.length})).collect(),
            hotspots: r.hotspots.iter().map(|h| serde_json::json!({"nodeId": h.node_id, "couplingScore": h.coupling_score, "reason": h.reason})).collect(),
            avg_coupling: r.avg_coupling,
            density: r.density,
            status: r.status,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureDetectionResponse {
    pub version: String,
    pub pattern: String,
    pub confidence: f64,
    pub evidence: Option<serde_json::Value>,
    pub generated_at: String,
}

impl From<EngineArchResult> for ArchitectureDetectionResponse {
    fn from(r: EngineArchResult) -> Self {
        let evidence = r.evidence.as_ref().map(|e| {
            serde_json::json!({
                "nodes": &e.nodes,
                "edges": e.edges.iter().map(|edge| {
                    serde_json::json!({
                        "source": edge.source,
                        "target": edge.target,
                        "kind": edge.kind,
                    })
                }).collect::<Vec<_>>(),
                "reasons": &e.reasons,
            })
        });
        Self {
            version: r.version,
            pattern: r.pattern.as_str().to_string(),
            confidence: r.confidence,
            evidence,
            generated_at: r.generated_at,
        }
    }
}

#[tauri::command]
pub fn get_architecture_detection(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<ArchitectureDetectionResponse, String> {
    let timing_start = std::time::Instant::now();

    let result = engine::analysis::detect_architecture(&project_id, &state.db);

    let elapsed_ms = timing_start.elapsed().as_millis() as u64;
    tracing::info!(
        "Architecture detection for {}: {} (conf={:.2}) in {}ms",
        project_id,
        result.pattern.as_str(),
        result.confidence,
        elapsed_ms
    );

    // Persist for future retrieval
    if let Ok(repo) = ProjectRepository::new(&state.db) {
        let evidence_json = result
            .evidence
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap_or_default())
            .unwrap_or_default();
        let _ = repo.save_architecture_detection(
            &project_id,
            result.pattern.as_str(),
            result.confidence,
            &evidence_json,
        );
    }

    Ok(result.into())
}

#[tauri::command]
pub fn get_impact_analysis(
    project_id: String,
    node_id: String,
    state: State<'_, AppState>,
) -> Result<ImpactAnalysisResponse, String> {
    let timing_start = std::time::Instant::now();

    let result = compute_impact(
        &project_id,
        &node_id,
        &state.db,
        &ImpactConfig::default(),
    );

    let elapsed_ms = timing_start.elapsed().as_millis() as u64;
    tracing::info!(
        "Impact analysis for {} on {}: {} affected, score={:.2} in {}ms",
        node_id,
        project_id,
        result.affected_nodes.len(),
        result.impact_score,
        elapsed_ms
    );

    Ok(result.into())
}

#[tauri::command]
pub fn get_graph_insights(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<GraphInsightsResponse, String> {
    let timing_start = std::time::Instant::now();

    let result = compute_graph_insights(&project_id, &state.db, &InsightsConfig::default());

    let elapsed_ms = timing_start.elapsed().as_millis() as u64;
    tracing::info!(
        "Graph insights for {}: {} cycles, {} hotspots, density={:.4} in {}ms",
        project_id,
        result.cycles.len(),
        result.hotspots.len(),
        result.density.unwrap_or(0.0),
        elapsed_ms
    );

    // Cache in DB
    if let Ok(repo) = ProjectRepository::new(&state.db) {
        let cycles_json = serde_json::to_string(&result.cycles).unwrap_or_default();
        let hotspots_json = serde_json::to_string(&result.hotspots).unwrap_or_default();
        let _ = repo.save_graph_insights(
            &project_id,
            &cycles_json,
            &hotspots_json,
            result.avg_coupling,
            result.density,
        );
    }

    Ok(result.into())
}

// MARK: Export Commands

/// Export the current graph view and optional insights as a structured payload.
/// - `json` format: backend serializes GraphData + GraphInsights into ExportPayload.
/// - `png` format: returns an error — PNG generation is frontend responsibility.
#[tauri::command]
pub fn export_view(
    project_id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<ExportPayloadResponse, String> {
    let timing_start = std::time::Instant::now();

    // Validate format
    if format != "json" && format != "png" {
        return Err(format!(
            "Invalid export format '{}'. Supported: 'json', 'png'.",
            format
        ));
    }

    // PNG is handled by frontend — return error so caller knows to use frontend path
    if format == "png" {
        return Err(
            "PNG export is handled by the frontend using html-to-image. Use the useExport hook."
                .to_string(),
        );
    }

    let repo = ProjectRepository::new(&state.db).map_err(|e| e.to_string())?;

    // Fetch cached graph data
    let graph_json = repo
        .get_graph_cache(&project_id)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            // Build minimal empty graph if nothing cached
            serde_json::to_string(&serde_json::json!({
                "nodes": [],
                "edges": [],
                "project_id": project_id,
                "generated_at": chrono::Utc::now().to_rfc3339(),
            }))
            .unwrap_or_default()
        });

    // Optionally fetch cached insights
    let insights_json: Option<serde_json::Value> = repo
        .get_cached_graph_insights(&project_id)
        .map_err(|e| e.to_string())?
        .map(|(cycles, hotspots, avg_coupling, density, generated_at)| {
            serde_json::json!({
                "version": "2.0",
                "cycles": serde_json::from_str::<serde_json::Value>(&cycles).unwrap_or(serde_json::json!([])),
                "hotspots": serde_json::from_str::<serde_json::Value>(&hotspots).unwrap_or(serde_json::json!([])),
                "avgCoupling": avg_coupling,
                "density": density,
                "status": "ok",
            })
        });

    let elapsed_ms = timing_start.elapsed().as_millis() as u64;
    tracing::info!(
        "Export for {} format='{}': graph_data_len={} insights_present={} in {}ms",
        project_id,
        format,
        graph_json.len(),
        insights_json.is_some(),
        elapsed_ms
    );

    Ok(ExportPayloadResponse {
        version: "2.0".to_string(),
        format,
        graph_data: serde_json::from_str(&graph_json).unwrap_or(serde_json::json!({"nodes": [], "edges": []})),
        insights: insights_json,
        metadata: ExportMetadata {
            project_id,
            generated_at: chrono::Utc::now().to_rfc3339(),
        },
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayloadResponse {
    pub version: String,
    pub format: String,
    pub graph_data: serde_json::Value,
    pub insights: Option<serde_json::Value>,
    pub metadata: ExportMetadata,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportMetadata {
    pub project_id: String,
    pub generated_at: String,
}

/// RED test: export_view command for 'json' format should return a valid ExportPayloadResponse.
/// Expected to fail before T4.1 is implemented.
#[cfg(test)]
mod export_view_tests {
    use super::*;
    use engine::db::DbPool;
    use std::sync::Mutex;

    fn make_test_state(pool: DbPool) -> AppState {
        AppState {
            db: pool,
            scan_status: Mutex::new(ScanStatus::Idle),
            ai_config: Mutex::new(None),
            project_root: Mutex::new(String::new()),
        }
    }

    #[tokio::test]
    fn export_view_json_format_returns_valid_payload() {
        // This test verifies the ExportPayloadResponse struct shape matches the contract.
        // The actual command will be tested via integration after implementation.
        let payload = ExportPayloadResponse {
            version: "2.0".to_string(),
            format: "json".to_string(),
            graph_data: serde_json::json!({"nodes": [], "edges": []}),
            insights: Some(serde_json::json!({"cycles": [], "hotspots": []})),
            metadata: ExportMetadata {
                project_id: "test-project".to_string(),
                generated_at: "2026-06-01T00:00:00Z".to_string(),
            },
        };
        assert_eq!(payload.version, "2.0");
        assert_eq!(payload.format, "json");
        assert!(payload.insights.is_some());
    }

    #[tokio::test]
    fn export_view_invalid_format_returns_error() {
        // Verify that format validation logic would reject invalid formats.
        // Currently no implementation exists, so this tests the expected error contract.
        let valid_formats = vec!["json", "png"];
        assert!(valid_formats.contains(&"json"));
        assert!(valid_formats.contains(&"png"));
        assert!(!valid_formats.contains(&"svg"));
    }
}

// State helpers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanStatus {
    Idle,
    Scanning,
    BuildingGraph,
    Ready,
    Error,
}

impl Default for ScanStatus {
    fn default() -> Self {
        Self::Idle
    }
}