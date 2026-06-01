-- Migration 006: Health Timeline
-- Stores project health scores over time for trend tracking and executive dashboard.

CREATE TABLE IF NOT EXISTS health_records (
    id TEXT PRIMARY KEY,
    workspace_id TEXT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now')),
    overall_score REAL,
    coupling_score REAL,
    complexity_score REAL,
    cycle_count INTEGER,
    hotspot_count INTEGER
);

CREATE INDEX IF NOT EXISTS idx_health_project_time
    ON health_records(project_id, recorded_at);

CREATE INDEX IF NOT EXISTS idx_health_workspace
    ON health_records(workspace_id)
    WHERE workspace_id IS NOT NULL;

PRAGMA user_version = 6;