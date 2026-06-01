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
