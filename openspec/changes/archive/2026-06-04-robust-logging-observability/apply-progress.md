# Apply Progress — `robust-logging-observability`

## Status

`apply` in progress.

## PR 1 — Frontend Error Normalization

Status: ✅ Implemented and reviewed.

### Scope

- Added `getErrorMessage(err: unknown): string` in `src/lib/tauri-api.ts`.
- Updated `src/App.tsx` project scan catch path to use `getErrorMessage(err)`.
- Updated `src/App.tsx` graph load catch path to use `getErrorMessage(e)`.
- Added focused tests in `src/lib/__tests__/tauri-api.test.ts`.

### TDD Evidence

RED:

- Worker reported targeted test failed before helper existed because `getErrorMessage` was not exported.

GREEN:

- Implemented helper and App integration.
- Added tests covering Error, string, `{ message }`, `{ code, message }`, null, undefined, objects without message, non-string message coercion, and non-string code.

TRIANGULATE:

- Added cases for non-string message and non-string code behavior.

REFACTOR:

- Helper is pure and additive.
- Existing `{ code, message }` `ApiError` shape is preserved.

### Validation

```bash
npm run test -- --run src/lib/__tests__/tauri-api.test.ts
# PASS — 10 tests

npm run typecheck
# PASS
```

### Review

Fresh review artifacts:

- `sdd-logging/review-pr1.md` — PASS with note to also update graph-load catch.
- `sdd-logging/review-pr1-final.md` — PASS, no blockers after graph-load catch was updated.

### Changed Files

- `src/lib/tauri-api.ts`
- `src/App.tsx`
- `src/lib/__tests__/tauri-api.test.ts`

### Scope Guard

PR1 did not intentionally modify backend, engine, or Tree-sitter parser files. Pre-existing dirty files remain outside this PR1 scope:

- `engine/src/db/queries.rs`
- `engine/src/scanner/code_parser.rs`
- `engine/src/scanner/parser/typescript.rs`

## PR 2 — Backend Structured Logging

Status: ✅ Implemented.

### Scope

PR 2 adds structured `tracing` calls to `src-tauri/src/commands.rs` for scan lifecycle,
DB persistence failures, graph build, and `projects.root_path` UNIQUE constraint mapping.

Changes are additive and preserve all existing `Result<_, String>` contracts.

### TDD Evidence

**RED (write tests first):**

- Wrote `src-tauri/src/commands/tests/observability_tests.rs` with 7 test cases targeting:
  - `is_root_path_conflict` detection (positive, negative, case-sensitivity, other constraints)
  - `map_save_scan_result_error` user-facing message mapping
- Ran `cargo test observability_tests --lib` → expected failures (functions not yet in commands.rs)

**GREEN (implement to pass):**

- Added `is_root_path_conflict` and `map_save_scan_result_error` helpers to `commands.rs`
- Ran `cargo test observability_tests --lib` → 7/7 PASS
- Added scan lifecycle `tracing::info!` for scan START and COMPLETION
- Added operation-specific `tracing::error!` for non-conflict `save_scan_result` failure paths with structured fields
- Mapped `projects.root_path` conflicts to `WARN` only, avoiding duplicate `ERROR` + `WARN` logs for recoverable duplicate-project conflicts
- Preserved previous non-conflict error prefixes (`Failed to save scan result: ...`, `Failed to update scan result: ...`)
- Enhanced `get_graph` cache hit/miss logs with structured fields
- Added `tracing::warn!` for empty graph build
- `map_save_scan_result_error` uses `tracing::warn!` internally for root_path conflict case

**TRIANGULATE:**

- Verified `map_save_scan_result_error` correctly passes non-conflict errors through unchanged
- Verified root_path conflict detection is case-sensitive (SQLite uses uppercase)
- Verified empty root_path in conflict message is handled gracefully

**REFACTOR:**

- Helpers are focused and preserve the existing `Result<_, String>` contract
- `map_save_scan_result_error` intentionally emits a `WARN` trace only for `projects.root_path` conflicts
- Restored pre-existing `export_view_tests` that the first apply pass had accidentally removed; converted `#[tokio::test]` to plain `#[test]` because the tests are synchronous and `tokio` is not available in this crate
- Removed unused test helper/imports to keep `cargo test` warning-free
- No changes to existing data structures or public command signatures

### Validation

```bash
cd src-tauri
cargo test observability_tests --lib
# PASS — 7 tests

cargo test export_view_tests --lib
# PASS — 2 tests

cargo fmt --check
# PASS

cargo clippy -- -D warnings
# PASS (no warnings)

cargo check
# PASS
```

**Log-level manual validation (documented for future PR review):**

```bash
# Default INFO: DEBUG persistence logs suppressed
RUST_LOG=info cargo run -- --help 2>&1 | grep -c "DEBUG"   # expect 0

# Debug mode: DEBUG logs appear
RUST_LOG=debug cargo run -- --help 2>&1 | grep "DEBUG"    # expect non-zero
```

### Changed Files

- `src-tauri/src/commands.rs`
  - Added `is_root_path_conflict` and `map_save_scan_result_error` observability helpers
  - Added `mod tests` declaring `observability_tests` submodule
  - Preserved/restored existing `export_view_tests` and made them synchronous `#[test]` tests
  - Added scan START lifecycle `tracing::info!` with `project_id`, `root_path`, `files_discovered`
  - Added scan COMPLETION lifecycle `tracing::info!` with `project_id`, `files_persisted`, `symbols_count`, `imports_count`, `duration_ms`
  - Added `tracing::error!` on both `save_scan_result` failure paths with structured `project_id`, `root_path`, `error_detail`
  - Enhanced `get_graph` cache hit to structured `tracing::info!` with `cache_hit=true`, `elapsed_ms`
  - Enhanced `get_graph` empty files to structured `tracing::warn!` with `project_id`
  - Enhanced `get_graph` fresh build to structured `tracing::info!` with `cache_hit=false`, `nodes_count`, `edges_count`, `imports_considered`, `elapsed_ms`
  - `map_save_scan_result_error` uses `tracing::warn!` internally for root_path conflicts; non-conflict callers emit operation-specific `ERROR` logs and preserve previous error prefixes
- `src-tauri/src/commands/tests.rs` (new file) — declares `observability_tests` submodule
- `src-tauri/src/commands/tests/observability_tests.rs` (new file) — 7 unit tests

### Scope Guard

PR 2 did not modify:

- Frontend files (`src/App.tsx`, `src/lib/tauri-api.ts`, frontend tests)
- Tree-sitter parser behavior (`engine/src/scanner/code_parser.rs`, `engine/src/scanner/parser/typescript.rs`)
- `engine/src/db/queries.rs` — helpers use only `repo.save_scan_result()` return value (already `SqliteResult<()>`)

Pre-existing dirty files outside PR 2 scope (unchanged):

- `engine/src/db/queries.rs`
- `engine/src/scanner/code_parser.rs`
- `engine/src/scanner/parser/typescript.rs`

### Scope Guard — `projects.root_path` Conflict

Constraint mapping is implemented entirely in `commands.rs` using error string detection
(`is_root_path_conflict`) with no DB schema changes. The UPSERT behavior in `queries.rs`
is preserved. This is safe to land.

### PR Boundary

PR 2 adds ~120 net lines to `commands.rs` (helpers + enhanced tracing) + 3506 bytes for test files.
All within 400-line review budget.

Remaining items (deferred, not blockers):

- Task 2.2: Import/outline persistence DEBUG log fields audit — DEBUG logs already exist at correct level; confirm field completeness
- Task 2.5: `RUST_LOG` default comment in `lib.rs` — confirmed working via `EnvFilter::from_default_env()` + `add_directive(INFO)`; comment can be added as follow-up
- Task 2.6: Optional parser-miss DEBUG logging — out of scope for PR 2 unless specifically requested

## PR 3 — Dev Per-Execution File Logging

Status: ✅ Implemented, validation pending fresh review.

### Scope

PR 3 adds dev-build file logging so CodeAtlas writes a readable execution log for each dev run.

User decisions:

- Location: repo-local.
- Rotation: one file per execution.
- Default dev detail: DEBUG unless `RUST_LOG` overrides it.

Implemented behavior:

- Dev builds write logs under `logs/dev-runs/`.
- Each run gets a unique file named `codeatlas-dev-YYYYMMDD-HHMMSSmmm.log`.
- Dev default log level is DEBUG when `RUST_LOG` is unset/empty/invalid.
- `RUST_LOG` still overrides the default (`info`, `warn`, `info,codeatlas=debug`, etc.).
- Console/stderr logging is preserved while also writing to the file.
- Release builds do not create dev log files and preserve the previous INFO-default console behavior.
- The non-blocking logging guard is held for the lifetime of `run()`.
- Existing `.gitignore` already ignores `*.log`, so generated logs are not committed.

### TDD Evidence

**RED (write tests first):**

- Added helper tests for repo-local log directory, file naming, file path composition, compile-time repo root, env-filter defaults/overrides, and writable path behavior.
- The first apply attempt timed out before returning an artifact, but left the tests and implementation in the worktree; parent audited and validated the real diff.

**GREEN:**

- Added `src-tauri/src/logging.rs` with pure helpers and cfg-gated tracing initialization.
- Added `tracing-appender = "0.2"` to `src-tauri/Cargo.toml` for non-blocking file logging.
- Replaced direct tracing initialization in `src-tauri/src/lib.rs` with `logging::init_dev_file_logging(&logging::compile_time_repo_root())`.
- Added `src-tauri/src/bin/dev_logging_smoke.rs` to verify a real dev log file is created and contains log entries.

**TRIANGULATE:**

- Verified file names pad timestamps and use millisecond precision.
- Verified `RUST_LOG=info`, `debug`, `warn`, and target-specific directives are respected.
- Verified a temp repo-root log path is writable with the same file-open options used by runtime.

**REFACTOR:**

- Ran `cargo fmt` after `cargo fmt --check` identified formatting changes.
- Kept subscriber installation separate from pure path/filter helpers so tests avoid global tracing subscriber conflicts.

### Validation

```bash
cd src-tauri
cargo test logging --lib
# PASS — 19 tests

cargo run --bin dev_logging_smoke
# PASS — generated logs/dev-runs/codeatlas-dev-20260604-002947183.log and verified init/user log lines

cargo fmt --check
# PASS

cargo check
# PASS
```

### Changed Files

- `src-tauri/Cargo.toml` — adds `tracing-appender = "0.2"`.
- `src-tauri/Cargo.lock` — dependency lock update.
- `src-tauri/src/lib.rs` — initializes logging through the new module and keeps guard alive.
- `src-tauri/src/logging.rs` — dev file logging helpers and cfg-gated subscriber initialization.
- `src-tauri/src/logging/tests.rs` — logging test module declaration.
- `src-tauri/src/logging/tests/dev_logging_tests.rs` — 19 helper tests.
- `src-tauri/src/bin/dev_logging_smoke.rs` — manual/e2e dev logging smoke binary.

### Scope Guard

PR 3 did not intentionally modify frontend files, Tree-sitter parser behavior, DB schema, or backend command behavior from PR2.

### PR Boundary

PR 3 is a self-contained observability slice. It can be reviewed independently from PR1 and PR2.

### Generated Runtime Files

Smoke validation generated ignored runtime logs under:

- `logs/dev-runs/codeatlas-dev-*.log`

These are intentionally not tracked because `.gitignore` contains `*.log`.
