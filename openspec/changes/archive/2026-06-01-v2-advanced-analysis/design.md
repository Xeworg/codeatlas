# Design — v2-advanced-analysis

## Estado

Aprobado para implementación (v2).

## Alcance de diseño

Este diseño cubre solo `v2-advanced-analysis` y se basa en:

- `openspec/changes/v2-advanced-analysis/proposal.md`
- `openspec/changes/v2-advanced-analysis/specs/project-understanding/spec.md`
- `docs/V2_READY_CHECKLIST.md`
- `docs/ARQUITECTURA_DATOS_V2_V3.md`

Incluye: detección de arquitectura, impacto, insights, exportes, vistas analíticas, migración aditiva v2 e i18n foundation (`es.json` + `t` helper, sin selector).
Excluye explícitamente alcance v3 (workspaces, snapshots colaborativos, anotaciones, health timeline).

---

## 1) Architecture slices (verticales)

### Slice A — Contratos v2 + comandos

**Objetivo:** exponer capacidades v2 sin romper v1.

- Backend: nuevos comandos `get_architecture_detection`, `get_impact_analysis`, `get_graph_insights`, `export_view`.
- Frontend: wrappers tipados en `src/lib/tauri-api.ts`.
- Contratos v2 en `src/lib/types.ts` con versionado menor y compatibilidad hacia atrás.

### Slice B — Detección de arquitectura

**Objetivo:** clasificar patrón con confianza y evidencia trazable.

- `engine/src/analysis/architecture_detector.rs`: reglas heurísticas sobre grafo + estructura de paths/símbolos.
- Persistencia en `architecture_detections`.
- Fallback degradado: `unknown`, `confidence=0.0`, `evidence=null`.

### Slice C — Impacto + insights

**Objetivo:** responder “qué afecta” y “dónde están riesgos estructurales”.

- `engine/src/analysis/impact_engine.rs`: BFS/DFS acotado para nodos afectados + score.
- `engine/src/analysis/graph_insights.rs`: ciclos, hotspots, acoplamiento promedio, densidad.
- Persistencia/cache en `graph_insights` por `project_id`.

### Slice D — Exportes de evidencia

**Objetivo:** compartir resultados en JSON/PNG.

- JSON: serialización de `GraphData` + `GraphInsights` opcional.
- PNG: captura de vista actual (frontend) con fallback automático a JSON si falla.
- Mensaje de degradación visible al usuario.

### Slice E — UX analítica + filtros persistentes

**Objetivo:** navegar vistas arquitectura/dependencias/flujo sin re-scan.

- Estado de vista/filtros en store (persistencia de sesión).
- Componentes para tarjeta de arquitectura, panel de impacto e insights.
- Cambios de modo reutilizan grafo cargado en memoria.

### Slice F — Foundation i18n (v2 acotado)

**Objetivo:** quitar hardcodes UI y preparar multi-idioma futuro sin selector.

- `src/locales/es.json` como catálogo único en v2.
- `src/lib/i18n.ts` con helper `t(key, vars?)`.
- Migración progresiva de strings visibles en superficies v2.
- Runtime fijo en español.

---

## 2) Module boundaries

### Backend (`engine/src`)

- **Domain/Analysis:** `analysis/*` (architecture, impact, insights).
- **Data access:** `db/*` (queries + migrations + mapping DTO).
- **Presentation:** `commands.rs` (validación input, orchestration, error mapping).

Reglas:

1. `commands.rs` no contiene lógica algorítmica pesada.
2. `analysis/*` no conoce Tauri ni tipos UI.
3. `db/*` no define reglas de negocio, solo persistencia/consulta.

### Frontend (`src`)

- **Contracts/API:** `lib/types.ts`, `lib/tauri-api.ts`.
- **State/App layer:** `stores/*`, `hooks/*`.
- **Presentation:** `components/*` (graph, panels, export, architecture card).
- **i18n:** `locales/es.json`, `lib/i18n.ts`.

Reglas:

1. Componentes no invocan Tauri directo.
2. Textos UI migrados usan `t('...')` en superficies v2.
3. No se introduce selector de idioma en v2.

---

## 3) Data flow (end-to-end)

### 3.1 Detección de arquitectura

1. UI solicita `get_architecture_detection(projectId)`.
2. `commands.rs` valida `projectId` y carga señales (grafo/imports/símbolos).
3. `architecture_detector` computa patrón + confianza + evidencia.
4. Resultado se persiste en `architecture_detections`.
5. UI renderiza tarjeta con patrón, score y evidencia.

### 3.2 Análisis de impacto

1. Usuario selecciona nodo y dispara impacto.
2. Backend calcula afectados + score en `impact_engine`.
3. UI pinta lista/ranking y highlight en grafo.

### 3.3 Insights de grafo

1. UI solicita `get_graph_insights(projectId)`.
2. Backend calcula o reutiliza `graph_insights` cacheada.
3. UI muestra ciclos, hotspots y métricas agregadas.
4. Si timeout/falla: payload vacío con estado de fallo explícito, UI sigue usable.

### 3.4 Export

1. UI solicita export JSON o PNG.
2. JSON: backend retorna `ExportPayload` serializable.
3. PNG: frontend intenta render exportable; si falla, ejecuta fallback a JSON y muestra aviso.

### 3.5 i18n runtime

1. Componente solicita `t('section.key')`.
2. `i18n.ts` resuelve desde `locales/es.json`.
3. Si clave ausente: fallback controlado (key literal + warning en dev).

---

## 4) DB migration strategy (v2)

### 4.1 Cambios de esquema (aditivos)

Migration `003_architecture_and_insights.sql`:

- `ALTER TABLE imports ADD COLUMN edge_type TEXT DEFAULT 'import'`
- `CREATE TABLE architecture_detections (...)`
- `CREATE TABLE graph_insights (...)`
- `PRAGMA user_version = 3`

### 4.2 Ejecución segura

1. Verificar/forzar `PRAGMA journal_mode=WAL`.
2. Crear backup previo `codeatlas.db.backup.<timestamp>`.
3. Ejecutar migración en transacción.
4. Validar lectura de entidades v1 (`projects/files/symbols/imports`).
5. Registrar versión aplicada.

### 4.3 Rollback

- Estrategia forward-only + restore backup.
- Ante fallo: detener comandos v2, restaurar backup, mantener workflows v1.

---

## 5) Contract definitions (v2)

```ts
type ArchitecturePattern = 'mvc' | 'layered' | 'clean' | 'hexagonal' | 'unknown'

type ArchitectureDetectionResult = {
  version: '2.0'
  pattern: ArchitecturePattern
  confidence: number // 0..1
  evidence: {
    nodes: string[]
    edges: Array<{ source: string; target: string; kind: string }>
    reasons: string[]
  } | null
  generatedAt: string
}

type ImpactAnalysisResult = {
  version: '2.0'
  changedNodeId: string
  affectedNodes: string[]
  impactScore: number // 0..1
  explanation: string
}

type GraphInsights = {
  version: '2.0'
  cycles: Array<{ nodes: string[]; length: number }>
  hotspots: Array<{ nodeId: string; couplingScore: number; reason: string }>
  avgCoupling: number | null
  density: number | null
  status?: 'ok' | 'timeout' | 'error'
}

type ExportPayload = {
  version: '2.0'
  format: 'json' | 'png'
  graphData: unknown
  insights: GraphInsights | null
  metadata: { projectId: string; generatedAt: string }
}
```

Compatibilidad v1:

- No se renombra ni elimina campos de contratos v1 existentes.
- Comandos v1 (`scan/get_graph/get_node_details/search`) permanecen estables.

---

## 6) Degraded-mode handling

1. **Architecture detector falla** → `unknown`, `confidence=0.0`, `evidence=null`.
2. **Insights timeout/falla** → `GraphInsights` vacío con `status='timeout'|'error'`; UI no bloquea grafo.
3. **PNG export falla** → fallback automático a JSON + warning visible.
4. **Error de escritura SQLite/disk full** → error actionable, se deshabilita persistencia pero lectura/export disponible.
5. **Desfase de contrato** → validación de `version` en payload; UI muestra “update required”.

Todos los modos degradados requieren test de integración dedicado.

---

## 7) NFR validation strategy

Objetivos (a validar en Verify):

- arquitectura <3s (5000 archivos)
- impacto <5s por nodo
- insights <2s (2000 nodos)
- export JSON <5s, PNG <10s
- respuesta filtros UI <200ms
- 0 deadlocks en 10 lecturas paralelas (WAL)

Estrategia:

1. Benchmarks backend en `tests/benchmarks/` (fixtures de tamaño controlado).
2. Instrumentación temporal por comando en `commands.rs` para medir latencias.
3. Pruebas de concurrencia SQLite (lecturas paralelas + migración aplicada).
4. Perfil de memoria con `dhat` o `heaptrack` sobre build release.
5. Gate en CI: fallar si regression >20% sobre baseline acordado.

---

## 8) Test design alignment

### Unit

- Detector de arquitectura (clasificación + confianza + evidence).
- Impact engine (propagación y score).
- Insights (detección de ciclos/hotspots).
- Helper i18n (`t` resolve/fallback).

### Integration

- Contratos Tauri v2 payload-shape.
- Migración `003` sobre DB con datos v1.
- Degraded-mode matrix (fallas simuladas).
- Compatibilidad workflows v1 intactos.

### E2E funcional

- Flujo completo: scan → architecture → impact → insights → export.
- Cambio de vistas y filtros persistentes durante sesión.
- UI con textos resueltos desde `es.json` en superficies v2.

---

## 9) Plan de cambios por archivo (forecast)

Backend:

- `engine/src/commands.rs` (nuevos comandos + mapeo errores)
- `engine/src/analysis/architecture_detector.rs` (nuevo)
- `engine/src/analysis/impact_engine.rs` (nuevo)
- `engine/src/analysis/graph_insights.rs` (nuevo)
- `engine/src/db/queries.rs` (persistencia/lecturas v2)
- `engine/src/db/migrations.rs` (nuevo registrador)
- `engine/migrations/003_architecture_and_insights.sql` (nuevo)

Frontend:

- `src/lib/types.ts` (contratos v2)
- `src/lib/tauri-api.ts` (wrappers v2)
- `src/stores/*` (estado vistas/filtros/insights/impact)
- `src/components/*` (arquitectura, impacto, insights, export)
- `src/lib/i18n.ts` (nuevo)
- `src/locales/es.json` (nuevo)

SDD:

- `openspec/changes/v2-advanced-analysis/tasks.md` (siguiente fase)

---

## 10) Guardrails de alcance

Bloquear en review cualquier intento de incluir:

- workspaces multi-proyecto,
- snapshots colaborativos,
- anotaciones/comentarios,
- health timeline/dashboard ejecutivo,
- selector de idioma.

Estos ítems pertenecen a v3 y no se implementan en `v2-advanced-analysis`.
