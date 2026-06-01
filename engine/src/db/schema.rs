//! SQLite schema for CodeAtlas v1

use rusqlite::{Connection, Result};

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            root_path TEXT NOT NULL UNIQUE,
            files_count INTEGER DEFAULT 0,
            symbols_count INTEGER DEFAULT 0,
            imports_count INTEGER DEFAULT 0,
            scan_duration_ms INTEGER,
            status TEXT DEFAULT 'idle',
            error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS files (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            extension TEXT NOT NULL,
            lines INTEGER DEFAULT 0,
            content_hash TEXT,
            parsed_at TEXT,
            UNIQUE(project_id, path)
        );

        CREATE TABLE IF NOT EXISTS symbols (
            id TEXT PRIMARY KEY,
            file_id TEXT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            line_end INTEGER NOT NULL,
            is_exported BOOLEAN DEFAULT FALSE
        );

        CREATE TABLE IF NOT EXISTS imports (
            id TEXT PRIMARY KEY,
            source_file_id TEXT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            target_file_id TEXT REFERENCES files(id) ON DELETE SET NULL,
            target_module TEXT,
            import_names TEXT NOT NULL,
            is_default BOOLEAN DEFAULT FALSE,
            is_type_import BOOLEAN DEFAULT FALSE
        );

        CREATE TABLE IF NOT EXISTS graph_cache (
            project_id TEXT PRIMARY KEY,
            graph_json TEXT NOT NULL,
            generated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS ai_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            provider TEXT NOT NULL DEFAULT 'anthropic',
            api_key TEXT,
            model TEXT NOT NULL DEFAULT 'minimax',
            endpoint TEXT,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_files_project ON files(project_id);
        CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_id);
        CREATE INDEX IF NOT EXISTS idx_imports_source ON imports(source_file_id);
        CREATE INDEX IF NOT EXISTS idx_imports_target ON imports(target_file_id);
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_initializes_without_error() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        assert!(init_schema(&conn).is_ok());

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"projects".into()));
        assert!(tables.contains(&"files".into()));
        assert!(tables.contains(&"symbols".into()));
        assert!(tables.contains(&"imports".into()));
        assert!(tables.contains(&"graph_cache".into()));
        assert!(tables.contains(&"ai_config".into()));
    }
}
