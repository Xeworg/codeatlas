# SDD Proposal — `robust-logging-observability`

## Change Metadata

| Field            | Value                                                                                                                                                                                               |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change ID        | `robust-logging-observability`                                                                                                                                                                      |
| SDD phase        | `proposal`                                                                                                                                                                                          |
| Target artifact  | `openspec/changes/robust-logging-observability/proposal.md`                                                                                                                                         |
| Author           | SDD executor (Pi session)                                                                                                                                                                           |
| Date             | 2026-06-03                                                                                                                                                                                          |
| Skill resolution | `paths-injected`                                                                                                                                                                                    |
| Skills loaded    | `/home/xeworg/.config/opencode/skills/work-unit-commits/SKILL.md`, `/home/xeworg/.config/opencode/skills/chained-pr/SKILL.md`, `/home/xeworg/.config/opencode/skills/cognitive-doc-design/SKILL.md` |

---

## Intent

Improve CodeAtlas diagnosability so that:

1. End users never see `[object Object]` or raw Rust panics in the UI.
2. Developers can trace scan lifecycle, DB persistence, and parser decisions via structured logs.
3. `projects.root_path` uniqueness conflicts are surfaced with actionable context.
4. Debug-level parser-miss logs are gated behind `RUST_LOG=debug`.

**Phase 2 (Tree-sitter TSX method-form recognition) is deferred to a separate SDD.**

---

## Motivation

From Phase 1 exploration (`/tmp/pi-subagents-uid-1000/chain-runs/744e9225/sdd-logging/explore.md`):

- **Root bug:** `src/App.tsx` line 161 calls `String(err)` on a plain `{ code, message }` object returned by `toApiError()`, producing `[object Object]`. All other catch sites already use the safe `instanceof Error ? err.message : String(e)` pattern.

- **Backend gap:** `save_scan_result` failure path has no `tracing` before `Err(...)` string return. DB constraint errors, persistence failures, and scan errors are invisible in production logs.

- **UX impact:** Project-open failure, scan failure, and graph-load failure all render opaque errors to the user or silently swallow errors.

- **Developer impact:** Without structured lifecycle logs, diagnosing scan regressions, DB persistence failures, or parser misses requires verbose manual logging or breakpoints.

---

## Scope

### In scope

| Area                                 | Files                                                | What changes                                                                             |
| ------------------------------------ | ---------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Frontend error normalization         | `src/lib/tauri-api.ts`, `src/App.tsx`                | Add `getErrorMessage(err: unknown): string` helper; fix catch block in App.tsx to use it |
| Frontend test scaffolding            | `src/lib/__tests__/tauri-api.test.ts` (new)          | Unit tests for `getErrorMessage`                                                         |
| Backend scan lifecycle logs          | `src-tauri/src/commands.rs`                          | Add `tracing::info!` for scan start/completion; `tracing::error!` for save failure       |
| Backend DB persistence debug logs    | `src-tauri/src/commands.rs`                          | Confirm `tracing::debug!` for import/outline persistence failures                        |
| Backend graph build logs             | `src-tauri/src/commands.rs`                          | Add `tracing::info!` for cache hit/miss; `tracing::warn!` for empty graph                |
| Backend `root_path` conflict mapping | `src-tauri/src/commands.rs`                          | Map SQLite UNIQUE constraint to user-facing message; emit `tracing::warn!`               |
| Backend log level configuration      | `src-tauri/src/lib.rs`                               | Confirm `RUST_LOG` respected; default is INFO                                            |
| Optional debug parser-miss logging   | `src-tauri/src/commands.rs` or `engine/src/scanner/` | `tracing::debug!` for unhandled syntax kinds; gated behind `RUST_LOG=debug`              |
| Backend tests                        | `src-tauri/tests/` or `engine/tests/`                | Integration tests for lifecycle and conflict logging                                     |

### Out of scope

- Phase 2 Tree-sitter TSX method-form recognition (separate SDD)
- Frontend Error subclass refactor (helper approach is Phase 1; class deferred if needed)
- Source code snippets in parser-miss logs
- Performance benchmarking
- Changes to `engine/src/scanner/code_parser.rs` (dirty worktree)
- Changes to `engine/src/scanner/parser/typescript.rs` (Phase 2)

---

## Affected Areas

### Frontend

| File                                  | Change type                                                           | Risk                                     |
| ------------------------------------- | --------------------------------------------------------------------- | ---------------------------------------- |
| `src/lib/tauri-api.ts`                | Add `getErrorMessage` helper; preserve `{ code, message }` contract   | Low — pure addition; no breaking changes |
| `src/lib/types.ts`                    | Unchanged (helper approach does not require `ApiError extends Error`) | None                                     |
| `src/App.tsx`                         | Replace `String(err)` with `getErrorMessage(err)` in catch block      | Low — one-line fix                       |
| `src/lib/__tests__/tauri-api.test.ts` | New test file                                                         | Low — isolated                           |

### Backend

| File                        | Change type                                                                                          | Risk                    |
| --------------------------- | ---------------------------------------------------------------------------------------------------- | ----------------------- | --- | ----------------------------------------------------------------------- |
| `src-tauri/src/commands.rs` | Add structured `tracing` calls on lifecycle boundaries and error paths; map SQLite constraint errors | Medium — many `map_err( | e   | e.to*string())`calls touched; must preserve`Result<*, String>` contract |
| `src-tauri/src/lib.rs`      | Confirm `RUST_LOG` configuration (likely already correct)                                            | Low                     |
| `src-tauri/tests/`          | Add integration tests for log output                                                                 | Low                     |

---

## Risk Register

| ID  | Risk                                                                                                               | Severity | Likelihood | Mitigation                                                                                             |
| --- | ------------------------------------------------------------------------------------------------------------------ | -------- | ---------- | ------------------------------------------------------------------------------------------------------ |
| R1  | Adding `DEBUG` logs in hot paths (file parsing loop) degrades scan performance                                     | Medium   | Low        | Gate behind `RUST_LOG=debug`; default INFO suppresses them                                             |
| R2  | Changing error `String` returns across `commands.rs` breaks frontend error handling                                | Medium   | Low        | Apply changes surgically; preserve `Result<_, String>` contract; add unit tests                        |
| R3  | `projects.root_path` conflict mapping requires catching SQLite errors in `save_scan_result` or command layer       | Medium   | Medium     | Add specific constraint error detection; do not swallow the error                                      |
| R4  | Frontend `getErrorMessage` helper must preserve existing `{ code, message }` shape for code-detection logic        | Low      | Low        | Helper only extracts `.message` for display; does not modify `toApiError` output                       |
| R5  | Review budget: Phase 1 estimated ~300–480 lines; auto-split recommended if exceeded                                | Low      | Medium     | Target PR 1 (frontend ~80–120 lines) and PR 2 (backend ~220–360 lines) stacked-to-main                 |
| R6  | Dirty worktree with uncommitted parser changes causes conflicts                                                    | Low      | Low        | Not modifying `engine/src/` files; only `src/` and `src-tauri/src/`                                    |
| R7  | `projects.root_path` UNIQUE conflict behavior not confirmed — re-scan may generate new project instead of conflict | Medium   | Medium     | Add structured logging to `save_scan_result` to confirm UPSERT behavior and identify the conflict path |
| R8  | Multiple catch sites silently swallow errors (analytics fetches)                                                   | Medium   | Medium     | Acceptable for non-blocking features; not a bug fix                                                    |

---

## Rollback Plan

- **PR 1 (frontend) rollback:** Revert `src/lib/tauri-api.ts` changes. Frontend returns to `[object Object]` for errors. No data migration needed.

- **PR 2 (backend) rollback:** Revert `src-tauri/src/commands.rs` and `src-tauri/src/lib.rs` changes. Backend returns to pre-logging state. No data migration needed.

- **Both PRs rollback:** No state corruption; structured logs simply stop being emitted.

- **In-flight scan interruption:** No partial state written; error returned to frontend before DB write. Safe to re-scan after rollback.

---

## Success Criteria

| ID  | Criterion                                                                                       | How verified                                                                |
| --- | ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| SC1 | Opening a project never renders `[object Object]`; it renders a readable message                | Manual test: scan on invalid path → UI shows message, not `[object Object]` |
| SC2 | If DB save fails, logs identify the failing operation and root path/project id                  | `RUST_LOG=info cargo run`, trigger failed scan → check log output           |
| SC3 | If graph load fails, UI message includes the actual backend error message                       | Manual test: load graph for empty project → UI shows readable error         |
| SC4 | Logging does not spam normal release output at info level                                       | `RUST_LOG=info cargo run` → no DEBUG logs in output                         |
| SC5 | `RUST_LOG=debug` enables parser-miss and per-failure DEBUG logs                                 | `RUST_LOG=debug cargo run` → DEBUG logs appear                              |
| SC6 | Duplicate root_path returns user-facing message (not raw SQLite error)                          | `RUST_LOG=info cargo test` → conflict test passes                           |
| SC7 | `toApiError` shape unchanged for existing `{ code, message }` consumers                         | `npm run typecheck` passes; existing code compiles                          |
| SC8 | `getErrorMessage` handles `Error`, `{ message }`, `{ code, message }`, strings, null, undefined | Unit tests pass                                                             |

---

## Validation Commands

```bash
# Frontend gates
npm run typecheck
npm run lint
npm run test -- --run src/lib/__tests__/tauri-api.test.ts

# Backend gates
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Log level verification
RUST_LOG=info cargo run -- --help 2>&1 | grep -c "DEBUG"   # should be 0
RUST_LOG=debug cargo run -- --help 2>&1 | grep "DEBUG"     # should show output

# Specific to PR 1 (frontend)
npm run test -- --run src/lib/__tests__/tauri-api.test.ts

# Specific to PR 2 (backend)
cargo test -- logging 2>/dev/null || cargo test
```

---

## Chained PR Strategy

| PR   | Focus                                                                               | Target lines | Gate                                                                             |
| ---- | ----------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------- |
| PR 1 | Frontend error normalization (`getErrorMessage` + App.tsx fix)                      | ~80–120      | `npm run typecheck && npm run test -- --run src/lib/__tests__/tauri-api.test.ts` |
| PR 2 | Backend structured logging (lifecycle + DB errors + graph logs + root_path mapping) | ~220–360     | `cargo clippy -- -D warnings && cargo test`                                      |

- **Chain strategy:** stacked-to-main
- **PR 1 must land before PR 2:** No. Both target `main` independently. Stacked-to-main allows PR 2 to build on PR 1 without depending on it in CI.
- **Task order within PR 1:** `getErrorMessage` helper → App.tsx integration → optional barrel export
- **Task order within PR 2:** Tasks 2.1–2.6 are independent; can parallelize if multiple developers available

---

## Phase Envelope

```yaml
status: proposal
executive_summary: >
  Write `getErrorMessage` helper to eliminate `[object Object]` errors in frontend.
  Add structured backend tracing around scan lifecycle, DB persistence failures, and graph
  build. Map SQLite `root_path` UNIQUE constraint to user-facing error messages. Gate
  parser-miss debug logs behind `RUST_LOG=debug`. Chained PRs: PR 1 (frontend ~80–120 lines),
  PR 2 (backend ~220–360 lines), stacked-to-main.
artifacts:
  - openspec/changes/robust-logging-observability/proposal.md   # this file
  - openspec/changes/robust-logging-observability/spec.md       # pre-existing
  - openspec/changes/robust-logging-observability/tasks.md      # pre-existing
next_recommended: apply  # TDD implementation following tasks.md task sequence
risks:
  - id: R1; description: "DEBUG logs in hot paths may degrade scan performance"; severity: medium
  - id: R2; description: "Error String returns in commands.rs may break frontend contract"; severity: medium
  - id: R3; description: "SQLite UNIQUE constraint error mapping requires surgical changes"; severity: medium
  - id: R4; description: "getErrorMessage must preserve {code, message} shape for existing consumers"; severity: low
  - id: R5; description: "Phase 1 ~300–480 lines exceeds 400-line review budget; auto-split recommended"; severity: low
skill_resolution: paths-injected
skills_loaded:
  - /home/xeworg/.config/opencode/skills/work-unit-commits/SKILL.md
  - /home/xeworg/.config/opencode/skills/chained-pr/SKILL.md
  - /home/xeworg/.config/opencode/skills/cognitive-doc-design/SKILL.md
```

---

## Key Decisions Made in Proposal

| Decision                                                                             | Rationale                                                                                                                                                           |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `getErrorMessage` helper approach over `ApiError extends Error` class                | Helper is more defensive — handles null, undefined, primitives, and non-standard shapes. `ApiError` class deferred to future SDD if type safety becomes necessary.  |
| `tracing::debug!` for per-failure DB logs, `tracing::info!` for lifecycle boundaries | Consistent with existing log convention; debug logs gated by `RUST_LOG` so INFO stays clean by default                                                              |
| `projects.root_path` conflict mapped at command layer, not repo layer                | `save_scan_result` returns `Result<_, String>`; catching SQLite constraint error in the command layer and re-mapping to user-facing string is surgical and testable |
| Optional parser-miss logging included in Phase 1 scope                               | Explicitly allowed by user intent; one `tracing::debug!` in `parse_all()` default case adds visibility without touching parser logic                                |
| Stacked-to-main chained PRs                                                          | PR 1 (frontend) and PR 2 (backend) are independent; stacking avoids CI dependency and allows incremental review                                                     |

---

## References

- **Spec:** `openspec/changes/robust-logging-observability/spec.md`
- **Tasks:** `openspec/changes/robust-logging-observability/tasks.md`
- **Explore:** `/tmp/pi-subagents-uid-1000/chain-runs/744e9225/sdd-logging/explore.md`
- **Session context:** `docs/next-session-logging-and-treesitter.md` (Phase 1)
