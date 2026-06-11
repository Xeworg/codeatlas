# Backend Ports and Services Specification

## Purpose

Define the backend port contracts, application services, and Tauri composition rules required for the first hexagonal migration wave in CodeAtlas, and the wave-2 extensions that harden the boundary before the wave-3 multi-crate split.

## Wave 2 Delta

**Added requirements** (new in this revision):
- `Wave 2 ports` — 5 new ports: `Clock`, `IdGenerator`, `Stopwatch`, `AnalysisDataSource`, `FileSourceReader`
- `QueryRepository trait` — read-side of the CQRS split
- `CommandRepository trait` — write-side of the CQRS split
- `Repository module reorganized for CQRS` — `engine/src/db/queries.rs` (2419 lines) becomes a directory with `mod.rs`, `commands.rs`, `queries.rs`

**Deferred requirements** (added in delta; deferred to wave 3 after C1.1 apply on 2026-06-10 — see ADR-009):
- ~~`Pub(crate) wave-1 ports` — 6 port traits are `pub(crate)`~~ — deferred to wave 3. See `hexagonal-ports/spec.md::Pub(crate) port traits` for the full status note and rationale. C1 ships with the 6 traits remaining `pub`; wave 3 (post multi-crate split) tightens to `pub(crate)` inside each sub-crate.

**Modified requirements** (existing requirements with wave-2 changes):
- `Port signatures stay infrastructure-agnostic` — adds a tuple→domain-type constraint for 9 `WorkspaceRepository` methods (D1, D2)
- `Additive repository adaptation is allowed` — relaxed: file-level restructuring into a `queries/` module is now allowed (CQRS split in C7)

## Requirements

### Requirement: Canonical wave-1 ports

The backend MUST expose the following canonical port traits for wave 1:

- `ScanRepository`
- `GraphRepository`
- `WorkspaceRepository`
- `AppStatePort`

#### Scenario: Ports exist in a dedicated engine module

- GIVEN the engine source tree
- WHEN inspecting the wave-1 port definitions
- THEN there MUST be a dedicated port module rooted at `engine/src/ports.rs`
- AND the engine crate MUST publicly export the four canonical wave-1 traits
- AND the port module MUST NOT import Tauri state or concrete repository implementations

#### Scenario: Port signatures stay infrastructure-agnostic

- GIVEN any wave-1 port method
- WHEN inspecting its parameters and return types
- THEN the signature MUST use primitives, owned domain data, references to domain data, or `crate::Result<T>`
- AND it MUST NOT expose `State<'_, _>`, `DbPool`, `rusqlite::Row`, `ProjectRepository`, or other concrete infrastructure types
- AND the 9 `WorkspaceRepository` methods that previously returned tuples MUST now return domain types (`WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`)
- AND no method on `WorkspaceRepository` exposes `db::queries::ExecutiveSummary`, `db::queries::SnapshotDiff`, or `db::queries::C4View` (these types move to `engine::models`)

#### Scenario: WorkspaceRepository methods return no tuples and no infra types

- GIVEN the wave-2 `WorkspaceRepository` port trait
- WHEN inspecting every method signature
- THEN zero methods return a tuple (e.g. `(String, String, String)`)
- AND zero methods expose any `crate::db::queries::*` type in either parameters or return position
- AND `rg "-> \(" engine/src/ports.rs` returns zero matches inside the `WorkspaceRepository` trait block

### Requirement: Additive repository adaptation is allowed

Wave 1 MUST avoid a structural split of `engine/src/db/queries.rs`, but it MAY adapt `ProjectRepository` to the new ports through additive code.

#### Scenario: queries.rs internals remain intact

- GIVEN the wave-1 backend diff
- WHEN inspecting `engine/src/db/queries.rs`
- THEN the file MAY receive additive trait impls or thin wrappers that delegate to existing methods
- BUT the SQL layout, core CRUD method bodies, and file split strategy MUST remain substantially unchanged

#### Scenario: Port adaptation does not create a second repository model

- GIVEN the port adaptation layer
- WHEN inspecting how services obtain persistence behavior
- THEN services MUST depend on the wave-1 ports, not directly on `ProjectRepository`
- AND the adapter layer MUST delegate to existing repository behavior instead of re-implementing it

#### Scenario: Wave 2 supersedes the additive-only restriction

- GIVEN the wave-2 boundary state
- WHEN inspecting `engine/src/db/queries/`
- THEN the additive-only restriction is no longer in force — the 2419-line `queries.rs` is restructured into a `queries/` module with `mod.rs`, `commands.rs`, and `queries.rs` for the CQRS split
- AND services continue to depend on `QueryRepository` / `CommandRepository` ports, not on the combined `ProjectRepository`
- AND the no-second-repository-model invariant from the previous scenario is preserved

### Requirement: Canonical backend services

The backend MUST define the following application services for wave 1:

- `ScanService`
- `GraphService`
- `WorkspaceService`
- `AnalysisService`

#### Scenario: Services exist per backend responsibility cluster

- GIVEN the engine source tree
- WHEN inspecting `engine/src/services/`
- THEN the module MUST contain service implementations for the four canonical backend services
- AND each service MUST be publicly exported for backend composition

#### Scenario: Services depend on ports, not Tauri presentation

- GIVEN any wave-1 service
- WHEN inspecting its fields and constructor
- THEN the service MUST depend on one or more wave-1 ports and explicit collaborators
- AND it MUST NOT depend on `State<'_, AppState>` or instantiate concrete Tauri-managed state directly

#### Scenario: Commands stop orchestrating use cases inline

- GIVEN a migrated Tauri command
- WHEN inspecting the command body
- THEN it MUST only extract managed state, call the relevant service, and map the result or error
- AND it MUST NOT contain multi-step use-case orchestration inline

### Requirement: Single Tauri composition root

Wave 1 MUST consolidate concrete wiring in one composition root.

#### Scenario: Concrete backend collaborators are wired centrally

- GIVEN the Tauri startup code
- WHEN inspecting `src-tauri/src/lib.rs`
- THEN concrete repositories, app-state adapters, and any required collaborators MUST be created or wired there
- AND the resulting services MUST be stored in managed application state

#### Scenario: Tauri commands do not instantiate infrastructure directly

- GIVEN any migrated command in `src-tauri/src/commands.rs`
- WHEN inspecting the function body
- THEN the command MUST NOT directly instantiate `ProjectRepository`, `FileWalker`, `ParserRegistry`, `PathResolver`, `GraphBuilder`, or concrete AI providers

### Requirement: Existing pure helpers remain usable

Wave 1 MUST preserve `engine/src/commands.rs` as an internal pure-helper module.

#### Scenario: Existing engine::commands helpers survive wave 1

- GIVEN the engine helper module
- WHEN inspecting `engine/src/commands.rs`
- THEN existing pure helpers such as `scan_files` and other current orchestration helpers MAY remain in place
- AND wave 1 MUST NOT require their removal to be considered complete

### Requirement: commands.rs becomes a thin presentation shim

The Tauri command module MUST be substantially reduced after the migration slices land.

#### Scenario: commands.rs line budget

- GIVEN the final wave-1 backend state
- WHEN measuring `src-tauri/src/commands.rs`
- THEN the recommended target is <=300 LOC of code
- AND the acceptable closing ceiling for wave 1 is <=350 LOC if DTO extraction is the only remaining reason to stay above the target

#### Scenario: commands.rs content is presentation-only

- GIVEN any migrated command body
- WHEN reviewing its contents
- THEN it MUST be limited to state extraction, service delegation, and result/error mapping
- AND business rules, persistence choreography, and filesystem orchestration MUST live outside the command body

### Requirement: Existing v3-related commands are refactorable in wave 1

Wave 1 MAY refactor the existing workspace, snapshot, annotation, health, and C4 command surface without treating that work as new product development.

#### Scenario: Refactor covers existing workspace-related commands only

- GIVEN commands already present in `src-tauri/src/commands.rs`
- WHEN they are migrated into `WorkspaceService`
- THEN the change MUST preserve their current observable behavior
- AND it MUST NOT introduce additional product capabilities beyond the already existing command surface

### Requirement: Wave 2 ports

The backend MUST expose 5 additional port traits in wave 2 to close the remaining application-layer leaks. Each port has a system adapter (production) and a mock adapter (deterministic tests). The port traits are: `Clock` (single `now()` method, system + mock adapters), `IdGenerator` (single `next_id()` method, random + mock adapters), `Stopwatch` (`start()` and `elapsed_ms(handle)` methods, system + mock adapters), `AnalysisDataSource` (returns neutral data like `Vec<FileMeta>` and `Vec<ImportEdge>` consumed by `engine::analysis/*`), and `FileSourceReader` (`read(path)` and `exists(path)`). Services and use cases MUST accept these ports as `Arc<dyn ...>` (or generic bounded types) and MUST NOT call `chrono::Utc::now()`, `uuid::Uuid::new_v4()`, `std::time::Instant::now()`, or `std::fs::read_to_string` directly. See `hexagonal-ports/spec.md` for full port contracts, adapter impls, and edge-case scenarios.

#### Scenario: Wave 2 port traits compile in the engine module

- GIVEN the wave-2 boundary state
- WHEN the engine crate compiles
- THEN the listed port traits are declared in `engine::ports` (or `engine::ports::hexagonal` for the 5 new ones) with a `System*` adapter and a `Mock*` adapter each
- AND `rg "trait Clock" engine/src/ports.rs`, `rg "trait IdGenerator"`, `rg "trait Stopwatch"`, `rg "trait AnalysisDataSource"`, and `rg "trait FileSourceReader"` each return exactly one match

#### Scenario: Services consume ports through Arc

- GIVEN a wave-2 service (`ScanService`, `GraphService`, `AnalysisService`, `AIService`)
- WHEN inspecting its constructor and fields
- THEN it MUST hold `Arc<dyn Clock>`, `Arc<dyn IdGenerator>`, `Arc<dyn Stopwatch>`, `Arc<dyn AnalysisDataSource>`, and `Arc<dyn FileSourceReader>` (or generic bounded types) and MUST NOT construct adapters inline

### Requirement: QueryRepository trait

The backend MUST expose a `QueryRepository` port trait covering all read operations previously performed by `engine::db::queries::ProjectRepository`. Each method MUST return owned domain data (`WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`, `Vec<FileInfo>`, `Option<String>`, etc.) and MUST NOT return tuples or types from `crate::db::queries::*`. The read side MUST NOT expose write methods. See `cqrs-repository/spec.md` for the full contract, adapter shape, and the no-`&DbPool`-leak invariant.

#### Scenario: Read methods return domain types only

- GIVEN `QueryRepository::get_workspace(workspace_id)` is called via the trait
- WHEN the call completes
- THEN the return type is `Result<Option<WorkspaceMeta>>` where `WorkspaceMeta` is defined in `engine::models`
- AND no method on the trait returns a tuple like `(String, String, String)` or a type from `crate::db::queries::*`

### Requirement: CommandRepository trait

The backend MUST expose a `CommandRepository` port trait covering all write/mutation operations previously performed by `ProjectRepository`. Each method MUST accept domain input types and return `Result<()>` or a minimal handle (e.g. the new entity ID as `String`). The write side MUST NOT expose read methods. See `cqrs-repository/spec.md` for the full contract and adapter shape.

#### Scenario: Write methods are isolated from reads

- GIVEN a `CommandRepository` adapter injected into a service
- WHEN the service performs a write (e.g. `create_workspace(name)`, `save_snapshot(...)`)
- THEN the call dispatches to a method on the write side
- AND the trait MUST NOT expose read methods like `get_workspace`, `list_workspaces`, `get_health_timeline`

### Requirement: Pub(crate) wave-1 ports

> **STATUS: Deferred to wave 3.** This requirement is documented for traceability but NOT applied in wave 2. See `hexagonal-ports/spec.md::Pub(crate) port traits` for the full status note and rationale (the `pub(crate)` semantics in Rust do not cross the `engine` ↔ `src-tauri` crate boundary, and 17 compilation errors during C1.1 apply confirmed the constraint on 2026-06-10).

The 6 port traits — `ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AppStatePort`, `AnalysisRepository`, and `AIServicePort` — are the wave-3 target for `pub(crate)` declaration inside their respective modules. Wave 2 ships with them remaining `pub`; the hexagonal boundary is held by the type system (`src-tauri` does not name `ProjectRepository` or concrete adapters). See `hexagonal-ports/spec.md` for the full migration contract and wave-3 plan.

#### Scenario: External crate cannot import a port trait by name

- GIVEN `src-tauri/src/commands.rs` after the migration
- WHEN the file is compiled
- THEN `use engine::ports::ScanRepository;` (or any of the other 5 traits) produces a "module `ports` is private" visibility error
- AND the file MUST obtain ports only via `state.scan_repo: Arc<dyn ScanRepository>`, `state.graph_repo: Arc<dyn GraphRepository>`, `state.workspace_repo: Arc<dyn WorkspaceRepository>`, `state.app_state_port: Arc<dyn AppStatePort>`, `state.analysis_repo: Arc<dyn AnalysisRepository>`, or `state.ai_service: Arc<dyn AIServicePort>`

#### Scenario: Integration tests remain green

- GIVEN the full `engine/tests/*.rs` suite (10 files, ~2896 lines)
- WHEN `cargo test --tests` runs after the migration
- THEN all integration tests pass
- AND no test imports a port trait from outside the `engine` crate

### Requirement: Repository module reorganized for CQRS

The 2419-line `engine/src/db/queries.rs` MUST be reorganized into a directory `engine/src/db/queries/` containing `mod.rs`, `commands.rs`, and `queries.rs`. `mod.rs` re-exports the `QueryRepository` and `CommandRepository` traits and their adapters. `commands.rs` holds the write-side SQL implementations; `queries.rs` holds the read-side SQL implementations. No single file under `engine/src/db/queries/` exceeds 1500 lines. The split is purely file-level reorganization plus trait extraction — no SQL changes, no schema changes, no behavioral changes. The wave-1 "additive-only" restriction on `queries.rs` is superseded by this requirement; structural changes to the file/module are now expected. See `cqrs-repository/spec.md` for the trait split contract.

#### Scenario: queries directory layout matches the spec

- GIVEN the wave-2 boundary state
- WHEN `ls engine/src/db/queries/` runs
- THEN the directory contains `mod.rs`, `commands.rs`, and `queries.rs`
- AND `mod.rs` re-exports `QueryRepository`, `CommandRepository`, and their adapters
- AND no single file under `engine/src/db/queries/` exceeds 1500 lines

#### Scenario: SQL is byte-identical after the split

- GIVEN the wave-2 boundary state
- WHEN diffing the SQL strings in `commands.rs` + `queries.rs` against the pre-split `queries.rs`
- THEN every `const` query and every prepared statement string is byte-identical (modulo whitespace at the developer's discretion)
- AND `cargo test --tests` remains green with no behavior change
