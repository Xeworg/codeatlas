//! WorkspaceService — application service for workspace orchestration.
//!
//! Orchestrates workspace lifecycle, snapshots, annotations, health, and executive
//! summary using the [`WorkspaceRepository`] port.
//!
//! The service is generic over `W: WorkspaceRepository`, so all operations are
//! routed through the port abstraction. In production, `WorkspaceRepositoryAdapter`
//! is passed to the constructor; in tests, a mock double is used.
//!
//! # Design (AD-3, AD-5)
//!
//! ```text
//! Tauri command shim
//!   -> WorkspaceService<WorkspaceRepositoryAdapter>
//!     -> WorkspaceRepository (port)
//!       -> ProjectRepository (infrastructure adapter)
//! ```

use crate::db::queries::{C4View, ExecutiveSummary, SnapshotDiff};
use crate::ports::WorkspaceRepository;
use crate::Result;

// ─────────────────────────────────────────────────────────────────────────────
// Response types — mirror the Tauri command response DTOs.
// Kept here so the service owns the response shape; commands are thin shims.
// ─────────────────────────────────────────────────────────────────────────────

/// Response for workspace operations.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

/// Response for workspace project attachment.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProjectResponse {
    pub workspace_id: String,
    pub project_id: String,
}

/// Response for snapshot operations.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotResponse {
    pub id: String,
    pub project_id: String,
    pub workspace_id: Option<String>,
    pub label: String,
    pub created_at: String,
    pub payload_json: Option<String>,
}

/// Response for annotation/comment operations.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationResponse {
    pub id: String,
    pub project_id: String,
    pub node_id: String,
    pub author: String,
    pub kind: String,
    pub text: String,
    pub created_at: String,
}

/// Health record within a timeline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthRecordResponse {
    pub id: String,
    pub recorded_at: String,
    pub overall_score: f64,
    pub coupling_score: f64,
    pub complexity_score: f64,
    pub cycle_count: i64,
    pub hotspot_count: i64,
}

/// Health timeline response.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthTimelineResponse {
    pub records: Vec<HealthRecordResponse>,
    pub project_id: String,
    pub from: String,
    pub to: String,
}

/// Hotspot item within executive summary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotspotItem {
    pub node_id: String,
    pub coupling_score: f64,
}

/// Executive summary response for a workspace.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutiveSummaryResponse {
    pub workspace_id: String,
    pub total_projects: i64,
    pub total_files: i64,
    pub avg_health_score: Option<f64>,
    pub trend: String,
    pub top_hotspots: Vec<HotspotItem>,
    pub generated_at: String,
}

/// Snapshot diff response.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SnapshotDiffResponse {
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

/// C4 view response.
#[derive(Debug, Clone, serde::Serialize)]
pub struct C4ViewResponse {
    pub level: u8,
    pub systems: Option<Vec<String>>,
    pub containers: Option<Vec<String>>,
    pub warning: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// WorkspaceService
// ─────────────────────────────────────────────────────────────────────────────

/// Application service for workspace orchestration.
///
/// Generic over `W: WorkspaceRepository` so all operations are routed through
/// the port abstraction. In production, pass `WorkspaceRepositoryAdapter`;
/// in tests, pass a mock double to verify the port is exercised.
pub struct WorkspaceService<'pool, W> {
    workspace_repo: W,
    _phantom: std::marker::PhantomData<&'pool ()>,
}

impl<'pool, W> WorkspaceService<'pool, W> {
    /// Construct a new WorkspaceService with the given workspace repository.
    pub fn new(workspace_repo: W) -> Self {
        Self {
            workspace_repo,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'pool, W: WorkspaceRepository> WorkspaceService<'pool, W> {
    /// Create a new workspace.
    pub fn create_workspace(&self, name: &str) -> Result<WorkspaceResponse> {
        let (id, name_out, created_at) = self.workspace_repo.create_workspace(name)?;
        Ok(WorkspaceResponse {
            id,
            name: name_out,
            created_at,
        })
    }

    /// List all workspaces ordered by creation date (newest first).
    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceResponse>> {
        let workspaces: Vec<WorkspaceResponse> = self
            .workspace_repo
            .list_workspaces()?
            .into_iter()
            .map(|(id, name, created_at)| WorkspaceResponse {
                id,
                name,
                created_at,
            })
            .collect();
        Ok(workspaces)
    }

    /// Attach a project to a workspace.
    pub fn attach_project_to_workspace(&self, workspace_id: &str, project_id: &str) -> Result<()> {
        self.workspace_repo
            .attach_project_to_workspace(workspace_id, project_id)
    }

    /// List projects attached to a workspace.
    pub fn list_workspace_projects(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceProjectResponse>> {
        Ok(self
            .workspace_repo
            .list_workspace_projects(workspace_id)?
            .into_iter()
            .map(|(workspace_id, project_id)| WorkspaceProjectResponse {
                workspace_id,
                project_id,
            })
            .collect())
    }

    /// Create a snapshot for a project.
    pub fn create_snapshot(
        &self,
        project_id: &str,
        label: &str,
        workspace_id: Option<&str>,
    ) -> Result<SnapshotResponse> {
        let (id, project_id_out, workspace_id_out, label_out, created_at, payload_json) = self
            .workspace_repo
            .create_snapshot(project_id, label, workspace_id)?;
        Ok(SnapshotResponse {
            id,
            project_id: project_id_out,
            workspace_id: workspace_id_out,
            label: label_out,
            created_at,
            payload_json,
        })
    }

    /// Get a snapshot by ID.
    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<SnapshotResponse>> {
        Ok(self.workspace_repo.get_snapshot(snapshot_id)?.map(
            |(id, project_id, workspace_id, label, created_at, payload_json)| SnapshotResponse {
                id,
                project_id,
                workspace_id,
                label,
                created_at,
                payload_json,
            },
        ))
    }

    /// List snapshots for a project, optionally filtered by workspace.
    pub fn list_snapshots(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<Vec<SnapshotResponse>> {
        Ok(self
            .workspace_repo
            .list_snapshots(project_id, workspace_id)?
            .into_iter()
            .map(
                |(id, project_id, workspace_id, label, created_at, payload_json): (
                    String,
                    String,
                    Option<String>,
                    String,
                    String,
                    Option<String>,
                )| SnapshotResponse {
                    id,
                    project_id,
                    workspace_id,
                    label,
                    created_at,
                    payload_json,
                },
            )
            .collect())
    }

    /// Add a comment/annotation to a node.
    #[allow(clippy::type_complexity)]
    pub fn add_comment(
        &self,
        project_id: &str,
        node_id: &str,
        author: &str,
        text: &str,
        kind: Option<&str>,
    ) -> Result<AnnotationResponse> {
        let (id, project_id_out, node_id_out, author_out, kind_out, text_out, created_at) = self
            .workspace_repo
            .add_comment(project_id, node_id, author, text, kind)?;
        Ok(AnnotationResponse {
            id,
            project_id: project_id_out,
            node_id: node_id_out,
            author: author_out,
            kind: kind_out,
            text: text_out,
            created_at,
        })
    }

    /// List comments for a project, optionally filtered by node.
    #[allow(clippy::type_complexity)]
    pub fn list_comments(
        &self,
        project_id: &str,
        node_id: Option<&str>,
    ) -> Result<Vec<AnnotationResponse>> {
        Ok(self
            .workspace_repo
            .list_comments(project_id, node_id)?
            .into_iter()
            .map(
                |(id, project_id, node_id, author, kind, text, created_at): (
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                )| AnnotationResponse {
                    id,
                    project_id,
                    node_id,
                    author,
                    kind,
                    text,
                    created_at,
                },
            )
            .collect())
    }

    /// Get health timeline for a project within a date range.
    #[allow(clippy::type_complexity)]
    pub fn get_health_timeline(
        &self,
        project_id: &str,
        from: &str,
        to: &str,
    ) -> Result<HealthTimelineResponse> {
        Ok(HealthTimelineResponse {
            records: self
                .workspace_repo
                .get_health_timeline(project_id, from, to)?
                .into_iter()
                .map(
                    |(
                        id,
                        recorded_at,
                        overall_score,
                        coupling_score,
                        complexity_score,
                        cycle_count,
                        hotspot_count,
                    ): (String, String, f64, f64, f64, i64, i64)| {
                        HealthRecordResponse {
                            id,
                            recorded_at,
                            overall_score,
                            coupling_score,
                            complexity_score,
                            cycle_count,
                            hotspot_count,
                        }
                    },
                )
                .collect(),
            project_id: project_id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
        })
    }

    /// Get executive summary for a workspace.
    pub fn get_executive_summary(&self, workspace_id: &str) -> Result<ExecutiveSummaryResponse> {
        let s: ExecutiveSummary = self
            .workspace_repo
            .compute_executive_summary(workspace_id)?;
        Ok(ExecutiveSummaryResponse {
            workspace_id: s.workspace_id,
            total_projects: s.total_projects,
            total_files: s.total_files,
            avg_health_score: s.avg_health_score,
            trend: s.trend,
            top_hotspots: s
                .top_hotspots
                .into_iter()
                .map(|(node_id, coupling_score): (String, f64)| HotspotItem {
                    node_id,
                    coupling_score,
                })
                .collect(),
            generated_at: s.generated_at,
        })
    }

    /// Compare two snapshots and return diff.
    pub fn compare_snapshots(
        &self,
        base_snapshot_id: &str,
        target_snapshot_id: &str,
    ) -> Result<SnapshotDiffResponse> {
        let d: SnapshotDiff = self
            .workspace_repo
            .compare_snapshots(base_snapshot_id, target_snapshot_id)?;
        Ok(SnapshotDiffResponse {
            base_snapshot_id: d.base_snapshot_id,
            target_snapshot_id: d.target_snapshot_id,
            nodes_added: d.nodes_added,
            nodes_removed: d.nodes_removed,
            nodes_modified: d.nodes_modified,
            edges_added: d.edges_added,
            edges_removed: d.edges_removed,
            coupling_delta: d.coupling_delta,
            complexity_delta: d.complexity_delta,
            cycles_delta: d.cycles_delta,
        })
    }

    /// Get C4 view for a project at a given level.
    pub fn get_c4_view(&self, project_id: &str, level: u8) -> Result<C4ViewResponse> {
        let v: C4View = self.workspace_repo.get_c4_view(project_id, level)?;
        Ok(C4ViewResponse {
            level: v.level,
            systems: v.systems,
            containers: v.containers,
            warning: v.warning,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::WorkspaceRepositoryAdapter;

    /// Verify WorkspaceService compiles with real adapters and basic trait bounds satisfied.
    #[test]
    fn workspace_service_compiles_with_real_adapters() {
        let pool = crate::db::DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        pool.with_connection(|conn| crate::db::migrations::run_pending_migrations(conn))
            .unwrap();

        let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
        let service = WorkspaceService::new(workspace_repo);
        // Service constructed — trait bounds satisfied
        assert!(true);
    }
}
