//! GraphService — application service for graph orchestration.
//!
//! Orchestrates graph operations using canonical ports:
//! - [`GraphRepository`] — graph cache persistence, file search, outline cache
//! - [`ScanRepository`] — file metadata (get_file_by_id for node details)
//! - [`AppStatePort`] — scan status tracking during graph operations
//!
//! The service owns the graph build lifecycle:
//! - cache hit → return cached JSON
//! - cache miss → build fresh from DB, cache result
//! - transitions AppStatePort through BuildingGraph → Ready|Error
//!
//! # Design (AD-3, AD-5)
//!
//! ```text
//! get_graph(project_id)
//!   -> AppStatePort.set(BuildingGraph)
//!   -> GraphRepository.get_graph_cache()
//!   -> if cached: deserialize and return
//!   -> ScanRepository.get_files() + get_imports()
//!   -> GraphBuilder.build()
//!   -> GraphRepository.save_graph_cache()
//!   -> AppStatePort.set(Ready|Error)
//!
//! get_node_details(node_id)
//!   -> ScanRepository.get_file_by_id()
//!
//! get_node_outline(node_id, root_path)
//!   -> GraphRepository.get_outline_items()
//!   -> if empty: on-demand parse via outline_for_file()
//!   -> GraphRepository.save_outline_items()
//!
//! search_nodes(project_id, query, limit)
//!   -> GraphRepository.search_files()
//! ```

use crate::commands::outline_for_file;
use crate::graph::GraphBuilder;
use crate::models::{FileInfo, GraphData, GraphNode, NodeType, OutlineItem, ScanStatus};
use crate::ports::{AppStatePort, GraphRepository, ScanRepository};
use crate::scanner::parser::ParserRegistry;
use crate::AppError;
use crate::Result;
use std::path::Path;

/// Application service for graph orchestration.
///
/// Generic over `G: GraphRepository`, `S: ScanRepository`, and `A: AppStatePort`
/// so tests can inject doubles without touching the database.
pub struct GraphService<G, S, A> {
    graph_repo: G,
    scan_repo: S,
    state: A,
}

impl<G, S, A> GraphService<G, S, A> {
    /// Construct a new GraphService.
    pub fn new(graph_repo: G, scan_repo: S, state: A) -> Self {
        Self {
            graph_repo,
            scan_repo,
            state,
        }
    }
}

impl<G: GraphRepository, S: ScanRepository, A: AppStatePort> GraphService<G, S, A> {
    /// Get all nodes that the given node depends on (outgoing import edges).
    ///
    /// Returns `Err(AppError::NotFound)` if the node does not exist.
    /// Returns `Ok(vec![])` for known nodes with no dependencies.
    pub async fn get_dependencies(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>> {
        let deps = self.graph_repo.get_dependencies(node_id)?;
        // If deps is empty, verify the node exists (not-found vs empty-result)
        if deps.is_empty() {
            if self.scan_repo.get_file_by_id(node_id)?.is_none() {
                return Err(AppError::NotFound(node_id.to_string()));
            }
        }
        Ok(deps)
    }

    /// Get all nodes that depend on the given node (incoming import edges).
    ///
    /// Returns `Err(AppError::NotFound)` if the node does not exist.
    /// Returns `Ok(vec![])` for known nodes with no dependents.
    pub async fn get_dependents(&self, node_id: &str) -> Result<Vec<crate::models::NodeRef>> {
        let deps = self.graph_repo.get_dependents(node_id)?;
        // If deps is empty, verify the node exists (not-found vs empty-result)
        if deps.is_empty() {
            if self.scan_repo.get_file_by_id(node_id)?.is_none() {
                return Err(AppError::NotFound(node_id.to_string()));
            }
        }
        Ok(deps)
    }

    /// Get the dependency graph for a project.
    ///
    /// 1. Transitions `AppStatePort` to `BuildingGraph`
    /// 2. Returns cached graph if available
    /// 3. On cache miss: loads files + imports from `ScanRepository`, builds via
    ///    `GraphBuilder`, caches result in `GraphRepository`
    /// 4. Transitions `AppStatePort` to `Ready` or `Error`
    ///
    /// # Errors
    ///
    /// Returns `AppError` on database failures or when the project has no files.
    pub fn get_graph(&self, project_id: &str) -> Result<GraphData> {
        // Phase 0: Transition to BuildingGraph
        self.state.set_scan_status(ScanStatus::BuildingGraph)?;

        // Phase 1: Cache hit — return cached graph
        if let Ok(Some(cached)) = self.graph_repo.get_graph_cache(project_id) {
            tracing::info!(
                project_id = %project_id,
                cache_hit = true,
                "graph retrieved from cache"
            );
            let graph: GraphData = serde_json::from_str(&cached)
                .map_err(|e| AppError::Internal(format!("cache deserialization failed: {}", e)))?;
            self.state.set_scan_status(ScanStatus::Ready)?;
            return Ok(graph);
        }

        // Phase 2: Cache miss — build fresh from DB
        let files = self.scan_repo.get_files(project_id)?;

        if files.is_empty() {
            tracing::warn!(
                project_id = %project_id,
                "graph build: no files found in DB"
            );
            self.state.set_scan_status(ScanStatus::Error)?;
            return Err(AppError::ProjectNotFound(format!(
                "Project {} has no files in database",
                project_id
            )));
        }

        // Load imports
        let all_imports = self.scan_repo.get_imports(project_id)?;

        // Get project root path for stable graph path semantics
        let root_path = self
            .scan_repo
            .get_project(project_id)?
            .map(|(_, root, _)| root)
            .unwrap_or_else(|| format!("/projects/{}", project_id));

        // Phase 3: Build graph
        let builder = GraphBuilder::new(&root_path);
        let mut graph = builder.build(&files, &all_imports)?;

        // ReactFlow expects edges to reference existing node ids.
        // Keep only internal edges that have both endpoints present.
        let node_ids: std::collections::HashSet<String> =
            graph.nodes.iter().map(|n| n.id.clone()).collect();
        graph
            .edges
            .retain(|e| node_ids.contains(&e.source) && node_ids.contains(&e.target));

        // Phase 4: Cache the result
        if let Ok(graph_json) = serde_json::to_string(&graph) {
            let _ = self.graph_repo.save_graph_cache(project_id, &graph_json);
        }

        tracing::info!(
            project_id = %project_id,
            cache_hit = false,
            nodes_count = %graph.nodes.len(),
            edges_count = %graph.edges.len(),
            "graph built fresh"
        );

        self.state.set_scan_status(ScanStatus::Ready)?;
        Ok(graph)
    }

    /// Get file metadata for a node (used by frontend to display node details).
    ///
    /// Delegates to `ScanRepository::get_file_by_id`. Returns error if the
    /// node_id does not correspond to a known file.
    pub fn get_node_details(&self, node_id: &str) -> Result<FileInfo> {
        self.scan_repo
            .get_file_by_id(node_id)?
            .ok_or_else(|| AppError::FileNotFound(node_id.to_string()))
    }

    /// Get outline items for a node.
    ///
    /// 1. Fast path: return cached outline from `GraphRepository`
    /// 2. On-demand fallback: if DB is empty, load FileInfo, resolve absolute path,
    ///    read source, parse via `outline_for_file`, persist result, return
    ///
    /// Safe: read/parse errors yield an empty outline; unsupported files return [].
    ///
    /// `root_path` is the session project root. If `None`, the service looks up
    /// the project root for the file via `GraphRepository::get_project_root_for_file`.
    pub fn get_node_outline(
        &self,
        node_id: &str,
        root_path: Option<&str>,
    ) -> Result<Vec<OutlineItem>> {
        // Phase 1: Fast path — cached outline
        let cached = self.graph_repo.get_outline_items(node_id)?;
        if !cached.is_empty() {
            return Ok(cached);
        }

        // Phase 2: On-demand fallback
        let file_info = match self.scan_repo.get_file_by_id(node_id)? {
            Some(f) => f,
            None => return Ok(vec![]),
        };

        // Resolve absolute source path
        let resolved_root = match root_path {
            Some(r) if !r.is_empty() => r.to_string(),
            _ => self
                .graph_repo
                .get_project_root_for_file(node_id)?
                .unwrap_or_default(),
        };

        if resolved_root.is_empty() {
            return Ok(vec![]);
        }

        let abs_path = Path::new(&resolved_root).join(&file_info.path);

        let content = match std::fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("get_node_outline: could not read {:?}: {}", abs_path, e);
                return Ok(vec![]);
            }
        };

        let registry = ParserRegistry::new();
        let outline = outline_for_file(
            &registry,
            node_id,
            &abs_path.to_string_lossy(),
            &content,
            &file_info.extension,
        );

        if !outline.is_empty() {
            if let Err(e) = self.graph_repo.save_outline_items(node_id, &outline) {
                tracing::debug!(
                    "get_node_outline: failed to persist on-demand outline for {}: {}",
                    node_id,
                    e
                );
            }
        }

        Ok(outline)
    }

    /// Search files by name substring (case-insensitive) and return as GraphNode list.
    ///
    /// Limits results to `limit` items (default: 20).
    pub fn search_nodes(
        &self,
        project_id: &str,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<GraphNode>> {
        let limit = limit.unwrap_or(20);
        let files = self.graph_repo.search_files(project_id, query, limit)?;

        let nodes: Vec<GraphNode> = files
            .into_iter()
            .map(|f| GraphNode {
                id: f.id,
                label: f.name,
                path: f.path,
                node_type: NodeType::Unknown,
                symbol_count: 0,
                position: None,
            })
            .collect();

        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Test helpers use RecordingGraphRepo + ScanRepositoryAdapter pattern.
    // Full integration tests are in engine/tests/graph_service_test.rs.

    /// Verify GraphService compiles and basic trait bounds are satisfied.
    #[test]
    fn graph_service_compiles_with_real_adapters() {
        let pool = crate::db::DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();

        let graph_repo = crate::ports::GraphRepositoryAdapter::new(&pool);
        let scan_repo = crate::ports::ScanRepositoryAdapter::new(&pool);
        let app_state = crate::ports::AppStatePortAdapter::new(
            Mutex::new(ScanStatus::Idle),
            Mutex::new(None),
            Mutex::new(String::new()),
        );

        let service = GraphService::new(graph_repo, scan_repo, app_state);
        // Service constructed — trait bounds satisfied
        assert!(true);
    }
}
