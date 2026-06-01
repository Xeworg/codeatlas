# Tasks — v2-advanced-analysis

> Generated from: `proposal.md`, `specs/project-understanding/spec.md`, `design.md`, `docs/V2_READY_CHECKLIST.md`.
> Language: español (convención proyecto). Out of scope explícito: todo alcance v3.

---

## Review Workload Forecast

| Field                   | Value                             |
| ----------------------- | --------------------------------- |
| Estimated changed lines | ~1800–2200                        |
| 400-line budget risk    | High                              |
| Chained PRs recommended | Yes                               |
| Suggested split         | PR1 → PR2 → PR3 → PR4 → PR5 → PR6 |
| Delivery strategy       | auto-chain                        |
| Chain strategy          | stacked-to-main                   |

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
```

**Justificación**: 7 slices de backend+frontend, migración DB, 4 contratos nuevos, i18n foundation. Cada PR debe caber en ~300–350 líneas cambiadas. Se encadenan secuencialmente porque cada slice depende del anterior.

---

## Dependency Graph

```
PR1 (contratos+migración)
 └─► PR2 (architecture detection)
      └─► PR3 (impact+insights)
           ├─► PR4 (export)
           └─► PR5 (UX analítica + filtros)
                └─► PR6 (i18n foundation + degraded-mode tests)
```

---

## PR1 — Contratos v2 + DB Migration

**Objetivo**: Exponer contratos v2 en frontend, crear migration framework y aplicar `003_architecture_and_insights.sql` sin romper v1.

**Changed files estimate**: ~250 líneas

### Tareas

- [ ] **T1.1** Agregar contratos v2 en `src/lib/types.ts`:
  - `ArchitecturePattern` (union literal)
  - `ArchitectureDetectionResult` (version: '2.0', pattern, confidence, evidence, generatedAt)
  - `ImpactAnalysisResult` (version: '2.0', changedNodeId, affectedNodes, impactScore, explanation)
  - `GraphInsights` (version: '2.0', cycles, hotspots, avgCoupling, density, status?)
  - `ExportPayload` (version: '2.0', format, graphData, insights, metadata)
  - No tocar contratos v1 existentes.

- [ ] **T1.2** Agregar wrappers v2 en `src/lib/tauri-api.ts`:
  - `getArchitectureDetection(projectId): Promise<ArchitectureDetectionResult>`
  - `getImpactAnalysis(projectId, nodeId): Promise<ImpactAnalysisResult>`
  - `getGraphInsights(projectId): Promise<GraphInsights>`
  - `exportView(projectId, format): Promise<ExportPayload>`
  - Cada wrapper usa `invoke` tipado y mapea errores.

- [ ] **T1.3** Crear archivo `engine/migrations/003_architecture_and_insights.sql`:
  - `ALTER TABLE imports ADD COLUMN edge_type TEXT DEFAULT 'import';`
  - `CREATE TABLE IF NOT EXISTS architecture_detections (...)`
  - `CREATE TABLE IF NOT EXISTS graph_insights (...)`
  - `PRAGMA user_version = 3;`

- [ ] **T1.4** Crear `engine/src/db/migrations.rs`:
  - Framework de migraciones: detectar `user_version`, aplicar scripts `.sql` pendientes.
  - Ejecutar en transacción.
  - Forzar `PRAGMA journal_mode=WAL` antes de migrar.
  - Crear backup automático `codeatlas.db.backup.<timestamp>` antes de aplicar.

- [ ] **T1.5** Registrar módulo en `engine/src/db/mod.rs`:
  - `pub mod migrations;`
  - Integrar auto-migration en startup de la app (llamar desde `main.rs` o init).

- [ ] **T1.6** Tests:
  - `cargo test` para migration framework (aplicar sobre DB v1 vacía → verificar tablas creadas).
  - `npm run test` para tipos v2 (compile-only check que contratos existen con campos correctos).
  - Test de idempotencia: migrar 2 veces no rompe.

**Dependencies**: None (primer slice).
**Acceptance**: `cargo test` verde, `npm run test` verde, `cargo clippy` limpio, DB con datos v1 se migra sin perder datos.

---

## PR2 — Architecture Detection (Backend)

**Objetivo**: Implementar detector de arquitectura con heurísticas, persistencia y fallback degradado.

**Changed files estimate**: ~350 líneas

### Tareas

- [ ] **T2.1** Crear `engine/src/analysis/mod.rs` con:
  - `pub mod architecture_detector;`
  - `pub mod impact_engine;`
  - `pub mod graph_insights;`

- [ ] **T2.2** Implementar `engine/src/analysis/architecture_detector.rs`:
  - Función `detect_architecture(project_id, db_pool) -> ArchitectureDetectionResult`.
  - Heurísticas:
    - `hexagonal`: detectar `ports/adapters` o `infrastructure/` en paths.
    - `clean`: detectar `domain/`, `application/`, `infrastructure/` en paths.
    - `layered`: detectar `controllers/`, `services/`, `repositories/` en paths.
    - `mvc`: detectar `models/`, `views/`, `controllers/` o `routes/` en paths.
    - `unknown`: fallback por defecto.
  - Confidence: proporcional a la cantidad de paths que coinciden.
  - Evidence: lista de nodos/aristas que respaldan el patrón.
  - Fallback: ante cualquier error → `unknown`, confidence=0, evidence=null.

- [ ] **T2.3** Agregar queries v2 en `engine/src/db/queries.rs`:
  - `save_architecture_detection(project_id, result)` → INSERT en `architecture_detections`.
  - `get_latest_architecture_detection(project_id)` → SELECT más reciente.

- [ ] **T2.4** Agregar comando `get_architecture_detection` en `engine/src/commands.rs`:
  - Valida projectId.
  - Llama a `detect_architecture`.
  - Persiste resultado.
  - Mapea errores a `String`.
  - Registra latencia para NFR tracking.

- [ ] **T2.5** Tests unitarios en `architecture_detector.rs`:
  - Test: proyecto con paths tipo MVC → devuelve `mvc` con confidence > 0.
  - Test: proyecto con paths tipo clean → devuelve `clean`.
  - Test: paths neutros → devuelve `unknown`.
  - Test: error en lectura DB → devuelve `unknown` sin crash.
  - Test: evidence contiene nodos del grafo.

**Dependencies**: PR1.
**Acceptance**: `cargo test` verde, comando Tauri registrado y responde shape correcto.

---

## PR3 — Impact Engine + Graph Insights (Backend)

**Objetivo**: Calcular impacto de cambios por nodo y generar insights de grafo (ciclos, hotspots, métricas).

**Changed files estimate**: ~350 líneas

### Tareas

- [ ] **T3.1** Implementar `engine/src/analysis/impact_engine.rs`:
  - Función `compute_impact(project_id, node_id, db_pool) -> ImpactAnalysisResult`.
  - BFS/DFS acotado desde `node_id` siguiendo aristas de `imports`.
  - Impact score normalizado (0..1) basado en profundidad y cantidad de nodos afectados.
  - `explanation`: texto descriptivo del camino de impacto.
  - Timeout configurable (default 5s).

- [ ] **T3.2** Implementar `engine/src/analysis/graph_insights.rs`:
  - Función `compute_graph_insights(project_id, db_pool) -> GraphInsights`.
  - Ciclos: detección de ciclos simples en el grafo de dependencias.
  - Hotspots: nodos con mayor grado de entrada+salida (top 10%).
  - `avgCoupling`: promedio de grado de acoplamiento.
  - `density`: `edges / (nodes * (nodes-1))`.
  - Timeout configurable (default 2s).
  - Fallback: ante timeout/error → payload vacío con `status: 'timeout'|'error'`.

- [ ] **T3.3** Agregar queries v2 en `engine/src/db/queries.rs`:
  - `save_graph_insights(project_id, insights)` → INSERT/UPSERT en `graph_insights`.
  - `get_cached_graph_insights(project_id)` → SELECT más reciente.

- [ ] **T3.4** Agregar comandos en `engine/src/commands.rs`:
  - `get_impact_analysis(project_id, node_id)`: llama impact engine, retorna `ImpactAnalysisResult`.
  - `get_graph_insights(project_id)`: intenta cache, si no existe calcula, persiste y retorna.

- [ ] **T3.5** Tests unitarios:
  - Impact: grafo lineal A→B→C, impacto de A afecta B y C.
  - Impact: nodo aislado → affectedNodes vacío.
  - Insights: grafo con ciclo A→B→A → cycles contiene el ciclo.
  - Insights: grafo vacío → density=0, avgCoupling=0.
  - Insights: timeout → status='timeout', payload vacío.

**Dependencies**: PR2 (usa `analysis/` module ya creado).
**Acceptance**: `cargo test` verde, comandos retornan shape v2 correcto.

---

## PR4 — Exportes (Backend + Frontend Wiring)

**Objetivo**: Implementar export JSON/PNG con fallback degradado.

**Changed files estimate**: ~300 líneas

### Tareas

- [ ] **T4.1** Agregar comando `export_view` en `engine/src/commands.rs`:
  - Recibe `project_id` y `format` ('json'|'png').
  - JSON: serializa `GraphData` + `GraphInsights` opcional → retorna `ExportPayload`.
  - PNG: retorna error (la generación PNG es responsabilidad frontend).

- [ ] **T4.2** Implementar hook `useExport` en `src/hooks/useExport.ts`:
  - Export JSON: llama comando Tauri, genera blob y descarga.
  - Export PNG: usa `html-to-image` o `dom-to-image` sobre el grafo, fallback a JSON si falla.
  - Estado: `idle | exporting | done | error`.
  - Mensaje de fallback visible si PNG falla.

- [ ] **T4.3** Componente `ExportButton`:
  - Dropdown: Export JSON / Export PNG.
  - Dispara `useExport`.
  - Muestra progreso y errores.

- [ ] **T4.4** Tests:
  - Backend: comando export_view retorna payload shape correcto.
  - Backend: format inválido → error controlado.
  - Frontend: hook fallback test (mock PNG failure → JSON export succeeds).

**Dependencies**: PR3 (usa insights para incluir en payload).
**Acceptance**: `cargo test` verde, `npm run test` verde, export JSON descarga archivo válido.

---

## PR5 — UX Analítica + Filtros Persistentes (Frontend)

**Objetivo**: Implementar vistas analíticas (arquitectura/dependencias/flujo beta), filtros persistentes y componentes de tarjeta de arquitectura, impacto e insights.

**Changed files estimate**: ~350 líneas

### Tareas

- [ ] **T5.1** Crear store `useAnalyticsStore` en `src/stores/analyticsStore.ts`:
  - Estado: `activeView: 'architecture' | 'dependencies' | 'flow-beta'`.
  - Filtros: `nodeTypeFilter`, `couplingThreshold`, `showCycles`, `showHotspots`.
  - Acciones: `setView`, `setFilter`, `resetFilters`.
  - Persistencia: session (Zustand, no localStorage).

- [ ] **T5.2** Componente `ArchitectureCard`:
  - Muestra patrón detectado, confianza (badge color-coded), evidencia expandible.
  - Si patrón = `unknown`: mostrar "Sin arquitectura detectada".

- [ ] **T5.3** Componente `ImpactPanel`:
  - Selección de nodo → muestra lista de nodos afectados con score.
  - Highlight en grafo del set impactado.
  - Explicación textual del camino de impacto.

- [ ] **T5.4** Componente `InsightsPanel`:
  - Pestañas: Ciclos / Hotspots / Métricas.
  - Ciclos: lista de ciclos con nodos, click navega a grafo.
  - Hotspots: ranking de nodos más acoplados.
  - Métricas: avgCoupling, density.

- [ ] **T5.5** Selector de vista analítica (toolbar):
  - 3 botones/tabs: Architecture / Dependencies / Flow (beta).
  - Cambio de vista no dispara re-scan.
  - Filtros persisten entre cambios de vista.

- [ ] **T5.6** Wiring completo:
  - Explorer → selección → dispara impacto.
  - Grafo → resalta impacto.
  - Vista architecture → tarjeta + filtros.
  - Vista flow-beta → placeholder con aviso "beta".

- [ ] **T5.7** Tests:
  - Store: cambio de vista y filtro persiste en sesión.
  - Store: resetFilters limpia todos los filtros.
  - Componentes: render básico con mock data.

**Dependencies**: PR4 (usa export y commands).
**Acceptance**: `npm run test` verde, vistas navegan sin re-scan, filtros persisten en sesión.

---

## PR6 — i18n Foundation + Degraded-Mode Tests + NFR Benchmarks

**Objetivo**: Extraer strings a `es.json`, crear helper `t()`, migrar superficies v2, agregar tests de degradación completa y benchmarks NFR.

**Changed files estimate**: ~300 líneas

### Tareas

- [ ] **T6.1** Crear `src/locales/es.json`:
  - Estructura por sección: `common`, `explorer`, `graph`, `details`, `architecture`, `impact`, `insights`, `export`, `settings`.
  - Extraer todos los strings visibles de superficies v2 (arch card, impact panel, insights panel, export button, toolbar vistas).

- [ ] **T6.2** Crear `src/lib/i18n.ts`:
  - Función `t(key: string, vars?: Record<string, string>): string`.
  - Resuelve desde `es.json` con dot notation.
  - Fallback: si clave no existe → devuelve key literal + warning en dev mode.
  - Runtime fijo en español (no selector).

- [ ] **T6.3** Migrar strings en superficies v2:
  - Reemplazar textos hardcodeados por llamadas `t('...')` en:
    - ArchitectureCard, ImpactPanel, InsightsPanel.
    - ExportButton, selector de vistas.
  - No migrar superficies v1 en este PR (out of scope).

- [ ] **T6.4** Tests degradados (integration):
  - `ArchitectureDetector falla` → unknown, 0, null.
  - `Insights timeout` → status='timeout', UI no bloquea.
  - `PNG export falla` → fallback JSON + warning.
  - `SQLite write fail` → error actionable, lectura disponible.
  - `Contract version mismatch` → UI muestra banner.
  - `IA no configurada` → AI panel oculto.
  - `IA timeout` → error en chat, UI no bloquea.
  - `Cycle detection timeout` → cycles vacío, grafo usable.

- [ ] **T6.5** Benchmarks NFR:
  - Benchmark architecture detection con fixture 5000 archivos → <3s.
  - Benchmark impact analysis por nodo → <5s.
  - Benchmark insights 2000 nodos → <2s.
  - Benchmark export JSON → <5s.
  - Test concurrencia WAL: 10 lecturas paralelas → 0 deadlocks.
  - Perfil de memoria con `dhat` o `heaptrack`.

- [ ] **T6.6** Tests i18n:
  - `t('nonexistent.key')` → devuelve literal key.
  - `t('common.loading')` → devuelve string del catálogo.
  - Superficie v2: verificar que no hay strings hardcodeados (spot check).

**Dependencies**: PR5.
**Acceptance**: `cargo test` verde, `npm run test` verde, benchmarks dentro de umbrales, degraded tests pasan.

---

## Go / No-Go Gate (aplicar al final de PR6)

Antes de merge a main, verificar contra checklist:

- [ ] 3 core contracts + ExportPayload implementados e integration-tested.
- [ ] NFR thresholds validados en fixture con 1000+ archivos.
- [ ] Migration runbook validado end-to-end.
- [ ] Degraded-mode matrix: 8/8 escenarios covered.
- [ ] CI: `cargo test` + `npm run test` + `cargo clippy` + `npm run lint` + `tauri build --debug`.
- [ ] Ningún PR excede 400 líneas cambiadas sin approval explícito.
- [ ] Sin scope v3 en ningún PR.

---

## Out of Scope Check (review-time)

Todo PR que incluya alguna de estas características debe ser rechazado:

- ❌ Multi-proyecto / workspaces.
- ❌ Snapshots colaborativos.
- ❌ Anotaciones / comentarios.
- ❌ Health timeline / dashboard ejecutivo.
- ❌ Selector de idioma.
- ❌ Virtualización avanzada >1000 nodos (solo si compromete salida v2).
