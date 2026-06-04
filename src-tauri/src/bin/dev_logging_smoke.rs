//! End-to-end smoke test for dev-mode per-execution file logging.
//!
//! Run with:
//!
//! ```bash
//! cd src-tauri && cargo run --bin dev_logging_smoke
//! RUST_LOG=info cargo run --bin dev_logging_smoke
//! RUST_LOG=warn cargo run --bin dev_logging_smoke
//! ```
//!
//! In a debug build this binary:
//! 1. Resolves the dev log file path the same way `lib::run()` does.
//! 2. Installs the non-blocking file writer via `init_dev_file_logging`.
//! 3. Emits a few log lines at multiple levels.
//! 4. Drops the guard and verifies the file contains the expected entries.
//!
//! In a release build the binary prints a "release build — dev file logging
//! disabled" message and exits 0; the existing release code path is
//! preserved.
//!
//! This is intentionally a binary (not a `#[test]`) because the global
//! tracing subscriber can only be set once per process; running it as a
//! regular test would interfere with the rest of the test suite.

use std::io::Read;

fn main() -> std::process::ExitCode {
    let repo_root = codeatlas_lib::logging::compile_time_repo_root();
    let log_path = codeatlas_lib::logging::dev_log_file_path(&repo_root, chrono::Utc::now());

    println!("dev_log_file (this run): {}", log_path.display());
    println!("(millisecond-precision suffix ensures each execution gets a unique file)");

    #[cfg(debug_assertions)]
    {
        run_debug(&repo_root, &log_path)
    }
    #[cfg(not(debug_assertions))]
    {
        eprintln!("release build — dev file logging disabled (this is expected)");
        let _ = codeatlas_lib::logging::init_dev_file_logging(&repo_root);
        std::process::ExitCode::SUCCESS
    }
}

/// Debug-build path: install the file writer, emit a few log lines, drop
/// the guard, then read the file back and assert the structural markers.
#[cfg(debug_assertions)]
fn run_debug(repo_root: &std::path::Path, log_path: &std::path::Path) -> std::process::ExitCode {
    let guard = codeatlas_lib::logging::init_dev_file_logging(repo_root);
    let guard = match guard {
        Some(g) => g,
        None => {
            eprintln!(
                "smoke FAIL: init_dev_file_logging returned None (file writer could not be installed)"
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    // Emit log lines at every level so the file has substance regardless
    // of which RUST_LOG directive is active in the calling shell.
    tracing::debug!("smoke: debug line");
    tracing::info!("smoke: info line");
    tracing::warn!("smoke: warn line");

    // Drop the guard to flush the non-blocking writer.
    drop(guard);

    check_log_file(log_path)
}

/// Verify the dev log file exists and contains the expected structural
/// markers.
fn check_log_file(log_path: &std::path::Path) -> std::process::ExitCode {
    let mut content = String::new();
    match std::fs::File::open(log_path).and_then(|mut f| f.read_to_string(&mut content)) {
        Ok(_bytes_read) => {
            let has_init_marker = content.contains("dev logging initialized");
            let has_user_line = content.contains("smoke:");
            println!(
                "file has init_marker={}, at least one user line={}",
                has_init_marker, has_user_line
            );
            if has_init_marker && has_user_line {
                println!("smoke OK");
                std::process::ExitCode::SUCCESS
            } else {
                eprintln!("smoke FAIL: file contents did not match expectations");
                eprintln!("---- file contents ----");
                eprintln!("{}", content);
                eprintln!("---- end file ----");
                std::process::ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("smoke FAIL: could not read log file: {}", e);
            std::process::ExitCode::FAILURE
        }
    }
}
