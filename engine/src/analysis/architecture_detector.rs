//! Architecture Detection — Heuristic-based pattern classification
//!
//! Detects architectural patterns (MVC, Layered, Clean, Hexagonal)
//! based on file path conventions and project structure.
//!
//! Falls back to `unknown` pattern with zero confidence on any error.

use crate::models::FileMeta;

/// Result of architecture detection for a project.
#[derive(Debug, Clone)]
pub struct ArchitectureDetectionResult {
    /// v2 contract version marker.
    pub version: String,
    /// Detected pattern.
    pub pattern: ArchitecturePattern,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Evidence supporting the classification.
    pub evidence: Option<ArchitectureEvidence>,
    /// ISO 8601 timestamp.
    pub generated_at: String,
}

/// Evidence collected during detection.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ArchitectureEvidence {
    /// File IDs matching pattern indicators.
    pub nodes: Vec<String>,
    /// Import relationships supporting the classification.
    pub edges: Vec<ArchitectureEdge>,
    /// Human-readable reasons for the classification.
    pub reasons: Vec<String>,
}

/// An edge relevant to architecture classification.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArchitectureEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

impl ArchitectureDetectionResult {
    /// Returns a degraded result when detection fails.
    pub fn unknown() -> Self {
        Self {
            version: "2.0".to_string(),
            pattern: ArchitecturePattern::Unknown,
            confidence: 0.0,
            evidence: None,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Returns a successful result for a given pattern and evidence.
    pub fn new(
        pattern: ArchitecturePattern,
        confidence: f64,
        nodes: Vec<String>,
        edges: Vec<ArchitectureEdge>,
        reasons: Vec<String>,
    ) -> Self {
        Self {
            version: "2.0".to_string(),
            pattern,
            confidence,
            evidence: Some(ArchitectureEvidence {
                nodes,
                edges,
                reasons,
            }),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Architecture pattern enum mirrored from TypeScript contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitecturePattern {
    Mvc,
    Layered,
    Clean,
    Hexagonal,
    Unknown,
}

impl ArchitecturePattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mvc => "mvc",
            Self::Layered => "layered",
            Self::Clean => "clean",
            Self::Hexagonal => "hexagonal",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for ArchitecturePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn detect_architecture(files: &[FileMeta]) -> ArchitectureDetectionResult {
    // Load pattern rules (defined inline to avoid const vec! issue).
    let rules: &[(ArchitecturePattern, &[&str], f64)] = &[
        (
            ArchitecturePattern::Hexagonal,
            &["ports/", "adapters/", "core/ports/", "hexagon/"],
            1.5,
        ),
        (
            ArchitecturePattern::Clean,
            &[
                "domain/",
                "application/",
                "infrastructure/",
                "presentation/",
            ],
            1.2,
        ),
        (
            ArchitecturePattern::Layered,
            &[
                "controllers/",
                "services/",
                "repositories/",
                "daos/",
                "dto/",
            ],
            1.0,
        ),
        (
            ArchitecturePattern::Mvc,
            &["models/", "views/", "controllers/", "routes/"],
            1.0,
        ),
    ];

    // Score each pattern based on matching paths
    let mut pattern_scores: Vec<(ArchitecturePattern, f64)> = vec![
        (ArchitecturePattern::Mvc, 0.0),
        (ArchitecturePattern::Layered, 0.0),
        (ArchitecturePattern::Clean, 0.0),
        (ArchitecturePattern::Hexagonal, 0.0),
    ];

    let mut matching_nodes: Vec<String> = Vec::new();
    let mut matching_reasons: Vec<String> = Vec::new();

    for file in files {
        let path_lower = file.path.to_lowercase();
        for (pattern, indicators, weight) in rules {
            for indicator in *indicators {
                if path_lower.contains(indicator) {
                    if let Some((_, score)) =
                        pattern_scores.iter_mut().find(|(p, _)| *p == *pattern)
                    {
                        *score += weight;
                    }
                    matching_nodes.push(file.id.clone());
                    matching_reasons.push(format!(
                        "Found '{}' in path '{}'",
                        indicator.trim_end_matches('/'),
                        file.path
                    ));
                }
            }
        }
    }

    // Find the best scoring pattern
    pattern_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (best_pattern, best_score) = pattern_scores[0];

    if best_score <= 0.0 {
        return ArchitectureDetectionResult::unknown();
    }

    // Normalize confidence: cap at 1.0 (max expected score ~8)
    let confidence = (best_score / 8.0).min(1.0);

    // Deduplicate nodes
    matching_nodes.sort();
    matching_nodes.dedup();

    ArchitectureDetectionResult::new(
        best_pattern,
        confidence,
        matching_nodes,
        vec![],
        matching_reasons,
    )
}

// ============================================================================
// TESTS — RED first (fail before implementation)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbPool;

    fn load_files(pool: &DbPool) -> Vec<FileMeta> {
        pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, name, extension, lines FROM files WHERE project_id = ?1",
            )?;
            let rows = stmt.query_map(["proj-test"], |row| {
                Ok(FileMeta {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    extension: row.get(3)?,
                    lines: row.get(4)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()
        })
        .unwrap()
    }

    fn init_schema_with_files(pool: &DbPool, files: &[(&str, &str)]) {
        pool.with_connection(|conn| {
            crate::db::schema::init_schema(conn).ok();
            conn.execute(
                "INSERT OR REPLACE INTO projects (id, name, root_path) VALUES ('proj-test', 'Test', '/tmp')",
                [],
            )
            .ok();
            for (id, path) in files {
                conn.execute(
                    "INSERT OR REPLACE INTO files (id, project_id, path, name, extension) VALUES (?1, 'proj-test', ?2, ?1, 'ts')",
                    rusqlite::params![id, path],
                )
                .ok();
            }
            Ok::<(), rusqlite::Error>(())
        })
        .unwrap();
    }

    #[test]
    fn mvc_project_returns_mvc_pattern_with_positive_confidence() {
        let pool = DbPool::in_memory().unwrap();
        init_schema_with_files(
            &pool,
            &[
                ("f1", "src/models/User.ts"),
                ("f2", "src/views/Home.tsx"),
                ("f3", "src/controllers/UserController.ts"),
                ("f4", "src/routes.ts"),
            ],
        );

        let files = load_files(&pool);
        let result = detect_architecture(&files);
        assert_eq!(result.pattern, ArchitecturePattern::Mvc);
        assert!(
            result.confidence > 0.0,
            "Expected confidence > 0 for MVC project, got {}",
            result.confidence
        );
    }

    #[test]
    fn clean_architecture_project_returns_clean() {
        let pool = DbPool::in_memory().unwrap();
        init_schema_with_files(
            &pool,
            &[
                ("f1", "src/domain/entities/User.ts"),
                ("f2", "src/application/UseCase.ts"),
                ("f3", "src/infrastructure/Database.ts"),
                ("f4", "src/presentation/Controller.ts"),
            ],
        );

        let files = load_files(&pool);
        let result = detect_architecture(&files);
        assert_eq!(result.pattern, ArchitecturePattern::Clean);
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn neutral_paths_return_unknown() {
        let pool = DbPool::in_memory().unwrap();
        init_schema_with_files(
            &pool,
            &[
                ("f1", "src/index.ts"),
                ("f2", "src/utils.ts"),
                ("f3", "src/app.ts"),
            ],
        );

        let files = load_files(&pool);
        let result = detect_architecture(&files);
        assert_eq!(result.pattern, ArchitecturePattern::Unknown);
    }

    #[test]
    fn empty_file_list_returns_unknown_without_crash() {
        let result = detect_architecture(&[]);
        assert_eq!(result.pattern, ArchitecturePattern::Unknown);
        assert_eq!(result.confidence, 0.0);
        assert!(result.evidence.is_none());
    }

    #[test]
    fn evidence_contains_matching_nodes_and_reasons() {
        let pool = DbPool::in_memory().unwrap();
        init_schema_with_files(
            &pool,
            &[
                ("f1", "src/domain/User.ts"),
                ("f2", "src/application/Service.ts"),
                ("f3", "src/infrastructure/Repo.ts"),
            ],
        );

        let files = load_files(&pool);
        let result = detect_architecture(&files);
        assert!(
            result.evidence.is_some(),
            "Expected evidence for matched paths"
        );
        let ev = result.evidence.as_ref().unwrap();
        assert!(
            !ev.nodes.is_empty(),
            "Expected non-empty nodes in evidence, got {:?}",
            ev.nodes
        );
        assert!(
            ev.reasons.len() >= 2,
            "Expected at least 2 reasons from 3 clean indicators, got {}",
            ev.reasons.len()
        );
    }
}
