# Changelog de Contratos API

## v1.1 (deferred to v3 — skipped in v1/v2 cycle)

- `scan_project`: agregar campo opcional `exclude_patterns?: string[]`
- `GraphNode`: agregar campo opcional `group?: string`
- `chat`: agregar campo opcional `session_id?: string`
- Nuevos comandos: `list_projects`, `delete_project`

> **Nota:** estos features no fueron priorizados en v1 ni v2. Planificados para v3.

## v2.0 (implementado — 2026-06-01, PR1–PR6 de v2-advanced-analysis)

- `get_architecture_detection(projectId) → ArchitectureDetectionResult` — implementado en `architecture_detector.rs` + comando Tauri
- `get_impact_analysis(projectId, nodeId) → ImpactAnalysisResult` — implementado en `impact_engine.rs` + comando Tauri
- `get_graph_insights(projectId) → GraphInsights` — implementado en `graph_insights.rs` + comando Tauri
- `get_export(projectId, format) → ExportPayload` — implementado en `export_view` + comando Tauri
- `GraphData`: agrega campo opcional `insights?: GraphInsights`
- Nuevo `NodeType` v2: `middleware`, `guard`, `interceptor`
- Migración aditiva `003_architecture_and_insights.sql` — 6 tests de integración pasando
- i18n foundation: `locales/es.json` (70+ keys) + helper `t()`

**Excepciones aceptadas post-v2:**

- NFR benchmarks: solo scaffold, sin mediciones reales en fixture
- Degraded-mode: 4/8 escenarios backend cubiertos; 4 frontend/IA diferidos a hardening
- App-level wiring (T5.6): componentes wire-ready pero no integrados en `App.tsx`

## v3.0 (planificado)

- `create_snapshot(projectId, label) → Snapshot`
- `add_comment(nodeId, text, author) → Comment`
- `share_view(viewId, recipients) → ShareLink`
- `get_health_timeline(projectId, from, to) → HealthScoreTimeline`
- `get_executive_summary(workspaceId) → ExecutiveArchitectureSummary`

## v1.0 (MVP — implementado)

### Comandos disponibles (Tauri invoke)

**Proyecto y Escaneo**

- `scan_project(path) → ScanResult` — incluye `scan_duration_ms`
- `get_scan_status(projectId) → ScanStatus`
- `cancel_scan(projectId) → void`

**Grafo**

- `get_graph(projectId) → GraphData` — con logging de timing
- `get_node_details(nodeId) → FileInfo`
- `search_nodes(projectId, query, limit) → GraphNode[]`
- `get_dependencies(nodeId) → GraphEdge[]`
- `get_dependents(nodeId) → GraphEdge[]`

**IA**

- `explain_node(nodeId, projectId) → NodeExplanation`
- `chat(projectId, message, history, contextNodeIds?) → ChatResponse`
- `configure_ai(config) → void`
- `get_ai_config() → AIConfig` (sin `api_key` en respuesta)

### Métricas de timing (medidas en backend Rust)

- `scan_project.discover_ms`: tiempo de descubrimiento de archivos
- `scan_project.parse_ms`: tiempo de parsing con Tree-sitter
- `scan_project.total_ms`: suma total
- `get_graph.build_ms`: tiempo de construcción del grafo (loggeado en tracing)

### Tipos de errores (`ErrorCode` en TS)

- `PATH_NOT_FOUND`: archivo/carpeta no encontrada
- `ACCESS_DENIED`: sin permisos
- `SCAN_TIMEOUT`: timeout de escaneo
- `INVALID_KEY`: API key inválida
- `UNREACHABLE`: proveedor de IA inalcanzable
- `RATE_LIMITED`: límite de peticiones
- `TOKEN_LIMIT`: contexto demasiado largo
- `INTERNAL`: error interno

### Contratos de UI (estados por componente)

- **Loading**: spinner con mensaje descriptivo
- **Empty**: icono + título + descripción + acción sugerida
- **Error**: mensaje descriptivo + acción (retry/retry-config)

### Contratos de testing

- Contratos de comandos Tauri: `tests/integration/contracts.test.ts`
- Benchmarks informativos: `tests/benchmarks/bench_scan.rs`
- E2E manual: `tests/e2e/checklist.md`

### Errores conocidos (v1.0)

- `get_dependencies` y `get_dependents` no implementados en backend (deferred a v3/backlog)
- `cancel_scan` no cancela realmente el scan (deferred a v3/backlog)
