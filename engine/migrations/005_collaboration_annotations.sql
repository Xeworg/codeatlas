-- Migration 005: Collaboration Annotations
-- Annotations table for node-level comments, todos, reviews, and issues.

BEGIN;

CREATE TABLE IF NOT EXISTS annotations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    author TEXT NOT NULL,
    kind TEXT DEFAULT 'comment' CHECK(kind IN ('comment', 'todo', 'review', 'issue')),
    text TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_annotations_project_node ON annotations(project_id, node_id);
CREATE INDEX IF NOT EXISTS idx_annotations_created ON annotations(created_at);

COMMIT;