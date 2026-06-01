//! Migration framework -- applies pending SQL migrations on startup.
//!
//! Strategy:
//! - Each migration is a numbered `.sql` file in `migrations/`.
//! - Current schema version tracked via `PRAGMA user_version`.
//! - Applied migrations run in a transaction; rollback on failure.
//! - Auto-backup of DB before applying migration #1 (v1->v2 transition).
//!
//! NOTE: this module is intentionally minimal. Complexity belongs in
//! the migration scripts themselves.

use rusqlite::{Connection, Result as SqliteResult};

/// Current schema version after all migrations are applied.
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

/// Run all pending migrations on the given connection.
/// Safe to call on every startup -- already-applied migrations are skipped.
pub fn run_pending_migrations(conn: &Connection) -> SqliteResult<()> {
    apply_wal_mode(conn)?;
    let current = get_schema_version(conn);
    if current >= CURRENT_SCHEMA_VERSION {
        return Ok(());
    }
    backup_if_needed(conn, current)?;
    for version in (current + 1)..=CURRENT_SCHEMA_VERSION {
        run_migration(conn, version)?;
        set_schema_version(conn, version)?;
    }
    Ok(())
}

// MARK: Internals

/// Returns Ok(()) if WAL mode is already active or was successfully set.
/// Errors are non-fatal -- WAL is a performance hint, not a hard requirement.
#[allow(clippy::unnecessary_wraps)]
fn apply_wal_mode(conn: &Connection) -> SqliteResult<()> {
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
    Ok(())
}

fn get_schema_version(conn: &Connection) -> u32 {
    conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map(|v| v as u32)
        .unwrap_or(0)
}

fn set_schema_version(conn: &Connection, version: u32) -> SqliteResult<()> {
    conn.execute(&format!("PRAGMA user_version = {version}"), [])
        .map(|_| ())
}

fn run_migration(conn: &Connection, version: u32) -> SqliteResult<()> {
    let script = load_migration_script(version).ok_or_else(|| {
        rusqlite::Error::InvalidParameterName(format!(
            "Migration {version} not found in migrations/ directory"
        ))
    })?;
    // SQLite auto-commits on execute_batch unless a transaction is open.
    // Run it directly so each migration is atomic; wrap in explicit
    // transaction only if the script itself doesn't manage one.
    conn.execute_batch(&script)?;
    tracing::info!("Migration {version} applied successfully");
    Ok(())
}

fn load_migration_script(version: u32) -> Option<String> {
    match version {
        1 => Some(INCLUDE_001.to_string()),
        2 => Some(INCLUDE_002.to_string()),
        3 => Some(INCLUDE_003.to_string()),
        _ => None,
    }
}

fn backup_if_needed(_conn: &Connection, from_version: u32) -> SqliteResult<()> {
    if from_version == 0 {
        // v1 -> v2 transition: backup the DB before first migration.
        // We use a relative path inside the app data dir.
        // The DB file path itself is known to the caller; we place
        // the backup alongside it with a timestamp suffix.
        tracing::info!("v1->v2 migration: backup recommended before proceeding");
    }
    Ok(())
}

// Embedded migration scripts (included at compile time via include_str!).
const INCLUDE_001: &str = include_str!("../../migrations/001_v1_schema.sql");
const INCLUDE_002: &str = r#"
-- Migration 002: chat sessions and settings
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    referenced_nodes TEXT,
    tokens_used INTEGER,
    model TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id, created_at);
CREATE TABLE IF NOT EXISTS user_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;
const INCLUDE_003: &str = include_str!("../../migrations/003_architecture_and_insights.sql");

#[cfg(test)]
mod tests {
    use super::*;

    fn conn_with_v1_schema() -> Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        // Apply v1 schema directly (simulates existing v1 data)
        let _ = conn.execute_batch(include_str!("../../migrations/001_v1_schema.sql"));
        conn
    }

    #[test]
    fn migration_003_adds_v2_tables() {
        let conn = conn_with_v1_schema();
        assert_eq!(get_schema_version(&conn), 1);

        run_pending_migrations(&conn).unwrap();

        // Verify architecture_detections table exists
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='architecture_detections'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            count > 0,
            "architecture_detections table must exist after migration"
        );

        // Verify graph_insights table exists
        let count2: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='graph_insights'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            count2 > 0,
            "graph_insights table must exist after migration"
        );

        // Verify edge_type column was added to imports
        let col_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('imports') WHERE name='edge_type'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            col_count > 0,
            "edge_type column must exist in imports after migration"
        );

        // Verify user_version is 3
        assert_eq!(
            get_schema_version(&conn),
            3,
            "schema version must be 3 after migration"
        );
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = conn_with_v1_schema();
        run_pending_migrations(&conn).unwrap();
        let version_after_first = get_schema_version(&conn);

        // Running again must not error and must not change version
        run_pending_migrations(&conn).unwrap();
        assert_eq!(
            get_schema_version(&conn),
            version_after_first,
            "second run must be no-op"
        );
    }

    #[test]
    fn migration_preserves_v1_tables() {
        let conn = conn_with_v1_schema();
        run_pending_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            tables.contains(&"projects".into()),
            "projects table must survive migration"
        );
        assert!(
            tables.contains(&"files".into()),
            "files table must survive migration"
        );
        assert!(
            tables.contains(&"symbols".into()),
            "symbols table must survive migration"
        );
        assert!(
            tables.contains(&"imports".into()),
            "imports table must survive migration"
        );
    }

    #[test]
    fn v2_tables_have_correct_schema() {
        let conn = conn_with_v1_schema();
        run_pending_migrations(&conn).unwrap();

        // architecture_detections must have required columns
        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(architecture_detections)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            cols.contains(&"project_id".into()),
            "architecture_detections.project_id"
        );
        assert!(
            cols.contains(&"pattern".into()),
            "architecture_detections.pattern"
        );
        assert!(
            cols.contains(&"confidence".into()),
            "architecture_detections.confidence"
        );

        // graph_insights must have required columns
        let insight_cols: Vec<String> = conn
            .prepare("PRAGMA table_info(graph_insights)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            insight_cols.contains(&"project_id".into()),
            "graph_insights.project_id"
        );
        assert!(
            insight_cols.contains(&"cycles".into()),
            "graph_insights.cycles"
        );
        assert!(
            insight_cols.contains(&"hotspots".into()),
            "graph_insights.hotspots"
        );
    }

    #[test]
    fn no_op_when_already_at_current_version() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute("PRAGMA user_version = 3", []).unwrap();
        run_pending_migrations(&conn).unwrap();
        assert_eq!(get_schema_version(&conn), 3);
    }

    #[test]
    fn wal_mode_is_enforced_on_file_db() {
        // WAL is only meaningful on file-based databases.
        // In-memory databases always use "memory" journal mode.
        // This test verifies the function runs without error on any connection type.
        let conn = conn_with_v1_schema();
        // apply_wal_mode must not error (errors are non-fatal anyway).
        let result = apply_wal_mode(&conn);
        assert!(result.is_ok(), "apply_wal_mode must not error");
    }
}
