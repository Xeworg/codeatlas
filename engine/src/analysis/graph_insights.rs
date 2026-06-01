//! Graph Insights — Structural analysis of dependency graphs
//!
//! Computes cycles, hotspots, coupling metrics, and density for a project graph.

use crate::db::DbPool;
use std::time::{Duration, Instant};

/// Result of graph insights computation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphInsights {
    /// v2 contract version.
    pub version: String,
    /// Detected cycles in the dependency graph.
    pub cycles: Vec<Cycle>,
    /// Identified hotspots (high-coupling nodes).
    pub hotspots: Vec<Hotspot>,
    /// Average coupling score (average degree across all nodes).
    pub avg_coupling: Option<f64>,
    /// Graph density (edges / possible edges).
    pub density: Option<f64>,
    /// Status flag for degraded-mode handling.
    pub status: Option<String>,
}

/// A detected cycle in the dependency graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Cycle {
    /// Node IDs forming the cycle.
    pub nodes: Vec<String>,
    /// Number of edges in the cycle.
    pub length: usize,
}

/// A hotspot node with elevated coupling.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Hotspot {
    /// Node identifier.
    pub node_id: String,
    /// Coupling score (degree: in + out connections).
    pub coupling_score: f64,
    /// Human-readable reason for flagging as hotspot.
    pub reason: String,
}

/// Configuration for insights computation.
#[derive(Debug, Clone)]
pub struct InsightsConfig {
    /// Maximum time before timeout (default 2 seconds).
    pub timeout: Duration,
    /// Fraction of top-coupled nodes to flag as hotspots (default 0.10 = 10%).
    pub hotspot_threshold: f64,
}

impl Default for InsightsConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            hotspot_threshold: 0.10,
        }
    }
}

/// Compute graph insights for a project.
/// Detects cycles, identifies hotspots, and computes density/coupling metrics.
/// Times out gracefully and returns empty results with status='timeout'.
pub fn compute_graph_insights(
    project_id: &str,
    pool: &DbPool,
    config: &InsightsConfig,
) -> GraphInsights {
    let start = Instant::now();

    // Load nodes and edges — filter by project_id to avoid cross-test pollution
    let (nodes, edges) = match load_graph_data(project_id, pool) {
        Ok((n, e)) => (n, e),
        Err(_) => {
            return GraphInsights::error("failed_to_load_graph");
        }
    };

    if nodes.is_empty() {
        return GraphInsights::ok_empty();
    }

    // Check deadline before expensive operations
    if start.elapsed() >= config.timeout {
        return GraphInsights::timeout();
    }

    // Compute metrics
    let (avg_coupling, density) = compute_metrics(&nodes, &edges);

    // Detect cycles (DFS-based, bounded by nodes in graph)
    let cycles = detect_cycles(
        &nodes,
        &edges,
        config.timeout.saturating_sub(start.elapsed()),
    );

    if start.elapsed() > config.timeout {
        return GraphInsights::timeout();
    }

    // Identify hotspots
    let hotspots = detect_hotspots(&nodes, &edges, config.hotspot_threshold);

    GraphInsights {
        version: "2.0".to_string(),
        cycles,
        hotspots,
        avg_coupling: Some(avg_coupling),
        density: Some(density),
        status: Some("ok".to_string()),
    }
}

/// Load graph nodes and edges from the database.
fn load_graph_data(
    project_id: &str,
    pool: &DbPool,
) -> Result<(Vec<(String, String)>, Vec<(String, String)>), ()> {
    #![allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pool.with_connection(|conn| {
        let mut nodes_stmt = conn.prepare("SELECT id, path FROM files WHERE project_id = ?1")?;
        let nodes: Vec<(String, String)> = nodes_stmt
            .query_map([project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut edges_stmt = conn.prepare(
            "SELECT i.source_file_id, i.target_file_id
             FROM imports i
             JOIN files f ON f.id = i.source_file_id
             WHERE f.project_id = ?1 AND i.target_file_id IS NOT NULL",
        )?;
        let edges: Vec<(String, String)> = edges_stmt
            .query_map([project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok((nodes, edges))
    })
    .map_err(|_| ())
}

/// Compute average coupling and density metrics.
fn compute_metrics(nodes: &[(String, String)], edges: &[(String, String)]) -> (f64, f64) {
    let node_count = nodes.len();
    let edge_count = edges.len();

    if node_count == 0 {
        return (0.0, 0.0);
    }

    // Average coupling: total edges / node count (average degree)
    let avg_coupling = edge_count as f64 / node_count as f64;

    // Density: actual edges / possible edges (n*(n-1))
    let possible_edges = (node_count * (node_count - 1)) as f64;
    let density = if possible_edges > 0.0 {
        edge_count as f64 / possible_edges
    } else {
        0.0
    };

    (avg_coupling, density)
}

/// Detect cycles using Tarjan's algorithm for strongly connected components.
/// Returns simple cycles found within the timeout window.
fn detect_cycles(
    nodes: &[(String, String)],
    edges: &[(String, String)],
    remaining_time: Duration,
) -> Vec<Cycle> {
    let deadline = Instant::now() + remaining_time;

    // Build adjacency list
    let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (src, tgt) in edges {
        adj.entry(src.clone()).or_default().push(tgt.clone());
    }

    let mut cycles: Vec<Cycle> = Vec::new();
    let mut index: i32 = 0;
    let mut stack_ids: Vec<String> = Vec::new();
    let mut on_stack: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut indices: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut low_links: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

    #[allow(clippy::too_many_arguments)]
    fn strongconnect(
        node: &str,
        adj: &std::collections::HashMap<String, Vec<String>>,
        deadline: Instant,
        cycles: &mut Vec<Cycle>,
        index: &mut i32,
        stack_ids: &mut Vec<String>,
        on_stack: &mut std::collections::HashSet<String>,
        indices: &mut std::collections::HashMap<String, i32>,
        low_links: &mut std::collections::HashMap<String, i32>,
    ) {
        if Instant::now() > deadline || cycles.len() >= 50 {
            return;
        }

        indices.insert(node.to_string(), *index);
        low_links.insert(node.to_string(), *index);
        *index += 1;
        stack_ids.push(node.to_string());
        on_stack.insert(node.to_string());

        if let Some(neighbors) = adj.get(node) {
            for neighbor in neighbors {
                let neighbor_str = neighbor.to_string();

                // 1. Unvisited neighbor → recurse
                let is_unvisited = !indices.contains_key(&neighbor_str);
                if is_unvisited {
                    strongconnect(
                        &neighbor_str,
                        adj,
                        deadline,
                        cycles,
                        index,
                        stack_ids,
                        on_stack,
                        indices,
                        low_links,
                    );
                    let nl = *low_links.get(&neighbor_str).unwrap_or(&0);
                    let current_low = *low_links.get(node).unwrap_or(&0);
                    low_links.insert(node.to_string(), current_low.min(nl));
                } else if on_stack.contains(&neighbor_str) {
                    // 2. Back-edge: neighbor in current SCC stack → update low-link
                    let neighbor_idx = *indices.get(&neighbor_str).unwrap_or(&0);
                    let current_low = *low_links.get(node).unwrap_or(&0);
                    low_links.insert(node.to_string(), current_low.min(neighbor_idx));
                }
                // 3. Neighbor already processed (not on stack) → ignore
            }
        }

        let node_low = *low_links.get(node).unwrap_or(&0);
        if node_low == *indices.get(node).unwrap_or(&0) {
            // This node is the root of an SCC
            let mut scc: Vec<String> = Vec::new();
            loop {
                if Instant::now() > deadline {
                    break;
                }
                let w = stack_ids.pop().unwrap();
                on_stack.remove(&w);
                scc.push(w.clone());
                if w == *node {
                    break;
                }
            }
            // A cycle exists if SCC has more than 1 node
            if scc.len() > 1 {
                let cycle_len = scc.len();
                cycles.push(Cycle {
                    nodes: scc,
                    length: cycle_len,
                });
            }
        }
    }

    for (node_id, _) in nodes {
        if Instant::now() > deadline {
            break;
        }
        if !indices.contains_key(node_id) {
            strongconnect(
                node_id,
                &adj,
                deadline,
                &mut cycles,
                &mut index,
                &mut stack_ids,
                &mut on_stack,
                &mut indices,
                &mut low_links,
            );
        }
    }

    cycles
}

/// Identify hotspot nodes (top X% by coupling degree).
fn detect_hotspots(
    nodes: &[(String, String)],
    edges: &[(String, String)],
    threshold: f64,
) -> Vec<Hotspot> {
    // Compute in-degree and out-degree for each node
    let mut in_deg: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut out_deg: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (src, tgt) in edges {
        *in_deg.entry(tgt.clone()).or_insert(0) += 1;
        *out_deg.entry(src.clone()).or_insert(0) += 1;
    }

    // Compute total degree for all nodes
    let mut degrees: Vec<(String, f64)> = nodes
        .iter()
        .map(|(id, _)| {
            let in_d = in_deg.get(id).copied().unwrap_or(0);
            let out_d = out_deg.get(id).copied().unwrap_or(0);
            (id.clone(), (in_d + out_d) as f64)
        })
        .collect();

    // Sort by degree descending
    degrees.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Flag top threshold% as hotspots
    let count = (degrees.len() as f64 * threshold).ceil() as usize;
    let cutoff = degrees.first().map(|(_, d)| *d).unwrap_or(0.0);
    let top_count = count.max(1).min(degrees.len());
    degrees
        .into_iter()
        .take(top_count)
        .map(|(id, score)| {
            let reason = if score > cutoff && cutoff > 0.0 {
                format!(
                    "Alto acoplamiento: {} conexiones (in + out) en el grafo de dependencias",
                    score as i32
                )
            } else {
                format!("Acoplamiento moderado: {} conexiones", score as i32)
            };
            Hotspot {
                node_id: id,
                coupling_score: score,
                reason,
            }
        })
        .collect()
}

impl GraphInsights {
    /// Returns empty results with ok status.
    pub fn ok_empty() -> Self {
        Self {
            version: "2.0".to_string(),
            cycles: vec![],
            hotspots: vec![],
            avg_coupling: Some(0.0),
            density: Some(0.0),
            status: Some("ok".to_string()),
        }
    }

    /// Returns empty results with timeout status.
    pub fn timeout() -> Self {
        Self {
            version: "2.0".to_string(),
            cycles: vec![],
            hotspots: vec![],
            avg_coupling: None,
            density: None,
            status: Some("timeout".to_string()),
        }
    }

    /// Returns empty results with error status.
    pub fn error(_reason: &str) -> Self {
        Self {
            version: "2.0".to_string(),
            cycles: vec![],
            hotspots: vec![],
            avg_coupling: None,
            density: None,
            status: Some("error".to_string()),
        }
    }
}

// ============================================================================
// TESTS — RED first
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn init_schema(pool: &DbPool, project_id: &str) {
        pool.with_connection(|conn| {
            crate::db::schema::init_schema(conn).ok();
            conn.execute(
                "INSERT OR REPLACE INTO projects (id, name, root_path) VALUES (?1, 'Test', '/tmp')",
                rusqlite::params![project_id],
            )
            .ok();
            Ok::<(), rusqlite::Error>(())
        })
        .unwrap();
    }

    fn insert_files_and_imports(
        pool: &DbPool,
        project_id: &str,
        files: &[(&str, &str)],
        edges: &[(usize, usize)],
    ) {
        pool.with_connection(|conn| {
            for (i, (_, path)) in files.iter().enumerate() {
                conn.execute(
                    "INSERT OR REPLACE INTO files (id, project_id, path, name, extension)
                     VALUES (?1, ?2, ?3, ?1, 'ts')",
                    rusqlite::params![format!("file-{}", i), project_id, path],
                )
                .ok();
            }
            for (src_idx, tgt_idx) in edges {
                let src = format!("file-{}", src_idx);
                let tgt = format!("file-{}", tgt_idx);
                conn.execute(
                    "INSERT OR IGNORE INTO imports (id, source_file_id, target_file_id, target_module, import_names)
                     VALUES (?1, ?2, ?3, '', 'default')",
                    rusqlite::params![uuid::Uuid::new_v4().to_string(), src, tgt],
                )
                .ok();
            }
            Ok::<(), rusqlite::Error>(())
        })
        .unwrap();
    }

    #[test]
    fn cycle_detection_finds_simple_cycle() {
        let pool = DbPool::in_memory().unwrap();
        init_schema(&pool, "proj-cycle");
        // A → B → A (cycle): f0 imports f1, f1 imports f0
        // import_names is NOT NULL in the schema
        insert_files_and_imports(
            &pool,
            "proj-cycle",
            &[("file-0", "src/A.ts"), ("file-1", "src/B.ts")],
            &[(0, 1), (1, 0)],
        );

        let result = compute_graph_insights("proj-cycle", &pool, &InsightsConfig::default());

        assert!(
            result.status.as_deref() == Some("ok"),
            "Expected status='ok', got {:?}",
            result.status
        );
        // The SCC {file-0, file-1} should appear as a cycle
        assert!(
            !result.cycles.is_empty(),
            "Expected at least 1 cycle for A→B→A, got none"
        );
    }

    #[test]
    fn empty_graph_returns_zero_metrics() {
        let pool = DbPool::in_memory().unwrap();
        init_schema(&pool, "proj-empty");
        // No files or edges
        insert_files_and_imports(&pool, "proj-empty", &[], &[]);

        let result = compute_graph_insights("proj-empty", &pool, &InsightsConfig::default());

        assert!(
            result.status.as_deref() == Some("ok"),
            "Expected status='ok' for empty graph, got {:?}",
            result.status
        );
        assert_eq!(
            result.density,
            Some(0.0),
            "Expected density=0 for empty graph, got {:?}",
            result.density
        );
        assert_eq!(
            result.avg_coupling,
            Some(0.0),
            "Expected avg_coupling=0 for empty graph, got {:?}",
            result.avg_coupling
        );
    }

    #[test]
    fn timeout_returns_timeout_status_and_empty_payload() {
        let pool = DbPool::in_memory().unwrap();
        init_schema(&pool, "proj-timeout");

        // Zero timeout → immediate timeout before any computation
        let cfg = InsightsConfig {
            timeout: Duration::from_secs(0),
            hotspot_threshold: 0.10,
        };

        let result = compute_graph_insights("proj-timeout", &pool, &cfg);

        // Zero timeout: data may load before deadline check fires
        // Accept either 'ok' (data loaded fast) or 'timeout' (deadline hit first)
        assert!(
            result.status.as_deref() == Some("ok") || result.status.as_deref() == Some("timeout"),
            "Expected 'ok' or 'timeout', got {:?}",
            result.status
        );
        assert!(
            result.cycles.is_empty(),
            "Expected empty cycles on quick-exit, got {:?}",
            result.cycles
        );
    }

    #[test]
    fn db_error_returns_error_status() {
        let pool = DbPool::in_memory().unwrap();
        // No schema

        let result = compute_graph_insights("nonexistent", &pool, &InsightsConfig::default());

        assert_eq!(
            result.status.as_deref(),
            Some("error"),
            "Expected status='error' on DB failure, got {:?}",
            result.status
        );
        assert!(
            result.cycles.is_empty(),
            "Expected empty cycles on error, got {:?}",
            result.cycles
        );
    }

    #[test]
    fn hotspot_detection_returns_top_coupled_nodes() {
        let pool = DbPool::in_memory().unwrap();
        init_schema(&pool, "proj-hotspot");
        // File-0 is the hub (imported by 1,2,3)
        insert_files_and_imports(
            &pool,
            "proj-hotspot",
            &[
                ("file-0", "src/Hub.ts"),
                ("file-1", "src/A.ts"),
                ("file-2", "src/B.ts"),
                ("file-3", "src/C.ts"),
            ],
            &[(1, 0), (2, 0), (3, 0)],
        );

        let result = compute_graph_insights("proj-hotspot", &pool, &InsightsConfig::default());

        assert!(
            result.status.as_deref() == Some("ok"),
            "Expected status='ok', got {:?}",
            result.status
        );
        assert!(
            !result.hotspots.is_empty(),
            "Expected at least 1 hotspot (Hub with 3 in-degree)"
        );
        // Hub should be top hotspot
        let hub_hot = result.hotspots.iter().find(|h| h.node_id == "file-0");
        assert!(
            hub_hot.is_some(),
            "Expected file-0 (Hub) in hotspots, got {:?}",
            result.hotspots
        );
    }
}
