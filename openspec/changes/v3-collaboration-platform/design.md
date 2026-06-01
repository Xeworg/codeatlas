# Design — v3-collaboration-platform

## 1) Executive design summary

v3 se implementa en 3 hitos (H1/H2/H3) con una regla dura: **no cerrar H1 sin completar los 3 carry-over gates de v2** (NFR fixture+evidencia, degraded-mode FE/IA faltante, wiring T5.6 en `App.tsx`).

La arquitectura mantiene compatibilidad v1/v2 (aditiva), local-first y separación UI/BE por contratos Tauri versionados.

---

## 2) Architecture slices by milestone

## H1 — Foundation + hardening gates

**Objetivo**

- Base multi-proyecto por workspace
- Cerrar deuda obligatoria heredada de v2

**Slices**

1. Workspace domain + persistence base
2. App-shell integration (`App.tsx`) de `AnalyticsViewSelector`, `ArchitectureCard`, `ImpactPanel`, `InsightsPanel`
3. Test/benchmark infra de evidencia NFR con fixture 1000+
4. Degraded-mode FE/IA integration matrix completada

**Exit criteria H1**

- 3 gates heredados de v2 en verde con evidencia trazable
- workspace CRUD/listing básico funcional

## H2 — Collaboration baseline

**Objetivo**

- Snapshots + annotations local-first

**Slices**

1. Snapshot creation/list/load + metadata
2. Annotation/comment persistence y recuperación por nodo/proyecto
3. Contratos Tauri v3 para colaboración

**Exit criteria H2**

- snapshots persistidos y rehidratables
- comentarios persistidos con autor/timestamp

## H3 — Executive insights

**Objetivo**

- Health timeline + surfaces ejecutivas + vistas C4/diff

**Slices**

1. Health record pipeline (persist + query window)
2. Executive summary aggregation
3. Diff representation entre snapshots y adaptación a vistas C4 L1/L2

**Exit criteria H3**

- timeline consultable por rango
- diff/c4 payload disponible para UI

---

## 3) Backend/Frontend boundaries and contracts

## Backend (Rust/Tauri)

Responsable de:

- Persistencia SQLite y migraciones
- Cálculo de health timeline y diff base
- Validación de invariantes de snapshots/annotations
- Fallback/error mapping degraded-mode

### Planned command families

1. **Workspaces / multi-project**
   - `create_workspace(name)`
   - `list_workspaces()`
   - `attach_project_to_workspace(workspaceId, projectId)`
   - `list_workspace_projects(workspaceId)`

2. **Snapshots**
   - `create_snapshot(projectId, label) -> Snapshot`
   - `list_snapshots(projectId | workspaceId)`
   - `get_snapshot(snapshotId)`

3. **Annotations / comments**
   - `add_comment(nodeId, text, author) -> Comment`
   - `list_comments(projectId, nodeId?)`

4. **Executive dashboard / health timeline**
   - `get_health_timeline(projectId|workspaceId, from, to) -> HealthScoreTimeline`
   - `get_executive_summary(workspaceId) -> ExecutiveArchitectureSummary`

5. **C4/diff views**
   - `compare_snapshots(baseSnapshotId, targetSnapshotId) -> SnapshotDiffPayload`
   - `get_c4_view(projectId|snapshotId, level) -> C4ViewPayload`

## Frontend (React)

Responsable de:

- Navegación por workspace/proyecto
- Flujo de snapshots y anotaciones
- Render de timeline, C4 y diff
- UX degraded-mode (warnings, retry, fallback)
- `App.tsx` orchestration de paneles analíticos (gate H1)

### UI contract rules

- No asumir campos requeridos nuevos en v1/v2 responses
- Feature flags por milestone (`v3_h1`, `v3_h2`, `v3_h3`)
- Mapeo tipado TS desde `tauri-api.ts` con validación runtime mínima

---

## 4) Data & migration strategy

## 004_workspace_and_snapshots.sql

- Tablas: `workspaces`, `workspace_projects`, `snapshots`
- Índices por `workspace_id`, `project_id`, `created_at`
- Additive-only, sin drops/renames

## 005_collaboration_annotations.sql

- Tabla: `annotations`
- Campos: `id`, `project_id`, `node_id`, `author`, `kind`, `text`, `created_at`
- Índices compuestos por `project_id,node_id` y `created_at`

## 006_health_timeline.sql

- Tabla: `health_records`
- Campos: `id`, `workspace_id?`, `project_id`, `recorded_at`, `overall_score`, `coupling_score`, `complexity_score`, `cycle_count`, `hotspot_count`
- Índices por `project_id,recorded_at`

## Rollout safety

- Pre-migration backup automático de DB
- Migraciones secuenciales 004→005→006
- `PRAGMA user_version` incremental
- Startup checks con fail-fast y mensaje recuperable

## Rollback notes

- Si falla migración: abortar startup de feature v3 y restaurar backup
- No se revierte parcialmente schema en caliente; rollback por restore-atómico
- Contratos v1/v2 permanecen operativos al desactivar flags v3

---

## 5) Mandatory V2 carry-over gates encoded as design decisions

1. **Benchmark fixture + NFR evidence flow (H1 gate)**
   - Crear fixture real 1000+ archivos versionado para benchmark
   - Pipeline de evidencia: run benchmark -> guardar resultados -> referenciar en verify
   - Threshold checks declarativos (scan/insights/export) con pass/fail explícito

2. **Degraded-mode frontend/IA matrix completion (H1 gate)**
   - Agregar tests integración FE/IA para: PNG fallback(mock), contract mismatch, IA no configurada, IA timeout
   - Cada test valida: no crash + estado UX visible + recuperación (retry/fallback)

3. **App.tsx wiring T5.6 (H1 gate)**
   - Integrar rutas/selector y paneles analíticos en flujo principal
   - Proteger con smoke tests UI: reachable + state sync + no regressions de v2

---

## 6) Risk controls

- **Scope creep control**: backlog explícito de out-of-scope (cloud multi-tenant, CRDT completo, features v4)
- **Contract drift control**: snapshots de contrato y type checks por PR
- **Performance control**: budgets por operación (scan, insights, export, compare)
- **Data growth control**: índices + políticas de retención para snapshots/health_records
- **Complexity control**: chained PRs por slice si supera budget de revisión

---

## 7) Observability hooks

- Métricas backend por comando v3: latency_ms, rows_scanned, payload_size
- Eventos de fallback degraded-mode y errores de contrato
- Logs estructurados por `workspace_id/project_id/snapshot_id`
- Contadores de uso: snapshots creados, comments por nodo, consultas timeline

---

## 8) Test strategy mapping by milestone

## H1

- Integration: migrations 004 smoke + workspace linkage
- UI integration: App.tsx wiring T5.6
- Degraded matrix FE/IA (4 escenarios faltantes)
- Benchmarks con fixture real + reporte evidencia

## H2

- Integration: snapshot roundtrip y annotation persistence
- Contract tests: `Snapshot`, `Comment`
- UI tests: create/list/load snapshot; add/list comment

## H3

- Integration: health timeline queries por rango
- Contract tests: `HealthScoreTimeline`, `ExecutiveArchitectureSummary`, diff/c4 payload
- UI tests: dashboard timeline, compare snapshots, C4 L1/L2 render

---

## 9) Non-goals and anti-scope-creep controls

## Non-goals (v3)

- Sync realtime multiusuario cloud
- CRDT/distributed conflict resolution completo
- Breaking rewrite de contratos v1/v2
- Features fuera de `v3-collaboration-platform`

## Controls

- Todo request fuera de scope requiere change separado aprobado
- No incorporar features v4 en PRs H1/H2/H3
- Verify gate falla si se detecta mezcla de scope
