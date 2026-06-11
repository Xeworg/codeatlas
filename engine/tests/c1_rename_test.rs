//! Tests for C1.1A: AD-001 rename — AnalysisRepository → AnalysisDataSource
//!
//! RED PHASE: This test references AnalysisDataSource which does NOT exist yet.
//! It fails to COMPILE before the rename (AnalysisRepository is the current name).
//! GREEN PHASE: After renaming AnalysisRepository → AnalysisDataSource, this compiles and passes.

use engine::ports::AnalysisDataSource;
use std::sync::Arc;

/// Verify that AnalysisDataSource trait is the correct rename target.
///
/// This test confirms the trait exists and is accessible from the ports module.
///
#[test]
fn analysis_datasource_trait_is_accessible() {
    // The trait should be re-exported from engine::ports
    fn _requires_trait(_: &dyn AnalysisDataSource) {}
}
