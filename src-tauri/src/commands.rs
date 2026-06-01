//! Tauri commands — Presentation layer
//! Exposes engine functionality to the frontend via invoke().

use engine::{
    db::{schema, DbPool, ProjectRepository},
    graph::{GraphBuilder, PathResolver},
    models::{ChatMessage, FileInfo, GraphData, NodeExplanation, ScanResult},
    scanner::{CodeParser, FileWalker},
    AppError, Result,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

// Global state
pub struct AppState {
    pub db: DbPool,
    pub scan_status: Mutex<ScanStatus>,
    pub ai_config: Mutex<Option<engine::models::AIConfig>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanStatusResponse {
    pub status: String,
    pub progress: f32,
}

// MARK: Project & Scanning Commands

#[tauri::command]
pub async fn scan_project(path: String, state: State<'_, AppState>) -> Result<ScanResult, String> {
    let status = state.scan_status.lock().map_err(|e| e.to_string())?;
    *status = ScanStatus::Scanning;

    let walker = FileWalker::new(&path);
    let discovered = walker.discover();

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
                            // lookup file by path
                            imp.target_file_id = files.iter().find(|f| f.path == p).map(|f| f.id.clone());
                        }
                        crate::graph::resolver::Resolution::External(_) => {
                            // external modules stay as-is
                        }
                        crate::graph::resolver::Resolution::Unresolved(_) => {}
                    }
                }
                imp
            })
            .collect();

        all_imports.extend(resolved_imports);
        files.push(file_info);
    }

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
        scan_duration_ms: 0, // TODO: measure
        status: engine::models::ScanStatus::Ready,
        error: None,
    };

    // Persist
    if let Ok(repo) = ProjectRepository::new(&state.db) {
        let _ = repo.save_scan_result(&result);
    }

    *status = ScanStatus::Ready;
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

    // Try to return cached graph first
    if let Ok(Some(cached)) = repo.get_graph_cache(&project_id) {
        serde_json::from_str(&cached).map_err(|e| e.to_string())
    } else {
        // Build fresh from DB
        let files = repo.get_files(&project_id).map_err(|e| e.to_string())?;
        let all_imports: Vec<engine::models::ImportInfo> = vec![]; // TODO: load from DB
        let builder = GraphBuilder::new(format!("/projects/{}", project_id));
        let graph = builder.build(&files, &all_imports).map_err(|e| e.to_string())?;

        // Cache it
        if let Ok(graph_json) = serde_json::to_string(&graph) {
            let _ = repo.save_graph_cache(&project_id, &graph_json);
        }

        Ok(graph)
    }
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
    state: State<'_, AppState>,
) -> Result<NodeExplanation, String> {
    let config = state.ai_config.lock().map_err(|e| e.to_string())?;
    if config.is_none() {
        return Err("AI not configured".to_string());
    }

    // TODO: get actual file content and graph
    let fake_context = "fn main() {}\n".repeat(10);
    Ok(NodeExplanation {
        node_id,
        summary: "Ejemplo de explicación".to_string(),
        details: "Esta es una explicación de ejemplo.".to_string(),
        dependencies_note: None,
        role: "Módulo del proyecto".to_string(),
    })
}

#[tauri::command]
pub async fn chat(
    project_id: String,
    message: String,
    history: Vec<ChatMessage>,
    state: State<'_, AppState>,
) -> Result<engine::models::ChatResponse, String> {
    let config = state.ai_config.lock().map_err(|e| e.to_string())?;
    if config.is_none() {
        return Err("AI not configured".to_string());
    }

    Ok(engine::models::ChatResponse {
        message: engine::models::ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: engine::models::ChatRole::Assistant,
            content: "Chat de ejemplo. Configurá tu API key para activar IA real.".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
        referenced_nodes: None,
    })
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