# SDD Phase Envelope — `robust-logging-observability` / `tasks`

## Status

**`tasks`** — Implementation tasks written from spec and design context.

---

## Review Workload Forecast

| Field                   | Value                                                               |
| ----------------------- | ------------------------------------------------------------------- |
| Estimated changed lines | 300–480 (frontend: 80–120, backend: 220–360)                        |
| 400-line budget risk    | **Medium** — Upper range exceeds budget; auto-split recommended     |
| Chained PRs recommended | **Yes**                                                             |
| Suggested split         | PR 1 → Frontend error normalization (self-contained, ~80–120 lines) |
|                         | PR 2 → Backend structured logging (larger, ~220–360 lines)          |
| Delivery strategy       | auto-chain                                                          |
| Chain strategy          | stacked-to-main                                                     |

**Rationale:** Spec requirements span frontend (`src/lib/tauri-api.ts`) and backend (`src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`). When combining frontend tests, backend logging additions, and backend tests, total exceeds 400-line budget risk threshold. Stacked-to-main preferred since both PRs can land independently.

---

## Decision needed before apply: No

## Chained PRs recommended: Yes

## Chain strategy: stacked-to-main

## 400-line budget risk: Medium

---

## Implementation Tasks

### PR 1: Frontend Error Normalization

**Target file:** `src/lib/tauri-api.ts`
**Tests:** `src/lib/__tests__/tauri-api.test.ts` (new file)
**Size estimate:** ~80–120 lines (helper + tests + integration)

---

#### Task 1.1: Add `getErrorMessage` helper function

**Files:** `src/lib/tauri-api.ts`

**TDD RED:**

- Write test in `src/lib/__tests__/tauri-api.test.ts`:
  - `getErrorMessage({ code, message })` → extracts `"CODE — message"`
  - `getErrorMessage({ message: "foo" })` → `"foo"`
  - `getErrorMessage("plain string")` → `"plain string"`
  - `getErrorMessage({ reason: "no message" })` → `"Unknown error"`
  - `getErrorMessage(null)` → `"Unknown error"`
  - `getErrorMessage(undefined)` → `"Unknown error"`

**TDD GREEN:**

- Add to `src/lib/tauri-api.ts`:
  ```typescript
  export function getErrorMessage(err: unknown): string {
    if (err === null || err === undefined) return 'Unknown error'
    if (typeof err === 'string') return err
    if (typeof err === 'object' && 'message' in err) {
      const msg = String((err as Record<string, unknown>).message)
      if ('code' in err && typeof (err as Record<string, unknown>).code === 'string') {
        return `${(err as Record<string, unknown>).code} — ${msg}`
      }
      return msg
    }
    return 'Unknown error'
  }
  ```

**TDD TRIANGULATE:**

- Add test: `getErrorMessage(new Error("test"))` → `"test"`
- Add test: `getErrorMessage({ code: "ERR", message: 123 })` → `"ERR — 123"` (type coercion)

**TDD REFACTOR:**

- Keep helper focused; no side effects; pure function.

**Verification:**

```bash
npm run typecheck
npm run test -- --run src/lib/__tests__/tauri-api.test.ts
```

---

#### Task 1.2: Integrate `getErrorMessage` in error rendering

**Files:** `src/App.tsx`

**TDD RED:**

- Verify existing tests fail if `String(err)` is still used in error display.

**TDD GREEN:**

- Find error display in `App.tsx` (e.g., `setError(err)` or `String(err)` in JSX)
- Replace with `getErrorMessage(err)` for display.

**TDD TRIANGULATE:**

- Test: scan failure error displays human-readable message (not `[object Object]`).

**TDD REFACTOR:**

- Ensure `{ code, message }` contract still works for code-detection logic.

**Verification:**

```bash
npm run lint
npm run test -- --run src/App.test.tsx 2>/dev/null || echo "No App tests found"
```

---

#### Task 1.3: Export `getErrorMessage` from barrel (optional)

**Files:** `src/lib/index.ts` (if exists)

**Verification:**

```bash
npm run typecheck
```

---

### PR 2: Backend Structured Logging

**Target files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
**Tests:** `src-tauri/tests/` (integration or unit)
**Size estimate:** ~220–360 lines (lifecycle logs + DB errors + graph logs + root_path mapping + tests)

---

#### Task 2.1: Audit `save_scan_result` error path and add lifecycle logging

**Files:** `src-tauri/src/commands.rs`

**TDD RED:**

- Write test: `save_scan_result` failure emits `tracing::error!` with `root_path` context.

**TDD GREEN:**

- Find `save_scan_result` call in `scan_project` command.
- Add `tracing::error!("failed to save scan result for {root_path}: {error_detail}")`.
- Add `tracing::info!("scan started: project_id={project_id} root_path={root_path} files_discovered={count}")`.
- Add `tracing::info!("scan completed: project_id={project_id} files_persisted={n} symbols={sym} imports={imp} duration_ms={ms}")`.

**TDD TRIANGULATE:**

- Test: successful scan emits two INFO logs (start, completion).
- Test: failed save emits ERROR log with root_path.

**TDD REFACTOR:**

- Extract `root_path` to variable once; reuse in logs.
- Use consistent field naming (`project_id`, `root_path`, `duration_ms`).

**Verification:**

```bash
cargo test
cargo clippy -- -D warnings
```

---

#### Task 2.2: Add DB persistence DEBUG logs for import/file failures

**Files:** `src-tauri/src/commands.rs`

**Context:** Per the spec, these must be behind `RUST_LOG=debug` gate. They already exist in some places per init.md; audit and ensure consistency.

**TDD RED:**

- Write test: `save_import` failure emits `tracing::debug!` with `source_file_id`, `target_module`.
- Write test: `save_outline_items` failure emits `tracing::debug!` with `file_id`.

**TDD GREEN:**

- Confirm existing `tracing::debug!` calls include required fields.
- Add missing fields where absent.
- Ensure no `INFO` or `WARN` logs for per-failure events.

**TDD TRIANGULATE:**

- Test: with `RUST_LOG=debug`, debug logs appear.
- Test: with `RUST_LOG=info` (default), debug logs suppressed.

**TDD REFACTOR:**

- Factor debug-log helper if pattern repeats (optional).

**Verification:**

```bash
RUST_LOG=debug cargo test
RUST_LOG=info cargo test
```

---

#### Task 2.3: Add graph build logging

**Files:** `src-tauri/src/commands.rs`

**TDD RED:**

- Write test: `get_graph` with cache hit emits `INFO` with `cache_hit: true`, `elapsed_ms`.
- Write test: `get_graph` with cache miss emits `INFO` with `cache_hit: false`, `nodes_count`, `edges_count`, `imports_considered`, `elapsed_ms`.
- Write test: `get_graph` with no files emits `WARN` with `project_id`.

**TDD GREEN:**

- Find `get_graph` handler or `GraphCache` usage.
- Add structured `tracing::info!` for cache hit path.
- Add structured `tracing::info!` for fresh build path (counts from graph builder).
- Add `tracing::warn!` for empty graph path; return error, not 200 OK.

**TDD TRIANGULATE:**

- Test: cache hit returns existing graph and logs.
- Test: cache miss builds and caches graph.
- Test: empty DB emits warn and returns error string.

**TDD REFACTOR:**

- Ensure `nodes_count`, `edges_count` come from graph builder return.

**Verification:**

```bash
cargo test
cargo clippy -- -D warnings
```

---

#### Task 2.4: Map `projects.root_path` UNIQUE constraint to user message

**Files:** `src-tauri/src/commands.rs`

**TDD RED:**

- Write test: duplicate `root_path` returns user-facing message (not raw SQLite error).
- Write test: `tracing::warn!` emitted with conflicting `root_path` value.

**TDD GREEN:**

- In `scan_project` command, wrap `save_scan_result` call in `match` that checks for SQLite constraint error.
- Map `rusqlite::Error::QueryReturnedNoRows` (not applicable here) or ` rusqlite::Error::SqliteFailure(code, msg)` where `code` indicates constraint.
- Return `"Project already exists at path: {root_path}"` instead of raw SQLite string.
- Emit `tracing::warn!("root_path conflict: {root_path}");`.

**TDD TRIANGULATE:**

- Test: attempt scan on existing project path → error message contains the path.
- Test: error contains `"Project already exists"` not `"UNIQUE constraint failed"`.

**TDD REFACTOR:**

- Consider adding a helper `map_db_constraint_error(err, root_path)` in `commands.rs`.

**Verification:**

```bash
cargo test
cargo clippy -- -D warnings
```

---

#### Task 2.5: Verify `RUST_LOG` configuration and default level

**Files:** `src-tauri/src/lib.rs`

**TDD RED:**

- Write test: default (no `RUST_LOG`) shows `INFO`, hides `DEBUG`.
- Write test: `RUST_LOG=debug` enables `DEBUG` logs.

**TDD GREEN:**

- Confirm existing subscriber init respects `RUST_LOG` env var.
- Confirm default is `INFO` when env var absent.
- Add comment documenting `RUST_LOG` values and effect.

**TDD TRIANGULATE:**

- Run with `RUST_LOG=info cargo run` → no DEBUG output.
- Run with `RUST_LOG=debug cargo run` → DEBUG output appears.

**TDD REFACTOR:**

- Ensure no hardcoded `tracing::Level` overrides subscriber init.

**Verification:**

```bash
RUST_LOG=info cargo run -- --help 2>&1 | grep -c "DEBUG"  # should be 0
RUST_LOG=debug cargo run -- --help 2>&1 | grep "DEBUG"    # should show parser-miss logs
```

---

#### Task 2.6: Add optional debug parser-miss logging (Phase 1 scope only)

**Files:** `src-tauri/src/commands.rs` or `engine/src/scanner/code_parser.rs`

**Note:** This covers Phase 1 debug logging for TSX parser misses. Phase 2 Tree-sitter improvements are out of scope.

**TDD RED:**

- Write test (if testable): `RUST_LOG=debug` enables parser-miss logs; `RUST_LOG=info` suppresses them.

**TDD GREEN:**

- If existing code already has `tracing::debug!` for parser misses, confirm they are behind `RUST_LOG` gate.
- If missing, add `tracing::debug!("parser miss: file={path} reason=\"unhandled syntax kind: {kind}\"")` without source snippets.
- Ensure no `INFO` or `WARN` for parser misses by default.

**TDD TRIANGULATE:**

- Test: `RUST_LOG=info` → no parser-miss output.
- Test: `RUST_LOG=debug` → parser-miss logs visible.

**TDD REFACTOR:**

- Keep logs behind gate; do not add conditional `if RUST_LOG == "debug"` in hot paths.

**Verification:**

```bash
RUST_LOG=info cargo test 2>&1 | grep -i "parser miss"  # should be empty
RUST_LOG=debug cargo test 2>&1 | grep -i "parser miss"  # may appear
```

---

## Non-Goals (Out of Scope)

- **Phase 2 Tree-sitter TSX method-form recognition** — deferred to separate SDD.
- **Source code snippets in parser-miss logs** — never included.
- **Performance benchmarking** — not in scope for Phase 1.
- **Frontend error boundary refactor** — only `getErrorMessage` helper added; Error subclass not implemented unless Task 1.2 reveals need.

---

## Validation Commands

```bash
# All gates (run after each PR)
cargo fmt --check
cargo clippy -- -D warnings
cargo test

npm run lint
npm run test
npm run typecheck

# Specific to PR 1 (frontend)
npm run test -- --run src/lib/__tests__/tauri-api.test.ts
npm run typecheck

# Specific to PR 2 (backend)
cargo test -- logging 2>/dev/null || cargo test  # run all if no specific test
RUST_LOG=info cargo run -- --help 2>&1 | grep "DEBUG"  # should be 0
RUST_LOG=debug cargo run -- --help 2>&1 | grep -c "DEBUG"  # should be >0
```

---

## Dependency Order

1. **PR 1 (frontend) must land before PR 2** — No; they are independent. PR 1 changes `src/` only; PR 2 changes `src-tauri/` only. **Stacked-to-main** is correct: both target `main` in sequence.
2. **Task order within PR 1:** Task 1.1 → 1.2 → 1.3 (1.1 is prerequisite for 1.2).
3. **Task order within PR 2:** Tasks 2.1–2.6 are independent; can parallelize if multiple developers available.
4. **Commits within each PR:** Group by work unit (e.g., "feat(frontend): add getErrorMessage helper", "feat(backend): add scan lifecycle logging", "feat(backend): add root_path conflict mapping").

---

## Rollback Plan

- **PR 1 rollback:** Revert `src/lib/tauri-api.ts` changes; frontend returns to `[object Object]` for errors. No data migration.
- **PR 2 rollback:** Revert `src-tauri/src/commands.rs` and `lib.rs` changes; backend returns to pre-logging state. No data migration.
- **Both PRs rollback:** No state corruption; logs simply stop being emitted.

---

## Skill Resolution

| Field             | Value                                                                                                                         |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| paths-injected    | `/home/xeworg/.config/opencode/skills/work-unit-commits/SKILL.md`, `/home/xeworg/.config/opencode/skills/chained-pr/SKILL.md` |
| fallback-registry | no                                                                                                                            |
| fallback-path     | none                                                                                                                          |
| skill_resolution  | `paths-injected`                                                                                                              |

---

## Next Recommended

| Phase    | Outputs                          | Focus                                                |
| -------- | -------------------------------- | ---------------------------------------------------- |
| `apply`  | TDD implementation with tests    | Follow RED → GREEN → TRIANGULATE → REFACTOR per task |
| `verify` | verify-report, evidence_of_tests | Run all gates; confirm logs appear in test output    |

---

## Engram Memory

If Engram is available, save:

- **title:** SDD tasks written for robust-logging-observability
- **type:** architecture
- **scope:** project
- **topic_key:** sdd/robust-logging-observability/tasks
- **content:**
  **What**: Tasks written for Phase 1 logging/observability SDD.
  **Why**: Spec required breakdown into reviewable TDD units; estimated >400 lines → recommended stacked-to-main chained PRs.
  **Where**: `openspec/changes/robust-logging-observability/tasks.md`
  **Learned**: Frontend error normalization is self-contained (~80–120 lines, PR 1). Backend logging is larger (~220–360 lines, PR 2). Tasks 2.1–2.6 are independent within PR 2.

---

### PR 3: Dev Per-Execution File Logging

**Decision source:** User requested a repo-local execution log in dev mode so future failures can be debugged by reading what the program was doing before the failure.

**User decisions:**

- Log location: repo-local.
- Rotation: one file per execution.
- Default dev detail: DEBUG unless `RUST_LOG` overrides it.

**Target files:** `src-tauri/src/lib.rs`, `src-tauri/src/logging.rs`, `src-tauri/src/logging/tests.rs`, `src-tauri/src/logging/tests/dev_logging_tests.rs`, `src-tauri/src/bin/dev_logging_smoke.rs`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`.

#### Task 3.1: Add dev log path and filter helpers

**TDD RED:**

- Add tests for `logs/dev-runs`, `codeatlas-dev-YYYYMMDD-HHMMSSmmm.log`, compile-time repo root, and `RUST_LOG` default/override behavior.

**TDD GREEN:**

- Implement pure helpers in `src-tauri/src/logging.rs`.

**Verification:**

```bash
cd src-tauri && cargo test logging --lib
```

#### Task 3.2: Initialize dev file logging

**TDD RED:**

- Add smoke binary or manual check proving a real file is created and contains emitted lines.

**TDD GREEN:**

- Add non-blocking tracing file writer in debug builds.
- Keep stderr logging.
- Keep guard alive for `run()` lifetime.
- Keep release builds console-only with INFO default.

**Verification:**

```bash
cd src-tauri && cargo run --bin dev_logging_smoke
```

#### Task 3.3: Validate and document generated log behavior

**Verification:**

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
```

**Non-goals:**

- No production file logging.
- No app-data logging for this slice.
- No frontend changes.
- No Tree-sitter parser behavior changes.
