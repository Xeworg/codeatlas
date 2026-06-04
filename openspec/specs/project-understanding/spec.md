# Project Understanding MVP Specification

## Purpose

Definir el comportamiento aceptado de CodeAtlas v1 para comprensión rápida de proyectos mediante escaneo estático, grafo de dependencias file-level y explicación contextual con IA, manteniendo límites estrictos de alcance.

## Requirements

### Requirement: Static Project Scan

The system MUST escanear proyectos de forma estática y segura sin ejecutar código, scripts ni artefactos del proyecto analizado.

#### Scenario: Eligible files are indexed

- GIVEN una carpeta de proyecto válida
- WHEN el usuario inicia `scan_project`
- THEN el sistema MUST indexar archivos `.ts`, `.tsx`, `.js`, `.jsx`, `.rs` y `.json`
- AND el sistema MUST excluir `.git`, `node_modules`, `dist`, `build`, `.next` y `coverage`

#### Scenario: Untrusted input is handled safely

- GIVEN un proyecto con scripts o binarios ejecutables
- WHEN se procesa el escaneo
- THEN el sistema MUST tratar la entrada como no confiable
- AND el sistema MUST NOT ejecutar código del proyecto

### Requirement: Multi-language Parsing for MVP

The system MUST parsear TypeScript, JavaScript y Rust en v1 para extraer metadatos estructurales mínimos del grafo.

#### Scenario: Parser extracts supported symbols

- GIVEN archivos compatibles TS/JS/Rust
- WHEN finaliza el parseo
- THEN el sistema MUST extraer imports y exports
- AND el sistema SHOULD extraer funciones, clases (TS/JS), structs/impl (Rust), interfaces (TS) y enums cuando existan

### Requirement: File-level Dependency Graph

The system MUST construir un grafo dirigido a nivel archivo donde nodos representan archivos y aristas representan imports.

#### Scenario: Graph is produced from imports

- GIVEN un conjunto de archivos con imports resolubles
- WHEN se construye el grafo
- THEN el sistema MUST crear nodos por archivo
- AND el sistema MUST crear aristas dirigidas por relación de import
- AND el sistema MUST NOT incluir nodos intra-archivo (clase/función) en v1

### Requirement: Interactive Graph Exploration

The system MUST proveer exploración visual interactiva del grafo en el layout v1.

#### Scenario: User navigates and inspects graph

- GIVEN un grafo generado
- WHEN el usuario interactúa en la vista central
- THEN el sistema MUST soportar zoom, pan, búsqueda, auto-layout y selección de nodo
- AND el sistema SHOULD resaltar dependencias del nodo seleccionado o enfocado

### Requirement: Explorer and Node Details Synchronization

The system MUST sincronizar el explorer read-only con la selección del grafo y mostrar detalles del nodo activo.

#### Scenario: Explorer selection focuses graph node

- GIVEN un archivo visible en el explorer
- WHEN el usuario selecciona ese archivo
- THEN el sistema MUST enfocar/seleccionar el nodo equivalente en el grafo
- AND el panel de detalles MUST mostrar path, símbolos, dependencias, dependientes y tipo de nodo

### Requirement: Contextual AI Assistant

The system MUST ofrecer explicación por nodo y chat contextual usando contexto acotado, con proveedor primario Anthropic y modelo inicial MiniMax en v1.

#### Scenario: Explain node uses bounded context

- GIVEN un nodo seleccionado
- WHEN el usuario solicita explicación
- THEN el sistema MUST enviar como contexto solo el archivo objetivo, top-5 dependencias y top-3 dependientes
- AND el sistema MUST limitar el contenido del archivo a ~8KB
- AND el sistema MUST NOT enviar el proyecto completo al proveedor IA

#### Scenario: Chat context remains project-grounded

- GIVEN una conversación activa del proyecto
- WHEN el usuario consulta relaciones o responsabilidades
- THEN la respuesta MUST basarse en contexto del proyecto escaneado
- AND el historial de chat MUST mantenerse en memoria en v1 (sin persistencia)

### Requirement: MVP Data Persistence Boundary

The system MUST persistir únicamente el conjunto mínimo de datos definido para v1.

#### Scenario: Minimal schema is enforced

- GIVEN una ejecución de escaneo y construcción de grafo
- WHEN se persisten resultados
- THEN el sistema MUST usar tablas `projects`, `files`, `symbols`, `imports`, `graph_cache` y `ai_config`
- AND el sistema MUST NOT requerir `chat_history` ni `user_settings` avanzadas en v1

### Requirement: Performance and Responsiveness Targets

The system MUST cumplir objetivos de performance del MVP en condiciones objetivo.

#### Scenario: Target project performance budget

- GIVEN un proyecto objetivo de hasta 5000 archivos
- WHEN se ejecuta el flujo abrir→escanear→visualizar
- THEN el escaneo inicial MUST completar en menos de 10 segundos
- AND el primer diagrama SHOULD ser visible en menos de 30 segundos
- AND la interacción de grafo SHOULD mantener latencia percibida menor a 100ms
- AND respuestas IA SHOULD llegar en menos de 5 segundos bajo condiciones nominales

### Requirement: Explicit Out-of-Scope Enforcement

The system MUST mantener bloqueados los no-goals definidos para v1.

#### Scenario: Out-of-scope feature request appears during v1

- GIVEN una solicitud de feature v2/v3 (por ejemplo detección automática de patrones, export Mermaid/PNG/SVG, colaboración, multi-proyecto)
- WHEN se evalúa la implementación en v1
- THEN la solicitud MUST marcarse como fuera de alcance
- AND la implementación MUST diferirse a la versión planificada

## v2 Additions — v2-advanced-analysis (archived 2026-06-01)

### Requirement: Architecture Detection with Evidence

The system MUST provide architecture detection results for each analyzed project, including detected pattern, confidence score, and supporting evidence.

#### Scenario: Architecture detection returns classified result

- GIVEN a scanned project with enough structural signals
- WHEN the user requests architecture detection
- THEN the system MUST return one of `mvc`, `layered`, `clean`, `hexagonal`, or `unknown`
- AND the system MUST include a confidence value
- AND the system MUST include evidence traceable to project graph elements

#### Scenario: Detection degrades safely on failure

- GIVEN a project where detection cannot be computed reliably
- WHEN architecture detection fails
- THEN the system MUST return `unknown` with zero confidence
- AND the system MUST NOT crash the analysis flow

### Requirement: Impact Analysis

The system MUST provide impact analysis for a selected node/file and return affected nodes with a bounded impact score.

#### Scenario: Impact result is returned for selected node

- GIVEN a scanned project graph and a selected node
- WHEN the user requests impact analysis
- THEN the system MUST return the selected node identifier
- AND the system MUST return a list of affected nodes
- AND the system MUST return an impact score for the analysis result

### Requirement: Graph Insights (Cycles and Hotspots)

The system MUST compute graph insights including cycles and hotspots for the current project graph.

#### Scenario: Insights are computed successfully

- GIVEN a project graph with dependency relations
- WHEN graph insights are requested
- THEN the system MUST return cycles and hotspots
- AND the system MUST include aggregate graph metrics for coupling and density

#### Scenario: Insights degrade without blocking graph usage

- GIVEN a graph where insight computation times out or fails
- WHEN the failure occurs
- THEN the system MUST return an empty insights payload with explicit failure state
- AND the graph view MUST remain usable

### Requirement: Exportable Analysis Evidence

The system MUST support exporting analysis evidence in JSON and PNG formats for sharing.

#### Scenario: User exports JSON evidence

- GIVEN an analyzed project with graph and insights
- WHEN the user selects JSON export
- THEN the system MUST generate an export payload containing graph data
- AND the payload MUST include insights when available

#### Scenario: PNG export fallback

- GIVEN an analyzed project where PNG export cannot be produced
- WHEN PNG export fails
- THEN the system MUST provide a non-crashing fallback path to JSON export
- AND the user MUST receive a clear warning message

### Requirement: v2 Analytical Views and Persistent Filters

The system MUST provide analytical views for architecture, dependencies, and application flow (beta), and MUST persist filter selections across the active session.

#### Scenario: User switches analytical views

- GIVEN an analyzed project
- WHEN the user switches between architecture, dependencies, and flow views
- THEN each view MUST render the corresponding analysis perspective without re-scanning the project

#### Scenario: Filters persist during session

- GIVEN user-defined filters/groupings in analytical view
- WHEN the user navigates between explorer, graph, and details panels
- THEN the selected filters MUST remain applied during the session

### Requirement: v2 Contract Compatibility

The system MUST expose v2 analysis contracts (`ArchitectureDetectionResult`, `ImpactAnalysisResult`, `GraphInsights`, `ExportPayload`) without breaking v1 consumers.

#### Scenario: v2 contract fields are available

- GIVEN a v2-capable frontend client
- WHEN it invokes analysis commands
- THEN the returned payloads MUST match the declared v2 contract fields

#### Scenario: v1 compatibility is preserved

- GIVEN an existing v1 workflow (scan, graph, details, search)
- WHEN v2 features are present
- THEN v1 workflows MUST continue functioning without required breaking field renames or removals

### Requirement: Additive v2 Data Migration

The system MUST apply v2 database migration changes additively and preserve existing v1 data.

#### Scenario: Migration updates schema without data loss

- GIVEN a database with v1 project data
- WHEN migration `003_architecture_and_insights.sql` is applied
- THEN new v2 schema elements MUST be added
- AND existing v1 rows in `projects`, `files`, `symbols`, and `imports` MUST remain readable

### Requirement: i18n Foundation for Spanish Catalog

The system MUST externalize UI copy into a Spanish catalog and resolve strings through translation keys, while keeping language runtime fixed to Spanish in v2.

#### Scenario: UI strings resolve from catalog

- GIVEN the v2 UI is rendered
- WHEN user-visible text is shown
- THEN text MUST be resolved through translation keys backed by `locales/es.json`
- AND the UI MUST NOT depend on hardcoded user-facing strings in components for migrated surfaces

#### Scenario: No language switcher in v2

- GIVEN v2 runtime configuration
- WHEN the user navigates the UI
- THEN the active language MUST remain Spanish
- AND the system MUST NOT expose a language selector in v2

### Requirement: v3 Scope Exclusion Enforcement

The system MUST reject v3 collaboration and multi-project capabilities from v2 implementation scope.

#### Scenario: v3 feature proposed during v2

- GIVEN a proposal to add workspaces, snapshots, annotations, or health timeline
- WHEN evaluated for v2 inclusion
- THEN the feature MUST be marked out of scope for v2
- AND implementation planning MUST defer it to the v3 change set

## v3 Additions — v3-collaboration-platform (archived 2026-06-01)

### Requirement: H1 Hardening Gates from v2 Carry-Over

Before closing H1, the system MUST complete and verify all three carry-over gates inherited from archived v2 exceptions.

#### Scenario: Gate 1 — Real fixture and NFR benchmark evidence

- GIVEN v3 H1 is in progress
- WHEN NFR validation is executed
- THEN the system MUST run benchmarks on a real fixture of 1000+ files
- AND the system MUST record measurable evidence for scan, graph insights, and export timing against declared thresholds

#### Scenario: Gate 2 — Remaining degraded-mode frontend/IA scenarios

- GIVEN degraded-mode validation for v2/v3 compatibility
- WHEN test suites run
- THEN the system MUST cover pending frontend/IA scenarios: PNG fallback via mock, contract mismatch handling, AI not configured, and AI timeout
- AND each scenario MUST verify non-crashing fallback behavior and user-visible error/warning states

#### Scenario: Gate 3 — App-level wiring T5.6

- GIVEN analytical components are wire-ready
- WHEN H1 is completed
- THEN `App.tsx` MUST integrate `AnalyticsViewSelector`, `ArchitectureCard`, `ImpactPanel`, and `InsightsPanel`
- AND users MUST be able to reach those views through the main app flow without manual patching

### Requirement: H1 Multi-Project Workspaces Foundation

The system MUST support workspace-level organization for multiple projects in v3 H1.

#### Scenario: Workspace groups projects

- GIVEN a user with more than one scanned project
- WHEN the user manages workspace context
- THEN the system MUST allow associating projects to a workspace boundary
- AND project-level analysis data MUST remain isolated per project identity

### Requirement: H2 Collaboration Baseline

The system MUST provide local-first collaboration primitives in v3 H2 through snapshots and annotations.

#### Scenario: Snapshot creation and retrieval

- GIVEN an analyzed project state
- WHEN the user creates a snapshot with a label
- THEN the system MUST persist a snapshot artifact that can be listed and reloaded later

#### Scenario: Node annotations are persisted

- GIVEN a graph node in a project context
- WHEN the user adds a comment/annotation
- THEN the system MUST persist the annotation with author and timestamp metadata
- AND the annotation MUST be retrievable in later sessions

### Requirement: H3 Executive Insight Surfaces

The system MUST provide executive-level views in v3 H3 with health timeline and architecture comparison outputs.

#### Scenario: Health timeline is available by period

- GIVEN historical health records for a project or workspace
- WHEN the user requests a time window
- THEN the system MUST return a timeline including overall and component architecture health metrics

#### Scenario: Comparative architecture view

- GIVEN at least two snapshots
- WHEN the user requests a comparison
- THEN the system MUST return a diff-capable representation suitable for C4-assisted and snapshot comparison views

### Requirement: V3 Contract and Migration Consistency

The system MUST keep v3 contracts and migrations consistent with documented plans while preserving additive compatibility with v1/v2 data.

#### Scenario: Planned contracts are exposed without v1/v2 breakage

- GIVEN a v3-capable client
- WHEN it invokes v3 collaboration/executive commands
- THEN responses MUST match documented v3 contract families (`Snapshot`, `Comment`, `SharedView`, `HealthScoreTimeline`, `ExecutiveArchitectureSummary`)
- AND existing v1/v2 command flows MUST remain functional without required breaking renames/removals

#### Scenario: Planned migrations are additive and recoverable

- GIVEN a database containing v1/v2 data
- WHEN migrations `004_workspace_and_snapshots.sql`, `005_collaboration_annotations.sql`, and `006_health_timeline.sql` are applied
- THEN schema changes MUST be additive
- AND rollback/recovery MUST remain possible via documented backup procedure

### Requirement: V3 Scope Protection and Non-Goals

The system SHALL enforce v3 non-goals to prevent scope creep during execution.

#### Scenario: Out-of-scope request appears during v3

- GIVEN a request for cloud multi-tenant realtime sync, full CRDT/distributed conflict resolution, or unrelated v4 capability
- WHEN evaluated for inclusion in `v3-collaboration-platform`
- THEN the request MUST be marked out of scope
- AND planning MUST defer it to a future approved change

## Logging & Observability Additions — robust-logging-observability (synced 2026-06-03)

> Cross-cutting extension to v3 covering frontend error normalization, backend structured logging at scan lifecycle and graph build boundaries, dev per-execution file logging, and `RUST_LOG`-controlled verbosity. Purely additive: no v1/v2/v3 requirements above are modified or removed by this section.

### Requirement: Frontend error normalization

The system MUST ensure that errors thrown from any Tauri invoke call never render as `[object Object]` in the UI.

**Rationale:** `toApiError()` returns a plain `{ code, message }` object. When `catch (err)` in `App.tsx` or other components calls `setError(err)` and later renders `String(err)`, the result is `[object Object]` because plain objects don't have a useful `toString()`.

#### Scenario: API error is rendered safely

- GIVEN the user triggers a scan on a non-existent path
- WHEN the backend returns a Tauri error string or the frontend `catch (err)` block executes
- THEN the error displayed to the user MUST be a human-readable string (either `err.message`, `err.code + " — " + err.message`, or a known-error-label)
- AND the display MUST NOT contain the literal text `[object Object]`

#### Scenario: Non-Error thrown is handled gracefully

- GIVEN a Tauri invoke throws a non-`Error` value (e.g., a plain object `{ code: "INTERNAL", message: "..." }`)
- WHEN `getErrorMessage(err)` is called
- THEN the returned string MUST be the `.message` field if present, otherwise a fallback label (e.g., `"Unknown error"`)
- AND the code MUST NOT call `String(err)` directly on the raw caught value

### Requirement: Tauri API error shape contract

The system MUST expose a `getErrorMessage(err: unknown): string` helper in `src/lib/tauri-api.ts` that handles all thrown shapes returned by the backend.

**This does not change the backend contract.** The `{ code, message }` shape returned from `toApiError()` is preserved for code-detection logic. The helper only ensures safe message extraction for UI rendering.

#### Scenario: Error with `message` property is extracted

- GIVEN `err` is `{ code: "PATH_NOT_FOUND", message: "Path /foo not found" }`
- WHEN `getErrorMessage(err)` is called
- THEN the returned string MUST be `"PATH_NOT_FOUND — Path /foo not found"` or `"Path /foo not found"`

#### Scenario: Error without `message` property is handled

- GIVEN `err` is a primitive `"Connection refused"` or a non-standard object `{ reason: "..." }`
- WHEN `getErrorMessage(err)` is called
- THEN the returned string MUST be `"Connection refused"` or `"Unknown error"`, respectively
- AND no exception is thrown

### Requirement: Backend structured logging at scan lifecycle boundaries

The system MUST emit structured `tracing` log entries with consistent fields at the following scan lifecycle boundaries:

- **Scan start:** `INFO` level with `project_id`, `root_path`, `files_discovered`
- **Scan completion:** `INFO` level with `project_id`, `files_persisted`, `symbols_count`, `imports_count`, `duration_ms`
- **Scan error:** `ERROR` level with `project_id`, `root_path`, `error_detail`

#### Scenario: Successful scan emits lifecycle logs

- GIVEN the user triggers a scan on a valid project
- WHEN the scan completes successfully
- THEN the backend MUST emit at least two `INFO` log entries: one for scan start (with `files_discovered` count) and one for scan completion (with final counts and `duration_ms`)
- AND no `[object Object]` or raw panic strings appear in the log output

#### Scenario: Failed scan emits error log with context

- GIVEN the user triggers a scan on a valid project
- WHEN `repo.save_scan_result()` fails and returns `Err`
- THEN the backend MUST emit an `ERROR` (not `DEBUG`) log entry that includes `root_path` and a human-readable `error_detail`
- AND the command MUST still return the error string to the frontend (no silent swallow)

### Requirement: Backend DB persistence error logging

The system MUST emit structured `tracing::debug` logs when individual DB persistence operations fail within a scan, so that degraded scans can be diagnosed without flooding INFO-level logs.

#### Scenario: Import persistence failure is debug-logged

- GIVEN the scan is processing import edges
- WHEN `repo.save_import(imp)` returns `Err(e)` for a specific import
- THEN the backend MUST emit a `DEBUG` log containing the import's `source_file_id`, `target_module`, and the error string
- AND the scan MUST continue processing remaining imports
- AND the final scan result MUST reflect the degraded state (`imports_count` reflects only persisted imports, `error` field is set)

#### Scenario: Outline persistence failure is debug-logged

- GIVEN the scan is processing outline items for a file
- WHEN `repo.save_outline_items()` returns `Err(e)`
- THEN the backend MUST emit a `DEBUG` log containing the `file_id` and error string
- AND the scan MUST continue processing remaining files

**Noise policy:** These debug logs are emitted per-failure, which could be thousands in a degraded scan. They MUST be gated behind `RUST_LOG=debug`. Default `RUST_LOG=info` MUST NOT show per-file/per-import failure logs.

### Requirement: Backend graph build logging

The system MUST emit structured `tracing` logs around graph construction.

#### Scenario: Graph cache hit

- GIVEN `get_graph` is called with a `project_id` that has a cached graph
- WHEN the cached graph is found and returned
- THEN the backend MUST emit an `INFO` log with `project_id`, `cache_hit: true`, and `elapsed_ms`

#### Scenario: Graph cache miss and fresh build

- GIVEN `get_graph` is called with a `project_id` that has no cached graph
- WHEN the graph is built fresh from DB
- THEN the backend MUST emit an `INFO` log with `project_id`, `cache_hit: false`, `nodes_count`, `edges_count`, `imports_considered`, and `elapsed_ms`

#### Scenario: Graph build with no files

- GIVEN `get_graph` is called with a `project_id` that exists in DB but has no files
- WHEN the builder returns an empty graph
- THEN the backend MUST emit a `WARN` log with `project_id` indicating no files were found
- AND the command MUST return an error to the frontend (not an empty graph with a 200 OK)

### Requirement: `projects.root_path` conflict logging

The system MUST log structured context when a `projects.root_path` UNIQUE constraint violation occurs, so that developers can identify which conflicting path caused the failure.

#### Scenario: Duplicate root_path conflict

- GIVEN a scan is initiated on a path that already exists in the `projects` table
- WHEN `repo.save_scan_result()` catches a DB constraint error for `root_path`
- THEN the backend MUST emit a `WARN` (not `DEBUG`) log containing the conflicting `root_path` value and the constraint error detail
- AND the frontend MUST receive an error message that references `root_path` conflict (e.g., `"Project already exists at path: {root_path}"` rather than a raw SQLite error code)

**Note:** This may require catching the SQLite error in `save_scan_result` or the command layer and re-mapping it. The raw SQLite error (e.g., `UNIQUE constraint failed: projects.root_path`) MUST NOT propagate directly to the frontend.

### Requirement: Log level configuration via `RUST_LOG`

The system MUST support `RUST_LOG` environment variable to control log verbosity, with `INFO` as the default.

- `RUST_LOG=info` (default): Shows `INFO`, `WARN`, `ERROR` logs; suppresses `DEBUG` logs.
- `RUST_LOG=debug`: Shows `DEBUG`, `INFO`, `WARN`, `ERROR` logs including parser-miss and per-failure persistence logs.
- `RUST_LOG=warn`: Shows only `WARN`, `ERROR` logs; suppresses `INFO` and `DEBUG`.

#### Scenario: Default log level is INFO

- GIVEN no `RUST_LOG` is set
- WHEN the backend starts
- THEN the tracing subscriber MUST be initialized so that only `INFO`, `WARN`, and `ERROR` messages are printed
- AND `DEBUG` messages MUST be suppressed

#### Scenario: Debug level enables parser miss logging

- GIVEN `RUST_LOG=debug` is set
- WHEN `CodeParser` encounters a file it cannot parse or a language variant it doesn't handle
- THEN the backend MAY emit a `DEBUG` log describing the parser miss (e.g., `"Unsupported syntax in {file_path}: {reason}"`)
- AND this logging MUST NOT appear in default (INFO) mode

### Requirement: Command error returns preserve human-readable context

The system MUST ensure that any `String` returned as a `Result<_, String>` error from a Tauri command contains a human-readable message, not a raw SQLite error code, Rust enum variant, or debug output.

#### Scenario: Database error is mapped to user-facing message

- GIVEN a DB operation (save, query, migration) fails with an error
- WHEN the error propagates to a Tauri command return
- THEN the returned `String` MUST be derived from `e.to_string()` where `e` is a meaningful error type (e.g., `rusqlite::Error`, custom error enum with Display impl), not a debug-format struct
- AND the string MUST NOT include internal field names like `Error { code: ...` unless those fields are intentionally user-facing

**Risk flag:** Changing error formatting in `commands.rs` `map_err(|e| e.to_string())` across many commands could affect existing error handling. This change should be applied surgically per command, with test coverage.

### Requirement: Optional debug parser miss logging (out of scope for Tree-sitter adaptation)

The system MAY emit debug-level logs when a Tree-sitter parser fails to recognize a syntax construct, provided the log is behind the `RUST_LOG=debug` gate and does not include source code snippets.

**Note:** This requirement covers Phase 1 debug logging only. Tree-sitter parser improvements (Phase 2) are out of scope and will be handled in a separate change spec.

#### Scenario: Parser miss in debug mode

- GIVEN `RUST_LOG=debug` is set
- WHEN `CodeParser::parse_file` encounters a TSX file with a method-like form it cannot categorize
- THEN the backend MAY log at `DEBUG` level: `"Parser miss: file={path} reason=\"unhandled syntax kind: {kind}\""`
- AND the log MUST NOT include the file's source code content

#### Scenario: Parser miss in default mode is silent

- GIVEN `RUST_LOG=info` (default) is set
- WHEN `CodeParser::parse_file` encounters an unrecognized syntax construct
- THEN no log MUST be emitted for this event
- AND the parse MUST continue or gracefully degrade without error
