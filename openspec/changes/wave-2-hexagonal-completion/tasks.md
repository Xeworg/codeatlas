# Wave 2 hexagonal completion — master tasks (REHYDRATED + CHECKBOXES)

> **Source**: Engram observation #679 (2026-06-10). C3a rehydrated from `#694` + `#698` + `#699` (commits `1df73bc`, `7d47cbe`, `608a847`); C3b rehydrated from `c3b/tasks.md` per Engram `#723` (2026-06-12).
> **Patched**: 2026-06-15 to clear native SDD blocker `tasks.md has no markdown task checkboxes`. C3b checkboxes mirror `c3b/tasks.md` (`#723`); C3a/C1/C2 reflect merged state.

## Review Workload Forecast

| Field                   | Value                                                 |
| ----------------------- | ----------------------------------------------------- |
| Estimated changed lines | 4250-6750 (full chain, 8 PRs)                         |
| 400-line budget risk    | Low (per-PR, within 800-line flex per preflight #673) |
| Chained PRs recommended | Yes (force-chained, orchestrator preflight)           |
| Chain strategy          | feature-branch-chain (locked since proposal #674)     |
| Delivery strategy       | force-chained (preflight)                             |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: Low

### Per-PR status

| PR  | Scope                         | Lines     | Risk | size:exception | Status                |
| --- | ----------------------------- | --------- | ---- | -------------- | --------------------- |
| M.0 | meta (planning artifacts)     | —         | —    | —              | ✅                    |
| C1  | Foundations + 3 dead commands | 250-350   | 🟢   | no             | ✅ PR #9 (`be33134`)  |
| C2  | Clock + IdGen + Stopwatch     | 450-600   | 🟡   | no             | ✅ PR #10 (`fe26f70`) |
| C3a | AI presentation extraction    | 400-550   | 🔴   | yes            | ✅ PR #12 (`a19dc32`) |
| C3b | Error-boundary cleanup        | 250-400   | 🟡   | no             | 📍 In progress        |
| C4  | Workspace port domain types   | 800-1000  | 🟡   | yes            | pending               |
| C5  | Service-level ports           | 500-700   | 🟡   | yes            | pending               |
| C6  | Cleanup + arch-guard          | 600-800   | 🟢   | no             | pending               |
| C7  | CQRS split                    | 1000-2000 | 🟡   | yes            | pending               |

## Phase 1: M.0 — Meta (planning)

- [x] 1.1 Rehydrate proposal/design/specs/tasks from Engram (`#674`, `#678`, `#675`, `#679`)

## Phase 2: C1 — Foundations (PR #9 ✅ at `be33134`)

- [x] 2.1 AD-001 rename `AnalysisRepository` → `AnalysisDataSource` (C1.1A)
- [x] 2.2 CI1: wire `npm run check:arch` in `.github/workflows/ci.yml`
- [x] 2.3 D5: rename `engine::commands` → `engine::scanner::dispatch`
- [x] 2.4 A3: adapter `new(&pool)` → `pub(crate)`; `from_arc` stays `pub`
- [x] 2.5 Implement 3 dead commands: `cancel_scan`, `get_dependencies`, `get_dependents`
- [x] 2.6 PR #9 merged at `be33134`

## Phase 3: C2 — Clock + IdGen + Stopwatch ports (PR #10 ✅ at `fe26f70`)

- [x] 3.1 D3: introduce `Clock` / `IdGenerator` port traits + adapters
- [x] 3.2 S3/S4/AI2: replace 14+ `chrono::Utc::now()` / `uuid::Uuid::new_v4()` call sites
- [x] 3.3 S6: introduce `Stopwatch` port; `SystemStopwatch`; `scan_duration_ms` via port
- [x] 3.4 Add `MockClock` / `MockIdGen` / `MockStopwatch` for golden tests
- [x] 3.5 PR #10 merged at `fe26f70`

## Phase 4: C3a — AI presentation extraction (PR #12 ✅ at `a19dc32`)

- [x] 4.1 `prepare_explain_context` + `ExplainContext` DTO
- [x] 4.2 `prepare_chat_context` + `ChatContext` DTO
- [x] 4.3 `explain_node` / `chat` shim refactor + chat double-push fix
- [x] 4.4 `ContextBuilder` → `pub(crate)`
- [x] 4.5 `AnalysisDataSource::list_files_for_project` + `FileMeta` model
- [x] 4.6 PR #12 merged at `a19dc32` (+ follow-up refactor `608a847`)

## Phase 5: C3b — Error-boundary cleanup (T4 + F2 + TE3)

- [x] 5.1 Verify `specs/error-contract/spec.md` matches locked decisions
- [x] 5.2 Cross-link to canonical `openspec/specs/error-contract/spec.md`
- [x] 5.3 RED: test `explain_node`/`chat` emit `IpcErrorPayload` `AI_UNAVAILABLE` (T4)
- [x] 5.4 RED: test `explain_node` emits `FILE_NOT_FOUND` + `details.path = node_id` (T4)
  - **Regression test**: `engine/tests/error_contract_test.rs` → `explain_node_returns_file_not_found_when_file_missing`
- [x] 5.5 GREEN: 2x `"AI not configured".to_string()` → `AppError::AIUnavailable` + `to_ipc_error` (T4)
- [x] 5.6 GREEN: `AppError::NotFound(format!("File not found: {}", node_id))` → `AppError::FileNotFound(node_id.to_string())` in `engine/src/ai/service.rs:483` (T4)
  - **Location**: After C3a thin-shim refactor the fix is in `AIServicePort::explain_node` (service layer), not `commands.rs`
  - **Validation**: `engine/tests/error_contract_test.rs::explain_node_returns_file_not_found_when_file_missing` + `src-tauri clippy -- -D warnings` clean
- [x] 5.7 REFACTOR: `rg '\.to_string\(\)' src-tauri/src/commands.rs` — only status/ID sites remain (T4)
- [x] 5.8 RED: test `toUserMessage`/`getErrorMessage` import from `src/lib/errors.ts` (F2)
- [x] 5.9 GREEN: create `src/lib/errors.ts` + re-export from `tauri-api.ts` (F2)
- [x] 5.10 GREEN: create `src/locales/es/errors.ts` with `ErrorCode → Spanish` mapping (F2)
- [x] 5.11 REFACTOR: `toUserMessage` reads from `src/locales/es/errors.ts`; drop inline literals (F2)
- [x] 5.12 RED: integration test `to_ipc_error` → `toApiError` → `toUserMessage` (TE3 roundtrip)
- [x] 5.13 GREEN: run full suite, fix contract drift
- [x] 5.14 DOCS: finalize spec delta (3 ADDED requirements + cross-refs)
- [x] 5.15 `cargo test` is green in `engine` + `src-tauri`; `src-tauri` `cargo fmt --check` + `clippy -- -D warnings` are clean after the C3b follow-up fix; `engine` still has unrelated pre-existing clippy debt outside C3b scope
- [x] 5.16 `npm run lint && typecheck && test && check:arch` clean
- [x] 5.17 `src-tauri/src/commands.rs` has no raw string error returns; remaining `AI not configured` text appears only inside `AppError::AIUnavailable(...)` constructors, which is the locked C3b shape
- [ ] 5.18 Open PR `feat/wave-2-c3b-error-boundary` (base `608a847`, chain-context block)

## Phase 6: Follow-up — Judgment Day Round 2

- [x] 6.1 Re-validated as stale: `src-tauri/src/commands.rs` already exposes thin-shim `explain_node` / `chat` handlers, while business-context assembly now lives in `engine/src/ai/service.rs` (`AIServicePort::explain_node` / `chat`). The old Judgment Day Round 2 follow-up no longer blocks C3b PR merge.

## Phase 7: C4 — Workspace port domain types

- [ ] 7.1 D1: 9 tuple-typed `WorkspaceRepository` methods → domain types (`WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`)
- [ ] 7.2 D2: move `ExecutiveSummary` / `SnapshotDiff` / `C4View` from `db::queries` to `engine::models`
- [ ] 7.3 PR (size:exception 800-1000L)

## Phase 8: C5 — Service-level ports

- [ ] 8.1 S1: `GraphService::get_node_outline` → `FileSourceReader` port
- [ ] 8.2 S2: `FileWalker` / `ParserRegistry` → port-injected or `engine::scanner` namespace
- [ ] 8.3 S5: `AnalysisService` unit tests (0 currently)
- [ ] 8.4 P1: real `AnalysisDataSource` port; `AnalysisRepository::pool()` removed
- [ ] 8.5 AN2: `serde_json::Value` evidence → typed `ArchitectureEvidence` / `Hotspot`
- [ ] 8.6 T2: `get_scan_status` enum→string → `impl Display` / `impl Serialize`

## Phase 9: C6 — Cleanup + arch-guard + docs

- [ ] 9.1 CI2: 8 strict regex patterns in `scripts/ci/check-architecture.mjs` + 4 self-test fixtures
- [ ] 9.2 F1: `scripts/check-error-codes.mjs` sync-check `BACKEND_TO_FRONTEND_CODE` ↔ `engine::lib.rs:74-80`
- [ ] 9.3 TE1-3: `AnalysisService` unit tests, deterministic AI tests, backend→frontend roundtrip
- [ ] 9.4 DOC1-3: spec updates, `docs/architecture.md`, change-folder alias
- [ ] 9.5 CI3: `cargo-llvm-cov` coverage gate
- [ ] 9.6 CI4: pay 17-error clippy baseline debt

## Phase 10: C7 — CQRS split of ProjectRepository

- [ ] 10.1 A1: split `engine/src/db/queries.rs` (2419 lines) into `engine/src/db/queries/{mod,commands,queries}.rs`
- [ ] 10.2 AD-004: `QueryRepositoryAdapter` + `CommandRepositoryAdapter` (`Arc<DbPool>` cloned)
- [ ] 10.3 4 wave-1 traits persist as thin façades over CQRS
- [ ] 10.4 PR (size:exception 1000-2000L, depends on C4 + C6)

## Blockers

- **B1 (Resolved)**: The current working tree is on `main` at `a19dc32` with the post-C3a thin-shim tree available locally. A dedicated feature branch is still needed before PR, but the old `fe26f70` base-branch blocker is obsolete.
- **B2 (Resolved)**: Re-validation shows `src-tauri/src/commands.rs` no longer assembles business context for AI flows. `engine/src/ai/service.rs` owns `AIServicePort::explain_node` / `chat`, which fetch project/file/graph state and build the contexts.

## Deferred to wave 3 (AD-009)

- C1.1: `pub(crate)` migration of 5 port traits (no cross-crate boundary in codeatlas; `engine` and `src-tauri` are independent crates).

## Artifacts

- `openspec/changes/wave-2-hexagonal-completion/tasks.md` (this file, master) — ESTE ARCHIVO
- Engram topic_key `sdd/wave-2-hexagonal-completion/tasks` (observation `#679`, source)
- Per-PR detail: `c3a/tasks.md`, `c3b/tasks.md`, `c3b/exploration.md`, `specs/error-contract/spec.md`, `state.yaml`

## Next step

Abrir `feat/wave-2-c3b-error-boundary` desde la base encadenada de C3a, preparar el chain-context block (PR-12 → este PR), y dejar explícito que `engine` mantiene deuda previa de clippy fuera del scope de C3b. Después de eso, seguir con C4.
