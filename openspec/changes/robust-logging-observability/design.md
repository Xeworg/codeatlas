# SDD Technical Design — `robust-logging-observability`

## Phase Envelope

```yaml
status: design
executive_summary: >
  Write `getErrorMessage` helper to eliminate frontend `[object Object]` errors.
  Add structured backend tracing at scan lifecycle, DB persistence, and graph-build boundaries.
  Map SQLite `projects.root_path` UNIQUE constraint to user-facing messages.
  Gate parser-miss DEBUG logs behind RUST_LOG=debug.
  Chained PRs: PR 1 frontend (~80-120 lines), PR 2 backend (~220-360 lines), stacked-to-main.
artifacts:
  - openspec/changes/robust-logging-observability/design.md      # this file
  - openspec/changes/robust-logging-observability/proposal.md    # pre-existing
  - openspec/changes/robust-logging-observability/spec.md        # pre-existing
  - openspec/changes/robust-logging-observability/tasks.md       # pre-existing
next_recommended: apply
risks:
  - id: R1; description: "DEBUG logs in hot paths may degrade scan performance"; severity: medium
  - id: R2; description: "Error String returns in commands.rs may break frontend contract"; severity: medium
  - id: R3; description: "SQLite UNIQUE constraint error mapping requires surgical changes"; severity: medium
  - id: R4; description: "getErrorMessage must preserve {code, message} shape for existing consumers"; severity: low
  - id: R5; description: "Phase 1 ~300-480 lines exceeds 400-line review budget; auto-split recommended"; severity: low
skill_resolution: paths-injected
skills_loaded:
  - /home/xeworg/.config/opencode/skills/work-unit-commits/SKILL.md
  - /home/xeworg/.config/opencode/skills/chained-pr/SKILL.md
  - /home/xeworg/.config/opencode/skills/cognitive-doc-design/SKILL.md
```

---

## 1. Context and Current Gaps

### 1.1 The `[object Object]` Bug — Root Cause

`src/lib/tauri-api.ts` `toApiError()` returns a plain `{ code, message }` object, not an `Error` subclass. In `src/App.tsx` line 161:

```typescript
} catch (err) {
  setError(err instanceof Error ? err.message : String(err))  // ← BUG
```

When `err` is the plain object from `toApiError()`, `instanceof Error` is `false`, so `String(err)` → `"[object Object]"`. All 5 other catch sites already use the safe pattern; this is the only broken site.

### 1.2 Backend Tracing Gaps

| Gap                                                                              | Location      | Severity   |
| -------------------------------------------------------------------------------- | ------------- | ---------- |
| `save_scan_result` error path has **no tracing** before `Err(...)` string return | `commands.rs` | **High**   |
| DB constraint errors propagated as `Err(String)` without structured context      | `commands.rs` | **Medium** |
| `projects.root_path` UNIQUE conflict has no user-facing mapping                  | `commands.rs` | **Medium** |
| AI command failures no `tracing::warn` at command boundary                       | `commands.rs` | **Low**    |

Existing traces at INFO level are already present for scan completion, graph cache hit/miss, and outline/import persistence failures. The gaps are in error-return paths and constraint handling.

### 1.3 Noise Policy Failure

Without the `RUST_LOG` gate, per-file DEBUG logs would flood INFO output. Default must be `INFO`; only `RUST_LOG=debug` enables parser-miss and persistence-failure DEBUG logs.

---

## 2. Architecture Decisions

### AD-1: `getErrorMessage` helper over `ApiError extends Error` class

**Decision:** Add a pure `getErrorMessage(err: unknown): string` helper in `src/lib/tauri-api.ts`.

**Rationale:**

- Handles `null`, `undefined`, primitives, `Error`, `{ message }`, and `{ code, message }` without type changes.
- Preserves `{ code, message }` shape returned by `toApiError()` for existing code-detection consumers.
- No `ApiError extends Error` class needed; helper is more defensive.
- One-liner in `App.tsx`: `setError(getErrorMessage(err))`.

### AD-2: Backend structured tracing over JSON logs

**Decision:** Use `tracing::info/warn/error!` with string interpolation, not structured JSON.

**Rationale:** Phase 1 does not need structured JSON log aggregation. Standard tracing output is readable in development and CI. Avoids adding `tracing-bottleneck` or `tracing-json` deps.

### AD-3: `projects.root_path` conflict mapping at command layer

**Decision:** Catch SQLite constraint error in `commands.rs` `scan_project` command, not in `queries.rs`.

**Rationale:** `save_scan_result` returns `Result<_, String>`. Mapping at the command layer is surgical and testable. The UPSERT on `id` means re-scans generate new projects; `root_path` UNIQUE conflicts only fire if `id` is reused. Add tracing to confirm this behavior.

### AD-4: Stacked-to-main chained PRs

**Decision:** PR 1 (frontend ~80-120 lines) and PR 2 (backend ~220-360 lines) both target `main` in sequence. No dependency in CI, but logical order preserved.

**Rationale:** Frontend-only PR 1 is self-contained and low-risk. PR 2 touches `commands.rs` which has many `map_err` calls; separate review is safer.

---

## 3. Frontend Design

### 3.1 `getErrorMessage(err: unknown): string`

**File:** `src/lib/tauri-api.ts`

**Interface contract:**

- `null` / `undefined` → `"Unknown error"`
- `string` → return as-is
- `Error` instance → return `err.message`
- `{ message: T, code?: string }` → if `code` exists, return `"CODE — message"`; else return `"message"`
- Fallback (non-standard objects, numbers, etc.) → `"Unknown error"`

**Implementation:**

```typescript
export function getErrorMessage(err: unknown): string {
  if (err === null || err === undefined) return 'Unknown error'
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  if (typeof err === 'object') {
    const obj = err as Record<string, unknown>
    const msg = String(obj.message ?? '')
    const code = typeof obj.code === 'string' ? obj.code : null
    return code ? `${code} — ${msg}` : msg || 'Unknown error'
  }
  return 'Unknown error'
}
```

**Placement:** Export from `src/lib/tauri-api.ts`. Optional barrel export from `src/lib/index.ts` if it exists.

### 3.2 Integration Points

| File                                   | Change                                                           | Pattern                                      |
| -------------------------------------- | ---------------------------------------------------------------- | -------------------------------------------- |
| `src/App.tsx` line ~161                | Replace `String(err)` with `getErrorMessage(err)` in outer catch | Safe; `{code, message}` consumers still work |
| `src/hooks/useGraph.ts`                | Already safe (`instanceof Error ? err.message : String(e)`)      | No change                                    |
| `src/components/panel/DetailPanel.tsx` | Already safe                                                     | No change                                    |
| `src/hooks/useExport.ts`               | Already safe                                                     | No change                                    |

**Backward compatibility:** `getErrorMessage` does not modify `toApiError()` output. The `{ code, message }` shape is preserved. Code-detection logic in existing consumers is unaffected.

### 3.3 Test Design

**File:** `src/lib/__tests__/tauri-api.test.ts` (new)

**Test cases:**

| Input                                                   | Expected output                     |
| ------------------------------------------------------- | ----------------------------------- |
| `null`                                                  | `"Unknown error"`                   |
| `undefined`                                             | `"Unknown error"`                   |
| `"plain string"`                                        | `"plain string"`                    |
| `new Error("test error")`                               | `"test error"`                      |
| `{ code: "PATH_NOT_FOUND", message: "Path not found" }` | `"PATH_NOT_FOUND — Path not found"` |
| `{ message: "foo" }`                                    | `"foo"`                             |
| `{ reason: "no message" }`                              | `"Unknown error"`                   |
| `{ code: "ERR", message: 123 }`                         | `"ERR — 123"` (type coercion)       |
| `1` (number)                                            | `"Unknown error"`                   |

**Run command:**

```bash
npm run test -- --run src/lib/__tests__/tauri-api.test.ts
```

---

## 4. Backend Design

### 4.1 Tracing Strategy by Path

#### Scan / Project Open Path

```rust
// scan_project command entry
tracing::info!(
  project_id = %project_id,
  root_path = %root_path,
  "scan started"
);

// After repo.save_scan_result() call
match repo.save_scan_result(&result) {
  Ok(_) => {
    tracing::info!(
      project_id = %project_id,
      files_persisted = %files.len(),
      symbols_count = %symbols.len(),
      imports_count = %imports.len(),
      duration_ms = %elapsed.as_millis() as u64,
      "scan completed"
    );
  }
  Err(e) => {
    tracing::error!(
      project_id = %project_id,
      root_path = %root_path,
      error_detail = %e,
      "failed to save scan result"
    );
    return Err(e.to_string());
  }
}
```

**Note:** `save_scan_result` error path must emit `ERROR` (not `DEBUG`) with `root_path` and `project_id`. This is the highest-severity gap identified in exploration.

#### DB / Persistence Path

```rust
// Import persistence in scan loop
if let Err(e) = repo.save_import(&imp) {
  tracing::debug!(
    source_file_id = %imp.source_file_id,
    target_module = %imp.target_module,
    error = %e,
    "import persistence failed"
  );
  // Continue processing; degraded scan OK
}

// Outline persistence in scan loop
if let Err(e) = repo.save_outline_items(file_id, &items) {
  tracing::debug!(
    file_id = %file_id,
    error = %e,
    "outline persistence failed"
  );
  // Continue processing
}
```

**Noise policy:** These are `DEBUG` only. `RUST_LOG=info` suppresses them. `RUST_LOG=debug` shows per-failure output (could be thousands in degraded scans).

#### Graph Build Path

```rust
// get_graph command
let elapsed = start.elapsed();

// Cache hit
tracing::info!(
  project_id = %project_id,
  cache_hit = true,
  elapsed_ms = %elapsed.as_millis() as u64,
  "graph retrieved from cache"
);

// Cache miss — fresh build
tracing::info!(
  project_id = %project_id,
  cache_hit = false,
  nodes_count = %graph.nodes.len(),
  edges_count = %graph.edges.len(),
  imports_considered = %imports.len(),
  elapsed_ms = %elapsed.as_millis() as u64,
  "graph built fresh"
);

// Empty graph
tracing::warn!(project_id = %project_id, "graph build: no files found in DB");
return Err("No files found for project".to_string());
```

#### Import Path

Import operations (load imports for project) do not need additional lifecycle tracing beyond existing DB query patterns. Focus on scan and graph paths.

#### Outline Path

Same as DB/persistence path above. Debug-level logging only.

#### Graph Paths

Covered in 4.1.3 above.

### 4.2 `projects.root_path` Conflict Handling Strategy

**Location:** `src-tauri/src/commands.rs` — `scan_project` command, wrapping `repo.save_scan_result()`.

**Trigger:** SQLite `rusqlite::Error::SqliteFailure(code, msg)` where `msg` contains `"UNIQUE constraint failed: projects.root_path"`.

**Mapping logic:**

```rust
match repo.save_scan_result(&result) {
  Ok(_) => { /* existing success path */ }
  Err(e) => {
    let err_str = e.to_string();
    if err_str.contains("UNIQUE constraint failed: projects.root_path") {
      tracing::warn!(root_path = %root_path, "projects.root_path UNIQUE conflict");
      return Err(format!("Project already exists at path: {}", root_path));
    }
    tracing::error!(
      project_id = %project_id,
      root_path = %root_path,
      error_detail = %err_str,
      "save_scan_result failed"
    );
    return Err(err_str);
  }
}
```

**Why this works:** UPSERT on `id` means re-scans typically generate new projects. The constraint fires only if `id` is reused (e.g., from project import). The explicit check is defensive and surfaces the conflict clearly.

**Logging before return:** Every error path in `commands.rs` should emit structured tracing before returning `Err(...)`. The `root_path` conflict is `WARN`; all other `save_scan_result` failures are `ERROR`.

### 4.3 Log Level / Noise Policy

| Level   | When used                                                                         | Gate                     |
| ------- | --------------------------------------------------------------------------------- | ------------------------ |
| `INFO`  | Scan lifecycle boundaries (start/completion), graph cache hit/miss, command entry | Default — always visible |
| `WARN`  | `root_path` UNIQUE conflict, empty graph build                                    | Default — visible        |
| `ERROR` | `save_scan_result` failure, DB constraint failure, command error propagation      | Default — visible        |
| `DEBUG` | Import/outline persistence failures, parser misses                                | `RUST_LOG=debug` only    |

**`RUST_LOG` values:**

- `RUST_LOG=info` (default): Shows `INFO`, `WARN`, `ERROR` only.
- `RUST_LOG=debug`: Shows all levels including parser-miss and persistence-failure DEBUG logs.
- `RUST_LOG=warn`: Shows only `WARN`, `ERROR`.
- No `RUST_LOG` set: Equivalent to `info` (tracing-subscriber reads env).

**Noise control:** Per-failure DEBUG logs (import/outline) can emit thousands of lines in degraded scans. This is intentional at DEBUG level. Default INFO output remains clean.

---

## 5. Strict TDD Validation Plan

### PR 1 — Frontend

**RED (write tests first):**

```bash
# Write test file src/lib/__tests__/tauri-api.test.ts
# Run: expect fail
npm run test -- --run src/lib/__tests__/tauri-api.test.ts
```

**GREEN (implement to pass):**

```bash
# Add getErrorMessage to src/lib/tauri-api.ts
# Fix App.tsx catch block
npm run test -- --run src/lib/__tests__/tauri-api.test.ts
npm run typecheck
npm run lint
```

**TRIANGULATE (add edge cases):**

```bash
# Add tests for: number input, nested object, Error with no message
npm run test -- --run src/lib/__tests__/tauri-api.test.ts
```

**REFACTOR (cleanup):**

- No side effects; helper is pure function.
- No changes to `toApiError()` output shape.

### PR 2 — Backend

**RED (write tests first):**

```bash
# Write src-tauri/tests/observability_tests.rs
# Test: scan success emits INFO (start + completion)
# Test: scan save failure emits ERROR with root_path
# Test: root_path UNIQUE conflict emits WARN with path
# Test: RUST_LOG=info suppresses DEBUG
# Test: RUST_LOG=debug enables DEBUG
cargo test observability  # expect failures
```

**GREEN (implement to pass):**

```bash
# Add tracing calls to commands.rs
# Add constraint error mapping
cargo test observability
cargo clippy -- -D warnings
cargo fmt --check
```

**TRIANGULATE (add integration cases):**

```bash
# Test: import persistence failure emits DEBUG (RUST_LOG=debug)
# Test: empty graph emits WARN
# Test: graph cache hit emits INFO
cargo test observability
```

**REFACTOR (cleanup):**

- Extract `root_path` to variable for reuse across log calls.
- Consistent field naming (`project_id`, `root_path`, `duration_ms`, `elapsed_ms`).
- No changes to `Result<_, String>` return contract.

### Full Gate Commands

```bash
# Frontend
npm run typecheck && npm run lint && npm run test -- --run src/lib/__tests__/tauri-api.test.ts

# Backend
cargo fmt --check && cargo clippy -- -D warnings && cargo test

# Log level verification
RUST_LOG=info cargo run -- --help 2>&1 | grep -c "DEBUG"   # expect 0
RUST_LOG=debug cargo run -- --help 2>&1 | grep -c "DEBUG"  # expect >0
```

---

## 6. Delivery Plan

### PR 1 — Frontend Error Normalization

| Task      | Description                                        | Lines est.  |
| --------- | -------------------------------------------------- | ----------- |
| Task 1.1  | `getErrorMessage` helper in `src/lib/tauri-api.ts` | ~20         |
| Task 1.2  | Fix `App.tsx` catch block to use `getErrorMessage` | ~5          |
| Task 1.3  | Tests in `src/lib/__tests__/tauri-api.test.ts`     | ~50         |
| Task 1.4  | Optional barrel export                             | ~5          |
| **Total** |                                                    | **~80–120** |

**Gates:** `npm run typecheck && npm run test -- --run src/lib/__tests__/tauri-api.test.ts`

### PR 2 — Backend Structured Logging

| Task      | Description                                         | Lines est.   |
| --------- | --------------------------------------------------- | ------------ |
| Task 2.1  | Scan lifecycle logging (start + completion + error) | ~40          |
| Task 2.2  | DB persistence DEBUG logs (import + outline)        | ~30          |
| Task 2.3  | Graph build logging (cache hit + miss + empty)      | ~35          |
| Task 2.4  | `root_path` UNIQUE conflict mapping + WARN log      | ~25          |
| Task 2.5  | `RUST_LOG` default level verification               | ~10          |
| Task 2.6  | Optional parser-miss DEBUG logging                  | ~20          |
| Task 2.7  | Backend integration tests                           | ~80–120      |
| **Total** |                                                     | **~220–360** |

**Gates:** `cargo clippy -- -D warnings && cargo test`

### Review Workload

| PR   | Estimated changed lines | Budget                |
| ---- | ----------------------- | --------------------- |
| PR 1 | ~80–120                 | Under 400-line budget |
| PR 2 | ~220–360                | Under 400-line budget |

Both PRs individually stay under the 400-line budget. Chained PR strategy: stacked-to-main.

---

## 7. Non-Goals

| Non-Goal                                                 | Rationale                                                                                                                        |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 2 Tree-sitter TSX method-form recognition**      | Deferred to separate SDD. Parser fixture creation and AST dumps are Phase 2 work.                                                |
| **Large frontend error-type refactor**                   | `ApiError extends Error` class not implemented unless future SDD justifies it. `getErrorMessage` helper is the Phase 1 approach. |
| **Source code snippets in parser-miss logs**             | Explicitly forbidden. Debug logs contain `file={path}` and `reason={kind}` only — no source content.                             |
| **Performance benchmarking**                             | Not in scope for Phase 1. DEBUG logs are behind RUST_LOG gate; default INFO has no per-file overhead.                            |
| **Changes to `engine/src/scanner/code_parser.rs`**       | Dirty worktree — excluded from Phase 1.                                                                                          |
| **Changes to `engine/src/scanner/parser/typescript.rs`** | Parser logic deferred to Phase 2. Only optional debug hook in Phase 1.                                                           |
| **JSON structured logging**                              | Plain `tracing!` macro interpolation is sufficient for Phase 1. No new deps.                                                     |

---

## 8. Risks and Rollback

### Risk Register

| ID  | Risk                                                                                                               | Severity | Likelihood | Mitigation                                                                          |
| --- | ------------------------------------------------------------------------------------------------------------------ | -------- | ---------- | ----------------------------------------------------------------------------------- |
| R1  | DEBUG logs in hot paths degrade scan performance                                                                   | Medium   | Low        | Gate behind `RUST_LOG=debug`; default INFO suppresses them                          |
| R2  | Changing error `String` returns in `commands.rs` breaks frontend contract                                          | Medium   | Low        | Surgical changes; preserve `Result<_, String>` contract; test coverage              |
| R3  | `root_path` UNIQUE constraint error mapping misses edge cases                                                      | Medium   | Medium     | Explicit string check on `e.to_string()`; WARN log confirms conflict                |
| R4  | `getErrorMessage` breaks `{ code, message }` shape for existing consumers                                          | Low      | Low        | Helper extracts `.message` only; does not modify `toApiError` output                |
| R5  | Both PRs together exceed 400-line review budget                                                                    | Low      | Medium     | Each PR individually is under budget; stacked-to-main handles sequencing            |
| R6  | Dirty worktree conflicts when landing PR 2                                                                         | Low      | Low        | Not modifying `engine/src/` files; PR 2 only touches `src-tauri/src/`               |
| R7  | `projects.root_path` UNIQUE conflict behavior not confirmed — re-scan may generate new project instead of conflict | Medium   | Medium     | Add structured logging to confirm UPSERT behavior before landing constraint mapping |
| R8  | Multiple catch sites silently swallow errors (analytics fetches)                                                   | Medium   | Medium     | Acceptable for non-blocking features; observability gap, not a bug                  |

### Rollback Plan

**PR 1 rollback:**

- Revert `src/lib/tauri-api.ts` changes (remove `getErrorMessage`).
- Revert `src/App.tsx` change (restore `String(err)` in catch block).
- Revert `src/lib/__tests__/tauri-api.test.ts` (delete file).
- Frontend returns to `[object Object]` for errors. No data migration.

**PR 2 rollback:**

- Revert `src-tauri/src/commands.rs` tracing additions and constraint mapping.
- Revert `src-tauri/src/lib.rs` if any changes made.
- Revert `src-tauri/tests/observability_tests.rs` if added.
- Backend returns to pre-logging state. No data migration.

**Both PRs rollback:**

- No state corruption. Structured logs stop being emitted.
- Scan behavior is unchanged.

**In-flight scan interruption:**

- `save_scan_result` writes atomically. No partial state if error returns before write.
- Safe to re-scan after rollback.

---

## 9. Key Decisions Made in Design

| Decision                                                              | Rationale                                                                       |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `getErrorMessage` helper approach over `ApiError extends Error` class | More defensive; handles all thrown shapes; preserves `{code, message}` contract |
| Plain `tracing!` macro over JSON structured logs                      | No new deps; readable in dev/CI; sufficient for Phase 1                         |
| Constraint error mapped at command layer, not repo layer              | Surgical; testable; preserves repo abstraction                                  |
| `projects.root_path` UNIQUE conflict emits `WARN` (not `ERROR`)       | Conflict is recoverable; user already has the project; WARN is appropriate      |
| Per-failure persistence logs are `DEBUG` only                         | Could emit thousands in degraded scans; INFO stays clean by default             |
| Optional parser-miss DEBUG logging in Phase 1 scope                   | Explicitly allowed; one debug! call per unhandled case; no source snippets      |
