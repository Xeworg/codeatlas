# Wave 2 hexagonal completion — proposal

## Intent

The wave-1 hexagonal migration (commit 503ad80) built a **thin hexagonal shell that does not hold under stress** (observation #654). The architecture looks correct in steady state — 5 `Arc<dyn Port>` in AppState, dual-language arch guard, IpcErrorPayload live in 38 commands — but the boundary is leaky: port traits are still `pub` (no encapsulation), 9 workspace port methods return DB-shaped tuples, 14+ call sites use `chrono::Utc::now()` and `uuid::Uuid::new_v4()` directly, `GraphService::get_node_outline` reads the filesystem with `std::fs::read_to_string` bypassing the port, the analysis module takes `&DbPool` directly via a back-door `pool()` method, and `ProjectRepository` is now a 2419-line god object that every adapter wraps. Wave 1 review marked 25+ items; pre-wave-2-foundation closed the structural worst; Wave 2 closes the rest.

**Wave 2 exists to harden the boundary before wave 3 lands.** Wave 3 will split `engine` into `codeatlas-domain` / `codeatlas-application` / `codeatlas-infrastructure` crates — that work is impossible if the current single-crate seams are still leaky. Wave 2 also enables the post-Wave-2 AI rewrite: the user said explicitly *"el punto 6 de la ia se reescribirá luego de la arquitectura hexagonal por ahora lo llevaremos como va mantendremos lo que tiene"* — meaning AI is deliberately deferred, but Wave 2's ports (Clock, IdGenerator, Stopwatch, AnalysisDataSource, domain-typed workspace port) are the precise substrate the AI rewrite will plug into. Wave 2 makes that rewrite safe.

The 7-PR chain in this proposal extends the 6-PR chain from the explore (observation #672) with the user's 6 preflight decisions: CQRS in Wave 2 (not wave 3), 3 dead backend commands implemented, `pub(crate)` migration in C1, Stopwatch port included, strict arch-guard purity, AI deferred. One new PR (C7) is added for the CQRS split that the explore had descoped.

## Scope

### In Scope

**C1 — Foundations + visibility + dead commands** (size: ok, ~250-350 lines, 🟢)
- D4: `pub(crate)` migration of 5 port traits (`ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AnalysisRepository`, `AIServicePort`) — now viable with `Arc<dyn Port>` in AppState since PR-B
- 3 dead backend commands implemented (user decision): `cancel_scan`, `get_dependencies`, `get_dependents` registered in `lib.rs:80-109`; `tauri-api.ts:204, 250, 263` currently always fail at runtime (grep confirmed 0 hits in `src-tauri/src/`)
- CI1: `npm run check:arch` wired in `.github/workflows/ci.yml` lint-and-typecheck job (was apply-progress pending item, observation #661)
- D5: `engine::commands` rename to `engine::scanner::dispatch` (generic name anti-pattern, #654 #21)
- A3: `new(&pool)` → `pub(crate)` on 4 adapters; only `from_arc` stays `pub`
- F3: inline `{id, role, content, timestamp}[]` → `ChatMessage[]` in `tauri-api.ts:302-318` (drift fix)
- T3: `configure_ai` / `get_ai_config` routed via `AppStatePort` (no more direct `state.ai_config.lock()`)

**C2 — Clock + IdGenerator + Stopwatch ports** (size: ok, ~450-600 lines, 🟡)
- D3: introduce `Clock` and `IdGenerator` port traits; `SystemClock` and `RandomIdGen` adapters
- S3, S4, AI2: replace 14+ `chrono::Utc::now()` and `uuid::Uuid::new_v4()` call sites in services
- S6: introduce `Stopwatch` port (user decision — included not deferred); `SystemStopwatch` adapter; `ScanService::scan_duration_ms` reads via port
- Deterministic test infra: `MockClock`, `MockIdGen`, `MockStopwatch` for golden tests (TE2 enabler)

**C3 — AI presentation extraction** (size:exception, ~700-900 lines, 🔴)
- T1: extract 160 lines of business logic from `src-tauri/src/commands.rs::explain_node` (264-311) and `chat` (320-392) into `AIService::prepare_explain_context` and `prepare_chat_context` use cases
- T4: 3 string literals in commands.rs:271, 278, 329 → `AppError::AiNotConfigured` / `AppError::FileNotFound` variants (constraint that justified leaving them as `String` is wrong — 5 lines, no cascade risk, per observation #671)
- AI1: `ContextBuilder` (`engine/src/ai/context.rs:9`) → `engine::ai::internal::ContextBuilder` (`pub(crate)`)
- F2: `toUserMessage` / `getErrorMessage` extracted from `tauri-api.ts:134-174` to new `src/lib/errors.ts` and `src/locales/es/errors.ts`

**C4 — Domain types in workspace port** (size:exception, ~800-1000 lines, 🟡)
- D1: 9 tuple-typed `WorkspaceRepository` methods (ports.rs:275, 278, 284, 287-300, 302-316, 318-333, 335-345, 347-353, 355-362, 526-549) → domain types (`WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`); service-side mapping deleted
- D2: `ExecutiveSummary`, `SnapshotDiff`, `C4View` structs moved from `db::queries` to `engine::models`; ports return domain types not infra types

**C5 — Service-level ports** (size:exception, ~500-700 lines, 🟡)
- S1: `GraphService::get_node_outline` (`graph_service.rs:202`) → `FileSourceReader` port (no more `std::fs::read_to_string` in service layer)
- S2: `FileWalker`, `ParserRegistry` instantiation in `ScanService` (`scan_service.rs:102, 108`) and `GraphService` (`graph_service.rs:210`) → port-injected or moved to `engine::scanner` public namespace (acceptable compromise)
- S5: `AnalysisService` unit tests (0 currently) — 5-10 unit tests complementing the 10 integration tests in `engine/tests/analysis_service_test.rs`
- P1: real `AnalysisDataSource` port that returns neutral data (`Vec<FileMeta>`, `Vec<ImportEdge>`) — `analysis/*` (`architecture_detector.rs:8`, `impact_engine.rs:6`, `graph_insights.rs:5`) no longer takes `&DbPool`; `AnalysisRepository::pool()` (ports.rs:565) removed
- AN2: `serde_json::Value` evidence (`analysis_service.rs:38, 56, 99, 117, 122`) → typed `ArchitectureEvidence` and `Hotspot` with `Serialize` derive
- T2: `get_scan_status` enum→string mapping (`commands.rs:129-135`) moved to `impl Display` / `impl Serialize` on `ScanStatus`

**C6 — Cleanup, tests, docs, strict arch-guard** (size: ok, ~600-800 lines, 🟢)
- CI2 strict patterns (user decision — tight enforcement, no exceptions) in `scripts/ci/check-architecture.mjs`:
  - `std::fs::` in `src-tauri/src/commands.rs` (presentation forbidden)
  - `use engine::analysis::` in `src-tauri/src/commands.rs` (presentation forbidden)
  - `chrono::Utc::now()` / `uuid::Uuid::new_v4()` in `engine/src/services/` (services use ports)
  - `engine::db::` in `src-tauri/src/commands.rs` (presentation can only use ports)
  - `engine::ai::anthropic::` / `engine::ai::resolved::` in presentation (AnthropicProvider, ResolvedProvider stay `pub(crate)`)
- F1: `scripts/check-error-codes.mjs` sync-check between `BACKEND_TO_FRONTEND_CODE` (`tauri-api.ts:35-46`) and `engine::lib.rs:74-80`
- TE1-3: `AnalysisService` unit tests, deterministic AI tests with `tokio::time::pause` + `Uuid::mock`, backend→frontend error roundtrip integration test
- DOC1: update `openspec/specs/backend-ports-and-services/spec.md` (5th port + new ports) and `error-contract/spec.md` (3 string literals closed)
- DOC2: new `docs/architecture.md` current-state tutorial
- DOC3: rename `hexagonal-architecture-wave-1-ports` change folder alias
- CI3: `cargo-llvm-cov` coverage gate (TE1/AN3/S5 coverage visible)
- CI4: pay 17-error clippy baseline debt (1 PR cleanup, prerequisite if Wave 2 wants `cargo clippy -- -D warnings` gate)

**C7 — CQRS split of ProjectRepository** (size:exception, ~1000-2000 lines, 🟡)
- A1: split `engine/src/db/queries.rs` (2419 lines) into `engine/src/db/queries/` directory:
  - `mod.rs` — `QueryRepository` and `CommandRepository` traits
  - `commands.rs` — write-side queries
  - `queries.rs` — read-side queries
  - 5 adapter structs now wrap smaller interfaces
- Justified in Wave 2 not Wave 3 (user decision): boundary must be fully sealed before any further work, including the AI rewrite
- Depends on C4 (domain types must be in place) and C6 (arch-guard strict patterns must enforce no `&DbPool` leak into new CQRS module)

### Out of Scope

- **AI refactor** — `AIServicePort` + `ChatMessage` + `AnthropicProvider` + `ResolvedProvider` stay as-is per user explicit decision
- **New features** — no UX additions, no new commands beyond the 3 dead ones being implemented, no new ports beyond what the 30 items require
- **Performance work** — no benchmarking, no query optimization
- **Multi-tenant / cloud sync** — single user, single project set, no remote backend
- **Multi-crate split** — `engine` stays one crate in Wave 2; the `codeatlas-domain` / `-application` / `-infrastructure` split is Wave 3
- **`pub(crate)` on adapter structs** (only the 5 traits migrate in C1; defer adapter tightening to follow-up if needed)

## Approach

### 7-PR chain (feature-branch-chain strategy)

| PR | Scope summary | Lines | Risk | size:exception | Depends on |
|----|---------------|-------|------|----------------|------------|
| C1 | Foundations + pub(crate) + 3 dead commands | 250-350 | 🟢 | no | — |
| C2 | Clock + IdGen + Stopwatch ports | 450-600 | 🟡 | no | C1 |
| C3 | AI presentation extraction + analysis port | 700-900 | 🔴 | **yes** | C2 |
| C4 | Workspace port domain types | 800-1000 | 🟡 | **yes** | C1 |
| C5 | Service-level ports + unit tests | 500-700 | 🟡 | **yes** | C2, C3 |
| C6 | Cleanup + tests + docs + strict arch-guard | 600-800 | 🟢 | no | all prior |
| C7 | CQRS split of ProjectRepository | 1000-2000 | 🟡 | **yes** | C4, C6 |

**Total: 7 PRs, ~4300-6350 lines, 10-15 days of focused work.** Review budget per PR: 800 lines with flex (D3 — user explicit authorization, observation #673). C3, C4, C5, C7 are expected to need `size:exception` (each is a coherent refactor that does not fragment cleanly).

**Chain strategy: feature-branch-chain** (each PR merges to main before the next starts). The user is the sole reviewer; stacked-to-main would block 6 reviews before the first PR merges, blocking learning. Intermediate risks (C2 exposes bugs latent under deterministic time; C3 can break frontend tests) are discoverable per-PR. Already established by `chained_pr_strategy: auto-forecast` and the precedent of pre-wave-2-foundation (PR #5, #6, #7, #8 all chained, observations #663, #667, #669, #671).

### Sequencing rationale

1. **C1 first** — establishes visibility boundary, registers 3 dead commands, wires arch guard into CI. Everything else compiles in a known-good state. D4 (pub(crate)) is now viable because AppState uses `Arc<dyn ...>` since PR-B enabled this.
2. **C2 before C3** — Clock/IdGen ports are the substrate for T1 (extracted `explain_node` / `chat` upstream builds `ChatMessage` with proper ID/timestamp). C3 cannot land without C2.
3. **C3 before C5** — S5 (AnalysisService unit tests) needs ports that C3's analysis port work defines; P1 (AnalysisDataSource) lives in C3 per the explore's DAG.
4. **C4 independent of C2/C3** — domain types in workspace port touch only `ports.rs`, `services/workspace_service.rs`, response DTOs. Pure refactor.
5. **C6 after all** — strict arch-guard patterns depend on every leak being closed. Putting C6 last lets every pattern be tested against a known-clean tree.
6. **C7 last** — CQRS split depends on C4 (domain types) and C6 (arch-guard patterns catching accidental `&DbPool` leaks in the new module structure). Putting C7 after C6 also avoids merge conflicts on `queries.rs`.

## Architecture decisions

### Resolved (with one-line justification)

| Decision | Justification |
|----------|---------------|
| **CQRS in Wave 2 (not wave 3)** | User decision; boundary must be fully sealed before any further work including the post-Wave-2 AI rewrite |
| **3 dead backend commands implemented (`cancel_scan`, `get_dependencies`, `get_dependents`)** | User decision; they are in `tauri-api.ts:204, 250, 263` but never registered in `lib.rs:80-109` — they always fail at runtime; no consumer in components, so implementing restores the contract without UI churn |
| **`pub(crate)` migration of 5 traits in C1** | User decision; viable now that AppState uses `Arc<dyn ...>` (PR-B enabled this) and integration tests don't reach into concrete trait paths |
| **`Stopwatch` port in C2** | User decision; included not deferred, even though S6 was L-confidence in explore. Clean win for `scan_duration_ms` and future bench infra |
| **Strict arch-guard purity in C6** | User decision; CI2 patterns must catch ALL layer violations — `std::fs::` in presentation, `use engine::analysis::` in presentation, `chrono::Utc::now()` / `uuid::Uuid::new_v4()` in services, etc. No exceptions |
| **AI deferred** | User decision; "lo llevaremos como va mantendremos lo que tiene". `AIServicePort` + `ChatMessage` stay as-is. AI rewrite is post-Wave-2 |

### Deferred (with what triggers reopening)

| Decision | Triggers reopening |
|----------|-------------------|
| **AI port/test design** | Post-Wave-2 — when wave 3 starts the AI rewrite, it will use Clock/IdGen/Stopwatch/AnalysisDataSource ports as substrate |
| **Multi-crate split (engine → domain/application/infrastructure crates)** | Wave 3 — when `engine` grows past 5000 LOC OR when a second adapter (HTTP, event bus) needs a real compilation barrier |
| **Provider enum → `Box<dyn AIProvider>`** | Wave 3+ AI rewrite — when OpenAI/Ollama variants are added |
| **`pub(crate)` on adapter structs (not just traits)** | If integration tests still reach into adapter concrete types after C1 |
| **`from_arc_refs` → `From<&AppState>`** | Cosmetic; P3 in explore, defer until C6 cleanup |
| **Crate-internal `pub(crate) mod db`, `pub(crate) mod commands`** | Wave 3 — when the multi-crate split forces real encapsulation |

## Risks

| # | Risk | Likelihood | Mitigation |
|---|------|------------|------------|
| **R1** | Architecture drift during chain — devs reintroduce `chrono::Utc::now()` in services between C2 and C6 | Med | CI2 strict patterns in C6 catch it; PR review per slice flags it |
| **R2** | C3 scope creep — T1 extraction tempts "while we're here" AI refactor | High | T1 has strict scope in C3; AI work is explicitly out of scope per user decision; PR review enforces; if scope creeps, split into C3a (T1) and C3b (T4+AI1) |
| **R3** | Deterministic tests break fixtures (Clock mocks, Uuid::mock with counter) | Med | Golden tests before/after C2; integration test runs as gating step |
| **R4** | CQRS scope creep into C7 — 2419 → 2× N lines, with "while we're here" refactor | High | C7 scoped strictly to commands/queries split; refactors like adding a `Repository` enum are out; user explicit that C7 is Wave 2 not wave 3 means scope discipline is critical |
| **R5** | C7 size estimation 1000-2000 lines is optimistic (could be 3000+) | Med | C7 isolated as last PR; if it overflows, split into C7a (commands) and C7b (queries) — do not fragment the coherent refactor unless forced |
| **R6** | CI2 strict patterns in C6 block unrelated work — false positives | Med | Each pattern has a self-test fixture; CI runs in local before push; pattern additions reviewed per PR |
| **R7** | Clippy baseline (17 errors pre-existing) blocks `cargo clippy -- -D warnings` as gate | Med | CI4 in C6 pays the debt; if it doesn't fit, C6 splits into C6a (clippy) and C6b (cleanup) |
| **R8** | Frontend `cancelScan`/`getDependencies`/`getDependents` was wanted-feature not dead code | Low | Already confirmed dead (grep 0 hits in `src-tauri/src/`); tauri-api.ts:204, 250, 263 have no consumer; implementing them in C1 restores the contract |
| **R9** | `pub(crate)` migration in C1 breaks an integration test that imports the trait | Low-Med | Integration tests are 2896 lines in 10 files; PR-B did not change integration test patterns so they don't reach into the trait pub path; C1 explicitly runs `cargo test --tests` as a gating step (lesson from W2-001 hotfix, observation #671) |

## Success Criteria

Wave 2 is complete when **all** of the following are measurable on `main`:

- [ ] **CI2 strict patterns enforced** — `npm run check:arch` wired in `.github/workflows/ci.yml` (CI1) AND all new patterns in C6 pass self-test: no `std::fs::` in `src-tauri/src/commands.rs`, no `use engine::analysis::` in presentation, no `chrono::Utc::now()` / `uuid::Uuid::new_v4()` in `engine/src/services/`, no `engine::db::` in `src-tauri/src/commands.rs`
- [ ] **Zero `pub` on 5 traits** — `ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AnalysisRepository`, `AIServicePort` are `pub(crate)` (D4); integration tests pass
- [ ] **Clock + IdGen + Stopwatch ports used everywhere** — 0 `chrono::Utc::now()` in `engine/src/services/*`, 0 `uuid::Uuid::new_v4()` in `engine/src/services/*`; `MockClock` / `MockIdGen` / `MockStopwatch` enable deterministic tests
- [ ] **9 workspace port signatures return domain types** — `WorkspaceRepository` methods return `WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`; service-side tuple→struct mapping deleted (D1 + D2)
- [ ] **`AnalysisRepository::pool()` removed** — `analysis/*` no longer takes `&DbPool`; `AnalysisDataSource` port is the only path to analysis data (P1)
- [ ] **`GraphService::get_node_outline` reads via a port** — no `std::fs::read_to_string` in `engine/src/services/*`; new `FileSourceReader` port injected (S1)
- [ ] **3 dead commands implemented** — `cancel_scan`, `get_dependencies`, `get_dependents` registered in `src-tauri/src/lib.rs:80-109` with tauri commands and integration tests
- [ ] **`ProjectRepository` split into Command/Query halves** — `engine/src/db/queries/` directory with `mod.rs`, `commands.rs`, `queries.rs`; 5 adapters wrap smaller interfaces; CQRS trait separation enforced (A1, user decision)
- [ ] **All tests green: 675+ baseline + new tests** — 675 baseline (279 engine + 29 src-tauri + 367 frontend) + tests for new ports (Clock/IdGen/Stopwatch mocks, AnalysisDataSource, FileSourceReader) + `AnalysisService` unit tests + deterministic AI tests + backend→frontend error roundtrip
- [ ] **No `pub(crate)` violations** — `cargo doc --no-deps --document-private-items` and integration test compilation both clean
- [ ] **AI surface unchanged** — `AIServicePort` + `ChatMessage` + `AnthropicProvider` + `ResolvedProvider` byte-identical at the API level; no AI tests added or removed
- [ ] **Wave 3 readiness** — `engine` compiles, all ports are coherent, CQRS split in place. A wave-3 `codeatlas-domain` / `-application` / `-infrastructure` split is mechanically possible without boundary rewrites

## Open questions for the user

**None — all 6 questions resolved in preflight.** The user's preflight (observation #673) explicitly closed:
1. CQRS in Wave 2 ✅
2. 3 dead backend commands implemented ✅
3. `pub(crate)` migration in C1 ✅
4. Stopwatch port included ✅
5. Strict arch-guard purity ✅
6. AI deferred ✅

If the user wants to re-open any of these during the proposal review, the orchestrator should reflect that before delegating to `sdd-spec`.

---

## Capabilities (contract with sdd-spec)

### New Capabilities

- `hexagonal-ports`: Ports for `Clock`, `IdGenerator`, `Stopwatch`, `AnalysisDataSource`, `FileSourceReader`, and `pub(crate)` migration of the 5 existing port traits. Each port has a system adapter and a mock adapter for tests. Backing specs: `ports.rs` trait definitions, `SystemClock` / `RandomIdGen` / `SystemStopwatch` / `AnalysisDataSourceAdapter` / `FileSourceReaderAdapter` impls.
- `cqrs-repository`: CQRS split of `ProjectRepository` into `CommandRepository` (writes) and `QueryRepository` (reads). Each trait has a focused method set; 5 adapter structs wrap the smaller interfaces. Backing specs: `queries/mod.rs`, `queries/commands.rs`, `queries/queries.rs` split, adapter rewiring.
- `command-bridge`: Implementation of `cancel_scan`, `get_dependencies`, `get_dependents` in `src-tauri/src/lib.rs:80-109` tauri command registry. Each command has integration tests covering happy path, error path, and cancellation/empty-state edge cases.

### Modified Capabilities

- `backend-ports-and-services`: Adding 5 new ports (`Clock`, `IdGenerator`, `Stopwatch`, `AnalysisDataSource`, `FileSourceReader`) and changing 9 `WorkspaceRepository` method signatures from tuples to domain types (`WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`). Existing capabilities retain intent; only signatures and adapters change.
- `error-contract`: Closing the 3 remaining string-literal errors in `src-tauri/src/commands.rs:271, 278, 329` — they become `AppError::AiNotConfigured` and `AppError::FileNotFound` variants. The `to_ipc_error` mapping now covers 100% of command returns, not 38/41.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `engine/src/ports.rs` (1069 → ~1500) | Modified | `pub(crate)` migration, 5 new port traits, tuple→domain type signatures, `pool()` removal |
| `engine/src/db/queries.rs` (2419 → split) | Reorganized | Split into `engine/src/db/queries/{mod,commands,queries}.rs` for CQRS (C7) |
| `engine/src/services/{scan,graph,analysis,workspace}_service.rs` | Modified | Clock/IdGen/Stopwatch injection, port-based file/source access, unit tests |
| `engine/src/ai/{service,anthropic,context}.rs` | Modified | Clock/IdGen injection, ContextBuilder → `pub(crate)`, upstream extraction of `explain_node`/`chat` business logic |
| `engine/src/analysis/{architecture_detector,impact_engine,graph_insights}.rs` | Modified | No more `use crate::db::DbPool`; `AnalysisDataSource` port (C3) |
| `engine/src/lib.rs` (151) | Modified | `commands` rename to `scanner::dispatch`; new port module re-exports |
| `engine/src/models/` | Modified | New domain types: `WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`, `ArchitectureEvidence`, `Hotspot` |
| `src-tauri/src/commands.rs` (635 → ~550) | Modified | `explain_node`/`chat` body moved to AIService; 3 string literals → AppError; `get_scan_status` mapping deleted; new `cancel_scan`/`get_dependencies`/`get_dependents` commands |
| `src-tauri/src/lib.rs` (112) | Modified | Register 3 new commands; `configure_ai`/`get_ai_config` via port |
| `src/lib/tauri-api.ts` (565) | Modified | Inline `history` type → `ChatMessage[]`; `toUserMessage`/`getErrorMessage` extracted to `errors.ts` |
| `src/lib/errors.ts` (new) | New | Cross-cutting error i18n |
| `src/locales/es/errors.ts` (new) | New | Spanish error messages |
| `scripts/ci/check-architecture.mjs` (208 → ~280) | Modified | New strict patterns (C6); self-test fixtures for each pattern |
| `.github/workflows/ci.yml` (186) | Modified | `npm run check:arch` wired (CI1); `cargo test --tests` step; optional `cargo clippy -- -D warnings` after C6 |
| `scripts/check-error-codes.mjs` (new) | New | Sync-check for `BACKEND_TO_FRONTEND_CODE` |
| `docs/architecture.md` (new) | New | Current-state architecture tutorial |
| `openspec/specs/backend-ports-and-services/spec.md` | Modified | 5 new ports + domain type signatures |
| `openspec/specs/error-contract/spec.md` | Modified | 3 string literals closed; 100% `to_ipc_error` coverage |
| `engine/tests/*.rs` (10 files, 2896 lines) | Modified | Re-verified after C1 `pub(crate)` migration; new tests for ports |
| `src/lib/tauri-api.ts` consumer components | Verified | 3 dead commands have no UI consumer (grep confirmed 0 hits in `src-tauri/src/`) |

## Rollback Plan

Each PR is independently revertible:

- **C1-C6**: revert the PR; pre-wave-2-foundation baseline is stable on `main` (d862fa9 + W2-001 hotfix, 675 tests green). All ports are additive; `pub(crate)` is a visibility tightening, not a behavior change. Reverting C1 brings 5 traits back to `pub`. Reverting C2-C5 removes port calls and re-inserts direct infra usage in services.
- **C7 (CQRS)**: revert the PR; `queries/` directory split is self-contained. `engine/src/db/queries.rs` returns to a single 2419-line file. No upstream code change.
- **Across the chain**: if any PR causes CI to fail in a way the orchestrator cannot diagnose in 30 minutes, that PR is reverted, the issue is fixed locally, and a new PR replaces it.

No database migration, no frontend state migration, no user data migration. Wave 2 is pure refactoring; user data, projects, and snapshots are byte-identical before and after.

## Dependencies

- **`chained_pr_strategy: auto-forecast`** already enabled in project config
- **`chained-from-start` (C3)** — every PR planned as a chain slice from the start; no PR can be the "last one we need to think about"
- **Single reviewer (user)** — feature-branch-chain chosen because stacked-to-main would block 6 reviews before the first PR merges
- **Pre-wave-2-foundation merged to main** (commit `da2b15a` + W2-001 hotfix `d862fa9`) — 675 tests green, AppState 100% hexagonal
- **Rust toolchain** — `chrono` and `uuid` stay as adapter dependencies; ports replace direct usage but do not remove the dependencies
- **No new external dependencies** — Clock/IdGen/Stopwatch are pure-stdlib implementations