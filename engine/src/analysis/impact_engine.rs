//! Impact Analysis Engine — Change propagation through dependency graph
//!
//! Computes which nodes are affected when a given node changes,
//! using BFS/DFS traversal over import edges.

use crate::db::DbPool;

/// Result of impact analysis for a changed node.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImpactAnalysisResult {
    /// v2 contract version.
    pub version: String,
    /// Node that was changed.
    pub changed_node_id: String,
    /// Nodes affected by the change.
    pub affected_nodes: Vec<String>,
    /// Normalized impact score (0.0 to 1.0).
    pub impact_score: f64,
    /// Human-readable explanation of the impact path.
    pub explanation: String,
}

/// Configuration for impact analysis.
#[derive(Debug, Clone)]
pub struct ImpactConfig {
    /// Maximum traversal depth (default 10).
    pub max_depth: usize,
}

impl Default for ImpactConfig {
    fn default() -> Self {
        Self { max_depth: 10 }
    }
}

/// Compute impact of changing a given node.
/// Traverses all downstream dependencies (nodes that import the changed node)
/// using BFS, collecting up to `max_depth` levels of impact.
///
/// Returns `ImpactAnalysisResult` with affected nodes and normalized score.
pub fn compute_impact(
    project_id: &str,
    node_id: &str,
    pool: &DbPool,
    config: &ImpactConfig,
) -> ImpactAnalysisResult {
    let files: Vec<(String, String)> = match pool.with_connection(|conn| {
        let mut stmt = conn.prepare("SELECT id, path FROM files WHERE project_id = ?1")?;
        let rows = stmt.query_map([project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    }) {
        Ok(f) => f,
        Err(_) => {
            return ImpactAnalysisResult::unknown(node_id);
        }
    };

    // Build a quick adjacency: for each file, list files it imports (by path prefix)
    // We need import edges: source imports target → edge source→target
    let imports: Vec<(String, Option<String>)> = match pool.with_connection(|conn| {
        // Get imports with resolved target file IDs
        let mut stmt = conn.prepare(
            "SELECT i.source_file_id, i.target_file_id
             FROM imports i
             JOIN files f ON f.project_id = ?1
             WHERE i.source_file_id = f.id",
        )?;
        let rows = stmt.query_map([project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    }) {
        Ok(rows) => rows,
        Err(_) => {
            return ImpactAnalysisResult::unknown(node_id);
        }
    };

    // Build reverse adjacency: what imports each file (downstream dependents)
    // target → [sources that import target]
    let mut downstream: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (source, target) in &imports {
        if let Some(target_id) = target {
            downstream
                .entry(target_id.clone())
                .or_default()
                .push(source.clone());
        }
    }

    // BFS from node_id through downstream dependents
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queue: Vec<(String, usize)> = vec![(node_id.to_string(), 0)];
    let mut affected: Vec<String> = Vec::new();
    let mut depths: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    while let Some((current, depth)) = queue.pop() {
        if depth > config.max_depth {
            continue;
        }
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());

        // The starting node is not "affected" — only its dependents
        if current != node_id {
            affected.push(current.clone());
            depths.insert(current.clone(), depth);
        }

        if let Some(dependents) = downstream.get(&current) {
            for dep in dependents {
                if !visited.contains(dep) {
                    queue.push((dep.clone(), depth + 1));
                }
            }
        }
    }

    // Normalize impact score based on depth and breadth
    let total_affected = affected.len();
    let max_possible = files.len().saturating_sub(1);
    let depth_factor = if depths.is_empty() {
        0.0
    } else {
        let max_depth_reached = *depths.values().max().unwrap_or(&0);
        // Closer impacts are higher impact
        1.0 - (max_depth_reached as f64 / (config.max_depth as f64 + 1.0))
    };
    let breadth_factor = if max_possible > 0 {
        total_affected as f64 / max_possible as f64
    } else {
        0.0
    };

    // Score: geometric mean of depth and breadth factors, scaled
    let impact_score = ((depth_factor + 0.1) * (breadth_factor + 0.1) * 0.8).min(1.0);

    // Build explanation
    let explanation = if affected.is_empty() {
        format!(
            "El archivo '{}' no tiene dependencias dependientes en el grafo de imports.",
            node_id
        )
    } else {
        let level_counts = {
            let mut counts = std::collections::HashMap::new();
            for &d in depths.values() {
                *counts.entry(d).or_insert(0) += 1;
            }
            counts
        };
        let mut levels: Vec<String> = level_counts
            .into_iter()
            .map(|(lvl, cnt)| format!("{} archivo(s) a {} nivel(es)", cnt, lvl))
            .collect();
        levels.sort();
        format!(
            "El cambio en '{}' afecta {} archivo(s) en {} niveles. Distribución: {}.",
            node_id,
            total_affected,
            levels.len(),
            levels.join(", ")
        )
    };

    ImpactAnalysisResult {
        version: "2.0".to_string(),
        changed_node_id: node_id.to_string(),
        affected_nodes: affected,
        impact_score,
        explanation,
    }
}

impl ImpactAnalysisResult {
    /// Returns a result for when computation fails.
    pub fn unknown(node_id: &str) -> Self {
        Self {
            version: "2.0".to_string(),
            changed_node_id: node_id.to_string(),
            affected_nodes: vec![],
            impact_score: 0.0,
            explanation: "No se pudo calcular el impacto para este nodo.".to_string(),
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
        imports: &[(&str, Option<&str>)],
    ) {
        pool.with_connection(|conn| {
            for (id, path) in files {
                conn.execute(
                    "INSERT OR REPLACE INTO files (id, project_id, path, name, extension)
                     VALUES (?1, ?2, ?3, ?1, 'ts')",
                    rusqlite::params![id, project_id, path],
                )
                .ok();
            }
            for (source, target) in imports {
                conn.execute(
                    "INSERT OR IGNORE INTO imports (id, source_file_id, target_file_id, target_module, import_names)
                     VALUES (?1, ?2, ?3, '', 'default')",
                    rusqlite::params![uuid::Uuid::new_v4().to_string(), source, target],
                )
                .ok();
            }
            Ok::<(), rusqlite::Error>(())
        })
        .unwrap();
    }

    #[test]
    fn linear_graph_impact_affects_downstream_chain() {
        let pool = DbPool::in_memory().unwrap();
        init_schema(&pool, "proj-linear");
        // A ← B ← C (B imports A, C imports B)
        // Downstream of A: nodes that import A (directly or transitively)
        insert_files_and_imports(
            &pool,
            "proj-linear",
            &[
                ("file-a", "src/A.ts"),
                ("file-b", "src/B.ts"),
                ("file-c", "src/C.ts"),
            ],
            &[
                ("file-b", Some("file-a")), // B imports A
                ("file-c", Some("file-b")), // C imports B
            ],
        );

        let result = compute_impact("proj-linear", "file-a", &pool, &ImpactConfig::default());

        assert_eq!(result.changed_node_id, "file-a");
        // file-a's downstream: B (imports A), C (imports B transitively)
        assert_eq!(
            result.affected_nodes.len(),
            2,
            "Expected 2 affected nodes (B and C), got {:?}",
            result.affected_nodes
        );
        assert!(
            result.affected_nodes.contains(&"file-b".to_string()),
            "Expected file-b in affected nodes"
        );
        assert!(
            result.affected_nodes.contains(&"file-c".to_string()),
            "Expected file-c in affected nodes"
        );
        assert!(
            result.impact_score > 0.0 && result.impact_score <= 1.0,
            "impact_score {} out of (0,1]",
            result.impact_score
        );
    }

    #[test]
    fn isolated_node_returns_empty_affected() {
        let pool = DbPool::in_memory().unwrap();
        init_schema(&pool, "proj-isolated");
        // D has no imports or dependents
        insert_files_and_imports(&pool, "proj-isolated", &[("file-d", "src/D.ts")], &[]);

        let result = compute_impact("proj-isolated", "file-d", &pool, &ImpactConfig::default());

        assert_eq!(result.changed_node_id, "file-d");
        assert!(
            result.affected_nodes.is_empty(),
            "Expected empty affected for isolated node, got {:?}",
            result.affected_nodes
        );
        assert!(
            result.impact_score >= 0.0 && result.impact_score <= 1.0,
            "impact_score {} out of [0,1]",
            result.impact_score
        );
    }

    #[test]
    fn db_error_returns_unknown_without_crash() {
        let pool = DbPool::in_memory().unwrap();
        // No schema — DB operations will fail
        let result = compute_impact("nonexistent", "node-x", &pool, &ImpactConfig::default());

        assert_eq!(result.changed_node_id, "node-x");
        assert_eq!(result.affected_nodes.len(), 0);
        assert_eq!(result.impact_score, 0.0);
        assert!(
            !result.explanation.is_empty(),
            "Expected fallback explanation on error"
        );
    }
}
