# Tasks — v3-collaboration-platform

> Generated from: `proposal.md`, `specs/project-understanding/spec.md`, `design.md`.
> Language: español (convención proyecto). Out of scope explícito: cloud multi-tenant, CRDT, features v4.

---

## Review Workload Forecast

| Field                   | Value                                         |
| ----------------------- | --------------------------------------------- |
| Estimated changed lines | ~2400–2900                                    |
| 400-line budget risk    | High                                          |
| Chained PRs recommended | Yes                                           |
| Suggested split         | PR1 → PR2 → PR3 → PR4 → PR5 → PR6 → PR7 → PR8 |
| Delivery strategy       | auto-chain                                    |
| Chain strategy          | stacked-to-main                               |

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
```

**Justificación**: 3 hitos, 8 PRs, 3 migraciones DB, 5 familias de contratos nuevos, 3 gates de hardening. Cada PR debe caber en ~280–360 líneas cambiadas. H1 es bloqueante para H2/H3 por dependencia de gates.

---

## Dependency Graph

```
PR1 (migration 004 + workspace types + persistence)
 ├─► PR2 (App.tsx wiring T5.6 — gate 3)
 ├─► PR3 (degraded-mode FE/IA — gate 2)
 └─► PR4 (benchmark fixture + NFR evidence — gate 1)
      └─► PR5 (snapshot contracts + persistence + backend)
           ├─► PR6 (annotations + migration 005)
           └─► PR7 (health timeline + migration 006)
                └─► PR8 (executive summary + diff/c4 views)
```

**Critical path**: PR1 → PR4 (H1 gates) → PR5 → PR7 → PR8

---

## Test Strategy by Milestone

| Milestone | Backend checks                              | Frontend checks                       | Evidence command                                      |
| --------- | ------------------------------------------- | ------------------------------------- | ----------------------------------------------------- |
| H1        | `cargo test`, `cargo clippy`                | `npm run test`, `npm run typecheck`   | `cargo bench` + benchmark report                      |
| H2        | `cargo test`, snapshot/annotation roundtrip | `npm run test`, contract TS checks    | `cargo test -- snapshot` + `cargo test -- annotation` |
| H3        | `cargo test`, timeline/diff/c4 payloads     | `npm run test`, component smoke tests | `cargo test -- health` + `cargo test -- executive`    |

---

# H1 — Foundation + Hardening Gates

> **H1 exit rule**: NO se cierra H1 sin evidencia verificable de los 3 gates heredados de v2.

---

## PR1 — Workspace Domain + Migration 004

**Objetivo**: Crear tablas workspace/workspace_projects, tipos TS v3, wrappers Tauri y persistencia base de workspaces.

**Changed files estimate**: ~320 líneas

### Tareas

- [ ] **T1.1** Crear `engine/migrations/004_workspace_and_snapshots.sql`:
  - `CREATE TABLE IF NOT EXISTS workspaces (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL)`
  - `CREATE TABLE IF NOT EXISTS workspace_projects (workspace_id TEXT REFERENCES workspaces(id), project_id TEXT REFERENCES projects(id), PRIMARY KEY(workspace_id, project_id))`
  - `CREATE TABLE IF NOT EXISTS snapshots (id TEXT PRIMARY KEY, project_id TEXT REFERENCES projects(id), workspace_id TEXT, label TEXT, created_at TEXT NOT NULL, payload_json TEXT)`
  - `PRAGMA user_version = 4;`

- [ ] **T1.2** Actualizar framework de migraciones en `engine/src/db/migrations.rs`:
  - Registrar `004_workspace_and_snapshots.sql` en el vector de migraciones.
  - Verificar que la secuencia 003→004 aplica en orden.
  - Backup automático antes de 004.

- [ ] **T1.3** Agregar contratos v3 en `src/lib/types.ts`:
  - `Workspace` (`id`, `name`, `createdAt`)
  - `WorkspaceProject` (`workspaceId`, `projectId`)
  - `Snapshot` (`id`, `projectId`, `workspaceId?`, `label`, `createdAt`, `payloadJson?`)
  - `Comment` (`id`, `projectId`, `nodeId`, `author`, `kind`, `text`, `createdAt`) — placeholder para H2
  - `HealthScoreTimeline` — placeholder para H3
  - `ExecutiveArchitectureSummary` — placeholder para H3

- [ ] **T1.4** Agregar wrappers v3 en `src/lib/tauri-api.ts`:
  - `createWorkspace(name): Promise<Workspace>`
  - `listWorkspaces(): Promise<Workspace[]>`
  - `attachProjectToWorkspace(workspaceId, projectId): Promise<void>`
  - `listWorkspaceProjects(workspaceId): Promise<WorkspaceProject[]>`
  - `createSnapshot(projectId, label): Promise<Snapshot>`
  - `listSnapshots(projectId): Promise<Snapshot[]>`

- [ ] **T1.5** Agregar queries workspace en `engine/src/db/queries.rs`:
  - `create_workspace(name) -> Workspace`
  - `list_workspaces() -> Vec<Workspace>`
  - `attach_project_to_workspace(workspace_id, project_id)`
  - `list_workspace_projects(workspace_id) -> Vec<WorkspaceProject>`

- [ ] **T1.6** Registrar comandos workspace en `engine/src/commands.rs`:
  - `create_workspace`, `list_workspaces`, `attach_project_to_workspace`, `list_workspace_projects`
  - `create_snapshot` (stub: persiste payload vacío, se completa en PR5)
  - `list_snapshots` (stub: retorna lista vacía, se completa en PR5)

- [ ] **T1.7** Tests:
  - Migration 004: aplicar sobre DB con datos v1/v2 → tablas creadas, datos intactos.
  - Workspace CRUD: crear, listar, attach, listar proyectos.
  - Idempotencia: migrar 2 veces no rompe.
  - `cargo test` verde, `npm run test` verde.

**Dependencies**: Ninguna (primer slice de v3).
**Acceptance**: `cargo test` verde, `npm run test` verde, `cargo clippy` limpio, workspace CRUD funcional.

---

## PR2 — App.tsx Wiring T5.6 (H1 Gate 3)

**Objetivo**: Integrar `AnalyticsViewSelector`, `ArchitectureCard`, `ImpactPanel`, `InsightsPanel` en el flujo principal de `App.tsx`. Cerrar gate heredado de v2.

**Changed files estimate**: ~250 líneas

### Tareas

- [ ] **T2.1** Integrar `AnalyticsViewSelector` en `App.tsx`:
  - Colocar selector de vistas en toolbar/principal area.
  - Navegación entre vistas architecture / dependencies / flow-beta.
  - Sin re-scan al cambiar vista.

- [ ] **T2.2** Integrar `ArchitectureCard` en flujo principal:
  - Mostrar tarjeta cuando hay `architecture_detection` disponible.
  - Ocultar gracefully si no hay datos (sin error).

- [ ] **T2.3** Integrar `ImpactPanel` en flujo principal:
  - Selección de nodo en Explorer dispara impacto.
  - Mostrar panel con affected nodes + explanation.
  - Highlight en grafo del set impactado.

- [ ] **T2.4** Integrar `InsightsPanel` en flujo principal:
  - Pestañas: Ciclos / Hotspots / Métricas.
  - Click en ciclo/hotspot navega a nodo en grafo.

- [ ] **T2.5** Feature flag `v3_h1`:
  - Wrap integraciones en flag condicional.
  - Si flag desactivado: comportamiento v2 preservado.
  - Default: activado en development.

- [ ] **T2.6** Smoke tests UI:
  - Test: navegar a vista architecture → ArchitectureCard visible.
  - Test: seleccionar nodo → ImpactPanel se muestra.
  - Test: vista insights → paneles renderizan sin error.
  - Test: feature flag desactivado → componentes no visibles.

**Dependencies**: PR1 (usa tipos v3 y store).
**Acceptance**: `npm run test` verde, componentes accesibles desde flujo principal, feature flag funcional.

---

## PR3 — Degraded-Mode Frontend/IA (H1 Gate 2)

**Objetivo**: Cubrir los 4 escenarios de degraded-mode faltantes. Cerrar gate heredado de v2.

**Changed files estimate**: ~280 líneas

### Tareas

- [ ] **T3.1** Test: PNG fallback via mock:
  - Mock `html-to-image` para que falle.
  - Verificar: fallback a JSON export con warning visible.
  - Verificar: UI no crashea, usuario puede descargar JSON.

- [ ] **T3.2** Test: Contract mismatch:
  - Mock respuesta Tauri con versión de contrato incorrecta.
  - Verificar: banner "Update required" visible.
  - Verificar: no se realizan llamadas con contrato stale.

- [ ] **T3.3** Test: IA no configurada:
  - Mock `get_ai_config` retornando `configured: false`.
  - Verificar: panel AI oculto.
  - Verificar: banner "Configure API key in Settings" visible.
  - Verificar: grafo e insights siguen funcionando.

- [ ] **T3.4** Test: IA timeout:
  - Mock `chat` con delay > timeout.
  - Verificar: error en chat panel, no bloquea UI.
  - Verificar: retry disponible.

- [ ] **T3.5** Actualizar matriz degraded-mode en `V2_READY_CHECKLIST.md`:
  - Marcar los 4 escenarios como cubiertos (8/8 total).
  - Referenciar evidencia de tests.

**Dependencies**: PR1 (usa wrappers v3 para simular respuestas).
**Acceptance**: `npm run test` verde con los 4 escenarios pasando, matriz 8/8 completa.

---

## PR4 — Benchmark Fixture + NFR Evidence (H1 Gate 1)

**Objetivo**: Crear fixture real de 1000+ archivos, ejecutar benchmarks contra umbrales documentados y generar evidencia trazable. Cerrar gate heredado de v2.

**Changed files estimate**: ~300 líneas

### Tareas

- [ ] **T4.1** Crear fixture de benchmark:
  - Generar directorio `tests/fixtures/benchmark-project/` con 1000+ archivos TypeScript.
  - Estructura realista: src/, components/, services/, utils/, tests/.
  - Archivos con imports/symbols para benchmarks de scan y grafo.
  - Versionar fixture en repo (no generación procedural en runtime).

- [ ] **T4.2** Benchmark: architecture detection:
  - Ejecutar sobre fixture → medir latencia.
  - Threshold: < 3s para 1000+ archivos.
  - Guardar resultado en `tests/benchmarks/results/architecture_detection.json`.

- [ ] **T4.3** Benchmark: graph insights:
  - Ejecutar sobre grafo generado del fixture → medir latencia.
  - Threshold: < 2s para 2000+ nodos.
  - Guardar resultado en `tests/benchmarks/results/graph_insights.json`.

- [ ] **T4.4** Benchmark: export JSON:
  - Ejecutar export sobre fixture → medir latencia.
  - Threshold: < 5s.
  - Guardar resultado en `tests/benchmarks/results/export_json.json`.

- [ ] **T4.5** Benchmark: impact analysis:
  - Ejecutar sobre nodo central del fixture → medir latencia.
  - Threshold: < 5s para single-node change.
  - Guardar resultado en `tests/benchmarks/results/impact_analysis.json`.

- [ ] **T4.6** Benchmark: WAL concurrency:
  - Test: 10 lecturas paralelas sobre DB con fixture → 0 deadlocks.
  - Guardar resultado en `tests/benchmarks/results/wal_concurrency.json`.

- [ ] **T4.7** Generar reporte de evidencia:
  - Consolidar resultados en `tests/benchmarks/benchmarks.md`.
  - Formato: métrica, threshold, resultado, PASS/FAIL.
  - Referenciar desde `verify-report` posterior.

**Dependencies**: PR1 (usa workspace + scan sobre fixture).
**Acceptance**: `cargo test` verde, benchmarks ejecutados con PASS en todos los umbrales, reporte de evidencia generado.

---

## H1 Gate Verification (review-time)

Antes de continuar a H2, verificar:

- [ ] **Gate 3**: App.tsx integra AnalyticsViewSelector, ArchitectureCard, ImpactPanel, InsightsPanel. Smoke tests pasan. (PR2)
- [ ] **Gate 2**: Degraded-mode matriz 8/8 escenarios con tests. (PR3)
- [ ] **Gate 1**: Fixture 1000+ archivos creado, benchmarks ejecutados con PASS, evidencia documentada. (PR4)
- [ ] Workspace CRUD funcional. (PR1)
- [ ] Todos los PRs de H1: `cargo test` + `npm run test` + `cargo clippy` + `npm run typecheck` + `npm run lint` verdes.

---

# H2 — Collaboration Baseline

---

## PR5 — Snapshot Contracts + Persistence + Backend

**Objetivo**: Implementar creación/list/load de snapshots con persistencia SQLite y comandos Tauri.

**Changed files estimate**: ~320 líneas

### Tareas

- [ ] **T5.1** Completar queries snapshot en `engine/src/db/queries.rs`:
  - `create_snapshot(project_id, label, workspace_id?) -> Snapshot`
  - `list_snapshots(project_id?, workspace_id?) -> Vec<Snapshot>`
  - `get_snapshot(snapshot_id) -> Snapshot`
  - Serializar payload como JSON comprimido opcional.

- [ ] **T5.2** Completar comandos snapshot en `engine/src/commands.rs`:
  - Reemplazar stubs de PR1 con implementación real.
  - `create_snapshot`: valida projectId, genera snapshot con timestamp.
  - `list_snapshots`: filtra por projectId o workspaceId.
  - `get_snapshot`: retorna snapshot completo con payload.

- [ ] **T5.3** Snapshot payload capture:
  - Capturar estado actual del grafo + insights como payload JSON.
  - Incluir: nodes, edges, insights, architecture_detection.
  - Excluir: AI chat history, ephemeral state.

- [ ] **T5.4** Frontend: componente `SnapshotManager`:
  - Crear snapshot con label.
  - Listar snapshots existentes.
  - Cargar snapshot (reemplaza vista actual con datos del snapshot).

- [ ] **T5.5** Frontend: store `useSnapshotStore`:
  - Estado: `snapshots[]`, `activeSnapshotId?`, `isLoading`.
  - Acciones: `createSnapshot`, `listSnapshots`, `loadSnapshot`, `clearActiveSnapshot`.

- [ ] **T5.6** Tests:
  - Backend: roundtrip crear → listar → obtener → payload completo.
  - Backend: snapshot de proyecto sin datos → payload vacío sin error.
  - Frontend: componente renderiza, crear funciona, listar carga.
  - Contract test: shape de `Snapshot` coincide entre TS y Rust.

**Dependencies**: PR1 (usa workspace + migration 004).
**Acceptance**: `cargo test` verde, `npm run test` verde, snapshots persistidos y rehidratables.

---

## PR6 — Annotations + Migration 005

**Objetivo**: Implementar comentarios/anotaciones sobre nodos con persistencia y comandos Tauri.

**Changed files estimate**: ~300 líneas

### Tareas

- [ ] **T6.1** Crear `engine/migrations/005_collaboration_annotations.sql`:
  - `CREATE TABLE IF NOT EXISTS annotations (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, node_id TEXT NOT NULL, author TEXT NOT NULL, kind TEXT DEFAULT 'comment', text TEXT NOT NULL, created_at TEXT NOT NULL)`
  - `CREATE INDEX IF NOT EXISTS idx_annotations_project_node ON annotations(project_id, node_id)`
  - `CREATE INDEX IF NOT EXISTS idx_annotations_created ON annotations(created_at)`
  - `PRAGMA user_version = 5;`

- [ ] **T6.2** Registrar migración 005 en `engine/src/db/migrations.rs`.

- [ ] **T6.3** Agregar queries annotation en `engine/src/db/queries.rs`:
  - `add_comment(project_id, node_id, author, text, kind?) -> Comment`
  - `list_comments(project_id, node_id?) -> Vec<Comment>`
  - `delete_comment(comment_id) -> bool`

- [ ] **T6.4** Agregar comandos annotation en `engine/src/commands.rs`:
  - `add_comment(nodeId, text, author): Comment`
  - `list_comments(projectId, nodeId?): Comment[]`

- [ ] **T6.5** Frontend: componente `AnnotationPanel`:
  - Mostrar comentarios del nodo seleccionado.
  - Agregar comentario con campo texto + author.
  - Tipos visuales: comment (default), todo, review, issue.

- [ ] **T6.6** Frontend: store `useAnnotationStore`:
  - Estado: `comments[]`, `activeNodeId?`, `isLoading`.
  - Acciones: `addComment`, `listComments`, `setActiveNode`.

- [ ] **T6.7** Tests:
  - Migration 005: aplicar sobre DB con datos v1/v2/v3(H1) → tabla creada.
  - Backend: roundtrip add → list → delete.
  - Backend: listar por proyecto vs por nodo específico.
  - Frontend: componente renderiza, agregar/listar funciona.
  - Contract test: shape de `Comment` coincide entre TS y Rust.

**Dependencies**: PR5 (depende de que snapshots funcione para no mezclar migraciones).
**Acceptance**: `cargo test` verde, `npm run test` verde, anotaciones persistidas con autor/timestamp.

---

## H2 Gate Verification (review-time)

Antes de continuar a H3, verificar:

- [ ] Snapshot roundtrip completo: crear → listar → cargar → payload válido. (PR5)
- [ ] Annotation persistence: crear → listar por nodo → listar por proyecto. (PR6)
- [ ] Migraciones 004+005 aplican en secuencia sin conflictos.
- [ ] Contratos `Snapshot` y `Comment` sincronizados entre TS y Rust.
- [ ] Feature flag `v3_h2` funcional (componentes H2 ocultos si desactivado).

---

# H3 — Executive Insights

---

## PR7 — Health Timeline + Migration 006

**Objetivo**: Implementar pipeline de health records con persistencia, queries por rango temporal y comando Tauri.

**Changed files estimate**: ~350 líneas

### Tareas

- [ ] **T7.1** Crear `engine/migrations/006_health_timeline.sql`:
  - `CREATE TABLE IF NOT EXISTS health_records (id TEXT PRIMARY KEY, workspace_id TEXT, project_id TEXT NOT NULL, recorded_at TEXT NOT NULL, overall_score REAL, coupling_score REAL, complexity_score REAL, cycle_count INTEGER, hotspot_count INTEGER)`
  - `CREATE INDEX IF NOT EXISTS idx_health_project_time ON health_records(project_id, recorded_at)`
  - `PRAGMA user_version = 6;`

- [ ] **T7.2** Registrar migración 006 en `engine/src/db/migrations.rs`.

- [ ] **T7.3** Health record capture pipeline:
  - Función `capture_health_record(project_id, workspace_id?)`:
    - Calcula overall_score (aggregación de coupling + complexity + cycle_count + hotspot_count).
    - coupling_score: promedio de grado de acoplamiento.
    - complexity_score: nodos con high cyclomatic complexity.
    - cycle_count: cantidad de ciclos detectados.
    - hotspot_count: nodos en top 10% de acoplamiento.
  - Persistir en `health_records`.

- [ ] **T7.4** Agregar queries health en `engine/src/db/queries.rs`:
  - `save_health_record(record) -> HealthRecord`
  - `get_health_timeline(project_id, from, to) -> HealthScoreTimeline`
  - `get_health_timeline_workspace(workspace_id, from, to) -> HealthScoreTimeline`

- [ ] **T7.5** Agregar comando en `engine/src/commands.rs`:
  - `get_health_timeline(projectId, from, to): HealthScoreTimeline`

- [ ] **T7.6** Frontend: componente `HealthTimeline`:
  - Render timeline con scores como chart (líneas/barras).
  - Selector de rango temporal (from/to).
  - Tooltip con detalle por score componente.

- [ ] **T7.7** Frontend: store `useHealthStore`:
  - Estado: `timeline[]`, `dateRange`, `isLoading`.
  - Acciones: `fetchTimeline`, `setDateRange`.

- [ ] **T7.8** Tests:
  - Migration 006: aplicar sobre DB con datos previos.
  - Health capture: calcula scores con datos conocidos.
  - Health query: rango sin datos → array vacío sin error.
  - Health query: rango con datos → timeline ordenada.
  - Frontend: componente renderiza, timeline muestra datos.
  - Contract test: shape de `HealthScoreTimeline` coincide.

**Dependencies**: PR5 (usa snapshots + graph data para calcular scores).
**Acceptance**: `cargo test` verde, `npm run test` verde, timeline consultable por rango.

---

## PR8 — Executive Summary + Diff/C4 Views

**Objetivo**: Implementar resumen ejecutivo por workspace, comparación entre snapshots y payloads C4 L1/L2.

**Changed files estimate**: ~350 líneas

### Tareas

- [ ] **T8.1** Executive summary aggregation:
  - Función `compute_executive_summary(workspace_id)`:
    - Aggrega: total_projects, total_files, avg_health_score, trend (up/down/stable).
    - Top hotspots across workspace.
    - Coupling evolution (últimos N health records).
  - Persistir como vista materializada o calcular on-demand con cache.

- [ ] **T8.2** Agregar queries executive en `engine/src/db/queries.rs`:
  - `compute_executive_summary(workspace_id) -> ExecutiveArchitectureSummary`
  - Cache opcional con TTL para evitar recálculo.

- [ ] **T8.3** Snapshot diff representation:
  - Función `compare_snapshots(base_id, target_id) -> SnapshotDiffPayload`:
    - Diff de nodos: added, removed, modified.
    - Diff de edges: added, removed.
    - Diff de métricas: coupling_delta, complexity_delta, cycles_delta.
    - Representación serializable para render frontend.

- [ ] **T8.4** C4 view adaptation:
  - Función `get_c4_view(project_id_or_snapshot_id, level) -> C4ViewPayload`:
    - Level 1 (System Context): sistemas externos + aplicación principal.
    - Level 2 (Container): módulos/contenedores internos del proyecto.
    - Mapear NodeType existente a abstracciones C4.
    - Fallback: si no hay datos suficientes → payload mínimo con aviso.

- [ ] **T8.5** Agregar comandos en `engine/src/commands.rs`:
  - `get_executive_summary(workspaceId): ExecutiveArchitectureSummary`
  - `compare_snapshots(baseSnapshotId, targetSnapshotId): SnapshotDiffPayload`
  - `get_c4_view(projectIdOrSnapshotId, level): C4ViewPayload`

- [ ] **T8.6** Frontend: componente `ExecutiveDashboard`:
  - Resumen ejecutivo: métricas clave, hotspots, trend.
  - Timeline de health embebido (reutiliza HealthTimeline de PR7).

- [ ] **T8.7** Frontend: componente `SnapshotDiffView`:
  - Dos snapshots seleccionables.
  - Visualización de diff: nodos added/removed/modified.
  - Delta de métricas con indicadores visuales.

- [ ] **T8.8** Frontend: componente `C4View`:
  - Selector de nivel (L1/L2).
  - Render adaptado del grafo a abstracciones C4.
  - Fallback si no hay datos suficientes.

- [ ] **T8.9** Tests:
  - Executive summary: workspace con proyectos → aggregación correcta.
  - Executive summary: workspace vacío → payload mínimo sin error.
  - Snapshot diff: mismos snapshots → diff vacío.
  - Snapshot diff: snapshots diferentes → added/removed correctos.
  - C4 view: proyecto con datos → payload con containers/systems.
  - C4 view level inválido → error controlado.
  - Frontend: componentes renderizan sin error.
  - Contract tests: shapes de todos los contratos H3 coinciden.

**Dependencies**: PR7 (usa health timeline para executive summary), PR5 (usa snapshots para diff).
**Acceptance**: `cargo test` verde, `npm run test` verde, timeline/diff/c4 payloads disponibles para UI.

---

## H3 Gate Verification (review-time)

Antes de merge final a main, verificar:

- [ ] Health timeline consultable por rango. (PR7)
- [ ] Executive summary funcional por workspace. (PR8)
- [ ] Snapshot diff entre 2 snapshots produce representación válida. (PR8)
- [ ] C4 view L1/L2 genera payload con fallback graceful. (PR8)
- [ ] Migraciones 004→005→006 aplican en secuencia sin conflictos.
- [ ] Todos los contratos v3 sincronizados entre TS y Rust.
- [ ] Feature flag `v3_h3` funcional.

---

# Go / No-Go Gate (aplicar al final de PR8)

Antes de merge final a main, verificar:

- [ ] 3 gates de hardening v2: PASS con evidencia (PR2, PR3, PR4).
- [ ] Workspaces CRUD funcional con attach/detach de proyectos. (PR1)
- [ ] Snapshots: crear, listar, cargar con payload completo. (PR5)
- [ ] Annotations: crear, listar por nodo/proyecto, persistencia. (PR6)
- [ ] Health timeline: query por rango, scores calculados. (PR7)
- [ ] Executive summary: aggregation por workspace. (PR8)
- [ ] Snapshot diff: comparación entre 2 snapshots. (PR8)
- [ ] C4 view L1/L2 con fallback. (PR8)
- [ ] CI: `cargo test` + `npm run test` + `cargo clippy` + `npm run lint` + `npm run typecheck` verdes.
- [ ] Migraciones 004→005→006 aplican en DB con datos v1/v2 sin pérdida.
- [ ] Ningún PR excede 400 líneas cambiadas sin approval explícito.
- [ ] Sin scope de cloud multi-tenant, CRDT, o features v4 en ningún PR.

---

## Out of Scope Check (review-time)

Todo PR que incluya alguna de estas características debe ser rechazado:

- ❌ Backend cloud multi-tenant o realtime sync.
- ❌ CRDT / distributed conflict resolution.
- ❌ Breaking rewrite de contratos v1/v2.
- ❌ Selector de idioma (ya implementado en v2 como runtime fijo).
- ❌ Virtualización avanzada de nodos (deferred, no bloqueante).
- ❌ Features fuera de `v3-collaboration-platform`.
