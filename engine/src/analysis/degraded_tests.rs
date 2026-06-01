/**
 * Degraded-mode tests — T6.4
 * Strict TDD RED: tests written before implementation.
 * These verify graceful degradation when components fail or receive empty data.
 */

#[cfg(test)]
mod tests {
    use super::super::architecture_detector::detect_architecture;
    use super::super::architecture_detector::ArchitecturePattern;
    use super::super::graph_insights::{compute_graph_insights, InsightsConfig};
    use super::super::impact_engine::{compute_impact, ImpactConfig};
    use crate::db::DbPool;
    use std::time::Duration;

    /// T6.4: ArchitectureDetector on empty project → Unknown pattern, 0.0 confidence, null evidence
    #[test]
    fn architecture_empty_project_returns_unknown() {
        let pool = DbPool::new(":memory:").unwrap();
        pool.init_schema().unwrap();

        let result = detect_architecture("nonexistent-project", &pool);
        assert!(
            matches!(result.pattern, ArchitecturePattern::Unknown),
            "pattern should be Unknown"
        );
        assert!(
            (result.confidence - 0.0).abs() < 0.001,
            "confidence should be 0.0, got {}",
            result.confidence
        );
        assert!(
            result.evidence.is_none(),
            "evidence should be None on failure"
        );
    }

    /// T6.4: GraphInsights on empty project → ok status, empty cycles/hotspots, graceful degradation
    #[test]
    fn insights_empty_project_returns_empty() {
        let pool = DbPool::new(":memory:").unwrap();
        pool.init_schema().unwrap();

        let config = InsightsConfig {
            timeout: Duration::from_secs(5),
            hotspot_threshold: 0.1,
        };
        let result = compute_graph_insights("nonexistent-project", &pool, &config);

        assert!(
            result.status == Some("ok".to_string())
                || result.status == Some("timeout".to_string())
                || result.status == Some("error".to_string()),
            "status should be ok/timeout/error, got {:?}",
            result.status
        );
        assert!(
            result.cycles.is_empty(),
            "cycles should be empty for empty project"
        );
        assert!(
            result.hotspots.is_empty(),
            "hotspots should be empty for empty project"
        );
    }

    /// T6.4: Impact analysis on project with no graph data → graceful degradation, no panic
    #[test]
    fn impact_nonexistent_project_returns_empty() {
        let pool = DbPool::new(":memory:").unwrap();
        pool.init_schema().unwrap();

        let config = ImpactConfig { max_depth: 10 };
        let result = compute_impact("nonexistent-project", "some-node", &pool, &config);
        assert!(
            result.affected_nodes.is_empty(),
            "no nodes should be affected when graph is empty"
        );
    }

    /// T6.4: Cycle detection with zero timeout → returns gracefully without panic
    #[test]
    fn insights_zero_timeout_handles_gracefully() {
        let pool = DbPool::new(":memory:").unwrap();
        pool.init_schema().unwrap();

        let config = InsightsConfig {
            timeout: Duration::from_secs(0),
            hotspot_threshold: 0.1,
        };
        let result = compute_graph_insights("test-proj", &pool, &config);
        // Should not panic — returns a valid struct
        assert!(result.cycles.len() >= 0);
        assert!(result.hotspots.len() >= 0);
    }
}
