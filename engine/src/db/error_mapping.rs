//! Error-mapping helpers for SQLite constraint violations.
//!
//! This module owns the canonical string-to-user-message translation for
//! `save_scan_result`-style errors. It lives in the engine crate (next to
//! the SQLite layer that produces the error strings) so that both the
//! service and the presentation layer can share one implementation.
//!
//! STRICT POLICY: presentation-layer code MUST call these helpers instead
//! of re-implementing the string parsing. The CI architecture guard
//! (`npm run check:arch`) enforces that presentation does not import
//! concrete types from `engine::db`; these helpers are an explicit
//! exception because they are pure string transformations with no
//! coupling to the underlying connection pool.

/// Returns true if the given error string indicates a SQLite UNIQUE
/// constraint violation on the `projects.root_path` column.
///
/// SQLite always uses uppercase in error messages. The check is
/// case-sensitive.
pub fn is_root_path_conflict(err: &str) -> bool {
    err.contains("UNIQUE constraint failed: projects.root_path")
}

/// Maps a `save_scan_result` error string to a user-facing message.
///
/// If the error is a `projects.root_path` UNIQUE constraint violation,
/// returns `"Project already exists at path: {root_path}"`. Otherwise
/// returns the original error string unchanged.
///
/// The root_path conflict case emits a WARN log here. Non-conflict
/// errors are returned unchanged so callers can add operation-specific
/// ERROR context.
#[allow(dead_code)]
pub fn map_save_scan_result_error(
    err: &str,
    root_path: &str,
    project_id: &str,
) -> String {
    if is_root_path_conflict(err) {
        tracing::warn!(
            project_id = %project_id,
            root_path = %root_path,
            "projects.root_path UNIQUE constraint conflict"
        );
        format!("Project already exists at path: {}", root_path)
    } else {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plain error string passes through unchanged.
    #[test]
    fn map_save_scan_result_error_passes_through_non_conflict() {
        let err = "Connection refused";
        let root_path = "/some/path";
        let result = map_save_scan_result_error(err, root_path, "p-id-123");
        assert_eq!(result, "Connection refused");
    }

    /// UNIQUE constraint error on root_path maps to user-facing message.
    #[test]
    fn map_save_scan_result_error_maps_root_path_conflict() {
        let err = "UNIQUE constraint failed: projects.root_path";
        let root_path = "/home/user/my-project";
        let result = map_save_scan_result_error(err, root_path, "proj-abc");
        assert!(result.contains("Project already exists"));
        assert!(result.contains("/home/user/my-project"));
        // Raw SQLite error must NOT appear in result.
        assert!(!result.contains("UNIQUE constraint"));
        assert!(!result.contains("rusqlite"));
    }

    /// `is_root_path_conflict` returns true only for root_path UNIQUE errors.
    #[test]
    fn is_root_path_conflict_true_for_root_path() {
        let err = "UNIQUE constraint failed: projects.root_path";
        assert!(is_root_path_conflict(err));
    }

    /// `is_root_path_conflict` returns false for other constraint errors.
    #[test]
    fn is_root_path_conflict_false_for_other_constraints() {
        let err_files = "UNIQUE constraint failed: files.id";
        let err_imports = "UNIQUE constraint failed: imports.source_file_id";
        let err_other = "UNIQUE constraint failed: some_table.other_column";
        assert!(!is_root_path_conflict(err_files));
        assert!(!is_root_path_conflict(err_imports));
        assert!(!is_root_path_conflict(err_other));
    }

    /// `is_root_path_conflict` returns false for non-constraint errors.
    #[test]
    fn is_root_path_conflict_false_for_non_constraint_errors() {
        let err = "Connection refused";
        let err_io = "No such file or directory";
        let err_generic = "something went wrong";
        assert!(!is_root_path_conflict(err));
        assert!(!is_root_path_conflict(err_io));
        assert!(!is_root_path_conflict(err_generic));
    }

    /// `is_root_path_conflict` is case-sensitive (SQLite errors are uppercase).
    #[test]
    fn is_root_path_conflict_case_sensitive() {
        let err_lower = "unique constraint failed: projects.root_path";
        assert!(!is_root_path_conflict(err_lower));
    }

    /// Empty root_path in conflict message is handled.
    #[test]
    fn map_save_scan_result_error_empty_root_path() {
        let err = "UNIQUE constraint failed: projects.root_path";
        let root_path = "";
        let result = map_save_scan_result_error(err, root_path, "proj-xyz");
        assert!(result.contains("Project already exists"));
        assert!(result.contains("path"));
        assert!(!result.contains("UNIQUE constraint"));
    }
}
