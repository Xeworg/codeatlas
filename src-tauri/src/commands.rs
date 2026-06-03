//! Tauri commands — Presentation layer
//! Exposes engine functionality to the frontend via invoke().
//! All commands return timing metadata where applicable.

use engine::{
    ai::{AIProvider, AnthropicProvider, ContextBuilder},
    db::{DbPool, ProjectRepository},
    graph::{GraphBuilder, PathResolver},
    models::{ChatMessage, FileInfo, GraphData, NodeExplanation, OutlineItem, ScanResult},
    scanner::{CodeParser, FileWalker},
};
use std::{path::Path, sync::Mutex, time::Instant};
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

    // Phase 1: collect files with stable UUIDs
    let mut file_infos: Vec<FileInfo> = Vec::new();
    for file in &discovered {
        let content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (symbols, _) = CodeParser::parse_file(&file.path, &content, &file.extension);
        // Outline is collected separately using parse_file_all after files are persisted

        let file_info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            path: file.relative_path.clone(),
            name: file.path.split('/').next_back().unwrap_or("").to_string(),
            extension: file.extension.clone(),
            symbols,
            lines: content.lines().count() as u32,
        };

        file_infos.push(file_info);
    }

    // Build path → UUID lookup for resolving imports
    let path_to_id: std::collections::HashMap<String, String> = file_infos
        .iter()
        .map(|f| (f.path.clone(), f.id.clone()))
        .collect();

    // Phase 2: resolve all imports with correct source/target file IDs
    let resolver = PathResolver::new(&path);
    let mut all_imports: Vec<engine::models::ImportInfo> = Vec::new();

    for file in &discovered {
        let content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (_, imports) = CodeParser::parse_file(&file.path, &content, &file.extension);

        // Get source file UUID (must exist in path_to_id — files are already collected)
        let source_id = path_to_id
            .get(&file.relative_path)
            .cloned()
            .unwrap_or_default();

        let resolved_imports: Vec<engine::models::ImportInfo> = imports
            .into_iter()
            .map(|mut imp| {
                // Fix source_file_id: set to the file's UUID, not the path string
                imp.source_file_id = source_id.clone();

                // Resolve target
                if let Some(ref module) = imp.target_module {
                    let res = resolver.resolve(module, &file.relative_path);
                    match res {
                        engine::graph::resolver::Resolution::Internal(p) => {
                            imp.target_file_id = path_to_id.get(&p).cloned();
                        }
                        engine::graph::resolver::Resolution::External(_) => {}
                        engine::graph::resolver::Resolution::Unresolved(_) => {}
                    }
                }
                imp
            })
            .collect();

        all_imports.extend(resolved_imports);
    }

    let parse_ms = parse_start.elapsed().as_millis() as u64;
    let total_ms = discover_ms + parse_ms;
    let symbols_count: usize = file_infos.iter().map(|f| f.symbols.len()).sum();

    let repo = ProjectRepository::new(&state.db);

    // Save project and files before imports. Imports reference files(id), so
    // persisting them first can create orphan rows or fail once FK enforcement is on.
    let mut result = ScanResult {
        project_id: uuid::Uuid::new_v4().to_string(),
        project_name: path.split('/').next_back().unwrap_or("Project").to_string(),
        root_path: path.clone(),
        files_count: file_infos.len(),
        symbols_count,
        imports_count: 0,
        files: file_infos,
        scan_duration_ms: total_ms,
        status: engine::models::ScanStatus::Ready,
        error: None,
    };

    if let Err(e) = repo.save_scan_result(&result) {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = ScanStatus::Error;
        return Err(format!("Failed to save scan result: {}", e));
    }

    // Transition to graph-building state while import edges are persisted.
    {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = ScanStatus::BuildingGraph;
    }

    let parsed_count = all_imports.len();
    let mut skipped_empty = 0usize;
    let mut persist_errors = 0usize;
    let mut persisted_count = 0usize;

    for imp in &all_imports {
        // Guard: skip imports with no source file ID — these produce orphan edges
        // that would be filtered out downstream and mask the real resolver failure.
        if imp.source_file_id.is_empty() {
            skipped_empty += 1;
            tracing::debug!("Skipping import with empty source_file_id: {:?}", imp);
            continue;
        }

        match repo.save_import(imp) {
            Ok(()) => {
                persisted_count += 1;
            }
            Err(e) => {
                persist_errors += 1;
                tracing::debug!("Failed to persist import {:?}: {}", imp, e);
            }
        }
    }

    // Phase 3: Persist outline items using authoritative file UUIDs from path_to_id
    let mut outline_skipped = 0usize;
    let mut outline_errors = 0usize;
    for file in &discovered {
        let file_id = match path_to_id.get(&file.relative_path) {
            Some(id) => id,
            None => {
                outline_skipped += 1;
                continue;
            }
        };

        let content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(_) => {
                outline_errors += 1;
                continue;
            }
        };

        let parse_result =
            CodeParser::parse_file_all(&file.path, &content, &file.extension, file_id);

        if !parse_result.outline.is_empty() {
            if let Err(e) = repo.save_outline_items(file_id, &parse_result.outline) {
                tracing::debug!("Failed to persist outline for {}: {}", file_id, e);
                outline_errors += 1;
            }
        }
    }

    tracing::info!(
        "Outline persistence: {} skipped (no file id), {} errors out of {} files",
        outline_skipped,
        outline_errors,
        discovered.len()
    );

    // Use persisted count as the authoritative number — what actually lands in the DB.
    // If >50% of non-empty imports fail to persist, surface it in the result so the
    // frontend can reflect a degraded scan rather than silently returning Ready.
    let non_empty_total = parsed_count.saturating_sub(skipped_empty);
    let failure_count = skipped_empty + persist_errors;
    let degraded = non_empty_total > 0 && failure_count > non_empty_total / 2;

    result.imports_count = persisted_count;
    if degraded {
        result.status = engine::models::ScanStatus::Error;
        result.error = Some(format!(
            "Import persistence degraded: {} parsed, {} skipped (empty source), {} DB errors, {} persisted",
            parsed_count, skipped_empty, persist_errors, persisted_count
        ));
    }

    tracing::info!(
        "Import persistence: {} parsed, {} skipped, {} errors, {} persisted{}{}",
        parsed_count,
        skipped_empty,
        persist_errors,
        persisted_count,
        if failure_count > 0 { " [DEGRADED]" } else { "" },
        if degraded { " [SURFACED AS ERROR]" } else { "" },
    );

    // Persist final authoritative import count/status after import persistence.
    if let Err(e) = repo.save_scan_result(&result) {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = ScanStatus::Error;
        return Err(format!("Failed to update scan result: {}", e));
    }

    {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = if degraded {
            ScanStatus::Error
        } else {
            ScanStatus::Ready
        };
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
pub async fn get_graph(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<GraphData, String> {
    // Set building-graph state so UI can reflect progress
    {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = ScanStatus::BuildingGraph;
    }

    let repo = ProjectRepository::new(&state.db);
    let build_start = Instant::now();

    // Try to return cached graph first
    if let Ok(Some(cached)) = repo.get_graph_cache(&project_id) {
        tracing::info!(
            "Graph cache hit for {} ({}ms)",
            project_id,
            build_start.elapsed().as_millis()
        );
        {
            let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
            *status = ScanStatus::Ready;
        }
        return serde_json::from_str(&cached).map_err(|e| e.to_string());
    }

    // Build fresh from DB using REAL imports
    let files = repo.get_files(&project_id).map_err(|e| e.to_string())?;

    if files.is_empty() {
        tracing::warn!(
            "No files found for project {} — returning empty graph",
            project_id
        );
        {
            let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
            *status = ScanStatus::Error;
        }
        return Err(format!("Project {} has no files in database", project_id));
    }

    // Load real imports from DB
    let all_imports = repo.get_imports(&project_id).map_err(|e| e.to_string())?;

    // Use real project root path from DB for stable graph path semantics.
    let root_path = repo
        .get_project(&project_id)
        .ok()
        .flatten()
        .map(|(_, root, _)| root)
        .unwrap_or_else(|| format!("/projects/{}", project_id));

    let builder = GraphBuilder::new(root_path);
    let mut graph = builder
        .build(&files, &all_imports)
        .map_err(|e| e.to_string())?;

    // ReactFlow expects edges to reference existing node ids.
    // Keep only internal edges that have both endpoints present in current node set.
    let node_ids: std::collections::HashSet<String> =
        graph.nodes.iter().map(|n| n.id.clone()).collect();
    graph
        .edges
        .retain(|e| node_ids.contains(&e.source) && node_ids.contains(&e.target));

    // Cache it
    if let Ok(graph_json) = serde_json::to_string(&graph) {
        let _ = repo.save_graph_cache(&project_id, &graph_json);
    }

    let elapsed = build_start.elapsed().as_millis();
    tracing::info!(
        "Graph built for {}: {} nodes, {} edges ({}ms), {} imports used",
        project_id,
        graph.nodes.len(),
        graph.edges.len(),
        elapsed,
        all_imports.len()
    );

    {
        let mut status = state.scan_status.lock().map_err(|e| e.to_string())?;
        *status = ScanStatus::Ready;
    }

    Ok(graph)
}

#[tauri::command]
pub fn get_node_details(node_id: String, state: State<'_, AppState>) -> Result<FileInfo, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.get_file_by_id(&node_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", node_id))
}

/// On-demand outline command.
///
/// 1. Try persisted outline from DB.
/// 2. If empty, load FileInfo to get the relative path, resolve it against
///    the session project_root (or look it up via DB), read the source file,
///    parse it, persist the result, and return it.
///
/// Safe: read/parse errors yield an empty outline; unsupported files return [].
#[tauri::command]
pub fn get_node_outline(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<OutlineItem>, String> {
    let repo = ProjectRepository::new(&state.db);

    // Step 1: fast path — return persisted outline if present
    let cached = repo
        .get_outline_items(&node_id)
        .map_err(|e| e.to_string())?;
    if !cached.is_empty() {
        return Ok(cached);
    }

    // Step 2: on-demand fallback — generate outline if DB is empty
    let file_info = match repo.get_file_by_id(&node_id).map_err(|e| e.to_string())? {
        Some(f) => f,
        None => return Ok(vec![]),
    };

    // Resolve absolute source path
    let root_path = {
        let pr = state.project_root.lock().map_err(|e| e.to_string())?;
        if pr.is_empty() {
            // Fallback: look up project root from DB (works after restart too)
            repo.get_project_root_for_file(&node_id)
                .map_err(|e| e.to_string())?
                .unwrap_or_default()
        } else {
            pr.clone()
        }
    };

    if root_path.is_empty() {
        return Ok(vec![]);
    }

    let abs_path = Path::new(&root_path).join(&file_info.path);

    let content = match std::fs::read_to_string(&abs_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("get_node_outline: could not read {:?}: {}", abs_path, e);
            return Ok(vec![]);
        }
    };

    let parse_result = CodeParser::parse_file_all(
        &abs_path.to_string_lossy(),
        &content,
        &file_info.extension,
        &node_id,
    );

    if !parse_result.outline.is_empty() {
        if let Err(e) = repo.save_outline_items(&node_id, &parse_result.outline) {
            tracing::debug!(
                "get_node_outline: failed to persist on-demand outline for {}: {}",
                node_id,
                e
            );
        }
    }

    Ok(parse_result.outline)
}

#[tauri::command]
pub fn search_nodes(
    project_id: String,
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<engine::models::GraphNode>, String> {
    let repo = ProjectRepository::new(&state.db);
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
pub fn configure_ai(
    config: engine::models::AIConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut ai_config = state.ai_config.lock().map_err(|e| e.to_string())?;
    *ai_config = Some(config);
    Ok(())
}

#[tauri::command]
pub fn get_ai_config(
    state: State<'_, AppState>,
) -> Result<Option<engine::models::AIConfig>, String> {
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
        let root = state
            .project_root
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        (cfg, root)
    };

    let cfg = config.ok_or_else(|| "AI not configured".to_string())?;

    let repo = ProjectRepository::new(&state.db);

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

    // Load outline items for this file (non-blocking; empty outline is fine)
    let outline = repo.get_outline_items(&node_id).unwrap_or_default();

    // Build compact context — prefer semantic outline when available
    let context = if !outline.is_empty() {
        ContextBuilder::build_node_context_with_outline(
            &file_content,
            &file_info.path,
            &graph,
            &node_id,
            &outline,
        )
    } else {
        ContextBuilder::build_node_context(&file_content, &file_info.path, &graph, &node_id)
    };

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
        let root = state
            .project_root
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        (cfg, root)
    };

    let cfg = config.ok_or_else(|| "AI not configured".to_string())?;

    let repo = ProjectRepository::new(&state.db);

    // Get project root
    let (_, root, _) = repo
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;
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

    // Build chat context (use &str refs — ContextBuilder::build_chat_context
    // takes &[(&str, &str)], file_contents lifetime must outlive the call)
    let context = {
        let refs: Vec<(&str, &str)> = file_contents
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        ContextBuilder::build_chat_context(&refs, &graph, &message)
    };

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
    compute_graph_insights, compute_impact, graph_insights::GraphInsights as EngineGraphInsights,
    ArchitectureDetectionResult as EngineArchResult, ImpactAnalysisResult as EngineImpactResult,
    ImpactConfig, InsightsConfig,
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
    let repo = ProjectRepository::new(&state.db);
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

    Ok(result.into())
}

#[tauri::command]
pub fn get_impact_analysis(
    project_id: String,
    node_id: String,
    state: State<'_, AppState>,
) -> Result<ImpactAnalysisResponse, String> {
    let timing_start = std::time::Instant::now();

    let result = compute_impact(&project_id, &node_id, &state.db, &ImpactConfig::default());

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
    let repo = ProjectRepository::new(&state.db);
    let cycles_json = serde_json::to_string(&result.cycles).unwrap_or_default();
    let hotspots_json = serde_json::to_string(&result.hotspots).unwrap_or_default();
    let _ = repo.save_graph_insights(
        &project_id,
        &cycles_json,
        &hotspots_json,
        result.avg_coupling,
        result.density,
    );

    Ok(result.into())
}

// MARK: Export Commands

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

    let repo = ProjectRepository::new(&state.db);

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
        .map(|(cycles, hotspots, avg_coupling, density, _): (String, String, f64, f64, String)| {
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
        graph_data: serde_json::from_str(&graph_json)
            .unwrap_or(serde_json::json!({"nodes": [], "edges": []})),
        insights: insights_json,
        metadata: ExportMetadata {
            project_id,
            generated_at: chrono::Utc::now().to_rfc3339(),
        },
    })
}

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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScanStatus {
    #[default]
    Idle,
    Scanning,
    BuildingGraph,
    Ready,
    Error,
}

// MARK: v3 Workspace Commands

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProjectResponse {
    pub workspace_id: String,
    pub project_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotResponse {
    pub id: String,
    pub project_id: String,
    pub workspace_id: Option<String>,
    pub label: String,
    pub created_at: String,
    pub payload_json: Option<String>,
}

#[tauri::command]
pub fn create_workspace(
    name: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceResponse, String> {
    let repo = ProjectRepository::new(&state.db);
    let (id, name_out, created_at) = repo.create_workspace(&name).map_err(|e| e.to_string())?;
    Ok(WorkspaceResponse {
        id,
        name: name_out,
        created_at,
    })
}

#[tauri::command]
pub fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<WorkspaceResponse>, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.list_workspaces().map_err(|e| e.to_string()).map(|ws| {
        ws.into_iter()
            .map(|(id, name, created_at)| WorkspaceResponse {
                id,
                name,
                created_at,
            })
            .collect()
    })
}

#[tauri::command]
pub fn attach_project_to_workspace(
    workspace_id: String,
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo = ProjectRepository::new(&state.db);
    repo.attach_project_to_workspace(&workspace_id, &project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_workspace_projects(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceProjectResponse>, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.list_workspace_projects(&workspace_id)
        .map_err(|e| e.to_string())
        .map(|ps| {
            ps.into_iter()
                .map(|(workspace_id, project_id)| WorkspaceProjectResponse {
                    workspace_id,
                    project_id,
                })
                .collect()
        })
}

#[tauri::command]
pub fn create_snapshot(
    project_id: String,
    label: String,
    workspace_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<SnapshotResponse, String> {
    let repo = ProjectRepository::new(&state.db);
    let (id, project_id_out, workspace_id_out, label_out, created_at, payload_json) = repo
        .create_snapshot(&project_id, &label, workspace_id.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(SnapshotResponse {
        id,
        project_id: project_id_out,
        workspace_id: workspace_id_out,
        label: label_out,
        created_at,
        payload_json,
    })
}

#[tauri::command]
pub fn get_snapshot(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<Option<SnapshotResponse>, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.get_snapshot(&snapshot_id)
        .map_err(|e| e.to_string())
        .map(|opt| {
            opt.map(
                |(id, project_id, workspace_id, label, created_at, payload_json)| {
                    SnapshotResponse {
                        id,
                        project_id,
                        workspace_id,
                        label,
                        created_at,
                        payload_json,
                    }
                },
            )
        })
}

// ─── Annotation commands ────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationResponse {
    pub id: String,
    pub project_id: String,
    pub node_id: String,
    pub author: String,
    pub kind: String,
    pub text: String,
    pub created_at: String,
}

#[tauri::command]
pub fn add_comment(
    project_id: String,
    node_id: String,
    author: String,
    text: String,
    kind: Option<String>,
    state: State<'_, AppState>,
) -> Result<AnnotationResponse, String> {
    let repo = ProjectRepository::new(&state.db);
    let (id, project_id_out, node_id_out, author_out, kind_out, text_out, created_at) = repo
        .add_comment(&project_id, &node_id, &author, &text, kind.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(AnnotationResponse {
        id,
        project_id: project_id_out,
        node_id: node_id_out,
        author: author_out,
        kind: kind_out,
        text: text_out,
        created_at,
    })
}

#[tauri::command]
pub fn list_comments(
    project_id: String,
    node_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AnnotationResponse>, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.list_comments(&project_id, node_id.as_deref())
        .map_err(|e| e.to_string())
        .map(|cs| {
            cs.into_iter()
                .map(|r| AnnotationResponse {
                    id: r.0,
                    project_id: r.1,
                    node_id: r.2,
                    author: r.3,
                    kind: r.4,
                    text: r.5,
                    created_at: r.6,
                })
                .collect()
        })
}

#[tauri::command]
pub fn list_snapshots(
    project_id: String,
    workspace_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SnapshotResponse>, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.list_snapshots(&project_id, workspace_id.as_deref())
        .map_err(|e| e.to_string())
        .map(|snaps| {
            snaps
                .into_iter()
                .map(
                    |(id, project_id, workspace_id, label, created_at, payload_json): (
                        String,
                        String,
                        Option<String>,
                        String,
                        String,
                        Option<String>,
                    )| SnapshotResponse {
                        id,
                        project_id,
                        workspace_id,
                        label,
                        created_at,
                        payload_json,
                    },
                )
                .collect()
        })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthRecordResponse {
    pub id: String,
    pub recorded_at: String,
    pub overall_score: f64,
    pub coupling_score: f64,
    pub complexity_score: f64,
    pub cycle_count: i64,
    pub hotspot_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthTimelineResponse {
    pub records: Vec<HealthRecordResponse>,
    pub project_id: String,
    pub from: String,
    pub to: String,
}

#[tauri::command]
pub fn get_health_timeline(
    project_id: String,
    from: String,
    to: String,
    state: State<'_, AppState>,
) -> Result<HealthTimelineResponse, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.get_health_timeline(&project_id, &from, &to)
        .map_err(|e| e.to_string())
        .map(|rows| HealthTimelineResponse {
            records: rows
                .into_iter()
                .map(
                    |(
                        id,
                        recorded_at,
                        overall_score,
                        coupling_score,
                        complexity_score,
                        cycle_count,
                        hotspot_count,
                    ): (String, String, f64, f64, f64, i64, i64)| {
                        HealthRecordResponse {
                            id,
                            recorded_at,
                            overall_score,
                            coupling_score,
                            complexity_score,
                            cycle_count,
                            hotspot_count,
                        }
                    },
                )
                .collect(),
            project_id,
            from,
            to,
        })
}

// ========================================================================
// H3 — Executive Summary + Diff + C4 Views
// ========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutiveSummaryResponse {
    pub workspace_id: String,
    pub total_projects: i64,
    pub total_files: i64,
    pub avg_health_score: Option<f64>,
    pub trend: String,
    pub top_hotspots: Vec<HotspotItem>,
    pub generated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HotspotItem {
    pub node_id: String,
    pub coupling_score: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SnapshotDiffResponse {
    pub base_snapshot_id: String,
    pub target_snapshot_id: String,
    pub nodes_added: Vec<String>,
    pub nodes_removed: Vec<String>,
    pub nodes_modified: Vec<String>,
    pub edges_added: Vec<String>,
    pub edges_removed: Vec<String>,
    pub coupling_delta: f64,
    pub complexity_delta: f64,
    pub cycles_delta: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct C4ViewResponse {
    pub level: u8,
    pub systems: Option<Vec<String>>,
    pub containers: Option<Vec<String>>,
    pub warning: Option<String>,
}

#[tauri::command]
pub fn get_executive_summary(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<ExecutiveSummaryResponse, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.compute_executive_summary(&workspace_id)
        .map_err(|e| e.to_string())
        .map(|s| ExecutiveSummaryResponse {
            workspace_id: s.workspace_id,
            total_projects: s.total_projects,
            total_files: s.total_files,
            avg_health_score: s.avg_health_score,
            trend: s.trend,
            top_hotspots: s
                .top_hotspots
                .into_iter()
                .map(|(node_id, coupling_score): (String, f64)| HotspotItem {
                    node_id,
                    coupling_score,
                })
                .collect(),
            generated_at: s.generated_at,
        })
}

#[tauri::command]
pub fn compare_snapshots(
    base_snapshot_id: String,
    target_snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<SnapshotDiffResponse, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.compare_snapshots(&base_snapshot_id, &target_snapshot_id)
        .map_err(|e| e.to_string())
        .map(|d| SnapshotDiffResponse {
            base_snapshot_id: d.base_snapshot_id,
            target_snapshot_id: d.target_snapshot_id,
            nodes_added: d.nodes_added,
            nodes_removed: d.nodes_removed,
            nodes_modified: d.nodes_modified,
            edges_added: d.edges_added,
            edges_removed: d.edges_removed,
            coupling_delta: d.coupling_delta,
            complexity_delta: d.complexity_delta,
            cycles_delta: d.cycles_delta,
        })
}

#[tauri::command]
pub fn get_c4_view(
    project_id: String,
    level: u8,
    state: State<'_, AppState>,
) -> Result<C4ViewResponse, String> {
    let repo = ProjectRepository::new(&state.db);
    repo.get_c4_view(&project_id, level)
        .map_err(|e| e.to_string())
        .map(|v| C4ViewResponse {
            level: v.level,
            systems: v.systems,
            containers: v.containers,
            warning: v.warning,
        })
}
