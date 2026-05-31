# Design — v1-mvp-core

## Estado
Aprobado para implementación (v1).

## Alcance de diseño
Este diseño cubre **solo v1** del MVP según spec canónica:
`openspec/changes/v1-mvp-core/specs/project-understanding/spec.md`.

Incluye: escaneo estático, parseo TS/JS/Rust, grafo file-level, exploración interactiva, detalles de nodo, explain/chat IA contextual, persistencia mínima.

---

## 1) Architecture slices (verticales)

### Slice A — Ingesta y análisis estático
**Objetivo:** de carpeta seleccionada a metadatos persistidos.
- `scanner/walker.rs`: recorre árbol, aplica exclusiones y extensiones permitidas.
- `scanner/parser.rs`: Tree-sitter TS/JS/Rust, extrae imports/exports/símbolos mínimos.
- `db/queries.rs`: persiste `projects/files/symbols/imports`.

### Slice B — Construcción de grafo
**Objetivo:** producir `GraphData` file-level para UI.
- `graph/resolver.rs`: resuelve imports relativos + aliases TS.
- `graph/builder.rs`: nodos por archivo, aristas por import.
- `db/graph_cache`: cache serializada por `project_id`.

### Slice C — Exploración en UI
**Objetivo:** navegar y entender dependencias.
- `src/components/graph/*`: React Flow (zoom/pan/search/layout/highlight).
- `src/components/layout/Sidebar.tsx`: explorer read-only.
- `src/components/panel/DetailPanel.tsx`: metadata del nodo seleccionado.
- `stores/*`: `selectedNodeId`, estados de carga/error.

### Slice D — IA contextual
**Objetivo:** explicar nodo/chat sin exfiltrar proyecto completo.
- `ai/context.rs`: arma contexto acotado (archivo ~8KB + top-5 deps + top-3 dependents).
- `ai/provider.rs`: trait `AIProvider`.
- `ai/anthropic.rs`: proveedor primario Anthropic; modelo inicial MiniMax.
- comandos `explain_node` y `chat`.

### Slice E — Presentación de comandos Tauri
**Objetivo:** frontera estable UI↔engine.
- `tauri_commands.rs`: validación de input, orquestación de use-cases, mapeo de errores.
- `src/lib/tauri-api.ts`: wrappers tipados de `invoke`.

---

## 2) Module boundaries

### Rust (engine)
- **Domain:** `models/*` (sin DB/HTTP/tauri).
- **Application:** `graph/*`, `ai/context.rs`, orquestación en `lib.rs`.
- **Infrastructure:** `scanner/*`, `db/*`, `ai/anthropic.rs`.
- **Presentation:** `tauri_commands.rs`.

**Regla:** dependencias de afuera hacia adentro; sin imports inversos.

### Frontend (React)
- **Presentation:** `components/*`.
- **Application:** `hooks/*`, `stores/*`.
- **Contracts/helpers:** `lib/types.ts`, `lib/tauri-api.ts`, `lib/graph-layout.ts`.

**Regla:** componentes no llaman `invoke` directo; pasan por hook/store.

---

## 3) Data flow (end-to-end)

### 3.1 Scan → Graph
1. UI llama `scan_project(path)`.
2. Walker filtra archivos elegibles.
3. Parser extrae metadata por archivo.
4. DB persiste entidades mínimas.
5. Builder crea `GraphData` file-level y cachea.
6. UI llama `get_graph(projectId)` y renderiza.

### 3.2 Selección de nodo → detalles
1. Usuario selecciona archivo en explorer o nodo en grafo.
2. Store actualiza `selectedNodeId`.
3. UI pide `get_node_details(nodeId)`.
4. Panel inferior muestra path, símbolos, deps/dependents, NodeType.

### 3.3 Explain/Chat IA
1. UI invoca `explain_node` o `chat`.
2. `context.rs` construye contexto acotado.
3. `AIProvider` ejecuta request a Anthropic/MiniMax.
4. Respuesta retorna con texto y referencias de nodos.
5. UI renderiza markdown y maneja errores.

---

## 4) API contract touchpoints (v1)

Comandos Tauri mínimos:
- `scan_project(path) -> ScanResult`
- `get_scan_status(projectId) -> { status, progress }`
- `get_graph(projectId) -> GraphData`
- `get_node_details(nodeId) -> FileInfoExtended`
- `search_nodes(projectId, query, limit) -> GraphNode[]`
- `configure_ai(config) -> void`
- `get_ai_config() -> AIConfigPublic`
- `explain_node(nodeId, symbolId?) -> NodeExplanation`
- `chat(projectId, message, history, contextNodeIds?) -> ChatResponse`

Contratos canónicos en TS↔Rust deben evolucionar en el mismo PR.

---

## 5) Error model

### Rust
`AppError` tipado (`thiserror`) con categorías:
- `PathNotFound`, `AccessDenied`, `ScanTimeout`
- `Database`, `InvalidKey`, `Unreachable`, `RateLimited`, `TokenLimit`
- `Internal`

### Bridge UI
Mapeo a `ApiError { code, message, details? }`.

### UX
Todos los paneles cubren `loading/empty/error/ready` con acción sugerida de recuperación.

---

## 6) Performance strategy (v1)

Objetivos:
- scan < 10s (hasta 5k archivos)
- primer diagrama < 30s
- interacción grafo < 100ms
- IA < 5s nominal

Tácticas:
1. Excluir directorios pesados temprano en walker.
2. Parseo lineal optimizado; sin procesamiento semántico profundo.
3. Grafo file-level (no intra-archivo).
4. Cache `graph_cache` por `project_id`.
5. UI con agrupación colapsable y minimapa.
6. Medición por etapa: `scan_ms`, `parse_ms`, `graph_ms`, `render_ms`, `ai_ms`.

---

## 7) Test strategy alignment

Alineado con `docs/PLAN_CALIDAD_TESTS_BENCHMARKS.md` y estándares:

### Unit
- Rust: walker/parser/resolver/builder/context.
- TS: hooks/stores/helpers de layout.

### Integration
- Rust con fixtures reales TS/JS/Rust.
- Contract tests de comandos Tauri + snapshots de payload.
- TS con mocks de `tauri-api` para estados UI.

### Acceptance/E2E (v1)
- Abrir carpeta → scan → grafo visible.
- Seleccionar nodo desde explorer y desde grafo.
- Detalles correctos del nodo.
- Explain/chat contextual con límites de contexto.

### Quality gates
- `cargo fmt`, `cargo clippy -D warnings`, `cargo test`
- `npm run lint`, `npm run test`, `npm run typecheck`

---

## 8) Explicit non-goals (enforced)

No implementar en v1:
- detección automática de patrones
- nodos intra-archivo y aristas usage/calls
- export Mermaid/PNG/SVG
- health score/ciclos/hotspots automáticos
- colaboración, snapshots, multi-proyecto
- persistencia de `chat_history`
- edición/refactor de código

Cualquier PR que introduzca estos ítems se bloquea y se mueve a backlog v2/v3.

---

## 9) Rollout técnico v1 (slices)

1. **Slice A** Scanner+Parser+DB mínima.
2. **Slice B** Graph builder + `get_graph`.
3. **Slice C** UI explorer+graph+details.
4. **Slice D** AI context + explain/chat.
5. **Slice E** hardening de errores/performance + contract tests.

PR slicing guiado por budget 400 líneas (auto-forecast).

---

## 10) Decisions log (v1)
- Layout: Dagre en v1 (ELK diferido).
- Parser Rust: soportar `use` + `mod` para relaciones file-level.
- Scanner: ejecución single-thread inicial; evaluar paralelismo si no cumple budget.
- IA: Anthropic primario, MiniMax inicial, sin multi-proveedor simultáneo.
