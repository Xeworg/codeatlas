//! AnalysisService — Application layer for advanced analysis commands.
//!
//! Orchestrates architecture detection, impact analysis, graph insights,
//! and export operations by delegating to canonical ports and the existing
//! `engine::analysis` module.
//!
//! # Design (AD-3, AD-5)
//!
//! ```text
//! Tauri command shim
//!   -> AnalysisService
//!     -> AnalysisDataSource  (persistence + pool accessor)
//!     -> GraphRepository      (graph cache)
//!     -> engine::analysis::*  (pure computation)
//! ```

use crate::analysis::{
    compute_graph_insights, compute_impact, detect_architecture,
    graph_insights::GraphInsights as EngineGraphInsights,
    ArchitectureDetectionResult as EngineArchResult, ImpactAnalysisResult as EngineImpactResult,
    InsightsConfig,
};
use crate::ports::hexagonal::Clock;
use crate::ports::{AnalysisDataSource, GraphRepository};
use crate::Result;
use serde::Serialize;

// ─────────────────────────────────────────────────────────────────────────────
// Response DTOs — mirror the Tauri IPC contract
// ─────────────────────────────────────────────────────────────────────────────

/// Response for architecture detection.
#[derive(Debug, Clone, Serialize)]
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

/// Response for impact analysis.
#[derive(Debug, Clone, Serialize)]
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

/// Response for graph insights.
#[derive(Debug, Clone, Serialize)]
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
            cycles: r
                .cycles
                .iter()
                .map(|c| serde_json::json!({ "nodes": &c.nodes, "length": c.length }))
                .collect(),
            hotspots: r
                .hotspots
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "nodeId": h.node_id,
                        "couplingScore": h.coupling_score,
                        "reason": h.reason,
                    })
                })
                .collect(),
            avg_coupling: r.avg_coupling,
            density: r.density,
            status: r.status,
        }
    }
}

/// Response for export view.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayloadResponse {
    pub version: String,
    pub format: String,
    pub graph_data: serde_json::Value,
    pub insights: Option<serde_json::Value>,
    pub metadata: ExportMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportMetadata {
    pub project_id: String,
    pub generated_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// AnalysisService
// ─────────────────────────────────────────────────────────────────────────────

/// Application service for advanced analysis and export operations.
///
/// Generic over `A: AnalysisDataSource`, `G: GraphRepository`, and `C: Clock` so that
/// all operations are fully testable with mock doubles. The `Clock` port is used
/// exclusively by `export_view` to produce deterministic `generated_at` timestamps.
///
/// # Methods
/// - `get_architecture_detection` — detect architectural pattern from file paths
/// - `get_impact_analysis` — compute change impact propagation through the graph
/// - `get_graph_insights` — compute cycles, hotspots, coupling, and density
/// - `export_view` — assemble export payload from cached graph and insights
pub struct AnalysisService<'pool, A, G, C> {
    analysis_repo: A,
    graph_repo: G,
    clock: C,
    _phantom: std::marker::PhantomData<&'pool ()>,
}

impl<'pool, A, G, C> AnalysisService<'pool, A, G, C>
where
    A: AnalysisDataSource,
    G: GraphRepository,
    C: Clock,
{
    /// Construct a new `AnalysisService` with the given repositories and clock.
    pub fn new(analysis_repo: A, graph_repo: G, clock: C) -> Self {
        Self {
            analysis_repo,
            graph_repo,
            clock,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Detect the architectural pattern for a project based on file paths.
    ///
    /// Runs `engine::analysis::detect_architecture` and persists the result
    /// via `AnalysisDataSource::save_architecture_detection`.
    pub fn get_architecture_detection(
        &self,
        project_id: &str,
    ) -> Result<ArchitectureDetectionResponse> {
        let timing_start = std::time::Instant::now();

        let pool = self.analysis_repo.pool();
        let result = detect_architecture(project_id, pool);

        let elapsed_ms = timing_start.elapsed().as_millis() as u64;
        tracing::info!(
            "Architecture detection for {}: {} (conf={:.2}) in {}ms",
            project_id,
            result.pattern.as_str(),
            result.confidence,
            elapsed_ms
        );

        // Persist result via port (best-effort)
        let evidence_json = result
            .evidence
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap_or_default())
            .unwrap_or_default();
        let _ = self.analysis_repo.save_architecture_detection(
            project_id,
            result.pattern.as_str(),
            result.confidence,
            &evidence_json,
        );

        Ok(result.into())
    }

    /// Compute the impact of changing a given node on the dependency graph.
    ///
    /// Uses BFS traversal over import edges to find affected downstream nodes.
    pub fn get_impact_analysis(
        &self,
        project_id: &str,
        node_id: &str,
    ) -> Result<ImpactAnalysisResponse> {
        let timing_start = std::time::Instant::now();

        let pool = self.analysis_repo.pool();
        let result = compute_impact(
            project_id,
            node_id,
            pool,
            &crate::analysis::ImpactConfig::default(),
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

    /// Compute graph insights: cycles, hotspots, coupling, and density.
    ///
    /// Persists the result via `AnalysisDataSource::save_graph_insights`.
    pub fn get_graph_insights(&self, project_id: &str) -> Result<GraphInsightsResponse> {
        let timing_start = std::time::Instant::now();

        let pool = self.analysis_repo.pool();
        let result = compute_graph_insights(project_id, pool, &InsightsConfig::default());

        let elapsed_ms = timing_start.elapsed().as_millis() as u64;
        tracing::info!(
            "Graph insights for {}: {} cycles, {} hotspots, density={:.4} in {}ms",
            project_id,
            result.cycles.len(),
            result.hotspots.len(),
            result.density.unwrap_or(0.0),
            elapsed_ms
        );

        // Persist via port (best-effort)
        let cycles_json = serde_json::to_string(&result.cycles).unwrap_or_default();
        let hotspots_json = serde_json::to_string(&result.hotspots).unwrap_or_default();
        let _ = self.analysis_repo.save_graph_insights(
            project_id,
            &cycles_json,
            &hotspots_json,
            result.avg_coupling,
            result.density,
        );

        Ok(result.into())
    }

    /// Export the current graph view and optional insights as a structured payload.
    ///
    /// - `json` format: assembles GraphData + GraphInsights into ExportPayload.
    /// - `png` format: returns an error — PNG generation is frontend responsibility.
    pub fn export_view(&self, project_id: &str, format: String) -> Result<ExportPayloadResponse> {
        let timing_start = std::time::Instant::now();

        // Validate format
        if format != "json" && format != "png" {
            return Err(crate::AppError::Internal(format!(
                "Invalid export format '{}'. Supported: 'json', 'png'.",
                format
            )));
        }

        // PNG is handled by frontend
        if format == "png" {
            return Err(crate::AppError::Internal(
                "PNG export is handled by the frontend using html-to-image. Use the useExport hook."
                    .to_string(),
            ));
        }

        // Fetch cached graph data via GraphRepository
        let graph_json = self
            .graph_repo
            .get_graph_cache(project_id)?
            .unwrap_or_else(|| {
                serde_json::to_string(&serde_json::json!({
                    "nodes": [],
                    "edges": [],
                    "project_id": project_id,
                    "generated_at": self.clock.now().to_rfc3339(),
                }))
                .unwrap_or_default()
            });

        // Optionally fetch cached insights via AnalysisDataSource
        let insights_json: Option<serde_json::Value> = self
            .analysis_repo
            .get_cached_graph_insights(project_id)?
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
                .unwrap_or(serde_json::json!({ "nodes": [], "edges": [] })),
            insights: insights_json,
            metadata: ExportMetadata {
                project_id: project_id.to_string(),
                generated_at: self.clock.now().to_rfc3339(),
            },
        })
    }
}
