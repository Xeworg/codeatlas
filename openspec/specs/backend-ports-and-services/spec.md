# Backend Ports and Services Specification

## Purpose

Define the backend port contracts, application services, and Tauri composition rules required for the first hexagonal migration wave in CodeAtlas.

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
