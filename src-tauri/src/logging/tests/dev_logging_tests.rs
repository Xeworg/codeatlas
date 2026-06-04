//! Tests for the dev-mode per-execution file logging helpers.
//!
//! These tests verify the pure helper functions that compose the per-execution
//! dev log file path. The actual subscriber installation in `init_dev_file_logging`
//! is global and is exercised manually (see `apply-progress.md` for the manual
//! verification matrix).
//!
//! Test surfaces:
//! - `dev_log_dir(repo_root)` — `<repo>/logs/dev-runs`
//! - `dev_log_file_name(at)` — `codeatlas-dev-YYYYMMDD-HHMMSS.log` (UTC)
//! - `dev_log_file_path(repo_root, at)` — combined result of the two above
//! - `compile_time_repo_root()` — parent of `CARGO_MANIFEST_DIR`
//! - `dev_default_env_filter(env_var)` — DEBUG default unless RUST_LOG is set

use crate::logging::{
    compile_time_repo_root, dev_default_env_filter, dev_log_dir, dev_log_file_name,
    dev_log_file_path, DEV_LOG_FILE_PREFIX, DEV_LOG_SUBDIR,
};

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// Helper to construct a fixed UTC timestamp without depending on local time.
fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
    let date = NaiveDate::from_ymd_opt(y, mo, d).expect("valid date");
    let time = chrono::NaiveTime::from_hms_opt(h, mi, s).expect("valid time");
    let ndt = NaiveDateTime::new(date, time);
    Utc.from_utc_datetime(&ndt)
}

#[test]
fn dev_log_dir_resolves_under_repo_root() {
    let root = std::path::Path::new("/repo");
    let dir = dev_log_dir(root);
    assert_eq!(dir, std::path::PathBuf::from("/repo/logs/dev-runs"));
}

#[test]
fn dev_log_subdir_constant_matches_expected_layout() {
    assert_eq!(DEV_LOG_SUBDIR, "logs/dev-runs");
    assert!(DEV_LOG_SUBDIR.ends_with("dev-runs"));
}

#[test]
fn dev_log_file_prefix_is_stable() {
    // Lock the prefix to keep the log file naming recognizable across runs.
    assert_eq!(DEV_LOG_FILE_PREFIX, "codeatlas-dev");
}

#[test]
fn dev_log_file_name_uses_utc_timestamp() {
    // 2026-06-03 18:30:45.000 UTC
    let at = utc(2026, 6, 3, 18, 30, 45);
    let name = dev_log_file_name(at);
    assert_eq!(name, "codeatlas-dev-20260603-183045000.log");
}

#[test]
fn dev_log_file_name_handles_midnight() {
    let at = utc(2026, 1, 1, 0, 0, 0);
    assert_eq!(
        dev_log_file_name(at),
        "codeatlas-dev-20260101-000000000.log"
    );
}

#[test]
fn dev_log_file_name_pads_single_digit_components() {
    // 2026-04-09 09:08:07 — every component is single digit.
    let at = utc(2026, 4, 9, 9, 8, 7);
    assert_eq!(
        dev_log_file_name(at),
        "codeatlas-dev-20260409-090807000.log"
    );
}

#[test]
fn dev_log_file_name_year_boundary() {
    // End of one year, start of the next
    let end = utc(2026, 12, 31, 23, 59, 59);
    let start = utc(2027, 1, 1, 0, 0, 0);
    assert_eq!(
        dev_log_file_name(end),
        "codeatlas-dev-20261231-235959000.log"
    );
    assert_eq!(
        dev_log_file_name(start),
        "codeatlas-dev-20270101-000000000.log"
    );
}

#[test]
fn dev_log_file_path_combines_dir_and_name() {
    let root = std::path::Path::new("/repo");
    let at = utc(2026, 6, 3, 18, 30, 45);
    let path = dev_log_file_path(root, at);
    assert_eq!(
        path,
        std::path::PathBuf::from("/repo/logs/dev-runs/codeatlas-dev-20260603-183045000.log")
    );
}

#[test]
fn dev_log_file_path_with_relative_repo_root() {
    let root = std::path::Path::new(".");
    let at = utc(2026, 6, 3, 0, 0, 0);
    let path = dev_log_file_path(root, at);
    assert_eq!(
        path,
        std::path::PathBuf::from("./logs/dev-runs/codeatlas-dev-20260603-000000000.log")
    );
}

#[test]
fn compile_time_repo_root_returns_parent_of_cargo_manifest_dir() {
    let root = compile_time_repo_root();
    // CARGO_MANIFEST_DIR is the src-tauri directory; repo root is its parent.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let expected = manifest.parent().expect("manifest has parent");
    assert_eq!(root, expected);
    // The parent of src-tauri/ should be the repo root which contains the
    // `engine` workspace member. Sanity-check the suffix on the file name.
    assert!(
        root.join("src-tauri").exists() || root.to_string_lossy().ends_with("src-tauri"),
        "expected compile_time_repo_root to point at repo root, got {:?}",
        root
    );
}

#[test]
fn dev_default_env_filter_defaults_to_debug_when_unset() {
    let filter = dev_default_env_filter(None);
    // When the env var is absent, the filter is permissive (debug).
    // We confirm by trying to enable a debug-level directive through it.
    assert!(
        filter.max_level_hint().is_none()
            || filter.max_level_hint().unwrap() >= tracing::Level::DEBUG
    );
}

#[test]
fn dev_default_env_filter_defaults_to_debug_when_empty() {
    let filter = dev_default_env_filter(Some(""));
    assert!(
        filter.max_level_hint().is_none()
            || filter.max_level_hint().unwrap() >= tracing::Level::DEBUG
    );
}

#[test]
fn dev_default_env_filter_respects_info_override() {
    let filter = dev_default_env_filter(Some("info"));
    let hint = filter
        .max_level_hint()
        .expect("info override should set a max level");
    // With RUST_LOG=info, debug events MUST be filtered out.
    assert!(
        hint <= tracing::Level::INFO,
        "expected INFO ceiling, got {:?}",
        hint
    );
}

#[test]
fn dev_default_env_filter_respects_debug_override() {
    let filter = dev_default_env_filter(Some("debug"));
    let hint = filter
        .max_level_hint()
        .expect("debug override should set a max level");
    assert_eq!(hint, tracing::Level::DEBUG);
}

#[test]
fn dev_default_env_filter_respects_warn_override() {
    let filter = dev_default_env_filter(Some("warn"));
    let hint = filter
        .max_level_hint()
        .expect("warn override should set a max level");
    assert_eq!(hint, tracing::Level::WARN);
}

#[test]
fn dev_default_env_filter_falls_back_to_debug_on_invalid_input() {
    // Invalid directives should not panic; the helper falls back to debug.
    let filter = dev_default_env_filter(Some("this_is_not_a_valid_directive=???"));
    assert!(
        filter.max_level_hint().is_none()
            || filter.max_level_hint().unwrap() >= tracing::Level::DEBUG
    );
}

#[test]
fn dev_default_env_filter_respects_target_specific_directive() {
    // RUST_LOG=info,codeatlas=debug means global INFO but our crate at DEBUG.
    let filter = dev_default_env_filter(Some("info,codeatlas=debug"));
    let hint = filter
        .max_level_hint()
        .expect("explicit directives should set a max level");
    // Max level from any directive is the higher of the two — DEBUG.
    assert_eq!(hint, tracing::Level::DEBUG);
}

/// Smoke test: the path the helper returns is writable using the same
/// `OpenOptions` config that `init_dev_file_logging` uses. This catches
/// regressions where the path changes (e.g. different separators on
/// Windows) and the file can no longer be created.
#[test]
fn dev_log_file_path_is_writable_via_std_fs() {
    use std::io::Write;

    let temp_root = std::env::temp_dir().join("codeatlas-dev-logging-smoke");
    let _ = std::fs::remove_dir_all(&temp_root);
    // Use a synthetic repo root inside the temp dir.
    let repo_root = temp_root.join("repo");
    std::fs::create_dir_all(&repo_root).expect("create repo root");

    let at = utc(2026, 6, 3, 18, 30, 45);
    let log_path = dev_log_file_path(&repo_root, at);

    // Mirror the create-then-open sequence from init_dev_file_logging.
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).expect("create log dir");
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("open dev log file");
    writeln!(f, "smoke test line").expect("write line");
    drop(f);

    let content = std::fs::read_to_string(&log_path).expect("read dev log file");
    assert!(content.contains("smoke test line"));

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_root);
}

/// Two executions at distinct timestamps must produce distinct file names.
/// Within the same second the name is identical by design (one file per
/// execution typically spans less than a second; collisions across two
/// real `run()` invocations are unlikely).
#[test]
fn dev_log_file_name_distinct_across_execution_times() {
    let t1 = utc(2026, 6, 3, 18, 30, 45);
    let t2 = utc(2026, 6, 3, 18, 30, 46);
    assert_ne!(dev_log_file_name(t1), dev_log_file_name(t2));
}
