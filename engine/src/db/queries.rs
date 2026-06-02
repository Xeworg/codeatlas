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
}
