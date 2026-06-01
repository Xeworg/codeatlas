//! Analysis module — v2 advanced analysis
//!
//! Contains heuristic-based analysis algorithms for graph data:
//! - Architecture detection (pattern classification)
//! - Impact analysis (change propagation)
//! - Graph insights (cycles, hotspots, coupling metrics)

pub mod architecture_detector;
#[cfg(test)]
pub mod degraded_tests;
pub mod graph_insights;
pub mod impact_engine;

pub use architecture_detector::{
    detect_architecture, ArchitectureDetectionResult, ArchitecturePattern,
};
pub use graph_insights::{compute_graph_insights, GraphInsights, InsightsConfig};
pub use impact_engine::{compute_impact, ImpactAnalysisResult, ImpactConfig};
