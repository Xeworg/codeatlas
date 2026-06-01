-- Migration 003: Architecture Detection + Graph Insights
-- Version: v2.0
-- Applied by: engine/migrations.rs on startup if user_version < 3
-- See: docs/ARQUITECTURA_DATOS_V2_V3.md §4

BEGIN;

-- Add edge_type to imports for progressive edge typing (v1 stays as 'import')
ALTER TABLE imports ADD COLUMN edge_type TEXT NOT NULL DEFAULT 'import';

-- Architecture detection results with evidence trace
CREATE TABLE IF NOT EXISTS architecture_detections (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    pattern     TEXT NOT NULL CHECK(pattern IN ('mvc','layered','clean','hexagonal','unknown')),
    confidence  REAL NOT NULL CHECK(confidence >= 0.0 AND confidence <= 1.0),
    evidence    TEXT,  -- JSON: { nodes: string[], edges: Array, reasons: string[] }
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_arch_detect_project ON architecture_detections(project_id);

-- Graph insights: cycles, hotspots, and coupling metrics
CREATE TABLE IF NOT EXISTS graph_insights (
    project_id   TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    cycles      TEXT,      -- JSON array of cycles: [{nodes: string[], length: number}]
    hotspots    TEXT,      -- JSON array: [{nodeId: string, couplingScore: number, reason: string}]
    avg_coupling REAL,
    density      REAL,
    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

COMMIT;

PRAGMA user_version = 3;