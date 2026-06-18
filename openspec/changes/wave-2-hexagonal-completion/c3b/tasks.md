# Wave 2 C3b — Tasks: Error-boundary cleanup

> **Change**: `wave-2-hexagonal-completion` · **Sub-scope**: `c3b/` · **Strict TDD**: RED → GREEN → REFACTOR
> **Locked**: existing `AppError`/`ErrorCode`/`ApiError` only; "AI not configured" → `AppError::AIUnavailable`; "File not found" → `AppError::FileNotFound`; helper `src/lib/errors.ts`; strings `src/locales/es/errors.ts`; spec delta at `specs/error-contract/spec.md`.

## Review Workload Forecast

| Field                   | Value                                                                                |
| ----------------------- | ------------------------------------------------------------------------------------ |
| Estimated changed lines | 250-400 (design AD-008)                                                              |
| 400-line budget risk    | Low (within 800-line flex, preflight #673)                                           |
| Chained PRs recommended | No — single PR, 3 logical commits                                                    |
| Delivery strategy       | `exception-ok` (within flex; pre-authorized)                                         |
| Chain strategy          | `feature-branch-chain` (base = `origin/feat/wave-2-c3a-ai-context-prep` @ `608a847`) |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: feature-branch-chain
400-line budget risk: Low

### Work Units (commits)

WU-1 T4 backend · WU-2 F2 frontend · WU-3 spec + TE3 roundtrip

## Phase 1: Spec foundation

- [x] 1.1 Verify `specs/error-contract/spec.md` matches locked decisions
- [x] 1.2 Cross-link to canonical `openspec/specs/error-contract/spec.md`

## Phase 2: Backend T4 (commit 1/3)

- [x] 2.1 RED: test — `explain_node`/`chat` emit `IpcErrorPayload` `AI_UNAVAILABLE` when `ai_config` is `None`
- [x] 2.2 RED: test — `explain_node` emits `FILE_NOT_FOUND` + `details.path = node_id` on missing file
  - **Implementation**: `engine/tests/error_contract_test.rs` → `explain_node_returns_file_not_found_when_file_missing` (proves `AIServicePort::explain_node` returns `AppError::FileNotFound` for missing file; would FAIL if old `AppError::NotFound` code remained)
- [x] 2.3 GREEN: replace `commands.rs:357,415` `"AI not configured".to_string()` → `AppError::AIUnavailable("AI not configured".to_string())` + `to_ipc_error`
- [x] 2.4 GREEN: replace `engine/src/ai/service.rs:483` `AppError::NotFound(format!("File not found: {}", node_id))` → `AppError::FileNotFound(node_id.to_string())`
  - **Location update**: After C3a thin-shim refactor, `commands.rs` delegates to `AIServicePort::explain_node` (in `engine/src/ai/service.rs`), so the fix lives at the service layer not the presentation layer.
  - **Validation**: `src-tauri/src/commands/tests/shim_tests.rs` → `explain_node_file_not_found_payload_when_node_missing` (IPC contract); `engine/tests/error_contract_test.rs` → `explain_node_returns_file_not_found_when_file_missing` (service-layer regression)
- [x] 2.5 REFACTOR: `rg '\.to_string\(\)' src-tauri/src/commands.rs` — only status/ID sites remain

## Phase 3: Frontend F2 (commit 2/3)

- [x] 3.1 RED: test in `src/lib/__tests__/errors.test.ts` — `toUserMessage`/`getErrorMessage` import from `src/lib/errors.ts`
- [x] 3.2 GREEN: create `src/lib/errors.ts` (move 2 functions from `tauri-api.ts:134-174`)
- [x] 3.3 GREEN: re-export from `tauri-api.ts` for back-compat
- [x] 3.4 GREEN: create `src/locales/es/errors.ts` with `ErrorCode → Spanish message` mapping
- [x] 3.5 REFACTOR: `toUserMessage` reads from `src/locales/es/errors.ts`; drop inline literals

## Phase 4: TE3 + docs (commit 3/3)

- [x] 4.1 RED: integration test — `to_ipc_error` → `toApiError` → `toUserMessage`; assert Spanish match for `AI_UNAVAILABLE` + `FILE_NOT_FOUND`
- [x] 4.2 GREEN: run full suite, fix contract drift
- [x] 4.3 DOCS: finalize spec delta (3 ADDED requirements, cross-refs)

## Phase 5: Verification

- [x] 5.1 `cargo test` is green in `engine` + `src-tauri`; `src-tauri` `cargo fmt --check` + `clippy -- -D warnings` are clean after the C3b follow-up fix; `engine` still has unrelated pre-existing clippy debt outside C3b scope
- [x] 5.2 `npm run lint && typecheck && test` — clean; `npm run check:arch` — no violations
- [x] 5.3 `src-tauri/src/commands.rs` has no raw string error returns; remaining `AI not configured` text appears only inside `AppError::AIUnavailable(...)` constructors, which is the locked C3b shape
- [ ] 5.4 Open PR with chain-context block (PR-12 → this PR)

## Blockers

- **B1 (Resolved)**: The working tree is now on `main` at `a19dc32` with the post-C3a thin-shim tree present locally. A dedicated feature branch is still needed before PR, but the old base-branch blocker from `fe26f70` is no longer current.
- **B2 (Resolved)**: `AppError::FileNotFound` + `AIUnavailable` exist (`engine/src/lib.rs:33, 48`); no new variants.
- **B3 (Resolved)**: Frontend imports resolve with `src/lib/errors.ts` in tests (`src/lib/__tests__/tauri-api-bridge.test.ts`), so no alias/path blocker remains for C3b.
- **B4 (Locked)**: Strict TDD active (`config.yaml sdd.strict_tdd: true`); Phase 2-4 follow RED-first.

## Artifacts

- `c3b/tasks.md` (this file)
- `specs/error-contract/spec.md`
- Engram `sdd/wave-2-hexagonal-completion/c3b/tasks`

## Next step

Open `feat/wave-2-c3b-error-boundary` from the C3a chain base, prepare the chain-context PR block (PR-12 → this PR), and carry forward the residual note that `engine` still has unrelated pre-existing clippy debt outside C3b scope.
