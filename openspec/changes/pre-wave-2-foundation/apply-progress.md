# Apply Progress — pre-wave-2-foundation PR-B-core

## Status: in progress

PR-A (#5) is **merged** to main (commits `0994db2`, `2c924a9`, `b1798cb`, `5258c6a`).
PR-B is now in active development on branch `feat/pre-wave-2-pr-b-refactor`.

## Strategy decisions (cached this session, 2026-06-09)

- **Budget**: `size:exception` granted. PR-B-core may exceed 500 lines (estimated
  550-700); no need to re-confirm.
- **Chain strategy**: split into **PR-B-core (backend)** + **PR-C (frontend
  services deletion)**. PR-C targets PR-B, only PR-B merges to main.
- **B.4 staged-review gap**: absorbed into B.10/B.11 (no retroactive shim).
- **B.2 rename**: delete the legacy `explain_node` / `chat` methods in B.10/B.11.
- **B.7 macro**: rewrite the `workspace_service!` macro to take the port
  (do NOT expand inline).
- **B.13 test reduction**: delegated to the apply sub-agent (read existing
  tests, propose 3 to keep).

## Commits on `feat/pre-wave-2-pr-b-refactor` (inherited from prior session)

| Commit     | Task | Description |
|------------|------|-------------|
| `40a99af`  | B.1  | `feat(commands): add to_ipc_error helper as single IPC error boundary` |
| `c6ae6db`  | B.2* | `feat(ai): introduce AIServicePort trait for presentation-layer consumption` |
| `8ed2b7b`  | B.3  | `refactor(ports): add from_arc constructors to 4 repository adapters` |
| `b78b7f0`  | B.4* | `feat(state): add ai_service_port to AppState for trait-object AI access` |

\* B.2 and B.4 landed the **infrastructure** (trait + field) but the
`explain_node` / `chat` commands still consume the legacy fields. The
shim refactor and legacy deletion are deferred to B.10/B.11.

## Task-by-task status

### B.1 `to_ipc_error` — **DONE**
- New `src-tauri/src/ipc_error.rs` (96 lines, 5 unit tests).
- Wired via `pub mod ipc_error;` in `src-tauri/src/lib.rs:7`.
- `#[allow(dead_code)]` is expected; B.12 will start consuming it.

### B.2 `AIServicePort` — **PARTIAL (infra done, legacy escape hatch retained)**
- Trait defined at `engine/src/ai/service.rs:54-99` with the spec signatures.
- `impl AIServicePort for AIService<R>` at `service.rs:101-142`.
- **Legacy `explain_node` / `chat` methods retained** at `service.rs:30-52`
  with docstring noting that the presentation layer migrates in B.10/B.11.
- **Action in B.10/B.11**: delete the legacy methods once the bodies move.

### B.3 `from_arc` — **DONE**
- All 4 adapters (`ScanRepositoryAdapter`, `GraphRepositoryAdapter`,
  `WorkspaceRepositoryAdapter`, `AnalysisRepositoryAdapter`) have
  `from_arc(Arc<DbPool>)`. Struct shape changed from `<'pool>` to `'static`.
- `::new(&pool)` preserved for internal tests.
- 5 new `from_arc_tests` at `engine/src/ports.rs:779-843`.

### B.4 AppState `Arc<dyn AIServicePort>` — **PARTIAL (field added, shim deferred)**
- `ai_service_port: Arc<dyn engine::ai::AIServicePort>` added at
  `src-tauri/src/commands.rs:73`.
- Constructed in `lib.rs:55-56`.
- Legacy `ai_service: engine::ai::AIService` field kept with `#[allow(dead_code)]`
  until B.9.
- **No shim refactor on `explain_node` / `chat`** — will land atomically
  with B.10/B.11.

### B.5 AppState `Arc<dyn ScanRepository>` — **PENDING** (Batch 1)
### B.6 AppState `Arc<dyn GraphRepository>` — **PENDING** (Batch 1)
### B.7 AppState `Arc<dyn WorkspaceRepository>` + macro — **PENDING** (Batch 2, HIGH)
### B.8 AppState `Arc<dyn AnalysisRepository>` — **PENDING** (Batch 1)
### B.9 Drop `pub db` + `pub ai_service` — **PENDING** (Batch 3)
### B.10 Move `explain_node` body to engine — **PENDING** (Batch 4)
### B.11 Move `chat` body to engine — **PENDING** (Batch 4)
### B.12 Atomic error contract — **PENDING** (Batch 4, HIGH)
### B.13 Delete `src/services/*.ts` (frontend) — **PENDING** (PR-C)
### B.14 PR-B-core verification — **PENDING** (Batch 5)

## Quality gate baseline (verified 2026-06-09)

| Gate | Result | Notes |
|------|--------|-------|
| `cargo fmt --check` (engine + src-tauri) | green | |
| `cargo check` (engine + src-tauri) | green | |
| `cargo test` (engine) | green (185 passed) | |
| `cargo test` (src-tauri) | green (29 passed) | includes 5 new ipc_error tests |
| `cargo clippy --lib --tests -- -D warnings` (engine) | 12 errors, **all pre-existing on main** | not introduced by PR-B |
| `cargo clippy --lib --tests -- -D warnings` (src-tauri) | 5 errors, **all pre-existing on main** | not introduced by PR-B |
| `npm run typecheck` | green | |
| `npm run lint` | green | |
| `npm run test` | green (391 tests / 215 files) | |
| `npm run check:arch` | green (no violations in `commands.rs`) | |

Clippy baseline: PR-B commits introduce **zero new** clippy regressions.

## Batch plan (PR-B-core)

| Batch | Tasks | Δ lines est. | Risk |
|-------|-------|--------------|------|
| 1 | B.5, B.6, B.8 | +120 to +150 | low (3 ports, mechanical) |
| 2 | B.7 | +60 to +80 | HIGH (13 commands + macro) |
| 3 | B.9 | −15 | medium (legacy field removal) |
| 4 | B.10, B.11, B.12 | +200 to +250 | HIGH (B.12 atomicity) |
| 5 | B.14 + open PR | small | low (verification only) |

Final diff estimate: ~700-800 lines vs spec's 500-line budget. `size:exception`
granted by user. PR-B-core will land as one PR against main; PR-C will
target it.

## Risks and follow-ups

1. **B.7 macro rewrite** is the largest single review unit in PR-B-core.
   Recommended macro signature: `($workspace_repo:expr) => { WorkspaceService::new($workspace_repo.clone()) }`.
2. **B.12 atomicity** requires `cargo test` AND `npm run test` green
   simultaneously in the same commit. Sub-agent must not commit until both
   suites pass.
3. **A.1 `pub(crate)` deferred** from PR-A: still unfulfilled. The
   introduction of `Arc<dyn ...>` in PR-B (B.5-B.8) is the enabler for
   actually doing the migration in a follow-up change — AppState will
   no longer name the port traits by their concrete paths from `commands.rs`.
4. **CI step not wired** (from PR-A): the `npm run check:arch` step must
   be added to `.github/workflows/ci.yml` manually. Suggest landing as a
   follow-up PR after PR-B-core.

## Spec traceability

| Spec requirement | Where addressed |
|------------------|-----------------|
| `IPC boundary emits structured IpcErrorPayload` | B.1 + B.12 |
| `AppState holds Arc<dyn> ports` | B.4 + B.5 + B.6 + B.7 + B.8 |
| `AIService is consumed through AIServicePort` | B.2 + B.4 + B.10 + B.11 |
| `Atomic rollout of error contract` (MODIFIED) | B.12 |
