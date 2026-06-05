-- Migration 007: Outline Items
-- Stores hierarchical outline data per file as JSON for fast retrieval.
-- Additive only: does not modify existing tables or contracts.

BEGIN;

CREATE TABLE IF NOT EXISTS outline_items (
    file_id TEXT PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    outline_json TEXT NOT NULL,
    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_outline_items_generated_at
ON outline_items(generated_at);

COMMIT;