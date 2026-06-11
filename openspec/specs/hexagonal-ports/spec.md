# Hexagonal Ports Specification

## Purpose

Define the cross-cutting port traits that close the remaining application-layer leaks in `engine`: time, identity, duration, analysis data, and source-file access. The wave-1 ports (wave 1 hexagonal migration + pre-wave-2-foundation) bound **persistence**; this spec bounds **determinism** (Clock, IdGenerator, Stopwatch) and **infrastructure reach-around** (AnalysisDataSource, FileSourceReader). It also retires `pub` on the five existing port traits so the boundary holds under stress when the engine crate is split in wave 3.

## Requirements

### Requirement: Clock port

The system MUST expose a `Clock` port trait with a single method `now() -> chrono::DateTime<chrono::Utc>`. Two adapters MUST be provided: `SystemClock` (delegates to `chrono::Utc::now()`) for production, and `MockClock` (holds a fixed `DateTime<Utc>` and advances only via `set`) for tests. Services and use cases MUST accept `Arc<dyn Clock>` (or generic `C: Clock`) and MUST NOT call `chrono::Utc::now()` directly.

#### Scenario: SystemClock returns real time

- GIVEN a `SystemClock` constructed in production code
- WHEN a service calls `clock.now()`
- THEN it returns a `DateTime<Utc>` within 1 ms of `chrono::Utc::now()` called immediately after

#### Scenario: MockClock returns deterministic time

- GIVEN a `MockClock` initialized with `MockClock::new(some_fixed_datetime)`
- WHEN a service calls `clock.now()` repeatedly
- THEN it returns `some_fixed_datetime` for every call until `set(new_time)` is invoked

#### Scenario: Service no longer imports chrono

- GIVEN a service migrated to use the port
- WHEN `rg "chrono::Utc::now" engine/src/services` runs
- THEN zero matches appear in the service layer

### Requirement: IdGenerator port

The system MUST expose an `IdGenerator` port trait with a single method `next_id() -> uuid::Uuid`. Two adapters MUST be provided: `RandomIdGen` (delegates to `uuid::Uuid::new_v4()`) for production, and `MockIdGen` (counter-based, returns `Uuid::nil()` then sequential v4-shaped values from a sequence) for tests. Services and adapters MUST accept `Arc<dyn IdGenerator>` and MUST NOT call `uuid::Uuid::new_v4()` directly.

#### Scenario: RandomIdGen produces unique IDs

- GIVEN a `RandomIdGen` in production
- WHEN the service calls `id_gen.next_id()` 1000 times
- THEN all 1000 returned `Uuid` values are unique (pairwise distinct)

#### Scenario: MockIdGen is deterministic

- GIVEN a `MockIdGen` initialized at counter 0
- WHEN the service calls `id_gen.next_id()` twice
- THEN the second call returns a UUID whose bytes 8-15 differ from the first by exactly 1 in the lowest byte (counter increments)

### Requirement: Stopwatch port

The system MUST expose a `Stopwatch` port trait with two methods: `start() -> StopwatchHandle` (returns an opaque handle) and `elapsed_ms(handle: StopwatchHandle) -> u64`. Two adapters MUST be provided: `SystemStopwatch` (uses `std::time::Instant`) and `MockStopwatch` (returns a fixed elapsed value that tests can advance). `ScanService::scan_duration_ms` MUST read elapsed time through this port.

#### Scenario: SystemStopwatch measures real elapsed time

- GIVEN a `SystemStopwatch` and a handle from `start()`
- WHEN 50 ms passes (via `std::thread::sleep`)
- THEN `elapsed_ms(handle)` returns a value >= 50

#### Scenario: ScanService uses the port

- GIVEN `ScanService::scan_duration_ms` is called
- WHEN inspecting the implementation
- THEN it MUST obtain a handle from the injected stopwatch, not call `Instant::now()` directly

### Requirement: AnalysisDataSource port

The system MUST expose an `AnalysisDataSource` port that returns neutral data (e.g. `Vec<FileMeta>`, `Vec<ImportEdge>`) consumed by the `engine::analysis` pure functions. The functions in `engine::analysis/{architecture_detector,impact_engine,graph_insights}` MUST NOT take `&crate::db::DbPool`. The `AnalysisRepository::pool()` back door MUST be removed. `AnalysisDataSourceAdapter` MUST be the only path from `AnalysisService` to analysis data.

#### Scenario: Analysis functions are pure data in / data out

- GIVEN `compute_impact(changed_node_id, files, edges, history)` is called
- WHEN it runs
- THEN the function signature MUST NOT include `&DbPool` or any `crate::db` type
- AND the same input MUST produce the same output regardless of database state

#### Scenario: pool() back door is removed

- GIVEN the wave-2 boundary state
- WHEN `rg "fn pool" engine/src/ports.rs` runs
- THEN zero matches appear in `AnalysisRepository`
- AND `AnalysisRepositoryAdapter::pool` no longer exists

### Requirement: FileSourceReader port

The system MUST expose a `FileSourceReader` port with `read(path: &Path) -> Result<String>` and `exists(path: &Path) -> bool`. `GraphService::get_node_outline` MUST read source files through this port and MUST NOT call `std::fs::read_to_string` directly. A `FileSourceReaderAdapter` (delegates to `std::fs`) is the production implementation.

#### Scenario: GraphService reads via the port

- GIVEN `GraphService::get_node_outline` is called with a node ID
- WHEN the service loads the source file
- THEN it MUST call `file_source.read(&abs_path)`, not `std::fs::read_to_string(&abs_path)`
- AND `rg "std::fs::read_to_string" engine/src/services` returns zero matches in the service layer

### Requirement: Pub(crate) port traits

> **STATUS: Deferred to wave 3** (discovered during C1.1 apply on 2026-06-10; see ADR-009 in `openspec/changes/wave-2-hexagonal-completion/design.md` and Engram observation #681 for the discovery log).
>
> **Why deferred**: in Rust, `pub(crate)` restricts visibility to the **same crate**. `engine` and `src-tauri` are **separate crates** (not a Cargo workspace), so making the 6 port traits `pub(crate)` in `engine` makes them **inaccessible** from `src-tauri/src/commands.rs`, where `AppState` is declared with typed `Arc<dyn Trait>` fields that name the trait. 17 compilation errors confirm the constraint.
>
> **Wave 3 plan**: when the multi-crate split (`engine` → `codeatlas-domain` + `codeatlas-application` + `codeatlas-infrastructure`, per proposal #674 deferred section) lands, the trait definitions live inside `codeatlas-domain` and `src-tauri` consumes them via the `codeatlas-application` public API. At that point `pub(crate)` is meaningful (applied to the `codeatlas-application` crate) and the visibility tightening can land.
>
> **What still holds in wave 2**: the hexagonal boundary is enforced by the type system — `src-tauri` does not name `ProjectRepository` or any concrete adapter, only `Arc<dyn Port>` and `AppStatePortAdapter`. The trait visibility is `pub` for now (and this requirement documents the **wave-3 target state**, not the wave-2 state).

The **six** port traits `ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AnalysisRepository`, `AppStatePort`, and `AIServicePort` MUST eventually be declared `pub(crate)` in their respective modules (currently `engine::ports` and `engine::ai::service`). External crates (e.g. `src-tauri`) MUST NOT be able to name these traits; they consume ports only through `Arc<dyn Port>`. Adapters MAY keep their own visibility (e.g. `pub` for crate-internal tests). All existing integration tests (10 files, ~2896 lines) MUST continue to pass.

#### Scenario: External crate cannot import a port trait

- GIVEN `src-tauri/src/commands.rs` after the migration
- WHEN the file is compiled
- THEN `use engine::ports::ScanRepository;` produces a "module `ports` is private" visibility error
- AND the file MUST obtain ports only via `state.scan_repo: Arc<dyn ScanRepository>`

#### Scenario: Integration tests pass after pub(crate) migration

- GIVEN the full `engine/tests/*.rs` suite
- WHEN `cargo test --tests` runs
- THEN all 86 integration tests pass
- AND no test imports a port trait from outside the `engine` crate
