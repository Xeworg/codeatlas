//! Database queries — CRUD for projects, files, symbols, imports.

use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};

use crate::models::{FileInfo, ImportInfo, ScanResult, ScanStatus, SymbolInfo, SymbolKind};

pub struct DbPool(pub Connection);

impl DbPool {
    pub fn new(path: &str) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        Ok(Self(conn))
    }

    pub fn in_memory() -> SqliteResult<Self> {
        Ok(Self(Connection::open_in_memory()?))
    }
}

pub struct ProjectRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> ProjectRepository<'conn> {
    pub fn new(pool: &'conn DbPool) -> Self {
        Self { conn: &pool.0 }
    }

    pub fn save_scan_result(&self, result: &ScanResult) -> SqliteResult<()> {
        self.conn.execute(
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
            self.save_file(result.project_id.as_str(), file)?;
        }

        Ok(())
    }

    pub fn save_file(&self, project_id: &str, file: &FileInfo) -> SqliteResult<()> {
        self.conn.execute(
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
            self.save_symbol(&file.id, symbol)?;
        }

        Ok(())
    }

    pub fn save_symbol(&self, file_id: &str, symbol: &SymbolInfo) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO symbols 
             (id, file_id, name, kind, line_start, line_end, is_exported)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                symbol.id,
                file_id,
                symbol.name,
                format!("{:?}", symbol.kind).to_lowercase(),
                symbol.line_start as i64,
                symbol.line_end as i64,
                symbol.exports,
            ],
        )?;
        Ok(())
    }

    pub fn get_project(&self, project_id: &str) -> SqliteResult<Option<(String, String, i64)>> {
        self.conn
            .query_row(
                "SELECT name, root_path, files_count FROM projects WHERE id = ?1",
                params![project_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
    }

    pub fn get_files(&self, project_id: &str) -> SqliteResult<Vec<FileInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, name, extension, lines FROM files WHERE project_id = ?1")?;

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
    }

    pub fn save_import(&self, import: &ImportInfo) -> SqliteResult<()> {
        self.conn.execute(
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::init_schema;

    #[test]
    fn save_and_retrieve_project() {
        let pool = DbPool::in_memory().unwrap();
        init_schema(&pool.0).unwrap();
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
}
