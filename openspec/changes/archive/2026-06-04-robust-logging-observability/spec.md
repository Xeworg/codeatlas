# Delta for Logging & Observability — `robust-logging-observability`

## Purpose

Improve diagnosability across the CodeAtlas frontend and backend so that:

1. End users never see `[object Object]` or raw Rust panics in the UI.
2. Developers can trace scan lifecycle, DB persistence, and parser decisions via structured logs.
3. `projects.root_path` uniqueness conflicts are surfaced with actionable context.
4. Debug-level parser miss logs are gated behind `RUST_LOG=debug`.

**Phase 2 (Tree-sitter TSX method-form recognition) is out of scope except for optional `debug` parser-miss logging.**

---

## ADDED Requirements

### Requirement: Frontend error normalization

The system MUST ensure that errors thrown from any Tauri invoke call never render as `[object Object]` in the UI.

**Rationale:** `toApiError()` returns a plain `{ code, message }` object. When `catch (err)` in `App.tsx` or other components calls `setError(err)` and later renders `String(err)`, the result is `[object Object]` because plain objects don't have a useful `toString()`.

#### Scenario: API error is rendered safely

- GIVEN the user triggers a scan on a non-existent path
- WHEN the backend returns a Tauri error string or the frontend `catch (err)` block executes
- THEN the error displayed to the user MUST be a human-readable string (either `err.message`, `err.code + " — " + err.message`, or a known-error-label)
- AND the display MUST NOT contain the literal text `[object Object]`

#### Scenario: Non-Error thrown is handled gracefully

- GIVEN a Tauri invoke throws a non-`Error` value (e.g., a plain object `{ code: "INTERNAL", message: "..." }`)
- WHEN `getErrorMessage(err)` is called
- THEN the returned string MUST be the `.message` field if present, otherwise a fallback label (e.g., `"Unknown error"`)
- AND the code MUST NOT call `String(err)` directly on the raw caught value

---

### Requirement: Tauri API error shape contract

The system MUST expose a `getErrorMessage(err: unknown): string` helper in `src/lib/tauri-api.ts` that handles all thrown shapes returned by the backend.

**This does not change the backend contract.** The `{ code, message }` shape returned from `toApiError()` is preserved for code-detection logic. The helper only ensures safe message extraction for UI rendering.

#### Scenario: Error with `message` property is extracted

- GIVEN `err` is `{ code: "PATH_NOT_FOUND", message: "Path /foo not found" }`
- WHEN `getErrorMessage(err)` is called
- THEN the returned string MUST be `"PATH_NOT_FOUND — Path /foo not found"` or `"Path /foo not found"`

#### Scenario: Error without `message` property is handled

- GIVEN `err` is a primitive `"Connection refused"` or a non-standard object `{ reason: "..." }`
- WHEN `getErrorMessage(err)` is called
- THEN the returned string MUST be `"Connection refused"` or `"Unknown error"`, respectively
- AND no exception is thrown

---

### Requirement: Backend structured logging at scan lifecycle boundaries

The system MUST emit structured `tracing` log entries with consistent fields at the following scan lifecycle boundaries:

- **Scan start:** `INFO` level with `project_id`, `root_path`, `files_discovered`
- **Scan completion:** `INFO` level with `project_id`, `files_persisted`, `symbols_count`, `imports_count`, `duration_ms`
- **Scan error:** `ERROR` level with `project_id`, `root_path`, `error_detail`

#### Scenario: Successful scan emits lifecycle logs

- GIVEN the user triggers a scan on a valid project
- WHEN the scan completes successfully
- THEN the backend MUST emit at least two `INFO` log entries: one for scan start (with `files_discovered` count) and one for scan completion (with final counts and `duration_ms`)
- AND no `[object Object]` or raw panic strings appear in the log output

#### Scenario: Failed scan emits error log with context

- GIVEN the user triggers a scan on a valid project
- WHEN `repo.save_scan_result()` fails and returns `Err`
- THEN the backend MUST emit an `ERROR` (not `DEBUG`) log entry that includes `root_path` and a human-readable `error_detail`
- AND the command MUST still return the error string to the frontend (no silent swallow)

---

### Requirement: Backend DB persistence error logging

The system MUST emit structured `tracing::debug` logs when individual DB persistence operations fail within a scan, so that degraded scans can be diagnosed without flooding INFO-level logs.

#### Scenario: Import persistence failure is debug-logged

- GIVEN the scan is processing import edges
- WHEN `repo.save_import(imp)` returns `Err(e)` for a specific import
- THEN the backend MUST emit a `DEBUG` log containing the import's `source_file_id`, `target_module`, and the error string
- AND the scan MUST continue processing remaining imports
- AND the final scan result MUST reflect the degraded state (`imports_count` reflects only persisted imports, `error` field is set)

#### Scenario: Outline persistence failure is debug-logged

- GIVEN the scan is processing outline items for a file
- WHEN `repo.save_outline_items()` returns `Err(e)`
- THEN the backend MUST emit a `DEBUG` log containing the `file_id` and error string
- AND the scan MUST continue processing remaining files

**Noise policy:** These debug logs are emitted per-failure, which could be thousands in a degraded scan. They MUST be gated behind `RUST_LOG=debug`. Default `RUST_LOG=info` MUST NOT show per-file/per-import failure logs.

---

### Requirement: Backend graph build logging

The system MUST emit structured `tracing` logs around graph construction.

#### Scenario: Graph cache hit

- GIVEN `get_graph` is called with a `project_id` that has a cached graph
- WHEN the cached graph is found and returned
- THEN the backend MUST emit an `INFO` log with `project_id`, `cache_hit: true`, and `elapsed_ms`

#### Scenario: Graph cache miss and fresh build

- GIVEN `get_graph` is called with a `project_id` that has no cached graph
- WHEN the graph is built fresh from DB
- THEN the backend MUST emit an `INFO` log with `project_id`, `cache_hit: false`, `nodes_count`, `edges_count`, `imports_considered`, and `elapsed_ms`

#### Scenario: Graph build with no files

- GIVEN `get_graph` is called with a `project_id` that exists in DB but has no files
- WHEN the builder returns an empty graph
- THEN the backend MUST emit a `WARN` log with `project_id` indicating no files were found
- AND the command MUST return an error to the frontend (not an empty graph with a 200 OK)

---

### Requirement: `projects.root_path` conflict logging

The system MUST log structured context when a `projects.root_path` UNIQUE constraint violation occurs, so that developers can identify which conflicting path caused the failure.

#### Scenario: Duplicate root_path conflict

- GIVEN a scan is initiated on a path that already exists in the `projects` table
- WHEN `repo.save_scan_result()` catches a DB constraint error for `root_path`
- THEN the backend MUST emit a `WARN` (not `DEBUG`) log containing the conflicting `root_path` value and the constraint error detail
- AND the frontend MUST receive an error message that references `root_path` conflict (e.g., `"Project already exists at path: {root_path}"` rather than a raw SQLite error code)

**Note:** This may require catching the SQLite error in `save_scan_result` or the command layer and re-mapping it. The raw SQLite error (e.g., `UNIQUE constraint failed: projects.root_path`) MUST NOT propagate directly to the frontend.

---

### Requirement: Log level configuration via `RUST_LOG`

The system MUST support `RUST_LOG` environment variable to control log verbosity, with `INFO` as the default.

- `RUST_LOG=info` (default): Shows `INFO`, `WARN`, `ERROR` logs; suppresses `DEBUG` logs.
- `RUST_LOG=debug`: Shows `DEBUG`, `INFO`, `WARN`, `ERROR` logs including parser-miss and per-failure persistence logs.
- `RUST_LOG=warn`: Shows only `WARN`, `ERROR` logs; suppresses `INFO` and `DEBUG`.

#### Scenario: Default log level is INFO

- GIVEN no `RUST_LOG` is set
- WHEN the backend starts
- THEN the tracing subscriber MUST be initialized so that only `INFO`, `WARN`, and `ERROR` messages are printed
- AND `DEBUG` messages MUST be suppressed

#### Scenario: Debug level enables parser miss logging

- GIVEN `RUST_LOG=debug` is set
- WHEN `CodeParser` encounters a file it cannot parse or a language variant it doesn't handle
- THEN the backend MAY emit a `DEBUG` log describing the parser miss (e.g., `"Unsupported syntax in {file_path}: {reason}"`)
- AND this logging MUST NOT appear in default (INFO) mode

---

### Requirement: Command error returns preserve human-readable context

The system MUST ensure that any `String` returned as a `Result<_, String>` error from a Tauri command contains a human-readable message, not a raw SQLite error code, Rust enum variant, or debug output.

#### Scenario: Database error is mapped to user-facing message

- GIVEN a DB operation (save, query, migration) fails with an error
- WHEN the error propagates to a Tauri command return
- THEN the returned `String` MUST be derived from `e.to_string()` where `e` is a meaningful error type (e.g., `rusqlite::Error`, custom error enum with Display impl), not a debug-format struct
- AND the string MUST NOT include internal field names like `Error { code: ...` unless those fields are intentionally user-facing

**Risk flag:** Changing error formatting in `commands.rs` `map_err(|e| e.to_string())` across many commands could affect existing error handling. This change should be applied surgically per command, with test coverage.

---

### Requirement: Optional debug parser miss logging (out of scope for Tree-sitter adaptation)

The system MAY emit debug-level logs when a Tree-sitter parser fails to recognize a syntax construct, provided the log is behind the `RUST_LOG=debug` gate and does not include source code snippets.

**Note:** This requirement covers Phase 1 debug logging only. Tree-sitter parser improvements (Phase 2) are out of scope and will be handled in a separate change spec.

#### Scenario: Parser miss in debug mode

- GIVEN `RUST_LOG=debug` is set
- WHEN `CodeParser::parse_file` encounters a TSX file with a method-like form it cannot categorize
- THEN the backend MAY log at `DEBUG` level: `"Parser miss: file={path} reason=\"unhandled syntax kind: {kind}\""`
- AND the log MUST NOT include the file's source code content

#### Scenario: Parser miss in default mode is silent

- GIVEN `RUST_LOG=info` (default) is set
- WHEN `CodeParser::parse_file` encounters an unrecognized syntax construct
- THEN no log MUST be emitted for this event
- AND the parse MUST continue or gracefully degrade without error

---

## MODIFIED Requirements

_No existing requirements are modified in Phase 1. All changes are additive. Phase 2 Tree-sitter changes will modify parser behavior but not logging/observability requirements._

---

## REMOVED Requirements

_None for Phase 1._

---

## Risks & Flags

| ID  | Risk                                                                                                                                                                                                       | Severity | Mitigation                                                                             |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------- |
| R1  | Adding `DEBUG` logs in hot paths (file parsing loop) could degrade scan performance                                                                                                                        | Medium   | Gate behind `RUST_LOG=debug`; default INFO suppresses them                             |
| R2  | Changing error `String` returns across `commands.rs` could break existing frontend error handling                                                                                                          | Medium   | Apply changes surgically; preserve `Result<_, String>` contract; add unit tests        |
| R3  | `projects.root_path` conflict mapping requires catching SQLite errors in `save_scan_result` or commands layer                                                                                              | Medium   | Add specific constraint error detection before returning; do not swallow the error     |
| R4  | Frontend `getErrorMessage` helper must preserve existing `{ code, message }` shape for code-detection logic                                                                                                | Low      | Helper only extracts `.message` for display; does not modify `toApiError` output       |
| R5  | Review budget: Phase 1 touches frontend (`tauri-api.ts`) and backend (`commands.rs`, `lib.rs`, repository). Estimated ~150–250 lines if scoped carefully. Auto-forecast will trigger PR split if exceeded. | Low      | Keep changes minimal; separate logging-only commits from any future error-type changes |
