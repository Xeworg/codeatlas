# Tasks — v1-mvp-core

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 2800–3600 (total across all PRs) |
| 400-line budget risk | Low (por PR individual) |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 → PR 2 → PR 3 → PR 4a → PR 4b → PR 5a → PR 5b → PR 6 |
| Delivery strategy | auto-chain |
| Chain strategy | stacked-to-main |

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Low
```

---

## Dependency Map

```
PR1: Foundation
 │
 ├─► PR2: Scanner + Parser (Slice A)
 │    │
 │    └─► PR3: Graph Engine (Slice B)
 │         │
 │         ├─► PR4a: UI Shell + Explorer + Stores (Slice C.1)
 │         │    │
 │         │    └─► PR4b: Graph View + Details + Sync (Slice C.2)
 │         │         │
 │         │         └─► PR5a: AI Backend (Slice D)
 │         │              │
 │         │              └─► PR5b: AI UI (Slice D.2)
 │         │                   │
 │         │                   └─► PR6: Hardening (Slice E)
 │         │
 │         └─► PR4a (ya bloqueado por PR3)
 │
 └─► PR3 (ya bloqueado por PR2)
```

Camino crítico: `PR1 → PR2 → PR3 → PR4a → PR4b → PR5a → PR5b → PR6`

---

## PR 1 — Foundation: Scaffolding + Domain Models

### Alcance
Inicializar proyecto completo y definir contratos canónicos.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T1.1 | Inicializar Tauri v2 + React 18 + TS + Vite | `package.json`, `vite.config.ts`, `tsconfig.json`, `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json` | `tauri dev` arranca |
| T1.2 | Configurar Tailwind CSS | `tailwind.config.ts`, `src/styles/index.css`, `postcss.config.js` | Tailwind aplica estilos |
| T1.3 | Instalar React Flow + Zustand | `package.json` | imports resuelven |
| T1.4 | Crear crate `engine/` con estructura base | `engine/Cargo.toml`, `engine/src/lib.rs`, `engine/src/models/mod.rs`, `engine/src/scanner/mod.rs`, `engine/src/graph/mod.rs`, `engine/src/ai/mod.rs`, `engine/src/db/mod.rs` | `cargo test` pasa |
| T1.5 | Definir domain models Rust | `engine/src/models/project.rs`, `engine/src/models/file.rs`, `engine/src/models/graph.rs`, `engine/src/models/ai.rs` | Unit tests de serialización serde |
| T1.6 | Definir tipos canónicos TypeScript | `src/lib/types.ts` | `npm run typecheck` pasa |
| T1.7 | Definir SQLite schema mínimo | `engine/src/db/schema.rs` | Test de creación de tablas |
| T1.8 | Definir AppError con thiserror | `engine/src/models/error.rs` | Tests de Display para variantes |
| T1.9 | Configurar ESLint + Prettier + clippy + fmt | `.eslintrc.cjs`, `.prettierrc`, `rustfmt.toml`, `clippy.toml` | Linters pasan |
| T1.10 | Configurar Husky + lint-staged pre-commit | `.husky/pre-commit`, `package.json` | Pre-commit ejecuta linters |

### Dependencias externas
Ninguna.

### Criterios de aceptación
- [ ] `tauri dev` abre ventana con UI vacía
- [ ] `cargo test` pasa en `engine/`
- [ ] `npm run typecheck && npm run lint` pasan
- [ ] Tipos TS y modelos Rust son estructuralmente equivalentes
- [ ] `cargo fmt --check && cargo clippy -- -D warnings` pasan

### Estimación
~350 líneas cambiadas (PR único).

---

## PR 2 — Scanner + Parser (Slice A)

### Alcance
De carpeta seleccionada a metadatos persistidos en SQLite.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T2.1 | Implementar walker con exclusiones | `engine/src/scanner/walker.rs` | Unit test: fixture con dirs excluidos |
| T2.2 | Integrar Tree-sitter TS/JS/Rust | `engine/src/scanner/parser.rs`, `engine/Cargo.toml` (deps) | Unit test: parse fixture TS, JS, Rust |
| T2.3 | Extraer imports, exports, símbolos | `engine/src/scanner/parser.rs` | Unit test: cada tipo de símbolo por lenguaje |
| T2.4 | Implementar queries SQLite | `engine/src/db/queries.rs` | Unit test: insert/retrieve project, files, symbols, imports |
| T2.5 | Implementar orquestación de scan | `engine/src/lib.rs` (fn `scan_project`) | Integration test con fixture TS |
| T2.6 | Exponer comando Tauri `scan_project` | `engine/src/tauri_commands.rs`, `src-tauri/src/main.rs` | Contract test: invoke devuelve ScanResult |
| T2.7 | Exponer comando Tauri `get_scan_status` | `engine/src/tauri_commands.rs` | Contract test |
| T2.8 | Wiring en `src-tauri/src/main.rs` | `src-tauri/src/main.rs` | `tauri dev` registra comandos |

### Dependencias
- PR 1 (domain models, schema, types)

### Criterios de aceptación
- [ ] Fixture TS de 3 archivos → ScanResult con 3 archivos y symbols correctos
- [ ] Fixture JS de 2 archivos → ScanResult con imports extraídos
- [ ] Fixture Rust de 2 archivos → ScanResult con `use` extraídos
- [ ] Directorios excluidos no aparecen en resultado
- [ ] Datos persistidos en SQLite y recuperables
- [ ] `cargo test` pasa con ≥80% cobertura en scanner/

### Estimación
~380 líneas cambiadas (PR único).

---

## PR 3 — Graph Engine (Slice B)

### Alcance
De metadatos persistidos a `GraphData` servible.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T3.1 | Implementar path resolver relativos | `engine/src/graph/resolver.rs` | Unit test: relativos, parent traversal |
| T3.2 | Implementar path resolver aliases TS | `engine/src/graph/resolver.rs` | Unit test: tsconfig paths ficticio |
| T3.3 | Implementar módulos externos como hoja | `engine/src/graph/resolver.rs` | Unit test: `react`, `lodash` → nodo external |
| T3.4 | Implementar graph builder | `engine/src/graph/builder.rs` | Unit test: 3 archivos interconectados → grafo correcto |
| T3.5 | Implementar clasificación NodeType | `engine/src/graph/builder.rs` | Unit test: cada NodeType se clasifica |
| T3.6 | Implementar graph_cache | `engine/src/db/queries.rs` | Unit test: cache hit/miss |
| T3.7 | Exponer `get_graph` Tauri command | `engine/src/tauri_commands.rs` | Contract test |
| T3.8 | Exponer `get_node_details` Tauri command | `engine/src/tauri_commands.rs` | Contract test |
| T3.9 | Exponer `search_nodes` Tauri command | `engine/src/tauri_commands.rs` | Contract test |
| T3.10 | Integration test con fixture real | `tests/integration/` | 3 archivos TS → grafo con 3 nodos, N aristas |

### Dependencias
- PR 2 (datos persistidos)

### Criterios de aceptación
- [ ] Fixture con imports cruzados → grafo con nodos y aristas correctas
- [ ] `get_graph` devuelve `GraphData` serializable
- [ ] `get_node_details` devuelve deps/dependents correctos
- [ ] `search_nodes` encuentra nodos por nombre parcial
- [ ] Graph cache funciona: segundo call es instantáneo
- [ ] `cargo test` pasa

### Estimación
~370 líneas cambiadas (PR único).

---

## PR 4a — UI Shell + Explorer + Stores (Slice C.1)

### Alcance
Layout base funcional, explorer read-only, stores Zustand, wiring inicial.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T4a.1 | Implementar AppShell (layout 3 columnas + bottom + top + status) | `src/components/layout/AppShell.tsx`, `TopBar.tsx`, `StatusBar.tsx` | Renderiza sin error |
| T4a.2 | Implementar ProjectSelector (diálogo Tauri) | `src/components/onboarding/ProjectSelector.tsx` | Mock invoke test |
| T4a.3 | Implementar projectStore | `src/stores/projectStore.ts` | Vitest: estados idle/scanning/ready/error |
| T4a.4 | Implementar graphStore | `src/stores/graphStore.ts` | Vitest: set graph, selectedNodeId |
| T4a.5 | Implementar chatStore | `src/stores/chatStore.ts` | Vitest: add message, clear |
| T4a.6 | Implementar tauri-api.ts (wrappers tipados) | `src/lib/tauri-api.ts` | Vitest: types compile |
| T4a.7 | Implementar Sidebar (árbol de archivos read-only) | `src/components/layout/Sidebar.tsx` | Renderiza árbol colapsable |
| T4a.8 | Implementar EmptyState, ErrorState, Spinner | `src/components/common/EmptyState.tsx`, `ErrorState.tsx`, `Spinner.tsx` | Vitest: renderiza por variante |
| T4a.9 | Wiring: scan_project → projectStore → UI | `src/App.tsx`, hooks | Integration: scan actualiza estado |

### Dependencias
- PR 3 (endpoints Tauri listos)
- PR 1 (tipos TS, Tailwind)

### Criterios de aceptación
- [ ] Layout se ve correcto (3 columnas + bars)
- [ ] Seleccionar carpeta dispara scan y actualiza estado
- [ ] Explorer muestra árbol de archivos después del scan
- [ ] Stores pasan tests unitarios
- [ ] `npm run typecheck && npm run lint && npm run test` pasan

### Estimación
~380 líneas cambiadas (PR único).

---

## PR 4b — Graph View + Details + Sync (Slice C.2)

### Alcance
Grafo interactivo, panel de detalles, sincronización explorer↔grafo.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T4b.1 | Implementar GraphView con React Flow | `src/components/graph/GraphView.tsx` | Renderiza con datos mock |
| T4b.2 | Implementar GraphNode customizado (colores por NodeType) | `src/components/graph/GraphNode.tsx` | Renderiza por cada NodeType |
| T4b.3 | Implementar auto-layout con Dagre | `src/lib/graph-layout.ts` | Test: layout produce posiciones válidas |
| T4b.4 | Implementar zoom/pan/minimap | `src/components/graph/MiniMap.tsx` | Renderiza |
| T4b.5 | Implementar SearchOverlay (búsqueda + highlight) | `src/components/graph/SearchOverlay.tsx` | Test: búsqueda encuentra nodo |
| T4b.6 | Implementar agrupación colapsable por carpeta | `src/components/graph/GraphView.tsx` | Test: grupo colapsa/expande |
| T4b.7 | Implementar DetailPanel (metadata del nodo) | `src/components/panel/DetailPanel.tsx`, `SymbolList.tsx` | Test: muestra path, deps, symbols |
| T4b.8 | Sincronizar Sidebar ↔ Grafo ↔ DetailPanel | `src/stores/projectStore.ts`, `src/App.tsx` | Test: click explorer → grafo enfoca → detalles actualizan |
| T4b.9 | Implementar useGraph hook | `src/hooks/useGraph.ts` | Vitest: estados loading/ready/error |
| T4b.10 | Highlight de dependencias al hover | `src/components/graph/GraphView.tsx` | Manual QA |

### Dependencias
- PR 4a (layout, stores, explorer)
- PR 3 (GraphData disponible)

### Criterios de aceptación
- [ ] Grafo visible con nodos coloreados por tipo
- [ ] Zoom/pan/minimap funcionales
- [ ] Búsqueda de nodos con resaltado
- [ ] Click en explorer → grafo enfoca → detalles se actualizan
- [ ] Click en grafo → detalles se actualizan
- [ ] Agrupación colapsable por carpeta
- [ ] Highlight de deps al hover
- [ ] Sin lag perceptible en proyecto de ~500 archivos (test manual)
- [ ] `npm run test` pasa

### Estimación
~390 líneas cambiadas (PR único).

---

## PR 5a — AI Backend (Slice D.1)

### Alcance
Trait AIProvider, proveedor Anthropic/MiniMax, motor de contexto, comandos explain_node y chat.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T5a.1 | Definir trait AIProvider | `engine/src/ai/provider.rs` | Compile + doc comment |
| T5a.2 | Definir prompts (explain_node, chat) | `engine/src/ai/prompts.rs` | Unit test: prompt contiene contexto esperado |
| T5a.3 | Implementar motor de contexto acotado | `engine/src/ai/context.rs` | Unit test: top-5 deps, top-3 dependents, 8KB cap |
| T5a.4 | Implementar proveedor Anthropic (MiniMax) | `engine/src/ai/anthropic.rs` | Unit test con mock HTTP |
| T5a.5 | Implementar configuración segura (keyring) | `engine/src/db/queries.rs` (ai_config) | Unit test: save/load config |
| T5a.6 | Exponer `configure_ai` Tauri command | `engine/src/tauri_commands.rs` | Contract test |
| T5a.7 | Exponer `get_ai_config` Tauri command | `engine/src/tauri_commands.rs` | Contract test |
| T5a.8 | Exponer `explain_node` Tauri command | `engine/src/tauri_commands.rs` | Contract test con mock |
| T5a.9 | Exponer `chat` Tauri command | `engine/src/tauri_commands.rs` | Contract test con mock |
| T5a.10 | Manejo de errores IA (timeout, rate-limit, no-key) | `engine/src/ai/anthropic.rs` | Unit test: cada error mapea a AppError |

### Dependencias
- PR 3 (grafo, node details, dependencias)
- PR 1 (AppError, models)

### Criterios de aceptación
- [ ] `explain_node` con proveedor mock → respuesta válida
- [ ] Contexto contiene archivo + deps + dependents (verificado en test)
- [ ] Contexto respeta cap de 8KB
- [ ] `configure_ai` guarda en keyring (test unitario con mock)
- [ ] Errores IA se mapean a variantes correctas de AppError
- [ ] `cargo test` pasa con ≥80% cobertura en ai/

### Estimación
~380 líneas cambiadas (PR único).

---

## PR 5b — AI UI (Slice D.2)

### Alcance
Panel de IA: explicación por nodo, chat contextual, manejo de estados.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T5b.1 | Implementar AIExplanation (resumen + detalles markdown) | `src/components/panel/AIExplanation.tsx` | Renderiza markdown |
| T5b.2 | Implementar MarkdownView reutilizable | `src/components/common/MarkdownView.tsx` | Test: renderiza headings, code, lists |
| T5b.3 | Implementar ChatPanel + ChatMessage + ChatInput | `src/components/chat/ChatPanel.tsx`, `ChatMessage.tsx`, `ChatInput.tsx` | Test: renderiza mensajes, input envía |
| T5b.4 | Implementar useAI hook | `src/hooks/useAI.ts` | Vitest: explain_node, chat loading/ready/error |
| T5b.5 | Wiring: nodo seleccionado → explicación IA automática | `src/App.tsx` | Test: selección dispara explain |
| T5b.6 | Wiring: chat input → chat command → respuesta | `src/components/chat/ChatPanel.tsx` | Test: envío + respuesta |
| T5b.7 | Sugerencias rápidas en chat | `src/components/chat/ChatInput.tsx` | Test: sugerencias renderizan |
| T5b.8 | Manejo UX de errores IA (no-key, timeout, rate-limit) | `src/components/chat/ChatPanel.tsx`, `AIExplanation.tsx` | Test: cada error muestra UI correcta |
| T5b.9 | Configuración de API key (pantalla onboarding) | `src/components/onboarding/ApiKeySetup.tsx` | Test: guarda config |

### Dependencias
- PR 5a (endpoints IA listos)
- PR 4b (nodo seleccionado disponible)

### Criterios de aceptación
- [ ] Seleccionar nodo → se muestra explicación IA en panel
- [ ] Chat envía pregunta y muestra respuesta
- [ ] Markdown se renderiza correctamente
- [ ] Errores IA muestran mensajes legibles con acción sugerida
- [ ] Configuración de API key funciona
- [ ] `npm run test` pasa

### Estimación
~370 líneas cambiadas (PR único).

---

## PR 6 — Hardening + Contract Tests (Slice E)

### Alcance
Error model end-to-end, performance measurement, contract tests, calidad CI.

### Tareas

| ID | Tarea | Archivos principales | Tests requeridos |
|----|-------|---------------------|------------------|
| T6.1 | Implementar ApiError bridge TS (codes mapeados) | `src/lib/types.ts` (ApiError, ErrorCode) | Vitest: cada code mapea |
| T6.2 | Implementar error handling global en tauri-api.ts | `src/lib/tauri-api.ts` | Vitest: error → ApiError tipado |
| T6.3 | Agregar estados loading/empty/error a todos los paneles | `src/components/**` | Vitest: cada panel tiene 3 estados testeados |
| T6.4 | Implementar medición de tiempos por etapa | `engine/src/scanner/walker.rs`, `graph/builder.rs` | Unit test: tiempos > 0 |
| T6.5 | Contract tests de todos los comandos Tauri | `tests/integration/contracts.rs` | Snapshot de cada comando |
| T6.6 | Benchmark de escaneo (fixture 100/500/1000 archivos) | `tests/benchmarks/` | Informativo (no bloqueante) |
| T6.7 | Verificar CI: fmt, clippy, lint, test, typecheck | `.github/workflows/ci.yml` | CI verde |
| T6.8 | Documentar endpoints en CHANGELOG_CONTRATOS | `docs/CHANGELOG_CONTRATOS.md` | Documento actualizado |
| T6.9 | Test E2E manual checklist | `tests/e2e/checklist.md` | Checklist documentado |
| T6.10 | Onboarding checklist actualizado | `docs/ESTANDARES_CODIGO_REUTILIZABLE_Y_ARQUITECTURA.md` | Apéndice A actualizado |

### Dependencias
- PR 5b (feature completa)
- Todos los PRs anteriores

### Criterios de aceptación
- [ ] Todos los comandos Tauri tienen contract test
- [ ] Todos los paneles UI cubren loading/empty/error
- [ ] Benchmarks informativos corren sin panic
- [ ] CI pasa: fmt, clippy, lint, test, typecheck
- [ ] CHANGELOG_CONTRATOS refleja estado actual
- [ ] E2E checklist documentado y ejecutable

### Estimación
~350 líneas cambiadas (PR único).

---

## Resumen de PRs

| PR | Slice | Líneas estimadas | Dependencias |
|----|-------|-----------------|--------------|
| PR 1 | Foundation | ~350 | — |
| PR 2 | A: Scanner+Parser | ~380 | PR 1 |
| PR 3 | B: Graph Engine | ~370 | PR 2 |
| PR 4a | C.1: Shell+Explorer+Stores | ~380 | PR 3, PR 1 |
| PR 4b | C.2: Graph+Details+Sync | ~390 | PR 4a, PR 3 |
| PR 5a | D.1: AI Backend | ~380 | PR 3, PR 1 |
| PR 5b | D.2: AI UI | ~370 | PR 5a, PR 4b |
| PR 6 | E: Hardening | ~350 | PR 5b, todos |
| **Total** | | **~2970** | |

**Todos los PRs están dentro del budget de 400 líneas.** No se requiere split adicional.

---

## Secuencia de Ejecución Recomendada

```
Semana 1-2:  PR 1 (Foundation)
Semana 3-4:  PR 2 (Scanner+Parser)
Semana 5-6:  PR 3 (Graph Engine)
Semana 7-8:  PR 4a (Shell+Explorer)
Semana 9-10: PR 4b (Graph+Details) + PR 5a (AI Backend) [paralelo]
Semana 11-12: PR 5b (AI UI)
Semana 13-14: PR 6 (Hardening) + demo final
```

---

## Definition of Done (global v1)

- [ ] Flujo abrir→escanear→visualizar→inspeccionar→preguntar funciona E2E
- [ ] Todos los PRs mergearon a main
- [ ] CI estable (fmt, clippy, lint, test, typecheck)
- [ ] Performance dentro de umbrales (scan <10s, grafo <100ms, IA <5s)
- [ ] Contratos TS↔Rust sincronizados
- [ ] No-goals v1 respetados (sin features v2/v3)
- [ ] API key en keyring, nunca en texto plano
- [ ] Checklist E2E ejecutado y documentado

---

*Documento generado por SDD Tasks para change `v1-mvp-core`. Última actualización: 2026-05-31.*
