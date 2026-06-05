//! Dev-mode per-execution file logging.
//!
//! In debug builds (`debug_assertions`), the Tauri `run()` entry point installs
//! an additional non-blocking tracing writer that records the execution log to
//! `<repo>/logs/dev-runs/codeatlas-dev-YYYYMMDD-HHMMSS.log`. This gives the
//! agent or developer a readable per-run log file they can inspect after a
//! failure without grepping through a busy terminal.
//!
//! Release builds keep the existing console-only INFO-default behavior: no
//! per-execution file is created, no extra dependency is loaded into the
//! production binary, and the existing `RUST_LOG` env var still controls the
//! level for the stderr writer.
//!
//! ## Design
//!
//! - The path/name helpers (`dev_log_dir`, `dev_log_file_name`,
//!   `dev_log_file_path`, `compile_time_repo_root`) are pure functions. They
//!   can be unit-tested without touching the global tracing subscriber.
//! - The subscriber installer (`init_dev_file_logging`) is global by nature
//!   (a process can only have one `tracing` subscriber) and is exercised
//!   manually; see `openspec/changes/robust-logging-observability/apply-progress.md`
//!   for the manual verification matrix.
//! - `tracing-appender` is only pulled in for debug builds via a `cfg`-gated
//!   import. The release build never references it.

use std::path::{Path, PathBuf};

/// Subdirectory of the repository root where dev-run log files are written.
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub const DEV_LOG_SUBDIR: &str = "logs/dev-runs";

/// Filename prefix for per-execution dev log files.
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub const DEV_LOG_FILE_PREFIX: &str = "codeatlas-dev";

/// Default `EnvFilter` directive used when `RUST_LOG` is unset or invalid.
///
/// DEBUG keeps the dev log informative; release builds continue to use INFO.
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub const DEV_DEFAULT_DIRECTIVE: &str = "debug";

/// Resolve the dev log directory under a given repo root.
///
/// Returns `<repo_root>/logs/dev-runs`. The directory is not created by this
/// function — callers decide how to handle a missing parent (typically
/// `create_dir_all` with a warning if it fails).
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub fn dev_log_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(DEV_LOG_SUBDIR)
}

/// Build the per-execution dev log file name from a UTC timestamp.
///
/// Format: `codeatlas-dev-YYYYMMDD-HHMMSSmmm.log` (UTC, second + millisecond
/// precision, fixed-width zero-padded).
///
/// The timestamp is rendered in UTC so that file ordering is consistent
/// regardless of the host machine's locale or timezone. Millisecond
/// precision keeps two executions started within the same second from
/// append-colliding into a single file, which preserves the "one log file
/// per execution" contract without sacrificing readability.
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub fn dev_log_file_name(at: chrono::DateTime<chrono::Utc>) -> String {
    format!(
        "{}-{}.log",
        DEV_LOG_FILE_PREFIX,
        at.format("%Y%m%d-%H%M%S%3f")
    )
}

/// Resolve the absolute dev log file path for a given repo root and timestamp.
///
/// Equivalent to `dev_log_dir(repo_root).join(dev_log_file_name(at))` but
/// kept as a separate helper so call sites stay readable and tests can
/// pin the combined result.
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub fn dev_log_file_path(repo_root: &Path, at: chrono::DateTime<chrono::Utc>) -> PathBuf {
    dev_log_dir(repo_root).join(dev_log_file_name(at))
}

/// Resolve the repository root at compile time.
///
/// `CARGO_MANIFEST_DIR` is `<repo>/src-tauri`. The repo root is its parent.
/// The lookup is purely compile-time so there is no runtime filesystem
/// dependency for the helper itself.
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub fn compile_time_repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(manifest)
}

/// Resolve the dev-mode `EnvFilter` from a `RUST_LOG` value.
///
/// - `None` or an empty/whitespace-only string: returns a filter that allows
///   `DEBUG` (the dev default).
/// - A parseable directive string: returns a filter built from those
///   directives (which can be a single level like `info` or a per-target
///   spec like `info,codeatlas=debug`).
/// - A directive that fails to parse: falls back to the dev default so the
///   process never panics on a bad env var.
///
/// The function does not consult the actual process environment directly;
/// callers pass the value they read. This keeps the helper pure and lets
/// tests exercise every branch.
#[allow(dead_code)] // used in dev builds + tests; release has no caller
pub fn dev_default_env_filter(rust_log_env: Option<&str>) -> tracing_subscriber::EnvFilter {
    match rust_log_env {
        Some(s) if !s.trim().is_empty() => tracing_subscriber::EnvFilter::try_new(s)
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEV_DEFAULT_DIRECTIVE)),
        _ => tracing_subscriber::EnvFilter::new(DEV_DEFAULT_DIRECTIVE),
    }
}

// ─── Subscriber installation (cfg-gated) ────────────────────────────────────

/// Initialize dev-mode per-execution file logging.
///
/// Returns `Some(WorkerGuard)` if a non-blocking file writer was installed.
/// The guard must be kept alive for the lifetime of `run()`; dropping it
/// flushes and stops the background log writer thread.
///
/// On any failure (cannot create dir, cannot open file, subscriber already
/// set, etc.) this function logs a warning to stderr and falls back to a
/// console-only subscriber at the dev default. It never panics.
///
/// In release builds the signature collapses to `()`; no file is written.
#[cfg(debug_assertions)]
pub fn init_dev_file_logging(
    repo_root: &Path,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_appender::non_blocking;
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

    let log_dir = dev_log_dir(repo_root);

    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!(
            "warning: failed to create dev log dir {}: {}",
            log_dir.display(),
            e
        );
        init_console_only_fallback();
        return None;
    }

    let now = chrono::Utc::now();
    let log_path = dev_log_file_path(repo_root, now);

    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "warning: failed to open dev log file {}: {}",
                log_path.display(),
                e
            );
            init_console_only_fallback();
            return None;
        }
    };

    let (file_writer, guard) = non_blocking(file);

    let env_filter = dev_default_env_filter(std::env::var("RUST_LOG").ok().as_deref());

    // Console (stderr) layer — preserves existing terminal logging so dev
    // output is still visible in the terminal the agent/developer is
    // looking at.
    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(env_filter.clone());

    // File layer — non-blocking, no ANSI escapes (logs are inspected in
    // editors / `less`, not in a terminal attached to this process).
    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_filter(env_filter);

    let result = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .try_init();

    match result {
        Ok(()) => {
            tracing::info!(
                dev_log_file = %log_path.display(),
                "dev logging initialized; execution log will be written to this file"
            );
            Some(guard)
        }
        Err(e) => {
            eprintln!("warning: failed to install tracing subscriber: {}", e);
            None
        }
    }
}

/// Release build: no dev file logging. The existing console-only INFO-default
/// subscriber is installed and a no-op guard is returned.
#[cfg(not(debug_assertions))]
pub fn init_dev_file_logging(_repo_root: &Path) -> Option<()> {
    // Mirror the pre-PR3 behavior so release builds are byte-identical to
    // before this change at the tracing-init level.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .try_init();
    None
}

/// Fallback for the dev-build path when the file writer cannot be installed.
///
/// Logs to stderr at the dev default level so the agent still sees DEBUG
/// output in the terminal even if the file failed to open.
#[cfg(debug_assertions)]
fn init_console_only_fallback() {
    let env_filter = dev_default_env_filter(std::env::var("RUST_LOG").ok().as_deref());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();
}

#[cfg(test)]
mod tests;
