-- Migration 004: Workspaces + Snapshots foundation (v3)
-- Version: v3.0
-- Applied by: engine/src/db/migrations.rs on startup if user_version < 4
-- See: openspec/changes/v3-collaboration-platform/design.md §Migration Strategy

BEGIN;

-- Workspaces: logical grouping of projects
CREATE TABLE IF NOT EXISTS workspaces (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Membership: which projects belong to which workspace
CREATE TABLE IF NOT EXISTS workspace_projects (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    PRIMARY KEY (workspace_id, project_id)
);

-- Snapshots: point-in-time captures of project state
-- payload_json stores serialized graph+insights (optional at PR1 stage)
CREATE TABLE IF NOT EXISTS snapshots (
    id           TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workspace_id  TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
    label         TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    payload_json  TEXT
);
CREATE INDEX IF NOT EXISTS idx_snapshots_project   ON snapshots(project_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_workspace  ON snapshots(workspace_id);

COMMIT;

PRAGMA user_version = 4;