//! WorkspaceService integration tests — PR-5 corrective repair (CR-2).
//!
//! CR-2 fixes the confirmed blocker: WorkspaceService was bypassing
//! `WorkspaceRepository` and directly instantiating `ProjectRepository`,
//! making the generic mock/test surface fake.
//!
//! After the fix:
//! - WorkspaceService is generic over `W: WorkspaceRepository` (single port)
//! - All operations delegate to the port — no direct `ProjectRepository` calls
//! - Mock tests prove the port is exercised, not bypassed
//!
//! # Contracts (from design.md AD-3, spec AD-5)
//!
//! - Workspace CRUD: create, list, attach projects
//! - Snapshot lifecycle: create, get, list
//! - Annotations: add_comment, list_comments
//! - Health timeline and executive summary
//! - Snapshot diff and C4 view

use engine::models::{
    CommentMeta, HealthRecord, SnapshotMeta, WorkspaceMeta, WorkspaceProjectMeta,
};
use engine::ports::WorkspaceRepository;
use std::sync::Mutex;

// ─────────────────────────────────────────────────────────────────────────────
// Recording mock for WorkspaceRepository — proves port abstraction is real
// ─────────────────────────────────────────────────────────────────────────────

/// Recording mock for WorkspaceRepository.
///
/// This mock records all calls and returns deterministic data. It proves that
/// WorkspaceService delegates to the port abstraction rather than bypassing it.
///
/// The mock implements ALL 12 methods of WorkspaceRepository so it is a valid
/// test double for the full service surface.
struct RecordingWorkspaceRepo {
    pub workspaces: Mutex<Vec<WorkspaceMeta>>,
    pub workspace_projects: Mutex<Vec<WorkspaceProjectMeta>>,
    pub snapshots: Mutex<Vec<SnapshotMeta>>,
    pub comments: Mutex<Vec<CommentMeta>>,
    pub health_timelines: Mutex<Vec<HealthRecord>>,
}

impl RecordingWorkspaceRepo {
    fn new() -> Self {
        Self {
            workspaces: Mutex::new(Vec::new()),
            workspace_projects: Mutex::new(Vec::new()),
            snapshots: Mutex::new(Vec::new()),
            comments: Mutex::new(Vec::new()),
            health_timelines: Mutex::new(Vec::new()),
        }
    }
}

impl WorkspaceRepository for RecordingWorkspaceRepo {
    fn create_workspace(&self, name: &str) -> engine::Result<WorkspaceMeta> {
        let id = format!("ws-{}", uuid::Uuid::new_v4());
        let ts = chrono::Utc::now().to_rfc3339();
        let meta = WorkspaceMeta {
            id: id.clone(),
            name: name.to_string(),
            created_at: ts.clone(),
        };
        self.workspaces.lock().unwrap().push(meta.clone());
        Ok(meta)
    }

    fn list_workspaces(&self) -> engine::Result<Vec<WorkspaceMeta>> {
        Ok(self.workspaces.lock().unwrap().clone())
    }

    fn attach_project_to_workspace(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> engine::Result<()> {
        self.workspace_projects
            .lock()
            .unwrap()
            .push(WorkspaceProjectMeta {
                workspace_id: workspace_id.to_string(),
                project_id: project_id.to_string(),
            });
        Ok(())
    }

    fn list_workspace_projects(
        &self,
        workspace_id: &str,
    ) -> engine::Result<Vec<WorkspaceProjectMeta>> {
        let all = self.workspace_projects.lock().unwrap().clone();
        Ok(all
            .into_iter()
            .filter(|meta| meta.workspace_id == workspace_id)
            .collect())
    }

    fn create_snapshot(
        &self,
        project_id: &str,
        label: &str,
        workspace_id: Option<&str>,
    ) -> engine::Result<SnapshotMeta> {
        let id = format!("snap-{}", uuid::Uuid::new_v4());
        let ts = chrono::Utc::now().to_rfc3339();
        let meta = SnapshotMeta {
            id: id.clone(),
            project_id: project_id.to_string(),
            workspace_id: workspace_id.map(String::from),
            label: label.to_string(),
            created_at: ts,
            payload_json: None,
        };
        self.snapshots.lock().unwrap().push(meta.clone());
        Ok(meta)
    }

    fn get_snapshot(&self, snapshot_id: &str) -> engine::Result<Option<SnapshotMeta>> {
        let all = self.snapshots.lock().unwrap().clone();
        Ok(all.into_iter().find(|m| m.id == snapshot_id))
    }

    fn list_snapshots(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
    ) -> engine::Result<Vec<SnapshotMeta>> {
        let all = self.snapshots.lock().unwrap().clone();
        Ok(all
            .into_iter()
            .filter(|meta| {
                meta.project_id == project_id && meta.workspace_id.as_deref() == workspace_id
            })
            .collect())
    }

    fn add_comment(
        &self,
        project_id: &str,
        node_id: &str,
        author: &str,
        text: &str,
        kind: Option<&str>,
    ) -> engine::Result<CommentMeta> {
        let id = format!("comment-{}", uuid::Uuid::new_v4());
        let ts = chrono::Utc::now().to_rfc3339();
        let meta = CommentMeta {
            id: id.clone(),
            project_id: project_id.to_string(),
            node_id: node_id.to_string(),
            author: author.to_string(),
            kind: kind.unwrap_or("comment").to_string(),
            text: text.to_string(),
            created_at: ts,
        };
        self.comments.lock().unwrap().push(meta.clone());
        Ok(meta)
    }

    fn list_comments(
        &self,
        project_id: &str,
        node_id: Option<&str>,
    ) -> engine::Result<Vec<CommentMeta>> {
        let all = self.comments.lock().unwrap().clone();
        Ok(all
            .into_iter()
            .filter(|meta| {
                meta.project_id == project_id && node_id.map_or(true, |n| meta.node_id == n)
            })
            .collect())
    }

    fn get_health_timeline(
        &self,
        project_id: &str,
        _from: &str,
        _to: &str,
    ) -> engine::Result<Vec<HealthRecord>> {
        // Return empty timeline for mock — real tests use DB adapter
        Ok(self
            .health_timelines
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.id == project_id)
            .cloned()
            .collect())
    }

    fn compute_executive_summary(
        &self,
        workspace_id: &str,
    ) -> engine::Result<engine::models::ExecutiveSummary> {
        Ok(engine::models::ExecutiveSummary {
            workspace_id: workspace_id.to_string(),
            total_projects: 0,
            total_files: 0,
            avg_health_score: None,
            trend: "stable".to_string(),
            top_hotspots: vec![],
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn compare_snapshots(
        &self,
        base_snapshot_id: &str,
        target_snapshot_id: &str,
    ) -> engine::Result<engine::models::SnapshotDiff> {
        Ok(engine::models::SnapshotDiff {
            base_snapshot_id: base_snapshot_id.to_string(),
            target_snapshot_id: target_snapshot_id.to_string(),
            nodes_added: vec![],
            nodes_removed: vec![],
            nodes_modified: vec![],
            edges_added: vec![],
            edges_removed: vec![],
            coupling_delta: 0.0,
            complexity_delta: 0.0,
            cycles_delta: 0,
        })
    }

    fn get_c4_view(&self, project_id: &str, level: u8) -> engine::Result<engine::models::C4View> {
        Ok(engine::models::C4View {
            level,
            systems: Some(vec![format!("sys-{}", project_id)]),
            containers: Some(vec![format!("container-{}", project_id)]),
            warning: None,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.17 — WorkspaceService depends on WorkspaceRepository port (CR-2 RED test)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn workspace_service_delegates_to_workspace_repository_port() {
    use engine::services::WorkspaceService;

    // Recording mock that proves the port is exercised
    let mock_repo = RecordingWorkspaceRepo::new();

    // WorkspaceService must accept W: WorkspaceRepository (not pool + dead fields)
    // If this fails to compile, the service is still bypassing the port.
    let service: WorkspaceService<'_, RecordingWorkspaceRepo> = WorkspaceService::new(mock_repo);

    // Exercise the port: create a workspace
    let result = service.create_workspace("Port Test");
    assert!(
        result.is_ok(),
        "create_workspace should delegate to WorkspaceRepository"
    );

    // Verify the mock recorded the call — proves port abstraction is real
    let workspaces = service.list_workspaces().unwrap();
    assert_eq!(
        workspaces.len(),
        1,
        "WorkspaceRepository should have recorded the call"
    );
    assert_eq!(workspaces[0].name, "Port Test");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.18 — WorkspaceService with real DB adapters compiles and works
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn workspace_service_compiles_with_real_adapters() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    // Smoke test: create and list workspaces
    let ws = service.create_workspace("Real DB Test").unwrap();
    let workspaces = service.list_workspaces().unwrap();

    assert!(
        workspaces.iter().any(|w| w.id == ws.id),
        "created workspace should appear in list"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.19 — create_workspace: creates a workspace and returns id, name, created_at
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn create_workspace_returns_workspace_response() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let result = service.create_workspace("My Workspace");

    assert!(
        result.is_ok(),
        "create_workspace should succeed, got: {:?}",
        result
    );
    let ws = result.unwrap();
    assert!(!ws.id.is_empty(), "workspace id should not be empty");
    assert_eq!(ws.name, "My Workspace");
    assert!(!ws.created_at.is_empty(), "created_at should not be empty");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.20 — list_workspaces: returns all workspaces
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_workspaces_returns_all_workspaces() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    // Create two workspaces first
    service.create_workspace("Workspace A").unwrap();
    service.create_workspace("Workspace B").unwrap();

    let result = service.list_workspaces();

    assert!(result.is_ok(), "list_workspaces should succeed");
    let workspaces = result.unwrap();
    assert_eq!(workspaces.len(), 2, "should have 2 workspaces");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.21 — attach_project_to_workspace: attaches a project to a workspace
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn attach_project_to_workspace_succeeds() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let ws = service.create_workspace("Test Workspace").unwrap();
    let result = service.attach_project_to_workspace(&ws.id, "proj-123");

    assert!(result.is_ok(), "attach_project_to_workspace should succeed");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.22 — list_workspace_projects: returns projects attached to a workspace
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_workspace_projects_returns_attached_projects() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let ws = service.create_workspace("Test Workspace").unwrap();
    service
        .attach_project_to_workspace(&ws.id, "proj-a")
        .unwrap();
    service
        .attach_project_to_workspace(&ws.id, "proj-b")
        .unwrap();

    let result = service.list_workspace_projects(&ws.id);

    assert!(result.is_ok());
    let projects = result.unwrap();
    assert_eq!(projects.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.23 — create_snapshot: creates a snapshot and returns full response
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn create_snapshot_returns_snapshot_response() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let result = service.create_snapshot("proj-1", "Baseline", None);

    assert!(result.is_ok(), "create_snapshot should succeed");
    let snap = result.unwrap();
    assert!(!snap.id.is_empty());
    assert_eq!(snap.project_id, "proj-1");
    assert_eq!(snap.label, "Baseline");
    assert!(snap.workspace_id.is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.24 — get_snapshot: returns snapshot by ID
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_snapshot_returns_snapshot_by_id() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let created = service
        .create_snapshot("proj-1", "Test Snap", None)
        .unwrap();
    let result = service.get_snapshot(&created.id);

    assert!(result.is_ok());
    let snap = result.unwrap();
    assert!(snap.is_some());
    let snap = snap.unwrap();
    assert_eq!(snap.id, created.id);
    assert_eq!(snap.label, "Test Snap");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.25 — get_snapshot returns None for unknown ID
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_snapshot_returns_none_for_unknown_id() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let result = service.get_snapshot("nonexistent-snapshot-id");

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.26 — list_snapshots: returns snapshots for a project
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_snapshots_returns_project_snapshots() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    service.create_snapshot("proj-1", "Snap A", None).unwrap();
    service.create_snapshot("proj-1", "Snap B", None).unwrap();
    service.create_snapshot("proj-2", "Snap C", None).unwrap();

    let result = service.list_snapshots("proj-1", None);

    assert!(result.is_ok());
    let snaps = result.unwrap();
    assert_eq!(snaps.len(), 2, "proj-1 should have 2 snapshots");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.27 — add_comment: adds a comment and returns annotation response
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn add_comment_returns_annotation_response() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let result = service.add_comment("proj-1", "node-1", "author1", "Test comment", None);

    assert!(result.is_ok(), "add_comment should succeed");
    let annotation = result.unwrap();
    assert!(!annotation.id.is_empty());
    assert_eq!(annotation.project_id, "proj-1");
    assert_eq!(annotation.node_id, "node-1");
    assert_eq!(annotation.text, "Test comment");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.28 — list_comments: returns comments for a project
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_comments_returns_project_comments() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    service
        .add_comment("proj-1", "node-1", "a1", "Comment 1", None)
        .unwrap();
    service
        .add_comment("proj-1", "node-1", "a2", "Comment 2", None)
        .unwrap();
    service
        .add_comment("proj-1", "node-2", "a3", "Comment 3", None)
        .unwrap();

    let result = service.list_comments("proj-1", None);

    assert!(result.is_ok());
    let comments = result.unwrap();
    assert_eq!(comments.len(), 3, "should have 3 comments");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.29 — list_comments filtered by node_id
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_comments_filtered_by_node_id() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    service
        .add_comment("proj-1", "node-1", "a1", "Comment1", None)
        .unwrap();
    service
        .add_comment("proj-1", "node-1", "a2", "Comment 2", None)
        .unwrap();
    service
        .add_comment("proj-1", "node-2", "a3", "Comment 3", None)
        .unwrap();

    let result = service.list_comments("proj-1", Some("node-1"));

    assert!(result.is_ok());
    let comments = result.unwrap();
    assert_eq!(comments.len(), 2, "node-1 should have 2 comments");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.30 — get_health_timeline: returns health records for a time range
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_health_timeline_returns_records() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    // With no health records, timeline should return empty records list
    let result = service.get_health_timeline("proj-nonexistent", "2020-01-01", "2030-01-01");

    assert!(result.is_ok(), "get_health_timeline should succeed");
    let timeline = result.unwrap();
    assert_eq!(timeline.project_id, "proj-nonexistent");
    assert_eq!(timeline.from, "2020-01-01");
    assert_eq!(timeline.to, "2030-01-01");
    assert!(
        timeline.records.is_empty(),
        "nonexistent project should have no health records"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.31 — get_executive_summary: returns workspace summary
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_executive_summary_returns_summary() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    // Create a workspace first
    let ws = service.create_workspace("Exec Summary Test").unwrap();

    let result = service.get_executive_summary(&ws.id);

    assert!(result.is_ok(), "get_executive_summary should succeed");
    let summary = result.unwrap();
    assert_eq!(summary.workspace_id, ws.id);
    assert_eq!(summary.total_projects, 0, "no projects attached yet");
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.32 — compare_snapshots: returns diff between two snapshots
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn compare_snapshots_returns_diff() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    // Compare nonexistent snapshots — should return empty diff (not error)
    let result = service.compare_snapshots("nonexistent-base", "nonexistent-target");

    assert!(result.is_ok(), "compare_snapshots should succeed");
    let diff = result.unwrap();
    assert_eq!(diff.base_snapshot_id, "nonexistent-base");
    assert_eq!(diff.target_snapshot_id, "nonexistent-target");
    assert!(diff.nodes_added.is_empty());
    assert!(diff.nodes_removed.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// T12.33 — get_c4_view: returns C4 view for a project
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_c4_view_returns_c4_view() {
    use engine::ports::WorkspaceRepositoryAdapter;
    use engine::services::WorkspaceService;

    let pool = engine::db::DbPool::in_memory().unwrap();
    pool.init_schema().unwrap();
    pool.with_connection(|conn| engine::db::migrations::run_pending_migrations(conn))
        .unwrap();

    let workspace_repo = WorkspaceRepositoryAdapter::new(&pool);
    let service = WorkspaceService::new(workspace_repo);

    let result = service.get_c4_view("nonexistent-project", 1);

    assert!(result.is_ok(), "get_c4_view should succeed");
    let view = result.unwrap();
    assert_eq!(view.level, 1);
}
