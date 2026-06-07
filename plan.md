# Implementation Plan — Chained-PR Cut for `hexagonal-architecture-wave-1-ports`

## Goal

Deliver the completed wave-1 hexagonal migration (ports, services, composition root, frontend hooks, error contract, AI boundary) as a **Feature Branch Chain** of reviewable PRs, each ≤400 changed lines, with a tracker branch and clean diffs.

## Current Working Tree State

- **Branch:** `feat/hexagonal-architecture-migration`
- **HEAD commit:** `df08684` — contains **PR-7 (AI boundary cleanup)** already committed (two commits: `8a52919` + `df08684`)
- **Working tree:** contains uncommitted changes for **PR-1 through PR-6 and PR-8**
- **Total uncommitted work:** ~1,500–2,000 changed lines across backend ports, services, Tauri shim thinning, and frontend services/hooks

## Strategy

**Feature Branch Chain** with a draft tracker PR.

- Tracker branch stays at `main` until all children merge.
- Each child PR targets the immediate parent branch.
- PR-7 is cherry-picked from existing commits; all other PRs are carved from the uncommitted working tree via selective checkout + hunk splitting.

---

## Exact PR Order

| #   | Branch                                        | Targets                                           | Scope                                                                    |
| --- | --------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------ |
| 1   | `feat/hex-ports-wave1-pr1-error-contract`     | `feat/hexagonal-architecture-migration` (tracker) | Structured `AppError` JSON serialization + frontend `toApiError` parsing |
| 2   | `feat/hex-ports-wave1-pr2-ports-adapters`     | PR-1 branch                                       | `engine/src/ports.rs` — 4 canonical traits + 4 additive adapters         |
| 3   | `feat/hex-ports-wave1-pr3-scan-service`       | PR-2 branch                                       | `ScanService` + thin Tauri shims for scan commands                       |
| 4   | `feat/hex-ports-wave1-pr4-graph-service`      | PR-3 branch                                       | `GraphService` + thin Tauri shims for graph commands                     |
| 5   | `feat/hex-ports-wave1-pr5-workspace-service`  | PR-4 branch                                       | `WorkspaceService` + thin Tauri shims for 13 workspace commands          |
| 6   | `feat/hex-ports-wave1-pr6-analysis-service`   | PR-5 branch                                       | `AnalysisService` + thin Tauri shims for analysis commands               |
| 7   | `feat/hex-ports-wave1-pr7-ai-boundary`        | PR-6 branch                                       | AI module visibility hardening (cherry-pick existing commits)            |
| 8a  | `feat/hex-ports-wave1-pr8a-frontend-services` | PR-7 branch                                       | `src/services/*.ts` + `tauri-api.ts` bridge fixes                        |
| 8b  | `feat/hex-ports-wave1-pr8b-frontend-hooks`    | PR-8a branch                                      | Hooks, component migration, `App.tsx` cleanup, stores                    |

---

## Cut-Order Commands

Run these from the repo root to carve the chain:

```bash
# 0. SAVE CURRENT STATE
git checkout feat/hexagonal-architecture-migration

# Commit uncommitted work to a temporary branch so we can cherry-pick/checkout selectively
git checkout -b temp/wave1-staging
git add -A
git commit -m "WIP: wave-1 staging (PR-1..PR-6 + PR-8 uncommitted)"

# Preserve the original branch (has PR-7 commits)
git branch backup/feat/hexagonal-architecture-migration

# 1. RESET TRACKER TO MAIN
git checkout feat/hexagonal-architecture-migration
git reset --hard origin/main

# 2. CUT PR-1 — Error Contract
git checkout -b feat/hex-ports-wave1-pr1-error-contract feat/hexagonal-architecture-migration
# Exclusive files:
git checkout temp/wave1-staging -- engine/tests/error_contract_test.rs
# Shared file — hunk-split required (see Hunk-Splitting section):
#   engine/src/lib.rs    → keep only IpcErrorPayload + Serialize impl changes
#   src/lib/tauri-api.ts → keep only toApiError structured-parsing + code mapping + tests
git add -A && git commit -m "feat(contract): structured error payload (PR-1)"

# 3. CUT PR-2 — Ports + Adapters
git checkout -b feat/hex-ports-wave1-pr2-ports-adapters feat/hex-ports-wave1-pr1-error-contract
# Exclusive files:
git checkout temp/wave1-staging -- engine/src/ports.rs engine/tests/ports_test.rs
# Shared file — hunk-split required:
#   engine/src/lib.rs → keep only `pub mod ports;` line
#   engine/src/db/queries.rs → keep only removal of `#[cfg(test)]` from in_memory()
git add -A && git commit -m "feat(ports): canonical ports + additive adapters (PR-2)"

# 4. CUT PR-3 — ScanService
git checkout -b feat/hex-ports-wave1-pr3-scan-service feat/hex-ports-wave1-pr2-ports-adapters
# Exclusive files:
git checkout temp/wave1-staging -- engine/src/services/scan_service.rs engine/tests/scan_service_test.rs
# Shared files — hunk-split required:
#   engine/src/lib.rs          → keep only `pub mod services;` line
#   engine/src/services/mod.rs → keep only scan_service mod/use lines
#   engine/src/ports.rs        → keep only `save_outline_items` addition to ScanRepository
#   src-tauri/src/commands.rs  → keep only scan command shims (scan_project, open_project_by_path, get_scan_status)
#   src-tauri/src/lib.rs       → keep only Arc<Mutex<...>> AppState changes + ScanStatus::Idle
git add -A && git commit -m "feat(services): ScanService + thin scan shims (PR-3)"

# 5. CUT PR-4 — GraphService
git checkout -b feat/hex-ports-wave1-pr4-graph-service feat/hex-ports-wave1-pr3-scan-service
# Exclusive files:
git checkout temp/wave1-staging -- engine/src/services/graph_service.rs engine/tests/graph_service_test.rs
# Shared files — hunk-split required:
#   engine/src/services/mod.rs → keep only graph_service mod/use lines
#   src-tauri/src/commands.rs  → keep only graph command shims (get_graph, get_node_details, get_node_outline, search_nodes)
git add -A && git commit -m "feat(services): GraphService + thin graph shims (PR-4)"

# 6. CUT PR-5 — WorkspaceService
git checkout -b feat/hex-ports-wave1-pr5-workspace-service feat/hex-ports-wave1-pr4-graph-service
# Exclusive files:
git checkout temp/wave1-staging -- engine/src/services/workspace_service.rs engine/tests/workspace_service_test.rs
# Shared files — hunk-split required:
#   engine/src/lib.rs          → keep only C4View/ExecutiveSummary/SnapshotDiff re-exports
#   engine/src/services/mod.rs → keep only workspace_service mod/use lines
#   engine/src/ports.rs        → keep only WorkspaceRepository extensions (6 new methods)
#   src-tauri/src/commands.rs  → keep only workspace command shims + workspace_service! macro
git add -A && git commit -m "feat(services): WorkspaceService + thin workspace shims (PR-5)"

# 7. CUT PR-6 — AnalysisService
git checkout -b feat/hex-ports-wave1-pr6-analysis-service feat/hex-ports-wave1-pr5-workspace-service
# Exclusive files:
git checkout temp/wave1-staging -- engine/src/services/analysis_service.rs engine/tests/analysis_service_test.rs
# Shared files — hunk-split required:
#   engine/src/services/mod.rs → keep only analysis_service mod/use lines + response DTO re-exports
#   engine/src/ports.rs        → keep only AnalysisRepository trait + adapter
#   src-tauri/src/commands.rs  → keep only analysis command shims (4 commands) + DTO removal
git add -A && git commit -m "feat(services): AnalysisService + thin analysis shims (PR-6)"

# 8. CUT PR-7 — AI Boundary (already committed on original branch)
git checkout -b feat/hex-ports-wave1-pr7-ai-boundary feat/hex-ports-wave1-pr6-analysis-service
# Cherry-pick the two existing commits from backup
git cherry-pick 8a52919e55b5a34c53807a73f22172720122ab4b  # regularize AI boundary
git cherry-pick df08684a150505bd69789c64fbdc6edcef5cd0e0  # harden module visibility

# 9. CUT PR-8a — Frontend Services
git checkout -b feat/hex-ports-wave1-pr8a-frontend-services feat/hex-ports-wave1-pr7-ai-boundary
# Exclusive files:
git checkout temp/wave1-staging -- \
  src/services/projectService.ts \
  src/services/graphService.ts \
  src/services/aiService.ts \
  src/services/snapshotService.ts \
  src/services/analysisService.ts \
  src/services/__tests__/services-boundary.test.ts
# Shared file — hunk-split required:
#   src/lib/tauri-api.ts → keep only toUserMessage() addition + Tauri prefix strip
git add -A && git commit -m "feat(frontend): domain services layer + bridge fixes (PR-8a)"

# 10. CUT PR-8b — Frontend Hooks + Components
git checkout -b feat/hex-ports-wave1-pr8b-frontend-hooks feat/hex-ports-wave1-pr8a-frontend-services
# Exclusive / modified files:
git checkout temp/wave1-staging -- \
  src/hooks/useProject.ts \
  src/hooks/useArchitecture.ts \
  src/hooks/useNodeDetails.ts \
  src/hooks/useNodeOutline.ts \
  src/hooks/useAIConfig.ts \
  src/hooks/__tests__/useAI-corrective.test.ts \
  src/components/panel/AIExplanation.tsx \
  src/components/panel/DetailPanel.tsx \
  src/components/onboarding/ApiKeySetup.tsx \
  src/components/chat/ChatPanel.tsx \
  src/stores/useSnapshotStore.ts \
  src/App.tsx \
  src/hooks/useGraph.ts \
  src/hooks/useAI.ts \
  src/hooks/useExport.ts
git add -A && git commit -m "feat(frontend): hooks + component migration + App.tsx cleanup (PR-8b)"

# 11. CLEANUP
git checkout feat/hexagonal-architecture-migration  # back to tracker at main
```

---

## Scope Boundary per PR

### PR-1 — Error Contract

- **Backend:** `AppError` serializes to `{"code","message","details"}` JSON string over IPC.
- **Frontend:** `toApiError` parses structured JSON first, falls back to legacy string heuristics.
- **Tests:** 16 backend tests + 27 frontend tests.
- **Out of scope:** Any service extraction, port creation, or component changes.

### PR-2 — Ports + Additive Adapters

- **Backend:** Create `engine/src/ports.rs` with `ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AppStatePort` traits + 4 adapters.
- **Adapter rule:** All adapters delegate to existing `ProjectRepository`; NO SQL reimplementation or `queries.rs` splitting.
- **Tests:** 10 integration tests verifying trait compilation and delegation.
- **Out of scope:** Services, Tauri shims, or frontend changes.

### PR-3 — ScanService

- **Backend:** Create `ScanService<S,A>` generic over `ScanRepository` + `AppStatePort`; owns `scan_project`, `open_project_by_path`, `get_scan_status` orchestration.
- **Tauri:** Replace 3 scan command bodies with thin shims that construct `ScanService` and delegate.
- **State fix:** `AppState` fields become `Arc<Mutex<T>>` so `AppStatePortAdapter` shares real state.
- **Tests:** 10 integration tests with mock ports.
- **Out of scope:** Graph, workspace, analysis, or AI commands.

### PR-4 — GraphService

- **Backend:** Create `GraphService<G,S,A>` with `get_graph`, `get_node_details`, `get_node_outline`, `search_nodes`.
- **Tauri:** Replace 4 graph command bodies with thin shims.
- **Tests:** 10 integration tests.
- **Out of scope:** Scan, workspace, analysis commands.

### PR-5 — WorkspaceService

- **Backend:** Create `WorkspaceService<W>` generic over `WorkspaceRepository` with 13 methods + 10 response DTOs.
- **Tauri:** Replace 13 workspace command bodies with thin shims using `workspace_service!` macro.
- **Tests:** 17 integration tests.
- **Out of scope:** Analysis commands or AI boundary changes.

### PR-6 — AnalysisService + Composition Cleanup

- **Backend:** Create `AnalysisService<'pool,A,G>` with 4 analysis methods + response DTOs.
- **Tauri:** Replace 4 analysis command bodies with thin shims.
- **Goal:** `commands.rs` reduced to ~666 LOC (from 1,526 original). Remaining code: AI commands + workspace macro calls + observability stubs.
- **Tests:** 10 integration tests.
- **Out of scope:** AI commands (PR-7) or frontend work.

### PR-7 — AI Boundary Cleanup

- **Backend:** Harden `engine/src/ai/mod.rs`: remove concrete adapter re-exports, make submodules private, keep only `AIService`, `AIProviderResolver`, `AIProvider`, `ContextBuilder` public.
- **Already committed:** Cherry-pick the two existing commits.
- **Tests:** 2 boundary tests.
- **Out of scope:** No behavioral change to AI functionality.

### PR-8a — Frontend Services

- **Frontend:** Create 5 service modules (`projectService`, `graphService`, `aiService`, `snapshotService`, `analysisService`) as thin typed wrappers around `tauri-api.ts`.
- **Bridge fixes:** `toApiError` strips Tauri `"Error: "` prefix; add `toUserMessage()` with Spanish error translations.
- **Tests:** 27 service-boundary tests.
- **Out of scope:** Hook changes, component migration, App.tsx changes.

### PR-8b — Frontend Hooks + Components

- **Frontend:** Create 5 new hooks (`useProject`, `useArchitecture`, `useNodeDetails`, `useNodeOutline`, `useAIConfig`); update 3 existing hooks (`useGraph`, `useAI`, `useExport`) to consume services.
- **Components:** Migrate 4 components + `App.tsx` + `useSnapshotStore` to use hooks instead of direct `tauri-api.ts` imports.
- **Tests:** 20 corrective tests for stale-state/stale-result guards.
- **Verification:** Zero direct `tauri-api.ts` imports in `src/components/**`, `src/App.tsx`, `src/stores/**`.

---

## File Groups per PR

| PR  | Exclusive Files (checkout as-is)                                                                                                                                                                    | Shared Files (hunk-split required)                                                                                                                  |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `engine/tests/error_contract_test.rs`                                                                                                                                                               | `engine/src/lib.rs`, `src/lib/tauri-api.ts`, `src/lib/__tests__/tauri-api.test.ts`                                                                  |
| 2   | `engine/src/ports.rs` (new), `engine/tests/ports_test.rs`                                                                                                                                           | `engine/src/lib.rs`, `engine/src/db/queries.rs`                                                                                                     |
| 3   | `engine/src/services/scan_service.rs`, `engine/tests/scan_service_test.rs`                                                                                                                          | `engine/src/lib.rs`, `engine/src/services/mod.rs`, `engine/src/ports.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`                       |
| 4   | `engine/src/services/graph_service.rs`, `engine/tests/graph_service_test.rs`                                                                                                                        | `engine/src/services/mod.rs`, `src-tauri/src/commands.rs`                                                                                           |
| 5   | `engine/src/services/workspace_service.rs`, `engine/tests/workspace_service_test.rs`                                                                                                                | `engine/src/lib.rs`, `engine/src/services/mod.rs`, `engine/src/ports.rs`, `src-tauri/src/commands.rs`                                               |
| 6   | `engine/src/services/analysis_service.rs`, `engine/tests/analysis_service_test.rs`                                                                                                                  | `engine/src/services/mod.rs`, `engine/src/ports.rs`, `src-tauri/src/commands.rs`                                                                    |
| 7   | Cherry-pick existing commits (no file checkout needed)                                                                                                                                              | None — isolated to `engine/src/ai/**`                                                                                                               |
| 8a  | `src/services/*.ts`, `src/services/__tests__/services-boundary.test.ts`                                                                                                                             | `src/lib/tauri-api.ts`                                                                                                                              |
| 8b  | `src/hooks/useProject.ts`, `src/hooks/useArchitecture.ts`, `src/hooks/useNodeDetails.ts`, `src/hooks/useNodeOutline.ts`, `src/hooks/useAIConfig.ts`, `src/hooks/__tests__/useAI-corrective.test.ts` | `src/hooks/useGraph.ts`, `src/hooks/useAI.ts`, `src/hooks/useExport.ts`, `src/components/**/*.tsx`, `src/stores/useSnapshotStore.ts`, `src/App.tsx` |

---

## Hunk-Splitting Requirements in Shared Files

### `engine/src/lib.rs`

- **PR-1:** Add `IpcErrorPayload` struct + update `AppError::Serialize` impl (bottom of file, near `Serialize` impl).
- **PR-2:** Add `pub mod ports;` (near other `pub mod` lines).
- **PR-3:** Add `pub mod services;` (near other `pub mod` lines).
- **PR-5:** Add `pub use db::queries::{C4View, ExecutiveSummary, SnapshotDiff};` (near other `pub use` lines).

**Splitting method:** Each hunk is in a distinct section of the file. Use `git checkout -p temp/wave1-staging -- engine/src/lib.rs` and select only the hunks for the current PR.

### `engine/src/ports.rs`

- **PR-2:** Entire file creation (4 traits + 4 adapters).
- **PR-3:** Add `save_outline_items` to `ScanRepository` trait + `ScanRepositoryAdapter` impl.
- **PR-5:** Extend `WorkspaceRepository` with 6 new methods + `WorkspaceRepositoryAdapter` impls.
- **PR-6:** Add `AnalysisRepository` trait + `AnalysisRepositoryAdapter` impl.

**Splitting method:** The file is additive. After checking out the full file from staging, reset to the parent-branch version, then apply only the needed additions. Or: checkout the full file for PR-2, and for later PRs use `git diff parent temp/wave1-staging -- engine/src/ports.rs | patch -p1` and edit if needed.

### `engine/src/services/mod.rs`

- **PR-3:** `pub mod scan_service;` + `pub use scan_service::ScanService;`
- **PR-4:** `pub mod graph_service;` + `pub use graph_service::GraphService;`
- **PR-5:** `pub mod workspace_service;` + `pub use workspace_service::*;`
- **PR-6:** `pub mod analysis_service;` + response DTO re-exports.

**Splitting method:** Each addition is 2–4 lines at the end of the file. Easy to split manually.

### `src-tauri/src/commands.rs`

- **PR-3:** Scan command shims (replace `scan_project`, `open_project_by_path`, `get_scan_status` bodies; remove local `ScanStatus` enum; add `is_root_path_conflict`/`map_save_scan_result_error` stubs).
- **PR-4:** Graph command shims (replace `get_graph`, `get_node_details`, `get_node_outline`, `search_nodes` bodies; remove unused imports).
- **PR-5:** Workspace command shims (replace 13 command bodies; add `workspace_service!` macro; remove local DTOs).
- **PR-6:** Analysis command shims (replace 4 command bodies; remove local DTOs and inline `export_view_tests`).

**Splitting method:** Each cluster replaces a distinct set of function bodies. Use `git checkout -p temp/wave1-staging -- src-tauri/src/commands.rs` and select only the hunks for the current PR's command cluster.

### `src-tauri/src/lib.rs`

- **PR-3 only:** Change `AppState` fields to `Arc<Mutex<T>>` and initialization to `Arc::new(Mutex::new(...))` + `ScanStatus::Idle`.

**Splitting method:** Single hunk, PR-3 exclusive. No splitting needed after PR-3.

### `src/lib/tauri-api.ts`

- **PR-1:** `toApiError` structured JSON parsing + `BACKEND_TO_FRONTEND_CODE` mapping.
- **PR-8a:** `toUserMessage()` addition + Tauri `"Error: "` prefix strip.

**Splitting method:** The PR-1 changes and PR-8a changes are in different parts of the file. Use `git checkout -p` or manual edit.

---

## PR-8 Split Decision: Yes → 8a + 8b

**PR-8 must be split.** The original PR-8 scope (services + hooks + components + tests) exceeds the 400-line review budget by a significant margin (estimated 800–1,000+ changed lines).

**Split rationale:**

- **8a — Services layer:** 5 service files (~200 lines) + `tauri-api.ts` fixes (~30 lines) + service tests (~300–400 lines) = ~530–630 lines. Still borderline over 400.
- **8b — Hooks + Components:** 5 new hooks + 3 updated hooks (~300 lines) + component migrations (~150 lines) + corrective tests (~200 lines) = ~650 lines.

**If 8a still exceeds 400 lines after counting, further options:**

1. Move service tests to a separate `pr8a-tests` branch (not recommended — tests belong with the code they verify).
2. Batch services in two PRs: 8a-1 (project + graph services) and 8a-2 (AI + snapshot + analysis services).

**Recommendation:** Start with 8a/8b. If the diff count for 8a comes in >400, split 8a into two service-group PRs.

---

## Minimal Validation per PR

| PR  | Backend Validation                                                                                                        | Frontend Validation                                                                                                                                               |
| --- | ------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `cargo test --test error_contract_test` (16 pass)                                                                         | `npm run test -- src/lib/__tests__/tauri-api.test.ts` (39 pass)                                                                                                   |
| 2   | `cargo test --test ports_test` (10 pass)                                                                                  | —                                                                                                                                                                 |
| 3   | `cargo test --test scan_service_test` (10 pass) + `cargo test -p src-tauri` (31 pass)                                     | —                                                                                                                                                                 |
| 4   | `cargo test --test graph_service_test` (10 pass) + `cargo test -p src-tauri` (31 pass)                                    | —                                                                                                                                                                 |
| 5   | `cargo test --test workspace_service_test` (17 pass) + `cargo test -p src-tauri` (31 pass)                                | —                                                                                                                                                                 |
| 6   | `cargo test --test analysis_service_test` (10 pass) + `cargo test -p src-tauri` (31 pass) + verify `commands.rs` ≤666 LOC | —                                                                                                                                                                 |
| 7   | `cargo test --test ai_boundary_test` (2 pass) + `cargo test -p engine` + `cargo test -p src-tauri`                        | —                                                                                                                                                                 |
| 8a  | —                                                                                                                         | `npm run typecheck` + `npm run lint` + `npm run test -- src/services/`                                                                                            |
| 8b  | —                                                                                                                         | `npm run typecheck` + `npm run lint` + `npm run test -- src/` + grep verify zero direct `tauri-api.ts` imports in `src/components/`, `src/App.tsx`, `src/stores/` |

**Global gates (run after any PR, must stay green):**

- `cargo fmt --check` (engine + src-tauri)
- `cargo clippy -- -D warnings` (engine + src-tauri)
- `cargo test -p engine` + `cargo test -p src-tauri`
- `npm run lint` + `npm run typecheck` + `npm run test`

---

## Dependencies

```text
PR-1 (Error Contract)
  └─► PR-8a (Frontend Services) — relies on structured error payload

PR-2 (Ports)
  ├─► PR-3 (ScanService)
  ├─► PR-4 (GraphService)
  ├─► PR-5 (WorkspaceService)
  └─► PR-6 (AnalysisService)

PR-6
  └─► PR-7 (AI Boundary)

PR-7
  └─► PR-8a

PR-8a
  └─► PR-8b (Frontend Hooks)
```

**Linear chain order:** PR-1 → PR-2 → PR-3 → PR-4 → PR-5 → PR-6 → PR-7 → PR-8a → PR-8b

**Parallelizable pairs (if parent wants tree instead of chain):**

- PR-1 and PR-2 are independent (both target tracker).
- PR-8a could target PR-1 directly instead of PR-7, but linear chain simplifies GitHub targeting.

---

## Risks

| #   | Risk                                                                                                                                                                                                                | Mitigation                                                                                                                                                                                       |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | **Hunk-splitting errors in shared files** — `lib.rs`, `ports.rs`, `services/mod.rs`, `commands.rs` are touched by multiple PRs. A wrong hunk in a child PR pollutes the diff.                                       | Use `git checkout -p` interactively. After each checkout, run `git diff --cached --stat` to verify only the expected files/hunks are staged. If polluted, reset and retry.                       |
| 2   | **PR-7 already committed on original branch** — if cherry-pick fails due to context changes in prior PRs, manual resolution may be needed.                                                                          | The PR-7 commits touch only `engine/src/ai/**` files. Prior PRs do not touch AI files, so cherry-pick should apply cleanly. If not, resolve by keeping the staged AI changes.                    |
| 3   | **PR-8a or PR-8b still >400 lines** — frontend services + tests may exceed budget even when split.                                                                                                                  | Measure diff with `git diff --stat` before opening PR. If 8a >400, split into two service-group PRs (project/graph vs ai/snapshot/analysis).                                                     |
| 4   | **Tauri `AppState` mid-migration inconsistency** — PR-3 changes `AppState` to `Arc<Mutex<T>>`. PR-4–PR-6 rely on this shape. If PR-3 is not merged before PR-4, PR-4's diff will include PR-3's `AppState` changes. | Linear chain enforces PR-3 before PR-4, so this is fine. Do not parallelize PR-3/4/5/6.                                                                                                          |
| 5   | **Test file churn** — corrective repairs added many tests. If tests are split from their services, reviewers lose context.                                                                                          | Keep tests with the code they verify. If a PR exceeds 400 lines because of tests, that is acceptable if the production code is small and the tests are mechanical. Document this in the PR body. |
| 6   | **Tracker branch reset to main loses PR-7** — if the parent forgets to cherry-pick PR-7, the backend chain is incomplete.                                                                                           | PR-7 is explicitly in the ordered list. The cut commands include the cherry-pick step.                                                                                                           |
| 7   | **`commands.rs` still at 666 LOC after PR-6** — the target was ≤350 LOC. The remaining ~300 LOC are AI commands + workspace macro calls + DTOs. This is acceptable for wave-1 closure but should be noted.          | Document in PR-6 body: "commands.rs reduced from 1,526 → 666 LOC. Remaining code is AI commands (PR-7 scope) and workspace DTOs (PR-5 scope), not analysis logic."                               |
| 8   | **Frontend `getErrorMessage` still used in `useProject.ts`** — one utility from `tauri-api.ts` remains in a hook. This is not a bridge call but a string helper.                                                    | Low risk. If flagged in review, move `getErrorMessage` to a shared `utils/error.ts` in PR-8b or a follow-up.                                                                                     |

---

## PR Body Template (for each child PR)

Each PR description should include this section (append to repo template):

```markdown
## 📍 Chain Context

| Field         | Value                                          |
| ------------- | ---------------------------------------------- |
| Tracker       | `feat/hexagonal-architecture-migration`        |
| This PR       | PR-N — <Title>                                 |
| Prior PR      | PR-(N-1) — <Title>                             |
| Next PR       | PR-(N+1) — <Title> (or "none — wave complete") |
| Changed lines | <additions + deletions>                        |
| Review budget | ≤400 lines                                     |

### Out of scope

- <list 2–3 items explicitly not in this PR>

### Validation

- [ ] Backend tests pass: `<command>`
- [ ] Frontend tests pass: `<command>` (if applicable)
- [ ] No direct `tauri-api.ts` imports in components (if applicable)
```

---

## skill_resolution

`paths-injected` — Skill loaded from `/home/xeworg/.config/opencode/skills/chained-pr/SKILL.md`.
