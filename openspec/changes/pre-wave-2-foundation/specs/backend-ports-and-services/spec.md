# Spec Deltas: pre-wave-2-foundation — backend-ports-and-services

> Delta spec for `openspec/specs/backend-ports-and-services/spec.md`.
> Endurece la frontera del crate `engine`: puertos `pub(crate)`, `AppState` con `Arc<dyn ...>`, guard de CI que bloquea regresiones, y un `AIServicePort` delgado para que la presentación consuma IA sin acoplarse a la estructura concreta.

## ADDED Requirements

### Requirement: Fifth canonical port AnalysisRepository

The canonical wave-1 port set is extended to five ports. The backend MUST expose `AnalysisRepository` as a peer of the existing four ports (`ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AppStatePort`) at `engine/src/ports.rs`.

#### Scenario: AnalysisRepository is part of the canonical set

- **WHEN** inspecting `engine/src/ports.rs`
- **THEN** the module MUST define `AnalysisRepository` as a trait with the same visibility rules as the other four ports
- **AND** `AnalysisRepositoryAdapter` MUST live alongside the other adapters in the same module

### Requirement: Port traits are crate-internal

The five port traits (`ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AnalysisRepository`, `AppStatePort`) MUST be declared `pub(crate)` in `engine/src/ports.rs`. They are part of the `engine` crate's internal seam and MUST NOT appear in the public API of the `engine` crate. The adapters that implement them may keep a different visibility (e.g. `pub` for tests inside the crate) as long as the trait itself stays crate-internal.

#### Scenario: external consumer cannot name a port trait

- **WHEN** code outside the `engine` crate attempts `use engine::ports::ScanRepository;`
- **THEN** the compiler rejects the import with "module `ports` is private" or equivalent visibility error

#### Scenario: internal adapter still implements the trait

- **WHEN** `ScanRepositoryAdapter` lives in `engine::db` (or an internal adapter submodule) and implements `ScanRepository`
- **THEN** the impl compiles because the trait is visible within the crate

#### Scenario: existing tests inside the crate continue to compile

- **WHEN** `engine/src/services/scan_service.rs:346-372` (or analogous test sites) references port traits
- **THEN** the references resolve because they are inside the same crate

### Requirement: AppState holds Arc<dyn> ports

`AppState` in `src-tauri/src/commands.rs` MUST hold its dependencies as `Arc<dyn Trait>` for each port. Concrete `DbPool`, `ProjectRepository`, and concrete `engine::ai::AIService` MUST NOT appear in the `AppState` struct fields. Primitive fields (e.g. `Mutex<...>` for in-memory collaborators) MAY remain if they do not represent infrastructure.

#### Scenario: AppState fields are all trait objects or primitives

- **WHEN** `AppState` is read
- **THEN** every collaborator field is `Arc<dyn SomePort>` or a primitive collaborator
- **AND** `rg "pub (db|ai_service):" src-tauri/src/commands.rs` returns 0 hits for infrastructure-typed fields

#### Scenario: composition root in lib.rs injects adapters

- **WHEN** `run()` in `src-tauri/src/lib.rs` constructs `AppState`
- **THEN** it builds concrete adapters (e.g. `Arc::new(ScanRepositoryAdapter::from_arc(pool.clone()))`) and wraps them as `Arc<dyn ScanRepository>` before placing them in `AppState`
- **AND** adapters expose both `Adapter::new(&pool)` (kept for internal tests) and `Adapter::from_arc(Arc<DbPool>)` (used by the composition root)

#### Scenario: commands extract Arc<dyn> from state

- **WHEN** a Tauri command body runs
- **THEN** it does `let scan = &state.scan_repo;` and passes the trait object to the service
- **AND** no command body constructs a new adapter from `&state.db` directly

#### Scenario: services accept Arc<dyn> ports

- **WHEN** a service field type is read
- **THEN** it is `Arc<dyn SomePort>` (or `&dyn SomePort` where the lifetime allows), not `&DbPool` or `&ProjectRepository`

### Requirement: CI guard blocks port leakage

A CI step `npm run check:arch` MUST fail the build if `src-tauri/src/commands.rs` imports any of:

- `use engine::db::*` (or any concrete subpath of `engine::db`)
- `use engine::ai::anthropic::*`, `use engine::ai::resolved::*`, `use engine::ai::provider::AIProvider`
- `use engine::ai::AIService` (after PR-B, once `AIServicePort` exists)
- `.map_err(|e| e.to_string())` as a final command-body error mapping (after PR-B, replaced by `to_ipc_error`)

The script MUST live at `scripts/ci/check-architecture.mjs` and be wired via `package.json` so existing CI workflow files do not need modification.

#### Scenario: forbidden import reintroduced

- **WHEN** a developer adds `use engine::db::queries::ProjectRepository;` to `commands.rs`
- **THEN** `npm run check:arch` exits non-zero and the CI fails
- **AND** the developer MUST refactor to use the `ScanRepository` port

#### Scenario: legacy map_err reintroduced

- **WHEN** a developer adds a new `.map_err(|e| e.to_string())` at a command body's return boundary
- **THEN** the guard fails and the developer MUST route the conversion through `to_ipc_error`

#### Scenario: legitimate import with allow comment is rejected

- **WHEN** a developer adds `// arch-allow: port-trait-only` above a forbidden-looking import
- **THEN** the guard still fails — allow comments are not yet honored; granting an exception requires a follow-up PR with the `arch-exception` label and a documented justification

#### Scenario: CI guard is tested by a fixture failure

- **WHEN** a developer adds a fixture import that the guard is designed to catch
- **THEN** running `npm run check:arch` locally reproduces the CI failure with a non-zero exit code
- **AND** removing the fixture import returns the guard to a green state

### Requirement: AIService is consumed through AIServicePort

`AIService` MUST be reachable from the presentation layer ONLY through the `AIServicePort` trait. A thin trait with the two methods used by the presentation (`explain_node_with_context`, `chat_with_context`) lives at `engine::ai::service::AIServicePort` and is implemented by `AIService` directly. Concrete adapter types (`AnthropicProvider`, `ResolvedProvider`, `ProviderFactory`) remain invisible to the presentation layer, consistent with `ai-module-boundary/spec.md`.

#### Scenario: AppState holds Arc<dyn AIServicePort>

- **WHEN** `AppState` is constructed
- **THEN** it holds `Arc<dyn AIServicePort>`, not `Arc<AIService>` or `engine::ai::AIService` directly

#### Scenario: AIService concrete struct not in AppState field

- **WHEN** the developer greps `src-tauri/src/commands.rs` for `AIService`
- **THEN** matches appear only in tests or in trait-bound position, never as a field type

#### Scenario: AIServicePort stays narrow

- **WHEN** the trait surface is read
- **THEN** it contains exactly two methods corresponding to the two paths used by the presentation (`explain_node_with_context`, `chat_with_context`)
- **AND** it does NOT expose provider selection, configuration, or other concerns already covered by `ai-module-boundary/spec.md`

## MODIFIED Requirements

> The existing requirements in `backend-ports-and-services/spec.md` remain stable. The new requirements above extend the contract. No `MODIFIED` blocks are emitted in this delta.
