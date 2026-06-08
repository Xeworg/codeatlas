//! Database queries — CRUD for projects, files, symbols, imports.

use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::sync::Mutex;

use crate::models::{FileInfo, ImportInfo, OutlineItem, ScanResult};

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

    /// Open an in-memory database.
    ///
    /// Intended for tests and scenarios where ephemeral storage is preferred.
    /// Also available to integration tests in `tests/` via the `engine` crate.
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
        self.with_connection(|conn| {
            schema_mod::init_schema(conn)?;
            // v7: outline_items table (not in schema_mod to avoid migration duplication)
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS outline_items (
                    file_id TEXT PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
                    outline_json TEXT NOT NULL,
                    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_outline_items_generated_at
                    ON outline_items(generated_at);",
            )?;
            Ok(())
        })
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
            // UPSERT project metadata (no CASCADE): update without deleting row.
            conn.execute(
                "INSERT INTO projects
                 (id, name, root_path, files_count, symbols_count, imports_count,
                  scan_duration_ms, status, error, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                  name = excluded.name,
                  root_path = excluded.root_path,
                  files_count = excluded.files_count,
                  symbols_count = excluded.symbols_count,
                  imports_count = excluded.imports_count,
                  scan_duration_ms = excluded.scan_duration_ms,
                  status = excluded.status,
                  error = excluded.error,
                  updated_at = datetime('now')",
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
        // UPSERT file metadata without deleting the existing row. `INSERT OR REPLACE`
        // deletes first, which cascades to symbols/imports/outline_items.
        conn.execute(
            "INSERT INTO files
             (id, project_id, path, name, extension, lines)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
              project_id = excluded.project_id,
              path = excluded.path,
              name = excluded.name,
              extension = excluded.extension,
              lines = excluded.lines",
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

    /// Look up a project by its root path. Returns full metadata without files.
    pub fn get_project_by_path(
        &self,
        root_path: &str,
    ) -> SqliteResult<Option<crate::models::ProjectMeta>> {
        self.pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, root_path, files_count, symbols_count, imports_count,
                        scan_duration_ms, status, error
                 FROM projects WHERE root_path = ?1",
            )?;
            stmt.query_row(params![root_path], |row| {
                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "scanning" => crate::models::ScanStatus::Scanning,
                    "building_graph" => crate::models::ScanStatus::BuildingGraph,
                    "ready" => crate::models::ScanStatus::Ready,
                    "error" => crate::models::ScanStatus::Error,
                    _ => crate::models::ScanStatus::Idle,
                };
                Ok(crate::models::ProjectMeta {
                    project_id: row.get(0)?,
                    project_name: row.get(1)?,
                    root_path: row.get(2)?,
                    files_count: row.get::<_, i64>(3)? as usize,
                    symbols_count: row.get::<_, i64>(4)? as usize,
                    imports_count: row.get::<_, i64>(5)? as usize,
                    scan_duration_ms: row.get::<_, i64>(6)? as u64,
                    status,
                    error: row.get(8)?,
                })
            })
            .optional()
        })
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

    /// Hydrate symbols from DB for a given file ID.
    fn get_symbols_for_file(
        &self,
        conn: &Connection,
        file_id: &str,
    ) -> SqliteResult<Vec<crate::models::SymbolInfo>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, file_id, line_start, line_end, is_exported\n             FROM symbols WHERE file_id = ?1",
        )?;
        let rows = stmt.query_map(params![file_id], |row| {
            let kind_str: String = row.get(2)?;
            let kind = match kind_str.as_str() {
                "class" => crate::models::SymbolKind::Class,
                "function" => crate::models::SymbolKind::Function,
                "arrowfunction" => crate::models::SymbolKind::ArrowFunction,
                "method" => crate::models::SymbolKind::Method,
                "interface" => crate::models::SymbolKind::Interface,
                "typealias" => crate::models::SymbolKind::TypeAlias,
                "enum" => crate::models::SymbolKind::Enum,
                "variable" => crate::models::SymbolKind::Variable,
                "const" => crate::models::SymbolKind::Const,
                "struct" => crate::models::SymbolKind::Struct,
                "impl" => crate::models::SymbolKind::Impl,
                _ => crate::models::SymbolKind::Unknown,
            };
            Ok(crate::models::SymbolInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                kind,
                file_id: row.get(3)?,
                line_start: row.get::<_, i64>(4)? as u32,
                line_end: row.get::<_, i64>(5)? as u32,
                exports: row.get::<_, Option<bool>>(6)?.unwrap_or(false),
            })
        })?;
        rows.collect()
    }

    pub fn get_files(&self, project_id: &str) -> SqliteResult<Vec<FileInfo>> {
        self.pool.with_connection(|conn| {
            let mut files_stmt = conn.prepare(
                "SELECT id, path, name, extension, lines
                     FROM files WHERE project_id = ?1",
            )?;

            let mut files = Vec::new();
            let mut rows = files_stmt.query(params![project_id])?;
            while let Some(row) = rows.next()? {
                let file_id: String = row.get(0)?;
                let path: String = row.get(1)?;
                let name: String = row.get(2)?;
                let ext: String = row.get(3)?;
                let lines: i64 = row.get(4)?;
                let symbols = self
                    .get_symbols_for_file(conn, &file_id)
                    .unwrap_or_default();
                files.push(FileInfo {
                    id: file_id,
                    path,
                    name,
                    extension: ext,
                    lines: lines as u32,
                    symbols,
                });
            }
            Ok(files)
        })
    }

    pub fn get_imports(&self, project_id: &str) -> SqliteResult<Vec<ImportInfo>> {
        self.pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, source_file_id, target_file_id, target_module, import_names,
                      is_default, is_type_import
                 FROM imports WHERE source_file_id IN
                       (SELECT id FROM files WHERE project_id = ?1)",
            )?;
            let rows = stmt.query_map(params![project_id], |row| {
                let names_json: String = row.get(4)?;
                let imports: Vec<String> = serde_json::from_str(&names_json).unwrap_or_default();
                Ok(ImportInfo {
                    id: row.get(0)?,
                    source_file_id: row.get(1)?,
                    target_file_id: row.get(2)?,
                    target_module: row.get(3)?,
                    imports,
                    is_default: row.get::<_, Option<bool>>(5)?.unwrap_or(false),
                    is_type: row.get::<_, Option<bool>>(6)?.unwrap_or(false),
                })
            })?;
            rows.collect()
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
            let row = match conn.query_row(
                "SELECT id, path, name, extension, lines FROM files WHERE id = ?1",
                params![file_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            ) {
                Ok(r) => r,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(e),
            };
            let symbols = self.get_symbols_for_file(conn, file_id).unwrap_or_default();
            Ok(Some(FileInfo {
                id: row.0,
                path: row.1,
                name: row.2,
                extension: row.3,
                lines: row.4 as u32,
                symbols,
            }))
        })
    }

    // ──────────────────────────────────────────────────────────────
    // PR2: Outline items persistence
    // ──────────────────────────────────────────────────────────────

    /// Returns the project root path for a file ID, or None if the file
    /// or its project is not found. Used by on-demand outline fallback
    /// when the current session's project_root is not set.
    pub fn get_project_root_for_file(&self, file_id: &str) -> SqliteResult<Option<String>> {
        self.pool.with_connection(|conn| {
            conn.query_row(
                "SELECT p.root_path FROM projects p JOIN files f ON f.project_id = p.id WHERE f.id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .optional()
        })
    }

    /// Save or replace outline items for a file. Uses INSERT OR REPLACE
    /// so rescan of the same file updates the outline JSON.
    pub fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> SqliteResult<()> {
        self.pool.with_connection(|conn| {
            let json = serde_json::to_string(items).map_err(|e| {
                rusqlite::Error::InvalidParameterName(format!("outline serialization: {}", e))
            })?;
            conn.execute(
                "INSERT OR REPLACE INTO outline_items (file_id, outline_json, generated_at)\n                 VALUES (?1, ?2, datetime('now'))",
                params![file_id, json],
            )?;
            Ok(())
        })
    }

    /// Get outline items for a file. Returns an empty vector if the file
    /// has no outline (or does not exist), matching safe fallback behavior.
    pub fn get_outline_items(&self, file_id: &str) -> SqliteResult<Vec<OutlineItem>> {
        self.pool.with_connection(|conn| {
            let json: Option<String> = conn
                .query_row(
                    "SELECT outline_json FROM outline_items WHERE file_id = ?1",
                    params![file_id],
                    |row| row.get(0),
                )
                .optional()?;
            match json {
                Some(j) => serde_json::from_str(&j).map_err(|e| {
                    rusqlite::Error::InvalidParameterName(format!("outline deserialization: {}", e))
                }),
                None => Ok(Vec::new()),
            }
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
            let mut stmt = conn
                .prepare("SELECT id, name, created_at FROM workspaces ORDER BY created_at DESC")?;
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
            let rows =
                stmt.query_map(params![workspace_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect()
        })
    }

    // ──────────────────────────────────────────────────────────────
    // v3: Snapshots (stub at PR1 — full payload capture in PR5)
    // ──────────────────────────────────────────────────────────────

    /// Create a snapshot with full payload capture (PR5).
    /// Captures: graph_json from graph_cache, latest graph_insights, latest architecture_detection.
    #[allow(clippy::type_complexity)]
    pub fn create_snapshot(
        &self,
        project_id: &str,
        label: &str,
        workspace_id: Option<&str>,
    ) -> SqliteResult<(
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
    )> {
        self.pool.with_connection(|conn| {
            let id = uuid::Uuid::new_v4().to_string();
            let created_at = chrono::Utc::now().to_rfc3339();

            // Capture graph_json from graph_cache
            let graph_json: Option<String> = conn
                .query_row(
                    "SELECT graph_json FROM graph_cache WHERE project_id = ?1",
                    params![project_id],
                    |row| row.get(0),
                )
                .optional()
                .unwrap_or(None);

            // Capture latest graph_insights
            let insights_json: Option<String> = conn
                .query_row(
                    "SELECT json_object('cycles', cycles, 'hotspots', hotspots, \
                        'avgCoupling', avg_coupling, 'density', density)
                     FROM graph_insights WHERE project_id = ?1 ORDER BY generated_at DESC LIMIT 1",
                    params![project_id],
                    |row| row.get(0),
                )
                .optional()
                .unwrap_or(None);

            // Capture latest architecture_detection
            let arch_json: Option<String> = conn
                .query_row(
                    "SELECT json_object('pattern', pattern, 'confidence', confidence, \
                        'evidence', evidence, 'detectedAt', detected_at)
                     FROM architecture_detections WHERE project_id = ?1 ORDER BY detected_at DESC LIMIT 1",
                    params![project_id],
                    |row| row.get(0),
                )
                .optional()
                .unwrap_or(None);

            // Build payload JSON
            let payload_json = serde_json::json!({
                "nodes": graph_json.as_ref().and_then(|g| {
                    serde_json::from_str::<serde_json::Value>(g).ok()
                        .and_then(|v| v.get("nodes").cloned())
                }).unwrap_or(serde_json::json!([])),
                "edges": graph_json.as_ref().and_then(|g| {
                    serde_json::from_str::<serde_json::Value>(g).ok()
                        .and_then(|v| v.get("edges").cloned())
                }).unwrap_or(serde_json::json!([])),
                "insights": insights_json.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                "architectureDetection": arch_json.and_then(|a| serde_json::from_str::<serde_json::Value>(&a).ok()),
            }).to_string();

            conn.execute(
                "INSERT INTO snapshots (id, project_id, workspace_id, label, created_at, payload_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, project_id, workspace_id, label, created_at, payload_json],
            )?;
            Ok((
                id,
                project_id.to_string(),
                workspace_id.map(String::from),
                label.to_string(),
                created_at,
                Some(payload_json),
            ))
        })
    }

    /// Get a single snapshot by ID.
    #[allow(clippy::type_complexity)]
    pub fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> SqliteResult<
        Option<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    > {
        self.pool.with_connection(|conn| {
            conn.query_row(
                "SELECT id, project_id, workspace_id, label, created_at, payload_json \
                 FROM snapshots WHERE id = ?1",
                params![snapshot_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
        })
    }

    /// List snapshots optionally filtered by project_id and/or workspace_id.
    #[allow(clippy::type_complexity)]
    pub fn list_snapshots(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
    ) -> SqliteResult<
        Vec<(
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
        )>,
    > {
        self.pool.with_connection(|conn| {
            let sql = match workspace_id {
                Some(_) => "SELECT id, project_id, workspace_id, label, created_at, payload_json \
                            FROM snapshots WHERE project_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC",
                None => "SELECT id, project_id, workspace_id, label, created_at, payload_json \
                        FROM snapshots WHERE project_id = ?1 ORDER BY created_at DESC",
            };
            let mut stmt = conn.prepare(sql)?;
            let rows: Vec<(String, String, Option<String>, String, String, Option<String>)> = if let Some(ws) = workspace_id {
                stmt.query_map(params![project_id, ws], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                })?
                .collect::<SqliteResult<Vec<_>>>()?
            } else {
                stmt.query_map(params![project_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                })?
                .collect::<SqliteResult<Vec<_>>>()?
            };
            Ok(rows)
        })
    }

    // ─── Annotation / Comment queries ──────────────────────────────────────────

    /// Add a new annotation to a node. Returns (id, project_id, node_id, author, kind, text, created_at).
    pub fn add_comment(
        &self,
        project_id: &str,
        node_id: &str,
        author: &str,
        text: &str,
        kind: Option<&str>,
    ) -> SqliteResult<(String, String, String, String, String, String, String)> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let kind = kind.unwrap_or("comment");
        self.pool.with_connection(|conn| {
            conn.execute(
                "INSERT INTO annotations (id, project_id, node_id, author, kind, text, created_at)\n                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, project_id, node_id, author, kind, text, created_at],
            )?;
            Ok((
                id,
                project_id.to_string(),
                node_id.to_string(),
                author.to_string(),
                kind.to_string(),
                text.to_string(),
                created_at,
            ))
        })
    }

    /// List annotations filtered by project_id and optionally node_id.
    #[allow(clippy::type_complexity)]
    pub fn list_comments(
        &self,
        project_id: &str,
        node_id: Option<&str>,
    ) -> SqliteResult<Vec<(String, String, String, String, String, String, String)>> {
        self.pool.with_connection(|conn| {
            if let Some(n) = node_id {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, node_id, author, kind, text, created_at\n                     FROM annotations WHERE project_id = ?1 AND node_id = ?2\n                     ORDER BY created_at ASC"
                )?;
                return stmt.query_map(params![project_id, n], |row| {
                    Ok((
                        row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                        row.get(4)?, row.get(5)?, row.get(6)?,
                    ))
                }).and_then(|rows| rows.collect());
            }
            let mut stmt = conn.prepare(
                "SELECT id, project_id, node_id, author, kind, text, created_at\n                 FROM annotations WHERE project_id = ?1\n                 ORDER BY created_at ASC"
            )?;
            stmt.query_map(params![project_id], |row| {
                Ok((
                    row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                    row.get(4)?, row.get(5)?, row.get(6)?,
                ))
            }).and_then(|rows| rows.collect())
        })
    }

    /// Delete an annotation by id. Returns true if a row was removed.
    pub fn delete_comment(&self, comment_id: &str) -> SqliteResult<bool> {
        self.pool.with_connection(|conn| {
            let affected =
                conn.execute("DELETE FROM annotations WHERE id = ?1", params![comment_id])?;
            Ok(affected > 0)
        })
    }

    // ========================================================================
    // H3 — Health Timeline
    // ========================================================================

    /// Persist a health record. Returns (id, recorded_at) of the inserted row.
    #[allow(clippy::too_many_arguments)]
    pub fn save_health_record(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
        overall_score: f64,
        coupling_score: f64,
        complexity_score: f64,
        cycle_count: i64,
        hotspot_count: i64,
    ) -> SqliteResult<(String, String)> {
        self.pool.with_connection(|conn| {
            let id = uuid::Uuid::new_v4().to_string();
            let recorded_at = chrono::Utc::now().to_rfc3339();

            conn.execute(
                "INSERT INTO health_records \
                 (id, workspace_id, project_id, recorded_at, \
                  overall_score, coupling_score, complexity_score, \
                  cycle_count, hotspot_count) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    workspace_id,
                    project_id,
                    recorded_at,
                    overall_score,
                    coupling_score,
                    complexity_score,
                    cycle_count,
                    hotspot_count
                ],
            )?;
            Ok((id, recorded_at))
        })
    }

    /// Retrieve health timeline for a project within a date range.
    /// Returns rows ordered by recorded_at ascending.
    #[allow(clippy::type_complexity)]
    pub fn get_health_timeline(
        &self,
        project_id: &str,
        from: &str,
        to: &str,
    ) -> SqliteResult<Vec<(String, String, f64, f64, f64, i64, i64)>> {
        self.pool.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, recorded_at, overall_score, coupling_score, \
                 complexity_score, cycle_count, hotspot_count \
                 FROM health_records \
                 WHERE project_id = ?1 \
                   AND recorded_at >= ?2 AND recorded_at <= ?3 \
                 ORDER BY recorded_at ASC",
            )?;

            let rows = stmt
                .query_map(params![project_id, from, to], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                        row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                        row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(6)?.unwrap_or(0),
                    ))
                })
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        })
    }
    // ========================================================================
    // H3 — Executive Summary + Diff + C4 Views
    // ========================================================================

    /// Compute executive summary for a workspace: project count, file count,
    /// average health score, trend, and top hotspots.
    pub fn compute_executive_summary(&self, workspace_id: &str) -> SqliteResult<ExecutiveSummary> {
        self.pool.with_connection(|conn| {
            // Count projects attached to this workspace
            let total_projects: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM workspace_projects WHERE workspace_id = ?1",
                    params![workspace_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            // Count files across all projects in workspace
            let total_files: i64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(f.files_count), 0) \
                     FROM files f \
                     JOIN workspace_projects wp ON wp.project_id = f.project_id \
                     WHERE wp.workspace_id = ?1",
                    params![workspace_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            // Fetch health records for workspace (ordered by recorded_at)
            let mut stmt = conn.prepare(
                "SELECT overall_score FROM health_records \
                 WHERE workspace_id = ?1 \
                 ORDER BY recorded_at ASC",
            )?;
            let records: Vec<f64> = stmt
                .query_map(params![workspace_id], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();

            let avg_health_score = if records.is_empty() {
                None
            } else {
                Some(records.iter().sum::<f64>() / records.len() as f64)
            };

            // Trend: compare last two records' overall scores
            let trend = if records.len() >= 2 {
                let last = records[records.len() - 1];
                let prev = records[records.len() - 2];
                if last - prev > 5.0 {
                    "up"
                } else if prev - last > 5.0 {
                    "down"
                } else {
                    "stable"
                }
            } else {
                "stable"
            };

            // Top hotspots: nodes with highest coupling score from latest health record
            let top_hotspots: Vec<(String, f64)> = conn
                .prepare(
                    "SELECT node_id, coupling_score FROM health_records \
                     WHERE workspace_id = ?1 \
                     ORDER BY recorded_at DESC LIMIT 5",
                )
                .ok()
                .map(|mut stmt| {
                    stmt.query_map(params![workspace_id], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect()
                })
                .unwrap_or_default();

            Ok(ExecutiveSummary {
                workspace_id: workspace_id.to_string(),
                total_projects,
                total_files,
                avg_health_score,
                trend: trend.to_string(),
                top_hotspots,
                generated_at: chrono::Utc::now().to_rfc3339(),
            })
        })
    }

    /// Compare two snapshots and return diff payload.
    /// If either snapshot does not exist, returns empty diff (zero deltas, empty lists).
    pub fn compare_snapshots(
        &self,
        base_snapshot_id: &str,
        target_snapshot_id: &str,
    ) -> SqliteResult<SnapshotDiff> {
        self.pool.with_connection(|conn| {
            // Load payloads
            let base_payload = conn
                .query_row(
                    "SELECT payload_json FROM snapshots WHERE id = ?1",
                    params![base_snapshot_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str::<SnapshotPayloadRaw>(&s).ok());

            let target_payload = conn
                .query_row(
                    "SELECT payload_json FROM snapshots WHERE id = ?1",
                    params![target_snapshot_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str::<SnapshotPayloadRaw>(&s).ok());

            match (base_payload, target_payload) {
                (Some(base), Some(target)) => {
                    let base_nodes: std::collections::HashSet<_> =
                        base.nodes.iter().map(|n| n.id.clone()).collect();
                    let target_nodes: std::collections::HashSet<_> =
                        target.nodes.iter().map(|n| n.id.clone()).collect();

                    let nodes_added: Vec<String> =
                        target_nodes.difference(&base_nodes).cloned().collect();
                    let nodes_removed: Vec<String> =
                        base_nodes.difference(&target_nodes).cloned().collect();

                    let base_edges: std::collections::HashSet<_> =
                        base.edges.iter().cloned().collect();
                    let target_edges: std::collections::HashSet<_> =
                        target.edges.iter().cloned().collect();

                    let edges_added: Vec<String> =
                        target_edges.difference(&base_edges).cloned().collect();
                    let edges_removed: Vec<String> =
                        base_edges.difference(&target_edges).cloned().collect();

                    let coupling_delta =
                        target.avg_coupling.unwrap_or(0.0) - base.avg_coupling.unwrap_or(0.0);
                    let complexity_delta =
                        target.avg_complexity.unwrap_or(0.0) - base.avg_complexity.unwrap_or(0.0);
                    let cycles_delta = target.cycle_count.unwrap_or(0) as f64
                        - base.cycle_count.unwrap_or(0) as f64;

                    Ok(SnapshotDiff {
                        base_snapshot_id: base_snapshot_id.to_string(),
                        target_snapshot_id: target_snapshot_id.to_string(),
                        nodes_added,
                        nodes_removed,
                        nodes_modified: vec![],
                        edges_added,
                        edges_removed,
                        coupling_delta,
                        complexity_delta,
                        cycles_delta: cycles_delta as i64,
                    })
                }
                _ => {
                    // One or both snapshots not found — return empty diff
                    Ok(SnapshotDiff {
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
            }
        })
    }

    /// Return a C4 view payload for a project/snapshot.
    /// Level 1 = System Context, Level 2 = Container.
    /// Returns error for invalid levels (< 1 or > 2).
    pub fn get_c4_view(&self, project_id: &str, level: u8) -> SqliteResult<C4View> {
        if !(1..=2).contains(&level) {
            return Err(rusqlite::Error::InvalidParameterName(
                "level must be 1 or 2".to_string(),
            ));
        }

        self.pool.with_connection(|conn| {
            // Load latest snapshot for project to derive C4 representation
            let payload_opt = conn
                .query_row(
                    "SELECT payload_json FROM snapshots \
                     WHERE project_id = ?1 \
                     ORDER BY created_at DESC LIMIT 1",
                    params![project_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str::<SnapshotPayloadRaw>(&s).ok());

            match (payload_opt, level) {
                (_, 0_u8..=0_u8) | (_, 3_u8..=u8::MAX) => {
                    unreachable!("level is validated before this match")
                }
                (Some(payload), 1) => {
                    // System Context: derive systems from node types or snapshot graph
                    let systems: Vec<String> = payload
                        .nodes
                        .iter()
                        .filter(|n| n.node_type == "service" || n.node_type == "repository")
                        .map(|n| n.label.clone())
                        .take(10)
                        .collect();
                    let is_empty = systems.is_empty();
                    Ok(C4View {
                        level: 1,
                        systems: Some(if is_empty {
                            vec!["CodeAtlas Application".to_string()]
                        } else {
                            systems
                        }),
                        containers: None,
                        warning: if is_empty {
                            Some("Limited data for C4 L1; showing system placeholder.".to_string())
                        } else {
                            None
                        },
                    })
                }
                (Some(payload), 2) => {
                    // Container: derive containers from node labels and paths
                    let containers: Vec<String> = payload
                        .nodes
                        .iter()
                        .map(|n| {
                            // Derive container name from path segments (first two after root)
                            n.path
                                .split('/')
                                .skip(1)
                                .take(2)
                                .collect::<Vec<_>>()
                                .join("/")
                        })
                        .filter(|s| !s.is_empty())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .take(20)
                        .collect();
                    Ok(C4View {
                        level: 2,
                        systems: None,
                        containers: if containers.is_empty() {
                            Some(vec!["No container data available".to_string()])
                        } else {
                            Some(containers)
                        },
                        warning: None,
                    })
                }
                (None, _) => {
                    // No snapshot — return minimal payload with warning
                    Ok(C4View {
                        level,
                        systems: if level == 1 {
                            Some(vec!["CodeAtlas Application".to_string()])
                        } else {
                            None
                        },
                        containers: if level == 2 {
                            Some(vec!["No snapshot data available".to_string()])
                        } else {
                            None
                        },
                        warning: Some(
                            "No snapshot found for this project. Create a snapshot first."
                                .to_string(),
                        ),
                    })
                }
            }
        })
    }
}

// ========================================================================
// H3 Response Types (returned by PR8 commands)
// ========================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub workspace_id: String,
    pub total_projects: i64,
    pub total_files: i64,
    pub avg_health_score: Option<f64>,
    pub trend: String, // "up", "down", "stable"
    pub top_hotspots: Vec<(String, f64)>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4View {
    pub level: u8,
    pub systems: Option<Vec<String>>,
    pub containers: Option<Vec<String>>,
    pub warning: Option<String>,
}

// Minimal payload shape for snapshot diff comparison
#[derive(Debug, Clone, Deserialize)]
struct SnapshotPayloadRaw {
    nodes: Vec<NodeRaw>,
    edges: Vec<String>,
    avg_coupling: Option<f64>,
    avg_complexity: Option<f64>,
    cycle_count: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct NodeRaw {
    id: String,
    label: String,
    path: String,
    node_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ScanStatus;
    use crate::models::{OutlineItemKind, SymbolInfo, SymbolKind};

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

    /// Regression: second save_scan_result must not delete previously persisted
    /// imports or outline_items via INSERT OR REPLACE cascade.
    #[test]
    fn save_scan_result_idempotent_no_import_outline_cascade() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        // Phase 1: save project + file
        let phase1 = ScanResult {
            project_id: "proj-regr".into(),
            project_name: "Regression".into(),
            root_path: "/tmp/regr".into(),
            files_count: 1,
            symbols_count: 0,
            imports_count: 0,
            files: vec![FileInfo {
                id: "file-regr-1".into(),
                path: "src/main.ts".into(),
                name: "main.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 5,
            }],
            scan_duration_ms: 100,
            status: ScanStatus::Scanning,
            error: None,
        };
        repo.save_scan_result(&phase1).unwrap();

        // Verify file persisted
        let files_after_phase1 = repo.get_files("proj-regr").unwrap();
        assert_eq!(files_after_phase1.len(), 1, "Setup: file should be saved");

        // Simulate Phase 2 of scan_project: persist import and outline
        // These are separate repo calls, same as in real scan_project
        let import = crate::models::ImportInfo {
            id: "imp-regr-1".into(),
            source_file_id: "file-regr-1".into(),
            target_file_id: None,
            target_module: Some("./utils".into()),
            imports: vec!["helper".into()],
            is_default: false,
            is_type: false,
        };
        repo.save_import(&import).unwrap();

        let outline = vec![crate::models::OutlineItem {
            id: "outline:file-regr-1:class:1:5:Main".into(),
            file_id: "file-regr-1".into(),
            name: "Main".into(),
            kind: crate::models::OutlineItemKind::Class,
            line_start: 1,
            line_end: 5,
            column_start: None,
            column_end: None,
            children: vec![],
        }];
        repo.save_outline_items("file-regr-1", &outline).unwrap();

        // Verify both persisted before Phase 2 save
        let imports_before = repo.get_imports("proj-regr").unwrap();
        assert_eq!(imports_before.len(), 1, "Setup: import must be saved");
        let outline_before = repo.get_outline_items("file-regr-1").unwrap();
        assert_eq!(outline_before.len(), 1, "Setup: outline must be saved");

        // Phase 2: re-save the project (simulates second save_scan_result in scan_project)
        // Critical: must NOT delete the import or outline rows.
        let phase2 = ScanResult {
            project_id: "proj-regr".into(),
            project_name: "Regression".into(),
            root_path: "/tmp/regr".into(),
            files_count: 1,
            symbols_count: 0,
            imports_count: 1,
            files: vec![FileInfo {
                id: "file-regr-1".into(),
                path: "src/main.ts".into(),
                name: "main.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 5,
            }],
            scan_duration_ms: 250,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&phase2).unwrap();

        // Core assertions: import and outline survived the second save
        let imports_after = repo.get_imports("proj-regr").unwrap();
        assert_eq!(
            imports_after.len(),
            1,
            "Import should survive second save_scan_result"
        );
        assert_eq!(imports_after[0].id, "imp-regr-1");

        let outline_after = repo.get_outline_items("file-regr-1").unwrap();
        assert_eq!(
            outline_after.len(),
            1,
            "Outline should survive second save_scan_result"
        );
        assert_eq!(outline_after[0].name, "Main");
    }

    #[test]
    fn save_scan_result_idempotent_symbols_preserved() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        // First save: project + file with one symbol
        let sym = SymbolInfo {
            id: "sym-regr-1".into(),
            name: "MyFunction".into(),
            kind: SymbolKind::Function,
            file_id: "file-regr-sym".into(),
            line_start: 1,
            line_end: 20,
            exports: true,
        };
        let phase1 = ScanResult {
            project_id: "proj-regr-2".into(),
            project_name: "Regression2".into(),
            root_path: "/tmp/regr2".into(),
            files_count: 1,
            symbols_count: 1,
            imports_count: 0,
            files: vec![FileInfo {
                id: "file-regr-sym".into(),
                path: "src/main.ts".into(),
                name: "main.ts".into(),
                extension: "ts".into(),
                symbols: vec![sym],
                lines: 20,
            }],
            scan_duration_ms: 100,
            status: ScanStatus::Scanning,
            error: None,
        };
        repo.save_scan_result(&phase1).unwrap();

        // Verify symbol present after first save
        let files1 = repo.get_files("proj-regr-2").unwrap();
        assert_eq!(files1.len(), 1);
        assert_eq!(files1[0].symbols.len(), 1);

        // Second save: same project, empty symbols array
        // This simulates the second save_scan_result call in scan_project where
        // Phase 1 files are passed again but the in-memory FileInfo has empty symbols.
        let phase2 = ScanResult {
            project_id: "proj-regr-2".into(),
            project_name: "Regression2".into(),
            root_path: "/tmp/regr2".into(),
            files_count: 1,
            symbols_count: 1,
            imports_count: 0,
            files: vec![FileInfo {
                id: "file-regr-sym".into(),
                path: "src/main.ts".into(),
                name: "main.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 20,
            }],
            scan_duration_ms: 300,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&phase2).unwrap();

        // Symbol must survive via INSERT OR REPLACE on symbols table
        let files2 = repo.get_files("proj-regr-2").unwrap();
        assert_eq!(files2.len(), 1);
        assert_eq!(
            files2[0].symbols.len(),
            1,
            "Symbol should survive second save"
        );
        assert_eq!(files2[0].symbols[0].name, "MyFunction");
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
    // PR? : Symbol hydration tests
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn get_files_returns_symbols_hydrated() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        // Save a project with files that have symbols
        let result = ScanResult {
            project_id: "proj-symbols".into(),
            project_name: "SymbolsTest".into(),
            root_path: "/tmp/symbols".into(),
            files_count: 1,
            symbols_count: 3,
            imports_count: 0,
            files: vec![FileInfo {
                id: "f-sym-test".into(),
                path: "src/hydrated.ts".into(),
                name: "hydrated.ts".into(),
                extension: "ts".into(),
                lines: 20,
                symbols: vec![
                    SymbolInfo {
                        id: "sym-1".into(),
                        name: "TopClass".into(),
                        kind: SymbolKind::Class,
                        file_id: "f-sym-test".into(),
                        line_start: 1,
                        line_end: 10,
                        exports: true,
                    },
                    SymbolInfo {
                        id: "sym-2".into(),
                        name: "helperFn".into(),
                        kind: SymbolKind::Function,
                        file_id: "f-sym-test".into(),
                        line_start: 12,
                        line_end: 18,
                        exports: true,
                    },
                    SymbolInfo {
                        id: "sym-3".into(),
                        name: "UnusedConst".into(),
                        kind: SymbolKind::Const,
                        file_id: "f-sym-test".into(),
                        line_start: 19,
                        line_end: 20,
                        exports: false,
                    },
                ],
            }],
            scan_duration_ms: 0,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&result).unwrap();

        // get_files should return FileInfo with symbols hydrated from DB
        let files = repo.get_files("proj-symbols").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].symbols.len(), 3);
        assert!(files[0].symbols.iter().any(|s| s.name == "TopClass"));
        assert!(files[0].symbols.iter().any(|s| s.name == "helperFn"));
        assert!(files[0].symbols.iter().any(|s| s.name == "UnusedConst"));
        assert!(files[0].symbols.iter().any(|s| s.kind == SymbolKind::Class));
        assert!(files[0]
            .symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Function));
    }

    #[test]
    fn get_file_by_id_returns_symbols_hydrated() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let result = ScanResult {
            project_id: "proj-fbid".into(),
            project_name: "FileByIdTest".into(),
            root_path: "/tmp/fbid".into(),
            files_count: 1,
            symbols_count: 2,
            imports_count: 0,
            files: vec![FileInfo {
                id: "f-bid-test".into(),
                path: "src/by_id.ts".into(),
                name: "by_id.ts".into(),
                extension: "ts".into(),
                lines: 30,
                symbols: vec![
                    SymbolInfo {
                        id: "sym-b1".into(),
                        name: "TargetInterface".into(),
                        kind: SymbolKind::Interface,
                        file_id: "f-bid-test".into(),
                        line_start: 1,
                        line_end: 5,
                        exports: true,
                    },
                    SymbolInfo {
                        id: "sym-b2".into(),
                        name: "targetFn".into(),
                        kind: SymbolKind::Function,
                        file_id: "f-bid-test".into(),
                        line_start: 7,
                        line_end: 25,
                        exports: true,
                    },
                ],
            }],
            scan_duration_ms: 0,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&result).unwrap();

        let file = repo.get_file_by_id("f-bid-test").unwrap();
        assert!(file.is_some());
        let f = file.unwrap();
        assert_eq!(f.name, "by_id.ts");
        assert_eq!(f.symbols.len(), 2);
        assert!(f.symbols.iter().any(|s| s.name == "TargetInterface"));
        assert!(f.symbols.iter().any(|s| s.name == "targetFn"));
        assert!(f.symbols.iter().any(|s| s.kind == SymbolKind::Interface));
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
        })
        .unwrap();

        repo.attach_project_to_workspace(&ws_id, "proj-ws-test")
            .unwrap();

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
        })
        .unwrap();

        let snap = repo
            .create_snapshot("proj-snap", "Baseline v1", None)
            .unwrap();
        assert!(!snap.0.is_empty());
        assert_eq!(snap.1, "proj-snap");
        assert_eq!(snap.3, "Baseline v1");
        assert!(
            snap.5.is_some(),
            "payload_json should be populated after PR5"
        );

        let snaps = repo.list_snapshots("proj-snap", None).unwrap();
        assert_eq!(snaps.len(), 1);
    }

    #[test]
    fn annotation_add_and_list() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let comment = repo
            .add_comment("proj1", "node1", "author1", "Test comment", None)
            .unwrap();
        assert!(!comment.0.is_empty());
        assert_eq!(comment.5, "Test comment");
        assert_eq!(comment.4, "comment");

        let comments = repo.list_comments("proj1", Some("node1")).unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn annotation_list_by_project() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        repo.add_comment("proj1", "node1", "a1", "Comment 1", None)
            .unwrap();
        repo.add_comment("proj1", "node2", "a2", "Comment 2", None)
            .unwrap();
        repo.add_comment("proj1", "node1", "a3", "Comment 3", None)
            .unwrap();

        let node1 = repo.list_comments("proj1", Some("node1")).unwrap();
        assert_eq!(node1.len(), 2);

        let all = repo.list_comments("proj1", None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn annotation_delete() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let c = repo
            .add_comment("proj1", "node1", "author1", "To delete", None)
            .unwrap();
        let deleted = repo.delete_comment(&c.0).unwrap();
        assert!(deleted);

        let remaining = repo.list_comments("proj1", Some("node1")).unwrap();
        assert_eq!(remaining.len(), 0);

        let not_found = repo.delete_comment("nonexistent").unwrap();
        assert!(!not_found);
    }

    #[test]
    fn annotation_kind_variants() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        repo.add_comment("proj1", "node1", "a", "A todo", Some("todo"))
            .unwrap();
        repo.add_comment("proj1", "node1", "b", "A review", Some("review"))
            .unwrap();
        repo.add_comment("proj1", "node1", "c", "An issue", Some("issue"))
            .unwrap();

        let comments = repo.list_comments("proj1", Some("node1")).unwrap();
        assert_eq!(comments[0].4, "todo");
        assert_eq!(comments[1].4, "review");
        assert_eq!(comments[2].4, "issue");
    }

    // ====================================================================
    // PR2 — Outline Items persistence
    // ====================================================================

    fn migrated_pool() -> DbPool {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        pool
    }

    fn seed_outline_file(repo: &ProjectRepository<'_>, file_id: &str) {
        repo.save_scan_result(&ScanResult {
            project_id: "outline-proj".into(),
            project_name: "OutlineProject".into(),
            root_path: "/tmp/outline-project".into(),
            files_count: 1,
            symbols_count: 0,
            imports_count: 0,
            files: vec![FileInfo {
                id: file_id.into(),
                path: "src/user_service.ts".into(),
                name: "user_service.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 20,
            }],
            scan_duration_ms: 1,
            status: ScanStatus::Ready,
            error: None,
        })
        .unwrap();
    }

    fn sample_outline(file_id: &str, name: &str) -> Vec<OutlineItem> {
        vec![OutlineItem {
            id: format!("outline:{file_id}:class:1:10:{name}"),
            file_id: file_id.into(),
            name: name.into(),
            kind: OutlineItemKind::Class,
            line_start: 1,
            line_end: 10,
            column_start: Some(0),
            column_end: Some(1),
            children: vec![OutlineItem {
                id: format!("outline:{file_id}:method:2:4:getUser"),
                file_id: file_id.into(),
                name: "getUser".into(),
                kind: OutlineItemKind::Method,
                line_start: 2,
                line_end: 4,
                column_start: Some(4),
                column_end: Some(11),
                children: vec![],
            }],
        }]
    }

    #[test]
    fn outline_save_and_retrieve_hierarchy() {
        let pool = migrated_pool();
        let repo = ProjectRepository::new(&pool);
        seed_outline_file(&repo, "file-outline-1");

        let outline = sample_outline("file-outline-1", "UserService");
        repo.save_outline_items("file-outline-1", &outline).unwrap();

        let retrieved = repo.get_outline_items("file-outline-1").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].name, "UserService");
        assert_eq!(retrieved[0].children.len(), 1);
        assert_eq!(retrieved[0].children[0].name, "getUser");
    }

    #[test]
    fn outline_retrieve_empty_for_unknown_file() {
        let pool = migrated_pool();
        let repo = ProjectRepository::new(&pool);

        let retrieved = repo.get_outline_items("missing-file").unwrap();
        assert!(retrieved.is_empty());
    }

    #[test]
    fn outline_replace_on_resave() {
        let pool = migrated_pool();
        let repo = ProjectRepository::new(&pool);
        seed_outline_file(&repo, "file-outline-2");

        repo.save_outline_items(
            "file-outline-2",
            &sample_outline("file-outline-2", "FirstService"),
        )
        .unwrap();
        repo.save_outline_items(
            "file-outline-2",
            &sample_outline("file-outline-2", "SecondService"),
        )
        .unwrap();

        let retrieved = repo.get_outline_items("file-outline-2").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].name, "SecondService");
    }

    #[test]
    fn outline_delete_cascades_with_file() {
        let pool = migrated_pool();
        let repo = ProjectRepository::new(&pool);
        seed_outline_file(&repo, "file-outline-3");
        repo.save_outline_items(
            "file-outline-3",
            &sample_outline("file-outline-3", "DeletedService"),
        )
        .unwrap();

        pool.with_connection(|conn| {
            conn.execute("DELETE FROM files WHERE id = ?1", params!["file-outline-3"])?;
            Ok(())
        })
        .unwrap();

        let retrieved = repo.get_outline_items("file-outline-3").unwrap();
        assert!(retrieved.is_empty());
    }

    #[test]
    fn get_project_root_for_file_returns_root() {
        let pool = migrated_pool();
        let repo = ProjectRepository::new(&pool);

        // Seed a file via scan result with a known root path
        let result = ScanResult {
            project_id: "proj-root-test".into(),
            project_name: "RootTest".into(),
            root_path: "/custom/root/path".into(),
            files_count: 1,
            symbols_count: 0,
            imports_count: 0,
            files: vec![FileInfo {
                id: "file-root-1".into(),
                path: "src/main.ts".into(),
                name: "main.ts".into(),
                extension: "ts".into(),
                symbols: vec![],
                lines: 10,
            }],
            scan_duration_ms: 10,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&result).unwrap();

        let root = repo.get_project_root_for_file("file-root-1").unwrap();
        assert_eq!(root, Some("/custom/root/path".into()));
    }

    #[test]
    fn get_project_root_for_file_unknown_file() {
        let pool = migrated_pool();
        let repo = ProjectRepository::new(&pool);

        let root = repo
            .get_project_root_for_file("nonexistent-file-id")
            .unwrap();
        assert_eq!(root, None);
    }

    // ====================================================================
    // PR8 — Executive Summary + Diff + C4 Views
    // ====================================================================

    #[test]
    fn executive_summary_empty_workspace() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let summary = repo.compute_executive_summary("ws-nonexistent").unwrap();
        assert_eq!(summary.total_projects, 0);
        assert!(summary.avg_health_score.is_none());
        assert_eq!(summary.trend, "stable");
        assert!(summary.top_hotspots.is_empty());
    }

    #[test]
    fn executive_summary_trend_up() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        // health_records has FK project_id REFERENCES projects(id)
        // Create project first so health records can reference it
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, name, root_path) VALUES ('proj-up', 'UpProj', '/tmp/up')",
            [],
        ).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        repo.create_workspace("ws-up").unwrap();
        repo.save_health_record("proj-up", Some("ws-up"), 70.0, 30.0, 40.0, 3, 5)
            .unwrap();
        repo.save_health_record("proj-up", Some("ws-up"), 90.0, 20.0, 30.0, 1, 2)
            .unwrap();

        let summary = repo.compute_executive_summary("ws-up").unwrap();
        assert_eq!(summary.trend, "up"); // 90 > 70
        let avg = summary.avg_health_score.unwrap();
        assert!((avg - 80.0).abs() < 0.5, "avg should be ~80, got {}", avg);
    }

    #[test]
    fn executive_summary_trend_down() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, name, root_path) VALUES ('proj-down', 'DownProj', '/tmp/down')",
            [],
        ).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        repo.create_workspace("ws-down").unwrap();
        repo.save_health_record("proj-down", Some("ws-down"), 90.0, 20.0, 30.0, 1, 2)
            .unwrap();
        repo.save_health_record("proj-down", Some("ws-down"), 50.0, 50.0, 60.0, 8, 12)
            .unwrap();

        let summary = repo.compute_executive_summary("ws-down").unwrap();
        assert_eq!(summary.trend, "down"); // 50 < 90
    }

    #[test]
    fn executive_summary_trend_stable() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, name, root_path) VALUES ('proj-stable', 'StableProj', '/tmp/stable')",
            [],
        ).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        repo.create_workspace("ws-stable").unwrap();
        repo.save_health_record("proj-stable", Some("ws-stable"), 72.0, 25.0, 35.0, 3, 4)
            .unwrap();
        repo.save_health_record("proj-stable", Some("ws-stable"), 74.0, 24.0, 36.0, 3, 4)
            .unwrap();

        let summary = repo.compute_executive_summary("ws-stable").unwrap();
        assert_eq!(summary.trend, "stable"); // 74 vs 72 within 5-point threshold
    }

    #[test]
    fn snapshot_diff_same_snapshot_zero_delta() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let snap = repo.create_snapshot("proj-same", "Baseline", None).unwrap();
        let diff = repo.compare_snapshots(&snap.0, &snap.0).unwrap();

        assert_eq!(diff.base_snapshot_id, snap.0);
        assert_eq!(diff.target_snapshot_id, snap.0);
        assert!(diff.nodes_added.is_empty());
        assert!(diff.nodes_removed.is_empty());
        assert_eq!(diff.coupling_delta, 0.0);
        assert_eq!(diff.complexity_delta, 0.0);
        assert_eq!(diff.cycles_delta, 0);
    }

    #[test]
    fn snapshot_diff_nonexistent_returns_empty_diff() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let diff = repo
            .compare_snapshots("nonexistent-base", "nonexistent-target")
            .unwrap();
        assert_eq!(diff.coupling_delta, 0.0);
        assert!(diff.nodes_added.is_empty());
        assert!(diff.nodes_removed.is_empty());
    }

    #[test]
    fn c4_view_invalid_level_returns_error() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let result = repo.get_c4_view("proj-c4", 99);
        assert!(result.is_err());
    }

    #[test]
    fn c4_view_level_1_no_data_returns_warning() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let result = repo.get_c4_view("proj-c4-empty", 1).unwrap();
        assert_eq!(result.level, 1);
        assert!(result.systems.is_some());
        assert!(result.warning.is_some());
    }

    #[test]
    fn c4_view_level_2_no_data_returns_warning() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let conn = pool.0.lock().unwrap();
        super::super::migrations::run_pending_migrations(&conn).unwrap();
        drop(conn);
        let repo = ProjectRepository::new(&pool);

        let result = repo.get_c4_view("proj-c4-l2", 2).unwrap();
        assert_eq!(result.level, 2);
        assert!(result.containers.is_some());
        assert!(result.warning.is_some());
    }

    // ====================================================================
    // Reopen-flow: get_project_by_path
    // ====================================================================

    #[test]
    fn get_project_by_path_returns_meta_when_exists() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let result = ScanResult {
            project_id: "proj-reopen".into(),
            project_name: "ReopenTest".into(),
            root_path: "/tmp/reopen-test".into(),
            files_count: 3,
            symbols_count: 10,
            imports_count: 2,
            files: vec![],
            scan_duration_ms: 500,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&result).unwrap();

        let meta = repo.get_project_by_path("/tmp/reopen-test").unwrap();
        assert!(meta.is_some());
        let meta = meta.unwrap();
        assert_eq!(meta.project_id, "proj-reopen");
        assert_eq!(meta.project_name, "ReopenTest");
        assert_eq!(meta.root_path, "/tmp/reopen-test");
        assert_eq!(meta.files_count, 3);
        assert_eq!(meta.symbols_count, 10);
        assert_eq!(meta.imports_count, 2);
        assert_eq!(meta.scan_duration_ms, 500);
        assert!(matches!(meta.status, ScanStatus::Ready));
        assert!(meta.error.is_none());
    }

    #[test]
    fn get_project_by_path_returns_none_when_missing() {
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let meta = repo.get_project_by_path("/nonexistent/path").unwrap();
        assert!(meta.is_none());
    }

    #[test]
    fn get_project_by_path_unique_per_root_path() {
        // Two projects with different ids but same root_path: only one can exist
        let pool = DbPool::in_memory().unwrap();
        pool.init_schema().unwrap();
        let repo = ProjectRepository::new(&pool);

        let result1 = ScanResult {
            project_id: "proj-first".into(),
            project_name: "First".into(),
            root_path: "/tmp/shared-path".into(),
            files_count: 1,
            symbols_count: 1,
            imports_count: 0,
            files: vec![],
            scan_duration_ms: 100,
            status: ScanStatus::Ready,
            error: None,
        };
        repo.save_scan_result(&result1).unwrap();

        // Same root_path, different project_id — UNIQUE constraint on root_path
        let result2 = ScanResult {
            project_id: "proj-second".into(),
            project_name: "Second".into(),
            root_path: "/tmp/shared-path".into(),
            files_count: 2,
            symbols_count: 5,
            imports_count: 1,
            files: vec![],
            scan_duration_ms: 200,
            status: ScanStatus::Ready,
            error: None,
        };
        let err = repo.save_scan_result(&result2).unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("UNIQUE constraint failed: projects.root_path"),
            "Expected UNIQUE constraint error, got: {err_str}"
        );

        // Only the first project is reachable by path
        let meta = repo.get_project_by_path("/tmp/shared-path").unwrap();
        assert!(meta.is_some());
        assert_eq!(meta.unwrap().project_id, "proj-first");
    }
}
