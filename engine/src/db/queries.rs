//! Database queries — CRUD for projects, files, symbols, imports.

use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::sync::Mutex;

use crate::models::{FileInfo, ImportInfo, ScanResult};

/// Thread-safe wrapper around rusqlite::Connection.
/// rusqlite::Connection is NOT Send+Sync. Guarding all access
/// through a Mutex makes it safe for Tauri's async multi-threaded runtime.
pub struct DbPool(Mutex<Connection>);

impl DbPool {
    /// Open a connection to a file-path database.
    pub fn new(path: &str) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        Ok(Self(Mutex::new(conn)))
    }

    /// Open an in-memory database (for tests).
    #[cfg(test)]
    pub fn in_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self(Mutex::new(conn)))
    }

    /// Execute a closure with a locked reference to the connection.
    /// Returns SqliteResult so callers can propagate errors cleanly.
    pub fn with_connection<T, F>(&self, f: F) -> SqliteResult<T>
    where
        F: FnOnce(&Connection) -> SqliteResult<T>,
    {
        let guard = self
            .0
            .lock()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        f(&guard)
    }

    /// Initialize the database schema (helper for tests & app init).
    /// Uses crate::db::schema which is accessible since this function
    /// is called on DbPool (not from within schema module itself).
    pub fn init_schema(&self) -> SqliteResult<()> {
        use crate::db::schema as schema_mod;
        self.with_connection(schema_mod::init_schema)
    }
}

/// Low-level repository that borrows a locked connection for the duration
/// of a single operation. Creates a new borrow per method call so that
/// multiple concurrent Tauri commands don't block each other.
pub struct ProjectRepository<'pool> {
    pool: &'pool DbPool,
}

impl<'pool> ProjectRepository<'pool> {
    pub fn new(pool: &'pool DbPool) -> Self {
        Self { pool }
    }

    pub fn save_scan_result(&self, result: &ScanResult) -> SqliteResult<()> {
        self.pool.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO projects
                 (id, name, root_path, files_count, symbols_count, imports_count,
                  scan_duration_ms, status, error, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
                params![
                    result.project_id,
                    result.project_name,
                    result.root_path,
                    result.files_count as i64,
                    result.symbols_count as i64,
                    result.imports_count as i64,
                    result.scan_duration_ms as i64,
                    format!("{:?}", result.status).to_lowercase(),
                    result.error,
                ],
            )?;

            for file in &result.files {
                self.save_file_internal(conn, result.project_id.as_str(), file)?;
            }

            Ok(())
        })
    }

    fn save_file_internal(
        &self,
        conn: &Connection,
        project_id: &str,
        file: &FileInfo,
    ) -> SqliteResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO files
             (id, project_id, path, name, extension, lines)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file.id,
                project_id,
                file.path,
                file.name,
                file.extension,
                file.lines as i64,
            ],
        )?;

        for symbol in &file.symbols {
            conn.execute(
                "INSERT OR REPLACE INTO symbols
                 (id, file_id, name, kind, line_start, line_end, is_exported)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    symbol.id,
                    file.id,
                    symbol.name,
                    format!("{:?}", symbol.kind).to_lowercase(),
                    symbol.line_start as i64,
                    symbol.line_end as i64,
                    symbol.exports,
                ],
            )?;
        }

        Ok(())
    }

    pub fn get_project(&self, project_id: &str) -> SqliteResult<Option<(String, String, i64)>> {
        self.pool.with_connection(|conn| {
            conn.query_row(
                "SELECT name, root_path, files_count FROM projects WHERE id = ?1",
                params![project_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
        })
    }

    pub fn get_files(&self, project_id: &str) -> SqliteResult<Vec<FileInfo>> {
        self.pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, name, extension, lines
                     FROM files WHERE project_id = ?1",
            )?;

            let files = stmt.query_map(params![project_id], |row| {
                Ok(FileInfo {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    extension: row.get(3)?,
                    lines: row.get::<_, i64>(4)? as u32,
                    symbols: vec![],
                })
            })?;

            files.collect()
        })
    }

    pub fn save_import(&self, import: &ImportInfo) -> SqliteResult<()> {
        self.pool.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO imports
                 (id, source_file_id, target_file_id, target_module, import_names,
                  is_default, is_type_import)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    import.id,
                    import.source_file_id,
                    import.target_file_id,
                    import.target_module,
                    serde_json::to_string(&import.imports).unwrap_or_default(),
                    import.is_default,
                    import.is_type,
                ],
            )?;
            Ok(())
        })
    }

    /// Save a serialized graph for a project.
    pub fn save_graph_cache(&self, project_id: &str, graph_json: &str) -> SqliteResult<()> {
        self.pool.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO graph_cache (project_id, graph_json, generated_at)
                 VALUES (?1, ?2, datetime('now'))",
                params![project_id, graph_json],
            )?;
            Ok(())
        })
    }

    /// Retrieve cached graph JSON for a project.
    pub fn get_graph_cache(&self, project_id: &str) -> SqliteResult<Option<String>> {
        self.pool.with_connection(|conn| {
            conn.query_row(
                "SELECT graph_json FROM graph_cache WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .optional()
        })
    }

    /// Search nodes (files) by name substring (case-insensitive).
    pub fn search_files(
        &self,
        project_id: &str,
        query: &str,
        limit: usize,
    ) -> SqliteResult<Vec<FileInfo>> {
        self.pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, name, extension, lines
                     FROM files
                     WHERE project_id = ?1 AND (name LIKE ?2 OR path LIKE ?2)
                     LIMIT ?3",
            )?;
            let pattern = format!("%{}%", query);
            let files = stmt.query_map(params![project_id, pattern, limit as i64], |row| {
                Ok(FileInfo {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    extension: row.get(3)?,
                    lines: row.get::<_, i64>(4)? as u32,
                    symbols: vec![],
                })
            })?;
            files.collect()
        })
    }

    /// Get file info by ID.
    pub fn get_file_by_id(&self, file_id: &str) -> SqliteResult<Option<FileInfo>> {
        self.pool.with_connection(|conn| {
            conn.query_row(
                "SELECT id, path, name, extension, lines FROM files WHERE id = ?1",
                params![file_id],
                |row| {
                    Ok(FileInfo {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        name: row.get(2)?,
                        extension: row.get(3)?,
                        lines: row.get::<_, i64>(4)? as u32,
                        symbols: vec![],
                    })
                },
            )
            .optional()
        })
    }

    // ──────────────────────────────────────────────────────────────
    // v2: Architecture detection persistence
    // ──────────────────────────────────────────────────────────────

    /// Persist an architecture detection result.
    pub fn save_architecture_detection(
        &self,
        project_id: &str,
        pattern: &str,
        confidence: f64,
        evidence_json: &str,
    ) -> SqliteResult<()> {
        self.pool.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO architecture_detections
                 (id, project_id, pattern, confidence, evidence, detected_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    project_id,
                    pattern,
                    confidence,
                    evidence_json,
                ],
            )?;
            Ok(())
        })
    }

    /// Get the most recent architecture detection for a project.
    pub fn get_latest_architecture_detection(
        &self,
        project_id: &str,
    ) -> SqliteResult<Option<(String, f64, String, String)>> {
        self.pool.with_connection(|conn| {
            conn.query_row(
                "SELECT pattern, confidence, evidence, detected_at
                 FROM architecture_detections
                 WHERE project_id = ?1
                 ORDER BY detected_at DESC LIMIT 1",
                params![project_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()
        })
    }

    // ──────────────────────────────────────────────────────────────
    // v2: Graph insights persistence
    // ──────────────────────────────────────────────────────────────

    /// Save graph insights for a project (upsert).
    pub fn save_graph_insights(
        &self,
        project_id: &str,
        cycles_json: &str,
        hotspots_json: &str,
        avg_coupling: Option<f64>,
        density: Option<f64>,
    ) -> SqliteResult<()> {
        self.pool.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO graph_insights
                 (project_id, cycles, hotspots, avg_coupling, density, generated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
                params![
                    project_id,
                    cycles_json,
                    hotspots_json,
                    avg_coupling,
                    density
                ],
            )?;
            Ok(())
        })
    }

    /// Get cached graph insights for a project (most recent).
    #[allow(clippy::type_complexity)]
    pub fn get_cached_graph_insights(
        &self,
        project_id: &str,
    ) -> SqliteResult<Option<(String, String, f64, f64, String)>> {
        self.pool.with_connection(|conn| {
            conn.query_row(
                "SELECT cycles, hotspots, avg_coupling, density, generated_at
                 FROM graph_insights
                 WHERE project_id = ?1
                 ORDER BY generated_at DESC LIMIT 1",
                params![project_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
        })
    }
    // ──────────────────────────────────────────────────────────────
    // v3: Workspaces
    // ──────────────────────────────────────────────────────────────

    /// Create a new workspace.
    pub fn create_workspace(&self, name: &str) -> SqliteResult<(String, String, String)> {
        self.pool.with_connection(|conn| {
            let id = uuid::Uuid::new_v4().to_string();
            let created_at = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO workspaces (id, name, created_at) VALUES (?1, ?2, ?3)",
                params![id, name, created_at],
            )?;
            Ok((id, name.to_string(), created_at))
        })
    }

    /// List all workspaces.
    pub fn list_workspaces(&self) -> SqliteResult<Vec<(String, String, String)>> {
        self.pool.with_connection(|conn| {
            let mut stmt =
                conn.prepare("SELECT id, name, created_at FROM workspaces ORDER BY created_at DESC")?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            rows.collect()
        })
    }

    /// Attach a project to a workspace.
    pub fn attach_project_to_workspace(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> SqliteResult<()> {
        self.pool.with_connection(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO workspace_projects (workspace_id, project_id) VALUES (?1, ?2)",
                params![workspace_id, project_id],
            )?;
            Ok(())
        })
    }

    /// List projects that belong to a workspace.
    pub fn list_workspace_projects(
        &self,
        workspace_id: &str,
    ) -> SqliteResult<Vec<(String, String)>> {
        self.pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT workspace_id, project_id FROM workspace_projects WHERE workspace_id = ?1",
            )?;
            let rows = stmt.query_map(params![workspace_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
            rows.collect()
        })
    }

    // ──────────────────────────────────────────────────────────────
    // v3: Snapshots (stub at PR1 — full payload capture in PR5)
    // ──────────────────────────────────────────────────────────────

    /// Create a snapshot with empty payload (stub for PR1).
    /// Full payload capture implemented in PR5.
    #[allow(clippy::type_complexity)]
    pub fn create_snapshot(
        &self,
        project_id: &str,
        label: &str,
        workspace_id: Option<&str>,
    ) -> SqliteResult<(String, String, Option<String>, String, String)> {
        self.pool.with_connection(|conn| {
            let id = uuid::Uuid::new_v4().to_string();
            let created_at = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO snapshots (id, project_id, workspace_id, label, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, project_id, workspace_id, label, created_at],
            )?;
            Ok((id, project_id.to_string(), workspace_id.map(String::from), label.to_string(), created_at))
        })
    }

    /// List snapshots for a project (stub — returns empty list until PR5).
    #[allow(clippy::type_complexity)]
    pub fn list_snapshots(
        &self,
        project_id: &str,
    ) -> SqliteResult<Vec<(String, String, Option<String>, String, String)>> {
        self.pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, workspace_id, label, created_at
                 FROM snapshots WHERE project_id = ?1 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map(params![project_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?;
            rows.collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ScanStatus;
    use crate::models::{SymbolInfo, SymbolKind};

    #[test]
    fn save_and_retrieve_project() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let result = ScanResult {
            project_id: "proj-1".into(),
            project_name: "TestProject".into(),
            root_path: "/tmp/test".into(),
            files_count: 2,
            symbols_count: 5,
            imports_count: 1,
            files: vec![
                FileInfo {
                    id: "file-1".into(),
                    path: "src/index.ts".into(),
                    name: "index.ts".into(),
                    extension: "ts".into(),
                    symbols: vec![SymbolInfo {
                        id: "s1".into(),
                        name: "main".into(),
                        kind: SymbolKind::Function,
                        file_id: "file-1".into(),
                        line_start: 1,
                        line_end: 10,
                        exports: true,
                    }],
                    lines: 10,
                },
                FileInfo {
                    id: "file-2".into(),
                    path: "src/utils.ts".into(),
                    name: "utils.ts".into(),
                    extension: "ts".into(),
                    symbols: vec![],
                    lines: 5,
                },
            ],
            scan_duration_ms: 500,
            status: ScanStatus::Ready,
            error: None,
        };

        repo.save_scan_result(&result).unwrap();

        let project = repo.get_project("proj-1").unwrap();
        assert!(project.is_some());

        let files = repo.get_files("proj-1").unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn save_and_retrieve_graph_cache() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        repo.save_graph_cache("proj-1", r#"{"nodes":[],"edges":[]}"#)
            .unwrap();
        let cached = repo.get_graph_cache("proj-1").unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), r#"{"nodes":[],"edges":[]}"#);
    }

    #[test]
    fn search_files_returns_matches() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let result = ScanResult {
            project_id: "proj-2".into(),
            project_name: "TestProject".into(),
            root_path: "/tmp/test".into(),
            files_count: 0,
            symbols_count: 0,
            imports_count: 0,
            files: vec![
                FileInfo {
                    id: "f-a".into(),
                    path: "src/UserService.ts".into(),
                    name: "UserService.ts".into(),
                    extension: "ts".into(),
                    symbols: vec![],
                    lines: 10,
                },
                FileInfo {
                    id: "f-b".into(),
                    path: "src/UserController.ts".into(),
                    name: "UserController.ts".into(),
                    extension: "ts".into(),
                    symbols: vec![],
                    lines: 20,
                },
                FileInfo {
                    id: "f-c".into(),
                    path: "src/utils.ts".into(),
                    name: "utils.ts".into(),
                    extension: "ts".into(),
                    symbols: vec![],
                    lines: 5,
                },
            ],
            scan_duration_ms: 0,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&result).unwrap();

        let results = repo.search_files("proj-2", "User", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn get_file_by_id_returns_file() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let result = ScanResult {
            project_id: "proj-3".into(),
            project_name: "Test".into(),
            root_path: "/tmp/test".into(),
            files_count: 0,
            symbols_count: 0,
            imports_count: 0,
            files: vec![FileInfo {
                id: "f-single".into(),
                path: "src/App.tsx".into(),
                name: "App.tsx".into(),
                extension: "tsx".into(),
                symbols: vec![],
                lines: 50,
            }],
            scan_duration_ms: 0,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&result).unwrap();

        let file = repo.get_file_by_id("f-single").unwrap();
        assert!(file.is_some());
        assert_eq!(file.unwrap().name, "App.tsx");

        let not_found = repo.get_file_by_id("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    // ──────────────────────────────────────────────────────────────
    // v3: Workspace tests
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn workspace_create_and_list() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let (id, name, created) = repo.create_workspace("Test Workspace").unwrap();
        assert!(!id.is_empty());
        assert_eq!(name, "Test Workspace");
        assert!(!created.is_empty());

        let workspaces = repo.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].1, "Test Workspace");
    }

    #[test]
    fn workspace_attach_project() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let (ws_id, _, _) = repo.create_workspace("WS1").unwrap();

        // Must have project in DB first
        repo.save_scan_result(&ScanResult {
            project_id: "proj-ws-test".into(),
            project_name: "WsTest".into(),
            root_path: "/tmp/ws".into(),
            files_count: 0,
            symbols_count: 0,
            imports_count: 0,
            files: vec![],
            scan_duration_ms: 0,
            status: ScanStatus::Ready,
            error: None,
        }).unwrap();

        repo.attach_project_to_workspace(&ws_id, "proj-ws-test").unwrap();

        let projects = repo.list_workspace_projects(&ws_id).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].1, "proj-ws-test");
    }

    #[test]
    fn snapshot_create_and_list_stub() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        repo.save_scan_result(&ScanResult {
            project_id: "proj-snap".into(),
            project_name: "SnapTest".into(),
            root_path: "/tmp/snap".into(),
            files_count: 0,
            symbols_count: 0,
            imports_count: 0,
            files: vec![],
            scan_duration_ms: 0,
            status: ScanStatus::Ready,
            error: None,
        }).unwrap();

        let snap = repo.create_snapshot("proj-snap", "Baseline v1", None).unwrap();
        assert!(!snap.0.is_empty());
        assert_eq!(snap.1, "proj-snap");
        assert_eq!(snap.3, "Baseline v1");

        let snaps = repo.list_snapshots("proj-snap").unwrap();
        assert_eq!(snaps.len(), 1);
    }
}
