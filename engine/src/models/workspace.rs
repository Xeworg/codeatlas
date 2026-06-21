//! Domain types for workspace, snapshot, comment, and health timeline operations.
//!
//! These types replace the tuple-typed return values in [`WorkspaceRepository`](crate::ports::WorkspaceRepository).
//! Moving them to the domain layer enforces compile-time type safety and self-documents intent.
//!
//! Design: AD-005 (wave-2 hexagonal completion).
//!
//! ## Types
//!
//! - [`WorkspaceMeta`] — workspace identity (id, name, created_at)
//! - [`WorkspaceProjectMeta`] — workspace ↔ project attachment
//! - [`SnapshotMeta`] — snapshot record (id, project, workspace, label, created, payload)
//! - [`CommentMeta`] — annotation/comment record
//! - [`HealthRecord`] — health timeline entry
//! - [`ExecutiveSummary`] — workspace health overview (moved from `db::queries`)
//! - [`SnapshotDiff`] — diff between two snapshots (moved from `db::queries`)
//! - [`C4View`] — C4 model view for a project (moved from `db::queries`)

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Workspace domain types
// ─────────────────────────────────────────────────────────────────────────────

/// Workspace identity returned by create/list operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMeta {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

/// Workspace ↔ project attachment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceProjectMeta {
    pub workspace_id: String,
    pub project_id: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Snapshot domain types
// ─────────────────────────────────────────────────────────────────────────────

/// Snapshot record returned by create/get/list snapshot operations.
///
/// Fields: (id, project_id, workspace_id, label, created_at, payload_json).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotMeta {
    pub id: String,
    pub project_id: String,
    pub workspace_id: Option<String>,
    pub label: String,
    pub created_at: String,
    pub payload_json: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Comment/annotation domain types
// ─────────────────────────────────────────────────────────────────────────────

/// Comment/annotation record returned by add/list comment operations.
///
/// Fields: (id, project_id, node_id, author, kind, text, created_at).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentMeta {
    pub id: String,
    pub project_id: String,
    pub node_id: String,
    pub author: String,
    pub kind: String,
    pub text: String,
    pub created_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Health timeline domain types
// ─────────────────────────────────────────────────────────────────────────────

/// Health record within a timeline.
///
/// Fields: (id, recorded_at, overall_score, coupling_score, complexity_score, cycle_count, hotspot_count).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthRecord {
    pub id: String,
    pub recorded_at: String,
    pub overall_score: f64,
    pub coupling_score: f64,
    pub complexity_score: f64,
    pub cycle_count: i64,
    pub hotspot_count: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Executive Summary / Snapshot Diff / C4View
// Moved from `db::queries` per AD-005 / C4 task 7.2.
// ─────────────────────────────────────────────────────────────────────────────

/// Hotspot item within an executive summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotItem {
    pub node_id: String,
    pub coupling_score: f64,
}

/// Executive summary for a workspace.
///
/// Produced by [`crate::ports::WorkspaceRepository::compute_executive_summary`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub workspace_id: String,
    pub total_projects: i64,
    pub total_files: i64,
    pub avg_health_score: Option<f64>,
    pub trend: String, // "up", "down", "stable"
    pub top_hotspots: Vec<(String, f64)>,
    pub generated_at: String,
}

/// Diff between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
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

/// C4 view for a project at a given level.
///
/// Level 1 = System Context, Level 2 = Container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4View {
    pub level: u8,
    pub systems: Option<Vec<String>>,
    pub containers: Option<Vec<String>>,
    pub warning: Option<String>,
}
