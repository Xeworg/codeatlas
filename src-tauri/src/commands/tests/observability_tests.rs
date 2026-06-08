//! Tests for SQLite error-mapping boundaries.
//!
//! As of pre-wave-2-foundation PR-A Task A.3, the canonical
//! `is_root_path_conflict` and `map_save_scan_result_error` helpers
//! live in `engine::db::error_mapping`. This file is the single
//! boundary check: it asserts that the presentation layer
//! (`src-tauri/src/commands.rs`) does NOT re-declare those helpers,
//! which would be a regression to the wave-1 duplication.

#[test]
fn commands_rs_does_not_declare_is_root_path_conflict() {
    let commands_rs = include_str!("../../commands.rs");
    assert!(
        !commands_rs.contains("fn is_root_path_conflict"),
        "is_root_path_conflict must live in engine::db::error_mapping, \
         not in src-tauri/src/commands.rs"
    );
}

#[test]
fn commands_rs_does_not_declare_map_save_scan_result_error() {
    let commands_rs = include_str!("../../commands.rs");
    assert!(
        !commands_rs.contains("fn map_save_scan_result_error"),
        "map_save_scan_result_error must live in engine::db::error_mapping, \
         not in src-tauri/src/commands.rs"
    );
}
