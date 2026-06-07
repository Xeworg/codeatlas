# Code Context — Hexagonal Migration Scout

> Read-only mapping of CodeAtlas for the proposed repository-wide hexagonal migration. Output of a parent-scoped scout; intended to seed `sdd-proposal` for a new OpenSpec change.

## Project snapshot (as found)

- **OpenSpec**: `openspec/config.yaml` declares `architecture.style: "Clean Architecture (adaptada a Tauri monolítica)"`, `strict_tdd: true`, `execution_mode: interactive`, `chained_pr_strategy: auto-forecast`, `review_budget_changed_lines: 400`, `active_change: null`. v3 is the pending open version (`openspec/README.md`).
- **Archive precedent** (naming/style template): `2026-06-01-v1-mvp-core`, `2026-06-01-v2-advanced-analysis`, `2026-06-04-robust-logging-observability`, `outline-parser-abstraction`.
- **Stack**: Tauri 2 desktop app. Rust engine crate (`engine/`) + Tauri shim (`src-tauri/`) + React/Vite/Zustand frontend (`src/`).
- **Recent AI hexagonal slice already merged**: a focused refactor of `engine/src/ai/*` introduced a `AIProviderResolver` port + `ProviderFactory` adapter + `AIService<R>` application service. Verified by `review-architecture.md` and `review-correctness.md` (23 AI tests, 31 src-tauri tests pass). That slice is the local precedent for the global wave.

## Files Retrieved (with line ranges)

### OpenSpec / governance

1. `openspec/config.yaml` (1-90) — scope, languages, test runners, architecture style label.
2. `openspec/README.md` (1-50) — active change status, phase expectations, v3 scope.
3. `docs/ESTANDARES_CODIGO_REUTILIZABLE_Y_ARQUITECTURA.md` (1-420) — current "Clean Architecture adapted" rules, layer diagram, dependency matrix, anti-patterns, refactor policy.
4. `review-architecture.md` (1-110) — official diff review of the AI hexagonal slice; lists 3 known seams still leaking.
5. `review-correctness.md` (1-80) — confirms the AI slice is correct; 5 minor non-blockers.
6. `openspec/changes/outline-parser-abstraction/` — prior in-flight change (skeleton, still pending).

### Engine crate (Rust — current state)

7. `engine/src/lib.rs` (1-62) — public re-exports `pub use models::*`, `AppError` enum, `pub mod ai/analysis/commands/db/graph/models/scanner`. Public surface already committed.
8. `engine/src/models/{mod,project,file,graph,ai}.rs` — domain layer; pure `serde` only. Good.
9. `engine/src/db/{mod,queries,schema,migrations}.rs` — infra. `queries.rs` is **2,383 LOC**, one mega `ProjectRepository` carrying v1+v2+v3 surfaces (projects, files, imports, outline, graph cache, snapshots, comments, workspaces, health). No trait boundary beyond `ProjectRepository`.
10. `engine/src/scanner/{mod,walker,code_parser,parser/*}.rs` — infra. `parser/traits.rs` already a proper port (`LanguageParser`, `ParserRegistry`). Good precedent.
11. `engine/src/graph/{mod,builder,resolver}.rs` — application. Pure. Good.
12. `engine/src/analysis/{mod,architecture_detector,impact_engine,graph_insights,degraded_tests}.rs` — application. Pure, depends on `DbPool` directly (signature `fn compute_graph_insights(project_id, &DbPool, &InsightsConfig)`). Architectural leak: app layer takes a concrete infra handle.
13. `engine/src/ai/{mod,provider,factory,resolved,service,anthropic,context}.rs` — mixed layer:
    - `provider.rs` — `AIProvider` trait (domain port).
    - `factory.rs` — `AIProviderResolver` (port) + `ProviderFactory` (concrete adapter) in the **same file**.
    - `service.rs` — `AIService<R>` application service.
    - `resolved.rs` — `ResolvedProvider` enum that names concrete adapters in app layer.
    - `anthropic.rs` — infra adapter.
    - `context.rs` — `ContextBuilder` (614 LOC), used by presentation.
    - `mod.rs` — re-exports `AnthropicProvider` at module root, so `engine::ai::AnthropicProvider` is the public path of an infra type.
14. `engine/src/commands.rs` (1-160) — claims to be a "pure orchestration layer", but the Tauri shim does **not** use it; `src-tauri/src/commands.rs` reinvents the same orchestration inline. The engine's pure-commands layer is dormant.

### Tauri shim (Rust — current state)

15. `src-tauri/src/main.rs` (1-3) — one-liner that calls `codeatlas_lib::run()`.
16. `src-tauri/src/lib.rs` (1-91) — composition root (Tauri `setup` hook). Builds `DbPool`, runs migrations, constructs `AppState { db, scan_status, ai_config, project_root, ai_service: AIService::default() }`. This is the only real composition point.
17. `src-tauri/src/commands.rs` (**1,526 LOC**) — presentation shim. **This is the single biggest coupling hotspot in the repo.** Mixes:
    - Direct calls to infra (`ProjectRepository::new(&state.db)`, `PathResolver::new(&path)`, `ParserRegistry::new()`, `FileWalker::new(&path)`).
    - Direct filesystem I/O (`std::fs::read_to_string(&path)` for AI context).
    - Application-layer calls from presentation (`ContextBuilder::build_node_context`, `ContextBuilder::build_chat_context`).
    - DTO conversion (`ImpactAnalysisResponse::from(EngineImpactResult)`, `GraphInsightsResponse::from(...)`, `ArchitectureDetectionResponse::from(...)`).
    - 28 `#[tauri::command]` handlers, including the v3 surface: `create_workspace`, `list_workspaces`, `attach_project_to_workspace`, `list_workspace_projects`, `create_snapshot`, `get_snapshot`, `list_snapshots`, `add_comment`, `list_comments`, `get_health_timeline`, `get_executive_summary`, `compare_snapshots`, `get_c4_view`.
    - AppState construction on the spot for AIService: **note in `review-architecture.md` already flagged that `commands.rs:673` and `:755` build `AIService::default()` per request** even though the service is also held in `AppState`.

### Frontend (TypeScript/React — current state)

18. `src/main.tsx` (1-34) — React root.
19. `src/App.tsx` (1-329) — orchestrates: imports `useProjectStore`, `useGraphStore`, **and** `tauri-api` directly (lines 23-29). `handleOpenProject` re-implements a 4-step "reopen → fresh scan → fetch graph" flow inline. Side-effects on `status === 'ready' && projectId` fetch analytics with no hook mediation. Mixed concerns.
20. `src/lib/types.ts` (1-212) — v1+v2 contract types. Clean.
21. `src/lib/types-v3.ts` (1-102) — v3 contract types (Workspace, Snapshot, Comment, HealthRecord, ExecutiveArchitectureSummary, SnapshotDiffPayload, C4ViewPayload). Mostly placeholder shapes; backend ports not yet defined.
22. `src/lib/tauri-api.ts` (**462 LOC**) — 32 typed `invoke` wrappers. **Hotspot**: `toApiError` (lines 12-46) does string-based error classification (`msg.includes('401') || msg.includes('InvalidApiKey') || msg.includes('invalid_api_key')`), depending on `AppError::Display` strings from Rust. The `getErrorMessage` UI shaper is also here. Infrastructure and presentation concerns co-exist.
23. `src/lib/graph-layout.ts`, `src/lib/i18n.ts` — domain helpers.
24. `src/stores/{projectStore,graphStore,chatStore,analyticsStore,useSnapshotStore,featureFlags}.ts` — Zustand state.
25. `src/hooks/{useProject,useGraph,useAI,useExport}.ts` — partial hook layer; **not used by `App.tsx`**, which calls `tauri-api.ts` directly.
26. `src/components/**` — presentation. `DetailPanel`, `AIExplanation`, `ChatPanel`, `ApiKeySetup` import directly from `lib/tauri-api` (4 files — small but a direct violation of the std doc's "presentation never invokes directly" rule).

## Architecture (how the pieces connect today)

```
                    ┌── Tauri Webview (React) ──────────────────────────┐
                    │  App.tsx ──► hooks (partial) ──► stores           │
                    │     │             │                  │            │
                    │     └──► tauri-api.ts (infra+UI mix) ──┘          │
                    └────────────┬──────────────────────────────────────┘
                                 │ invoke(...)         (28 commands)
                    ┌────────────▼──────────────────────────────────────┐
                    │ src-tauri/src/commands.rs (1526 LOC shim)         │
                    │  - state AppState{db, ai_service, locks}          │
                    │  - calls engine::ai::ContextBuilder  ← app leak  │
                    │  - calls ProjectRepository::new(&state.db)        │
                    │  - reads std::fs, traces, maps DTOs               │
                    └────────────┬──────────────────────────────────────┘
                                 │
        ┌────────────────────────┴────────────────────────┐
        │ engine crate                                   │
        │ ┌─ models/    (domain, serde-only)              │
        │ ├─ commands/  (pure orchestration, UNUSED)     │
        │ ├─ graph/     (app, pure)                      │
        │ ├─ analysis/  (app, but takes &DbPool directly)│
        │ ├─ ai/  ──── port(AIProvider) + port(Resolver)│
        │ │       └─ app(AIService) + app(ContextBuilder)│
        │ │       └─ infra(AnthropicProvider, Factory)  │
        │ ├─ scanner/   (infra, has parser port)        │
        │ └─ db/        (infra, 2383 LOC mega-repo)     │
        └────────────────────────────────────────────────┘
```

**Net effect of the current "Clean Architecture adapted"**: the dependency direction is mostly correct, but **the application seam does not exist** — the Tauri shim talks to both application (ContextBuilder) and infrastructure (DbPool, filesystem, tree-sitter) directly, and the engine's pure orchestration layer (`engine::commands`) is bypassed entirely. The AI slice is the only true hexagonal-shaped pocket.

## Coupling Hotspots that conflict with hexagonal

Severity ordered (high → low).

| #   | File:line                                                                                                                | Hotspot                                                                                                                                                                                                | Hexagonal conflict                                                                                                                            |
| --- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| H1  | `src-tauri/src/commands.rs` (entire 1,526-LOC file)                                                                      | Tauri handlers reach into both infra (`ProjectRepository::new(&state.db)`, `ParserRegistry::new()`, `FileWalker::new(&path)`, `std::fs::read_to_string`) and app (`ContextBuilder::build_*`) directly. | Presentation layer holds the composition root and contains use-case orchestration. No port boundary.                                          |
| H2  | `src-tauri/src/commands.rs:673,755`                                                                                      | `let ai_service = AIService::default();` per request, even though `AppState.ai_service` exists.                                                                                                        | Duplicate composition. Leaks factory decision into presentation. Already flagged in `review-architecture.md`.                                 |
| H3  | `src-tauri/src/lib.rs:73`                                                                                                | `ai_service: engine::ai::AIService::default()` built at startup but never used by commands (see H2).                                                                                                   | Composition root is half-built; presentation continues to construct services.                                                                 |
| H4  | `src-tauri/src/commands.rs` (state struct + 28 handlers)                                                                 | `AppState` bundles DB, scan status, AI config, project root, AIService behind hand-rolled `Mutex`es. No use-case service objects.                                                                      | No "current project" / "open workspace" application object. Implicit shared state with no contract.                                           |
| H5  | `engine/src/analysis/{architecture_detector,impact_engine,graph_insights}.rs`                                            | Pure analysis functions take `&DbPool` directly.                                                                                                                                                       | App layer depends on a concrete infra handle instead of a `Repository`/`SnapshotStore` port.                                                  |
| H6  | `engine/src/ai/factory.rs` (whole file)                                                                                  | Defines both `AIProviderResolver` (port) and `ProviderFactory` (adapter) in the same module.                                                                                                           | Port/adapter co-location blurs the boundary. Adding a provider forces editing the port file.                                                  |
| H7  | `engine/src/ai/resolved.rs`                                                                                              | `ResolvedProvider` enum names `Anthropic(AnthropicProvider)` inside the app layer.                                                                                                                     | App layer enumerates concrete adapters; adding OpenAI changes app code.                                                                       |
| H8  | `engine/src/ai/mod.rs`                                                                                                   | `pub use anthropic::AnthropicProvider;` at module root.                                                                                                                                                | Infra type is reachable from any `engine::ai::AnthropicProvider` import. Encourages infra leaks.                                              |
| H9  | `engine/src/db/queries.rs` (2,383 LOC)                                                                                   | One mega `ProjectRepository` covers v1+v2+v3 (projects, files, imports, outline, graph cache, snapshots, comments, workspaces, health, insights cache).                                                | No v1/v2/v3 port split; no `SnapshotRepo` / `WorkspaceRepo` / `HealthRepo` ports. Test doubles are impractical.                               |
| H10 | `engine/src/commands.rs` (160 LOC, dormant)                                                                              | Pure orchestration layer exists but is unused; Tauri shim re-implements it inline.                                                                                                                     | Layer is a fiction: no real separation, just duplicated logic.                                                                                |
| H11 | `src/lib/tauri-api.ts:12-46`                                                                                             | `toApiError` does string-based error classification (`msg.includes('401')`, `'InvalidApiKey'`, `'invalid_api_key'`, etc.) over the Rust `AppError::Display` text.                                      | Frontend "infra adapter" depends on the string shape of a backend error type. Use a typed `{code, message}` payload at the contract boundary. |
| H12 | `src/lib/tauri-api.ts` (whole file)                                                                                      | Mixes raw `invoke` plumbing (infra) with `getErrorMessage` UI shaper (presentation) and 32 domain-specific command wrappers (use-case orchestration).                                                  | Three layers in one file. No use-case service objects.                                                                                        |
| H13 | `src/App.tsx:23-29` and `handleOpenProject` (lines 102-167)                                                              | Top-level component imports `tauri-api.ts` directly, fetches architecture/impact analytics with bare `useEffect`, and orchestrates the "reopen-or-scan" flow.                                          | Presentation contains use-case orchestration that should live in a `useProject`/`useAnalytics` hook.                                          |
| H14 | `src/hooks/*`                                                                                                            | Hooks exist (`useGraph`, `useAI`, `useExport`) but `App.tsx` reimplements parallel flows inline.                                                                                                       | Application hook layer is partial and bypassed.                                                                                               |
| H15 | `src/components/panel/{DetailPanel,AIExplanation,ChatPanel}.tsx` and `src/components/onboarding/ApiKeySetup.tsx`         | Components import `tauri-api.ts` directly.                                                                                                                                                             | Direct violation of the std doc's §2.3 "components never invoke" rule.                                                                        |
| H16 | `src/stores/featureFlags.ts`                                                                                             | `V3_H1_ENABLED = true`, `V3_H2/H3_ENABLED = false`; Tauri commands for H2/H3 surface are still wired in `commands.rs` and `tauri-api.ts`.                                                              | No plugin/adapter boundary around v3 features; the flag is patching a layering problem.                                                       |
| H17 | `src-tauri/src/commands.rs` (cross-cutting `tracing::*` calls and `is_root_path_conflict`, `map_save_scan_result_error`) | Presentation layer owns telemetry string formatting and error classification.                                                                                                                          | No `telemetry`/`error_mapping` port; presentation reaches into library concerns.                                                              |
| H18 | `src/lib/types-v3.ts`                                                                                                    | v3 entities (HealthRecord, ExecutiveArchitectureSummary, C4ViewPayload, SnapshotDiffPayload) are declared as TS types only; no matching Rust ports.                                                    | Drift risk: TS shape is the only contract for v3.                                                                                             |
| H19 | `engine/src/commands.rs` (signature pattern)                                                                             | Engine takes `&DbPool` not a `&dyn ProjectRepo` for any new orchestration.                                                                                                                             | Same as H5 but at the orchestration level.                                                                                                    |

## Suggested SDD change

- **Slug**: `hexagonal-architecture-wave-1-ports` (sibling to v3, not a replacement). Shorter aliases: `hex-ports-wave1`, `v3-hex-ports`.
- **Why a sibling change (not folded into v3)**: v3 already has a large surface (workspaces, snapshots, comments, health timeline, C4, comparatives — per `docs/PLAN_MAESTRO_SPRINTS_UI_BACKEND_V1_A_V3.md` and `docs/ARQUITECTURA_DATOS_V2_V3.md`). Building v3 features on top of the current shim couples them to the leaky presentation. Ports first, then v3 slots into adapter-shaped workspaces/snapshots.
- **Scope of the first wave (proposal-level)**: introduce **ports + a composition root**, do not change behavior.
  1. **Engine ports layer** — new `engine::ports` (or extend each existing module with a `ports.rs`) defining traits: `ProjectRepo`, `FileProvider`, `GraphCache`, `OutlineRepo`, `SnapshotStore`, `WorkspaceStore`, `CommentStore`, `HealthStore`, `InsightsCache`, `AIService` (port), `AIProvider` (already exists), `Telemetry`, `FileReader` (filesystem port), `Clock`. Each port returns `Result<_, AppError>`.
  2. **Engine adapters move** — relocate concrete impls under `engine::infra::{db,scanner,ai,fs,telemetry,clock}`. `AnthropicProvider` becomes `engine::infra::ai::anthropic::AnthropicProvider`. Re-export at `engine::ai::AnthropicProvider` is removed.
  3. **Engine application services** — new `engine::application::{ScanService, GraphService, InsightsService, ImpactService, ArchitectureService, SnapshotService, WorkspaceService, AnnotationService, HealthService, AIService}` (rename existing `AIService` to `ai::ApplicationService` if needed for namespace). Each takes only port traits as constructor args.
  4. **Tauri composition root** — `src-tauri/src/composition.rs` builds the `AppState { services: Arc<...>, ports: Arc<...> }` once. AppState fields become `Arc<ScanService>`, `Arc<GraphService>`, `Arc<AIService>`, etc. The 28 handlers shrink to thin adapters that pull a service from state, call one method, and serialize.
  5. **Frontend `services/` module** — new `src/services/{projectService,graphService,analyticsService,aiService,snapshotService,workspaceService,annotationService,healthService}.ts` that own `invoke` + DTO mapping + error classification. Move `toApiError` behind a `mapInvokeError` port that takes a typed payload (requires backend returning `{code,message,details}` instead of `Display` strings — see prerequisite P3).
  6. **Frontend hook consolidation** — extend `useProject`, add `useAnalytics`, `useSnapshots`, `useWorkspace`, `useHealth`; remove direct `tauri-api` imports from `App.tsx` and `components/*`.
  7. **Error contract upgrade** — `AppError` gains structured serialization (`{code, message, details}`) and `tauri-api.ts` decodes that payload, dropping the `msg.includes('401')` text matching.
  8. **Document update** — rewrite §1-§3 of `docs/ESTANDARES_CODIGO_REUTILIZABLE_Y_ARQUITECTURA.md` to call the style "Hexagonal (Ports & Adapters)" and codify port/adapter placement.
- **Out of scope (explicit non-goals for wave 1)**: v3 feature implementation, multi-process deployment, new AI providers beyond the existing `anthropic`/`custom` aliases, schema migrations, breaking Tauri command signatures, deprecating v1 commands.

## Risks & Prerequisites the proposal/spec/design/tasks must capture

### Hard prerequisites (block wave 1 start)

- **P1. v3 scope decision.** The OpenSpec README marks v3 as "SDD pendiente de iniciar" and references workspaces, snapshots, annotations, health timeline, C4, comparatives. Decide whether wave-1 ports must anticipate the v3 ports (recommended) or whether v3 ports land as a follow-up wave. This affects the trait surface size and the review budget.
- **P2. OpenSpec `active_change` is `null`.** This change will be a fresh new entry. Confirm the slug is unique and aligned with archive naming (`kebab-case`, version suffix optional). Update `openspec/changes/hexagonal-architecture-wave-1-ports/` with the standard `proposal.md` / `spec.md` / `design.md` / `tasks.md`.
- **P3. Error contract change is prerequisite to the frontend service split.** `toApiError` in `src/lib/tauri-api.ts` currently parses `AppError::Display` strings. To make frontend services honest, Rust must serialize `AppError` as `{code, message, details?}` (replacing the existing `serialize_str(&self.to_string())` in `engine/src/lib.rs:42-49`). This is a contract change gated by `docs/CHANGELOG_CONTRATOS.md`.
- **P4. Strict TDD is on** (`openspec/config.yaml: strict_tdd: true`). Every port needs a contract test on the Rust side and a port-shape test on the TS side **before** callers are migrated. Plan for a test-first port-extraction phase.
- **P5. Public API of `engine` crate is exposed** (`pub use models::*` in `engine/src/lib.rs`). Refactor must preserve the public surface or bump a contract version. New ports should be additive where possible.

### Architectural risks

- **R1. Massive diff budget.** 28 invoke handlers + 7 Zustand stores + 4 hooks + 1,526-LOC shim + 2,383-LOC DB module + AI/Analysis service splits. The project enforces a **400-line review budget** and uses `chained_pr_strategy: auto-forecast`. Wave 1 must be split into chained PRs by subsystem (db ports → scanner ports → AI ports → application services → tauri shim slim-down → frontend services → frontend hooks consolidation). The proposal should enumerate the chain.
- **R2. Review workload guard** triggers at >400 changed lines. Any chained PR touching more than one major subsystem is at risk of rejection. Tasks should be sized <400 lines and the chained strategy committed up-front.
- **R3. `AppState` design is a single point of failure.** Wave 1 reshapes `AppState`. If a partial state is left mid-migration, the Tauri shim is broken in subtle ways (handlers that construct services on the spot while the canonical service also lives in state). Recommend: introduce the new `AppState` shape in one PR behind a feature flag, switch handlers one cluster at a time, then remove the old shape in a follow-up.
- **R4. `engine::commands` (the "pure orchestration" layer) is dormant.** The new application services may collide with it. Decision needed: kill `engine::commands` and migrate its work into the new `application` services, or keep it as a low-level façade. Suggest deletion in the design phase.
- **R5. v3 entities are declared in `src/lib/types-v3.ts` only.** If wave 1 ports do not define matching Rust port types, the contract drift is permanent. The design should pair each TS v3 type with a Rust port.
- **R6. `featureFlags.ts` mask a layering problem.** H1 is on, H2/H3 are off, but the Tauri shim still wires their commands. If wave 1 does not formalize v3 as a plugin-shaped adapter, the flag keeps papering over the gap. Decision needed: should v3 features land as separate adapter crates, or stay in `src-tauri`?
- **R7. `DbPool` is shared mutable state across the whole process.** Async trait objects and `&DbPool`/`Arc<DbPool>` ownership need explicit policy. Wave 1 must define whether services own their `DbPool` clone or share `Arc<DbPool>`.
- **R8. `#[allow(async_fn_in_trait)]` is already used** in `engine/src/ai/provider.rs`. New ports with `async fn` must follow the same allow pattern; document the rationale in the design.
- **R9. No second driving adapter exists.** Today the only "primary actor" is the Tauri shim. Hexagonal architecture pays off when a second adapter appears (CLI, server, web). Honest framing in the proposal: ports are a hygiene and testability investment, not yet a multi-adapter requirement. Don't oversell.
- **R10. Cross-cutting `tracing` instrumentation is in presentation.** A `Telemetry` port is needed to keep presentation thin. Risk: incomplete migration leaves presentation logging the same way. Plan a full sweep.
- **R11. Tests must keep passing at every step.** 32 cargo tests, 31 src-tauri tests, vitest store tests, integration tests, contract tests are the safety net. Chained PRs must keep them green.
- **R12. Workspace/test count inflation.** Adding a port layer multiplies module count and CI time. Estimate a 10-20% CI time bump; declare budget impact in the proposal.
- **R13. `AppError::serialize` change (P3) is breaking** for any consumer that relied on the current `serialize_str` shape. Confirm with the v1/v2 contract owners and update `docs/CHANGELOG_CONTRATOS.md`.

### Open questions the proposal must answer

- **Q1.** Should wave 1 own the v3 ports (workspaces, snapshots, comments, health, C4) as forward-planning, or should a wave 2 deliver them? Recommend: include v3 ports in wave 1 as **stubs only** so wave 2 only fills implementations.
- **Q2.** Should `engine::commands` (the dormant pure-orchestration module) be kept, refactored, or removed? Recommend: remove in wave 1, re-derive any needed orchestration in `application` services.
- **Q3.** Is the migration gated on a v3 release, or can it proceed in parallel? Recommend: parallel but as the **prerequisite change** for v3, not its replacement.
- **Q4.** What is the error-payload contract version? Recommend: bump to `v2.1` (or `v3` if v3 is open) with `{code, message, details?}` and document the deprecation of the string-only payload.

## Start Here (for the parent / next agent)

1. Open `review-architecture.md` first — it documents the _already-merged_ AI hexagonal slice and the exact seams still leaking. It is the template for what wave 1 should do across the rest of the codebase.
2. Open `docs/ESTANDARES_CODIGO_REUTILIZABLE_Y_ARQUITECTURA.md` §1-§3 to understand the current stated rules (which the code partially violates) and the rename target.
3. Open `engine/src/ai/{mod,provider,factory,service,resolved,anthropic,context}.rs` together as the **reference pattern** for what a hexagonal subsystem looks like in this repo, including the known flaws (H6-H8).
4. Open `src-tauri/src/commands.rs` to see the full scope of the biggest hotspot (H1) and `engine/src/db/queries.rs` to see the size of the next-biggest one (H9).
5. Open `src/App.tsx` and `src/lib/tauri-api.ts` to see the frontend side of the same problem (H11-H15).
6. Then read `openspec/README.md` to confirm the active-change slot for the new slug.

## Supervisor coordination

- No `contact_supervisor` call needed. The scout task is read-only, the parent owns the orchestration decision, and the change slug is a recommendation (not a committed artifact). If the parent wants me to draft `proposal.md` skeleton next, that should be a fresh subagent task in `interactive` SDD mode.
