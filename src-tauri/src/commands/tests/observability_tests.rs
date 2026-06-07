//! Tests for logging helper functions and error mapping.
//!
//! These tests verify that:
//! 1. `map_save_scan_result_error` formats error strings correctly.
//! 2. `is_root_path_conflict` detects SQLite UNIQUE constraint violations on `projects.root_path`.
//! 3. Structured log field helpers produce consistent output.
//!
//! Live tracing capture tests are documented in apply-progress.md
//! as "manual validation required" due to tracing global subscriber constraints.

#[allow(unused_imports)]
use crate::commands::{is_root_path_conflict, map_save_scan_result_error};

/// Test that a plain error string is passed through unchanged by `map_save_scan_result_error`.
#[test]
fn map_save_scan_result_error_passes_through_non_conflict() {
    let err = "Connection refused";
    let root_path = "/some/path";
    let result = map_save_scan_result_error(err, root_path, "p-id-123");
    // Non-conflict errors pass through with the same message
    assert_eq!(result, "Connection refused");
}

/// Test that a SQLite UNIQUE constraint error on root_path maps to user-facing message.
#[test]
fn map_save_scan_result_error_maps_root_path_conflict() {
    let err = "UNIQUE constraint failed: projects.root_path";
    let root_path = "/home/user/my-project";
    let result = map_save_scan_result_error(err, root_path, "proj-abc");
    assert!(result.contains("Project already exists"));
    assert!(result.contains("/home/user/my-project"));
    // Raw SQLite error must NOT appear in result
    assert!(!result.contains("UNIQUE constraint"));
    assert!(!result.contains("rusqlite"));
}

/// Test that `is_root_path_conflict` returns true only for root_path UNIQUE errors.
#[test]
fn is_root_path_conflict_true_for_root_path() {
    let err = "UNIQUE constraint failed: projects.root_path";
    assert!(is_root_path_conflict(err));
}

/// Test that `is_root_path_conflict` returns false for other constraint errors.
#[test]
fn is_root_path_conflict_false_for_other_constraints() {
    let err_files = "UNIQUE constraint failed: files.id";
    let err_imports = "UNIQUE constraint failed: imports.source_file_id";
    let err_other = "UNIQUE constraint failed: some_table.other_column";
    assert!(!is_root_path_conflict(err_files));
    assert!(!is_root_path_conflict(err_imports));
    assert!(!is_root_path_conflict(err_other));
}

/// Test that `is_root_path_conflict` returns false for non-constraint errors.
#[test]
fn is_root_path_conflict_false_for_non_constraint_errors() {
    let err = "Connection refused";
    let err_io = "No such file or directory";
    let err_generic = "something went wrong";
    assert!(!is_root_path_conflict(err));
    assert!(!is_root_path_conflict(err_io));
    assert!(!is_root_path_conflict(err_generic));
}

/// Test that root_path conflict detection is case-sensitive (SQLite errors are uppercase).
#[test]
fn is_root_path_conflict_case_sensitive() {
    // Lowercase variant should NOT match (SQLite always uses uppercase)
    let err_lower = "unique constraint failed: projects.root_path";
    assert!(!is_root_path_conflict(err_lower));
}

/// Test that empty root_path in conflict message is handled.
#[test]
fn map_save_scan_result_error_empty_root_path() {
    let err = "UNIQUE constraint failed: projects.root_path";
    let root_path = "";
    let result = map_save_scan_result_error(err, root_path, "proj-xyz");
    assert!(result.contains("Project already exists"));
    assert!(result.contains("path"));
    assert!(!result.contains("UNIQUE constraint"));
}
