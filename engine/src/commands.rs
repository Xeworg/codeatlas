//! Pure orchestration commands for the engine layer.
//!
//! These functions are Tauri-independent: they take no `AppState`, no DB connection,
//! and no tracing. They are the single-parse-per-file dispatch layer.
//!
//! The Tauri shim layer (`src-tauri/src/commands.rs`) calls these functions and
//! handles persistence, tracing, and state updates around the calls.
//!
//! # Design
//!
//! Each file is discovered once and parsed once. The registry is called exactly
//! once per file. All output categories (symbols, imports, outline) come from
//! the same `ParseResult` so there is no drift between the flat and outline paths.
//!
//! # Contracts
//!
//! - `scan_files`: calls registry `parse_file` exactly once per discovered file.
//! - `outline_for_file`: calls registry `parse_file` exactly once.

use crate::models::{FileInfo, ImportInfo, OutlineItem};
use crate::scanner::parser::{ParseResult, ParserRegistry};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

// ── Types ──────────────────────────────────────────────────────────────────────────

/// Output of a project scan via `scan_files`.
#[derive(Debug, Clone, Default)]
pub struct ScanFilesOutput {
    /// Flat file records with symbols populated.
    pub file_infos: Vec<FileInfo>,
    /// All import records across all files (used to populate import edges).
    pub all_imports: Vec<ImportInfo>,
    /// Cached outlines keyed by relative file path, derived from the same
    /// `ParseResult` as symbols and imports — no second parse required.
    pub outlines: HashMap<String, Vec<OutlineItem>>,
    /// Time spent in the registry parse calls (ms).
    pub parse_ms: u64,
    /// How many times the registry was called (one per file + any gracefully
    /// skipped files that produced zero output — those still counted a call).
    pub registry_call_count: usize,
    /// Number of files where read or parse failed gracefully.
    pub files_failed: usize,
    /// Number of files successfully read from disk.
    pub files_read: usize,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Parse every discovered file exactly once and return all output categories.
///
/// This is the pure heart of the single-dispatch requirement: the registry is
/// called exactly `files.len()` times, once per file. All IR categories
/// (`symbols`, `imports`, `outline`) come from the same `ParseResult` instance.
///
/// # Arguments
/// * `registry` — typed parse dispatch; `ParserRegistry` for production, a
///   tracking impl in tests.
/// * `files` — files discovered by `FileWalker::discover()`.
/// * `root` — project root path (used to resolve relative paths in `FileInfo`).
pub fn scan_files(
    registry: &dyn ParseFile,
    files: &[DiscoveredFile],
    _root: &Path,
) -> ScanFilesOutput {
    let parse_start = Instant::now();
    let mut file_infos: Vec<FileInfo> = Vec::with_capacity(files.len());
    let mut all_imports: Vec<ImportInfo> = Vec::new();
    let mut outlines: HashMap<String, Vec<OutlineItem>> = HashMap::new();
    let mut registry_call_count: usize = 0;
    let mut files_failed: usize = 0;
    let mut files_read: usize = 0;

    for file in files {
        let source = match std::fs::read_to_string(&file.path) {
            Ok(s) => {
                files_read += 1;
                s
            }
            Err(_) => {
                files_failed += 1;
                continue;
            }
        };

        let result: ParseResult =
            registry.parse_file(&file.path, &source, &file.extension, &file.relative_path);
        registry_call_count += 1;

        let file_info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            path: file.relative_path.clone(),
            name: file.path.split('/').next_back().unwrap_or("").to_string(),
            extension: file.extension.clone(),
            symbols: result.symbols,
            lines: source.lines().count() as u32,
        };

        all_imports.extend(result.imports);
        outlines.insert(file.relative_path.clone(), result.outline);
        file_infos.push(file_info);
    }

    let parse_ms = parse_start.elapsed().as_millis() as u64;
    ScanFilesOutput {
        file_infos,
        all_imports,
        outlines,
        parse_ms,
        registry_call_count,
        files_failed,
        files_read,
    }
}

/// Return one-file outline via a single registry call.
///
/// # Arguments
/// * `registry` — parse dispatch.
/// * `file_id` — the DB-assigned UUID for this file (used to tag outline items).
/// * `path` — absolute path to the source file.
/// * `source` — source content (caller reads from disk first).
/// * `extension` — file extension (e.g. "ts", "rs").
pub fn outline_for_file(
    registry: &dyn ParseFile,
    file_id: &str,
    path: &str,
    source: &str,
    extension: &str,
) -> Vec<OutlineItem> {
    let result: ParseResult = registry.parse_file(path, source, extension, file_id);
    result.outline
}

// ── DiscoveredFile re-export ─────────────────────────────────────────────────

pub use crate::scanner::walker::DiscoveredFile;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Abstracts the single-dispatch parse call so tests can inject a counting mock.
///
/// `ParserRegistry` already implements this via its concrete `parse_file` method.
/// Tests use a `TrackingRegistry` that wraps a `ParserRegistry` and an atomic
/// counter and forwards `parse_file` calls to the inner registry while
/// incrementing the counter each time.
pub trait ParseFile: Send + Sync {
    fn parse_file(&self, path: &str, source: &str, extension: &str, file_id: &str) -> ParseResult;
}

impl ParseFile for ParserRegistry {
    fn parse_file(&self, path: &str, source: &str, extension: &str, file_id: &str) -> ParseResult {
        ParserRegistry::parse_file(self, path, source, extension, file_id)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests;
