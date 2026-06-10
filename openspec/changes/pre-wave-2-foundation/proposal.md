# Proposal: pre-wave-2-foundation

## Resumen ejecutivo

La migración hexagonal de wave 1 (merge PR #3, commit `503ad80`) construyó una "concha" hexagonal delgada, pero la frontera no aguanta presión: la presentación (`src-tauri/src/commands.rs`) sigue orquestando lógica de negocio, `AppState` expone tipos concretos, el contrato de errores estructurados está destruido en el límite IPC, y la capa `src/services/*.ts` del frontend son re-exports sin valor. Este change `pre-wave-2-foundation` cierra las fugas estructurales más críticas con un refactor **quirúrgico** de dos PRs encadenados, dejando la base firme para que wave 2 (split de crates + CQRS + events) sea barato de ejecutar.

La estrategia es de **fundaciones primero, refactor después**. PR-A es la cama de traits: `pub(crate)` en los 5 ports, un CI guard que bloquee regresiones de la frontera, dedup de helpers de error duplicados, y un docstring que documente el contrato de `Arc<Mutex<...>>`. PR-B es la mudanza: 160 líneas de orquestación de IA que migran a `AIService` con su port trait, `AppState` con `Arc<dyn ...>`, el helper `to_ipc_error` que honra `IpcErrorPayload`, y el borrado de los 5 archivos `src/services/*.ts`. Cada PR cabe en el review budget de 400 líneas y deja la base en verde antes del siguiente.

El hotfix PR-0 (PR #4, commit `04e4c73`/`bdd6686`) — bug de contrato `NodeExplanation.nodeId` — **ya está mergeado a main** y se documenta como RESUELTO. Este change no lo rehace.

## Motivación y contexto

El review pre-wave-2 de la migración hexagonal (Engram `architecture/hexagonal-wave-1-review`, obs #654) identificó **25 fugas de dependencia** numeradas con `file:line` y 10 wave-2 blockers rankeados por severidad. Los 5 blockers que este change cierra son los items #2, #6, #7, #8 y #9 de ese ranking, más el quick win #5 (docstring de `Arc<Mutex<...>>`).

La matriz de decisiones cerradas por el usuario es:

| # | Pregunta | Decisión | Recomendación del agente | Fecha |
|---|----------|----------|--------------------------|-------|
| Q1 | Port trait para `AIService` | (a) trait delgado `AIServicePort` (~30 líneas, 2 métodos) | Opción (a) por minimizar blast radius | 2026-06-08 |
| Q2 | Tests frontend con strings de error | Atómico con PR-B (error contract PR) | Atómico, alineado con `error-contract/spec.md:120-128` | 2026-06-08 |
| Q3 | CI guard | `npm run check:arch` (JS, no bash) | JS vía `package.json` evita tocar `.github/workflows/ci.yml` | 2026-06-08 |
| Q4 | Chained PR strategy | 2 PRs (PR-A foundations, PR-B refactor) | 2 PRs, encaja con `chained_pr_strategy: auto-forecast` y `review_budget_changed_lines: 400` | 2026-06-08 |
| Q5 | Hotfix `NodeExplanation` | Extraído a main como PR #4 — DONE | Extraer, reduce PR-A y desbloquea tests | 2026-06-08 |

> **Esta pre-wave-2 es QUIRÚRGICA.** No incluye split de crates (`codeatlas-domain` / `codeatlas-application` / `codeatlas-infrastructure`), no CQRS sobre `ProjectRepository`, no events ni `Clock`/`IdGenerator` ports. Eso es wave 2, y este change es la pre-condición barata para que wave 2 sea seguro.

## Objetivo

Establecer **enforzabilidad** de la pureza hexagonal a nivel de compilación y de CI, con la meta explícita de que wave 2 (Crate split + CQRS + events) arranque con confianza: límites de visibilidad firmes, guard automatizado, contratos IPC cumplidos, y la presentación adelgazada a "thin shims" de 5-10 líneas como dice el comentario aspiracional en `commands.rs:1`.

## Alcance y entregables

El change se entrega en **dos PRs encadenados** que se planifican juntos, se implementan en orden, y se mergean de forma secuencial.

### PR-A — foundations (PR-1, este PR de planificación cubre todo el change)

Branch: `feat/pre-wave-2-pr-a-foundations` (ya creada desde `main@04e4c73`). Estimación: ~150-200 líneas, dentro del review budget de 400.

| Item | Footprint | Descripción |
|------|-----------|-------------|
| 5 | `engine/src/ports.rs:37, 139, 224, 499, 607` | `pub` → `pub(crate)` en los 5 traits (`ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AnalysisRepository`, `AppStatePort`) |
| 6 | `scripts/ci/check-architecture.mjs` + `package.json` + `.github/workflows/ci.yml` | Script JS ejecutable vía `npm run check:arch`; greps prohibidos: `use engine::db::` en `src-tauri/src/commands.rs`, imports directos de `engine::ai::(anthropic\|resolved\|provider::AIProvider)`, y `use engine::ai::AIService` después de PR-B. Wired como step de CI |
| 9 | `src-tauri/src/commands.rs:477-501` + `src-tauri/src/commands/tests/observability_tests.rs:1-83` | Eliminar `is_root_path_conflict` y `map_save_scan_result_error` (duplicados de `engine/src/services/scan_service.rs:320-336`); migrar los 7 tests de `observability_tests.rs` para testear la versión del servicio |
| 10 | `src-tauri/src/commands.rs:24-29` | Espejar el comentario de `engine/src/ports.rs:632-635` que documenta el contrato de `Arc<Mutex<...>>` y su lifetime |

> **Nota Item 1 (bug `nodeId`)**: el fix está mergeado en `04e4c73` (PR #4). `src/lib/types.ts:126` ya tiene `nodeId`, fixtures actualizados. RESUELTO.

### PR-B — refactor (PR-2, encadenado al merge de PR-A)

Branch: se creará como `feat/pre-wave-2-pr-b-refactor` desde `main` post-merge de PR-A. Estimación: ~400-500 líneas (justifica chained PR por exceder el budget de 400).

| Item | Footprint | Descripción |
|------|-----------|-------------|
| 2 | `src-tauri/src/commands.rs:240-317` → `engine/src/ai/service.rs` | `explain_node` → `AIService::explain_node_with_context`; las 78 líneas (file read, `ProjectRepository::new(&state.db)`, `ContextBuilder::build_node_context`, filtro de edges) migran a un método del port trait |
| 3 | `src-tauri/src/commands.rs:319-398` → `engine/src/ai/service.rs` | `chat` → `AIService::chat_with_context`; análogo a item 2, 80 líneas. Migrar también la construcción manual de `ChatMessage` con `uuid::Uuid::new_v4()` + `chrono::Utc::now()` (clock leak menor) |
| 4 | `src-tauri/src/commands.rs:30-38, 61-668` + `src-tauri/src/lib.rs:48-54` + `engine/src/ports.rs:68-78, 162-172, 335-345, 533-545, 636-677` | `AppState` con `Arc<dyn ScanRepository>`, `Arc<dyn GraphRepository>`, `Arc<dyn WorkspaceRepository>`, `Arc<dyn AnalysisRepository>`, `Arc<dyn AIServicePort>`, `Arc<dyn AppStatePort>`. 22 comandos extraen el `Arc<dyn>` en vez de `&state.db`. Adapters aceptan `Arc<DbPool>` internamente (constructor `from_arc` adicional, preserva `::new(&pool)` para tests internos) |
| 7 | `src-tauri/src/commands.rs` (37 ocurrencias) + `engine/src/lib.rs:74-150` + `src/lib/tauri-api.ts:58-126` + `src/lib/__tests__/tauri-api.test.ts` + `src/services/__tests__/services-boundary.test.ts` | Helper `to_ipc_error(e: AppError) -> String` que serializa `IpcErrorPayload` a JSON string. Reemplazar los 37 `.map_err(\|e\| e.to_string())`. Mapear strings sueltos (`"AI not configured"`, `format!("File not found: {}", node_id)`) a variantes `AppError::AIUnavailable`/`AppError::FileNotFound`. Actualizar tests atómicamente |
| 8 | `src/services/{ai,project,graph,snapshot,analysis}Service.ts` (5 archivos, 257 líneas) + 9 call sites en `src/hooks/*.ts`, `src/stores/useSnapshotStore.ts` | Borrar los 5 archivos; migrar imports directos a `src/lib/tauri-api.ts`; renombrar `src/services/__tests__/services-boundary.test.ts` a `src/lib/__tests__/tauri-api-bridge.test.ts` y reducir a tests del parser de errores + smoke test |

### Fuera de alcance (deferred a wave 2, con razón)

- **Crate split** (`codeatlas-domain` / `codeatlas-application` / `codeatlas-infrastructure`): decisión del usuario de hacer **después** de wave 2, no antes. Razón: pre-wave-2 deja la frontera firme y `pub(crate)` ya documenta la intención; el split mecánico se hace con wave 2 cuando se introduce CQRS, events y múltiples adapters que JUSTIFICAN el costo de compilación separada.
- **CQRS split de `ProjectRepository`** (2385 líneas en `engine/src/db/queries.rs`): wave 2 con el crate split.
- **`AnalysisRepository` real port** (hoy recibe `&DbPool` directamente, defeating the port): wave 2 introducirá `AnalysisDataSource` con tipos neutros.
- **`WorkspaceRepository` tuple-typed methods → domain types**: wave 2 cuando el esquema cambie; ahora es cambio de alto costo, bajo beneficio inmediato.
- **Domain extraction, events, `Clock`/`IdGenerator` ports**: wave 2 explícitamente.

## Decisiones cerradas

Ver la matriz completa en la sección **Motivación y contexto** (Q1-Q5). Resumen:

- **Q1 (port trait)**: trait delgado `AIServicePort` con 2 métodos. No use cases separados — los 2 métodos cubren los 2 paths críticos sin over-engineering.
- **Q2 (frontend tests atómicos)**: atómico con PR-B. `error-contract/spec.md:120-128` exige atomic rollout.
- **Q3 (CI guard)**: `npm run check:arch` en JS, no bash. La CI ya corre `npm run typecheck` y `npm run lint`; añadir 1 línea a `package.json` evita modificar el workflow.
- **Q4 (chained PR)**: 2 PRs. `chained_pr_strategy: auto-forecast` ya está declarado en `openspec/config.yaml:12`.
- **Q5 (NodeExplanation hotfix)**: extraído a main como PR #4 — DONE.

## Criterios de éxito

### PR-A — foundations
- [ ] Diff ≤ 200 líneas (dentro de review budget de 400)
- [ ] `cargo test` verde, `cargo clippy -- -D warnings` verde, `npm run lint`/`test`/`typecheck` verde
- [ ] CI guard `npm run check:arch` falla correctamente cuando se reintroduce una fuga prohibida (test del propio guard)
- [ ] `pub(crate)` aplicado a los 5 traits; `rg "use engine::ports::ScanRepository" src-tauri/` confirma 0 hits directos
- [ ] Funciones duplicadas `is_root_path_conflict` / `map_save_scan_result_error` borradas; 7 tests de `observability_tests.rs` migrados a la versión del servicio
- [ ] Merge a main sin conflictos

### PR-B — refactor
- [ ] Diff ≤ 500 líneas (justifica la estrategia chained)
- [ ] Quality gates verdes (mismas que PR-A)
- [ ] `commands.rs` reducido en ~240 líneas netas; 22 comandos extraen `Arc<dyn ...>` consistentemente
- [ ] 37 ocurrencias de `.map_err(|e| e.to_string())` reemplazadas por `to_ipc_error`
- [ ] `src/services/*.ts` borrados; 9 call sites actualizados; tests renombrados
- [ ] `AIService::explain_node_with_context` y `chat_with_context` con tests propios (mock repository)
- [ ] `IpcErrorPayload` JSON observable en tests E2E del parser frontend
- [ ] Merge a main sin conflictos

### Wave 2 readiness
- [ ] Límites de visibilidad firmes: `pub(crate)` en ports, CI guard activo bloqueando regresiones
- [ ] `src-tauri/src/commands.rs` ≤ 350 líneas (de las 669 actuales)
- [ ] Wave 2 planning puede arrancar con confianza: split de crates, CQRS, events ya no están bloqueados por fugas de presentación

## Riesgos identificados

| # | Riesgo | Likelihood | Mitigación |
|---|--------|------------|------------|
| 1 | El dedup de error-mapping (item 9) rompe `observability_tests.rs` por orden de matching de strings | Media | Los tests cubren los paths críticos; si fallan, ajustar visibilidad a `pub(crate)` en el helper del servicio para que el test los pueda invocar |
| 2 | `pub(crate)` en los 5 traits rompe un doc-test o test externo no grepeado | Media | Verificación previa con `rg "use engine::ports::" src-tauri/` y `rg "engine::ports::" .` antes de aplicar; el `grep` actual muestra 0 hits en `src-tauri` para los traits. Si algo se rompe, ajustar a `pub(super)` para los adapters de tests |
| 3 | `AppState` con `Arc<dyn ...>` (item 4) cascada de firmas a services y adapters | Alta (ya flaggeada en explore) | Refactor quirúrgico: introducir `Adapter::from_arc(Arc<DbPool>)` preservando `Adapter::new(&pool)` para tests; los services aceptan `Arc<dyn ...>` y los adapters se inyectan una sola vez en `lib.rs:48-54` |
| 4 | CI guard (`check:arch`) da falsos positivos con comentarios `// allow check:arch` | Baja | Documentar la política en el README del script: el comentario requiere PR con label `arch-exception` y justificación |
| 5 | `IpcErrorPayload` puede tener variantes que el frontend no conoce (e.g. `ProjectExists`) | Baja | El catálogo actual de 10 codes ya está cubierto en `tauri-api.ts:35-46`. `ProjectExists` solo aparece en fallback legacy, no se introduce como nueva variante en este change |
| 6 | `std::fs::read_to_string` en `explain_node`/`chat` (items 2, 3) podría no compilar desde `engine` si el crate tiene un `target_os` distinto | Baja | `engine` ya usa `std::fs` indirectamente vía `FileWalker` y `ParserRegistry`; no introduce dependencia nueva |

## Plan de rollback

- **PR-A**: revert del merge commit de PR-A. `pub(crate)` no rompe callers externos verificados; el CI guard se puede desactivar via `npm pkg delete scripts.check:arch` sin re-deploy. Riesgo bajo.
- **PR-B**: revert del merge commit de PR-B. Como PR-B introduce cambios de signatura en 22 comandos + 5 adapters, el revert toca los mismos archivos; no hay migraciones de esquema o datos. Riesgo medio (rollback mecánico, pero extenso en líneas). Si falla, plan B: cerrar PR-B como `draft`, dejar PR-A mergeado, reabrir wave 2 con scope revisado.

## Dependencias y prerrequisitos

- **PR-0 (PR #4)** — hotfix `NodeExplanation` — ✅ mergeado a main en `bdd6686` (basado en `04e4c73`).
- **PR-A branch** — `feat/pre-wave-2-pr-a-foundations` — ya creada desde `main@04e4c73`.
- **PR-B branch** — se creará después del merge de PR-A, desde `main` con PR-A aplicado.
- **`openspec/config.yaml`** — `chained_pr_strategy: auto-forecast` y `review_budget_changed_lines: 400` ya declarados (líneas 12-13).
- **Specs existentes relevantes** (a verificar en `sdd-spec` phase): `error-contract/spec.md` (atomic rollout), `backend-ports-and-services/spec.md` (shape de ports), `frontend-service-layer/spec.md` (justifica el borrado de `src/services/*.ts`).

## Plan de entrega (tentativo, no compromiso)

- **Día 1 (hoy)**: PR-A foundations — implementar items 5, 6, 9, 10; abrir PR; review + merge.
- **Día 2**: PR-B refactor — implementar items 2, 3, 4, 7, 8; abrir PR encadenado; review + merge.
- **Día 3**: `sdd-verify` (correr quality gates completos + tests E2E del parser) y `sdd-archive` (sync delta specs a `openspec/specs/`).

## Apéndice: artefactos relacionados

- **Engram `architecture/hexagonal-wave-1-review`** (obs #654) — el review pre-wave-2 que identificó las 25 fugas y 10 wave-2 blockers. Items #2, #6, #7, #8, #9 del ranking son los que este change cierra.
- **OpenSpec `explore.md`** — `openspec/changes/pre-wave-2-foundation/explore.md` (267 líneas) — exploración detallada con footprint de archivos, riesgos y preguntas abiertas resueltas.
- **OpenSpec specs relevantes**:
  - `openspec/specs/error-contract/spec.md` — el contrato JSON que el helper `to_ipc_error` honra.
  - `openspec/specs/backend-ports-and-services/spec.md` — shape canónico de los 5 ports.
  - `openspec/specs/frontend-service-layer/spec.md` — justificación arquitectónica para borrar `src/services/*.ts`.
- **GitHub PR #4** — el hotfix `04e4c73` que arregla `NodeExplanation.nodeId` y mergea en `bdd6686`. **RESUELTO**.
- **Wave 1 merge** — `503ad80` (PR #3) — la base sobre la que se construye este change.
