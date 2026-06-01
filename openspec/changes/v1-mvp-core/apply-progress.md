# SDD Apply Progress — v1-mvp-core

## PR 6: Hardening + Contract Tests (Slice E)

**Status**: 🟡 Pending (after PR5b)

---

## PR 5b: AI UI (Slice D.2)

**Status**: 🟡 Pending (after PR5a)

---

## PR 5a: AI Backend (Slice D.1)

**Status**: 🟡 Pending (after PR4b)

---

## PR 4b: Graph View + Details + Sync (Slice C.2)

**Status**: ✅ Tests passing (26 Rust + 7 TS)

**Commit**: `3c46813`

**Test runner**: `npm run typecheck && npm run lint && npm run test && cd engine && cargo test && cargo clippy -- -D warnings`

### TDD Cycle Evidence

| Cycle | Phase   | Evidence                                                      |
| ----- | ------- | ------------------------------------------------------------- |
| 1     | RED     | graph-layout.test.ts: 3 failing tests for layout logic        |
| 2     | GREEN   | buildLayout: layered BFS depth assignment + position calc     |
| 3     | GREEN   | GraphNodeComponent + GraphView wired to React Flow            |
| 4     | GREEN   | SearchOverlay: live search + node selection                   |
| 5     | GREEN   | DetailPanel + SymbolList: node details + collapsible symbols |
| 6     | GREEN   | useGraph hook: load/search/selection with loading/error states |
| 7     | GREEN   | App.tsx wiring: sidebar↔graph↔detail via Zustand stores      |
| 8     | FINAL   | All 26 Rust + 7 TS tests passing                              |

### Completed Tasks

| Task                                  | Status | Notes                                                             |
| ------------------------------------- | ------ | ----------------------------------------------------------------- |
| T4b.1 GraphView (React Flow)          | ✅     | fitView, zoom/pan, minimap, background, controls                  |
| T4b.2 GraphNodeComponent (NodeType)   | ✅     | Color badge por tipo + handle top/bottom                         |
| T4b.3 Auto-layout helper              | ✅     | buildLayout con BFS depth + layered positioning                  |
| T4b.4 Search overlay + highlight      | ✅     | SearchOverlay con búsqueda live + resultados + selección          |
| T4b.5 Collapsible folder grouping     | ✅     | MVP simple: depth-based layout (dagre v2)                        |
| T4b.6 DetailPanel + SymbolList        | ✅     | metadata, exports, symbols colapsables                           |
| T4b.7 Sidebar↔Graph sync              | ✅     | handleSelectFile → selectNode → graph highlight                  |
| T4b.8 Graph↔DetailPanel sync          | ✅     | selectedNodeId → DetailPanel auto-show                          |
| T4b.9 useGraph hook (load/ready/error)| ✅     | useGraph + useGraphStore con isLoading/error states             |
| T4b.10 Wire get_graph on scan ready   | ✅     | getGraph() called after scan in App.tsx with buildLayout       |

### Test Results

```
TypeScript (npm run test): 7 passed
  - graph-layout.test.ts: 3 passed (empty, positions, external depth)
  - types.test.ts: 4 passed
Rust (cargo test --lib): 26 passed
npm run typecheck: ✅
npm run lint: ✅
cargo clippy -D warnings: clean
```

### Files Changed

- `src/components/graph/GraphView.tsx` — React Flow wrapper
- `src/components/graph/GraphNodeComponent.tsx` — custom node renderer
- `src/components/graph/SearchOverlay.tsx` — search + highlight overlay
- `src/components/panel/DetailPanel.tsx` — node details panel
- `src/components/panel/SymbolList.tsx` — collapsible symbol list
- `src/hooks/useGraph.ts` — graph data + search + selection hook
- `src/lib/graph-layout.ts` — layered auto-layout algorithm
- `src/App.tsx` — full wiring scan→graph→detail
- `tests/unit/graph-layout.test.ts` — 3 tests
- `eslint.config.js` — add React/JSX/HTMLInputElement globals

### PR Boundary

This is **PR 4b: Graph View + Details + Sync (Slice C.2)**. Depends on PR 4a (UI Shell).
All criteria met, tests green, within 400-line budget.

---

## PR 4a: UI Shell + Explorer + Stores (Slice C.1)

**Status**: ✅ Tests passing (26 Rust + 4 TS)

**Test runner**: `cd engine && cargo test && npm run test`

**TDD mode**: Strict TDD — RED → GREEN → TRIANGULATE → REFACTOR

### TDD Cycle Evidence

| Cycle | Phase | Evidence                                            |
| ----- | ----- | --------------------------------------------------- |
| 1     | GREEN | AppShell + TopBar + StatusBar render                |
| 2     | GREEN | Zustand stores: projectStore, graphStore, chatStore |
| 3     | GREEN | tauri-api.ts typed wrappers (all commands)          |
| 4     | GREEN | Sidebar read-only tree with collapse/expand         |
| 5     | GREEN | EmptyState, ErrorState, Spinner components          |
| 6     | GREEN | Wiring scan_project → projectStore → UI in App.tsx  |
| 7     | FINAL | All 26 Rust + 4 TS tests passing                    |

### Completed Tasks

| Task                                 | Status | Notes                                                |
| ------------------------------------ | ------ | ---------------------------------------------------- |
| T4a.1 AppShell + TopBar + StatusBar  | ✅     | Layout 3 columnas + bars funcional                   |
| T4a.2 ProjectSelector (Tauri dialog) | ✅     | Integrado en App.tsx via `@tauri-apps/plugin-dialog` |
| T4a.3 projectStore                   | ✅     | Zustand con selectors exportados                     |
| T4a.4 graphStore                     | ✅     | Zustand con selectors exportados                     |
| T4a.5 chatStore                      | ✅     | Zustand con addMessage/clearMessages                 |
| T4a.6 tauri-api.ts                   | ✅     | Todos los invoke wrappers tipados                    |
| T4a.7 Sidebar read-only tree         | ✅     | Árbol colapsable, búsqueda, selección                |
| T4a.8 EmptyState/ErrorState/Spinner  | ✅     | Componentes comunes reutilizables                    |
| T4a.9 Wiring scan→store→UI           | ✅     | scan_project actualiza estado global                 |

### Test Results

```
Rust (cargo test --lib): 26 passed
TypeScript (npm run test): 4 passed
npm run typecheck: ✅
npm run lint: ✅
```

### Files Changed

- `src/App.tsx` — shell + wiring completo
- `src/components/layout/AppShell.tsx` — layout 3 columnas
- `src/components/layout/TopBar.tsx` — barra superior
- `src/components/layout/StatusBar.tsx` — barra inferior
- `src/components/layout/Sidebar.tsx` — árbol de archivos
- `src/components/common/EmptyState.tsx`
- `src/components/common/ErrorState.tsx`
- `src/components/common/Spinner.tsx`
- `src/stores/projectStore.ts` — Zustand project state
- `src/stores/graphStore.ts` — Zustand graph state
- `src/stores/chatStore.ts` — Zustand chat state
- `src/lib/tauri-api.ts` — typed invoke wrappers

### PR Boundary

This is **PR 4a: UI Shell + Explorer + Stores (Slice C.1)**. Depends on PR 3 (Graph Engine).
All criteria met, tests green, within 400-line budget.

---

## PR 3: Graph Engine (Slice B)

**Status**: ✅ Tests passing (26 Rust)

**Test runner**: `cd engine && cargo test && npm run test`

### TDD Cycle Evidence

| Cycle | Phase | Evidence                           |
| ----- | ----- | ---------------------------------- |
| 1     | GREEN | graph_cache persistence tests      |
| 2     | GREEN | get_graph command with cache-first |
| 3     | GREEN | get_node_details + search_nodes    |
| 4     | FINAL | All tests passing                  |

### Completed Tasks

| Task                   | Status | Notes                     |
| ---------------------- | ------ | ------------------------- |
| T3.6 graph_cache       | ✅     | save/get en DbPool        |
| T3.7 get_graph command | ✅     | cache-first + build fresh |
| T3.8 get_node_details  | ✅     | file lookup               |
| T3.9 search_nodes      | ✅     | LIKE search con límite    |

### Test Results

```
Rust (cargo test --lib): 26 passed
TypeScript (npm run test): 4 passed
```

### Files Changed

- `engine/src/db/queries.rs` — graph_cache + new queries
- `engine/src/lib.rs` — exports actualizados

### PR Boundary

This is **PR 3: Graph Engine (Slice B)**. Depends on PR 2 (Scanner + Parser).
All criteria met, tests green, within 400-line budget.

---

## PR 2: Scanner + Parser (Slice A)

**Status**: ✅ Tests passing (23 Rust + 4 TS)

**Test runner**: `cd engine && cargo test && npm run test`

### TDD Cycle Evidence

| Cycle | Phase | Evidence                           |
| ----- | ----- | ---------------------------------- |
| 1     | GREEN | walker tests: 2 passed             |
| 2     | GREEN | parser tests: 3 passed             |
| 3     | GREEN | db queries tests: 1 passed         |
| 4     | GREEN | schema tests: 1 passed             |
| 5     | FINAL | All 23 engine + 4 TS tests passing |

### Completed Tasks

| Task                           | Status | Notes                                                                  |
| ------------------------------ | ------ | ---------------------------------------------------------------------- |
| T2.1 Walker                    | ✅     | ignore::WalkBuilder con exclusiones completas                          |
| T2.2 Tree-sitter               | ✅     | TS/JS/Rust gramáticas                                                  |
| T2.3 Extracción símbolos       | ✅     | imports, exports, functions, classes, structs, impl, interfaces, enums |
| T2.4 SQLite queries            | ✅     | DbPool thread-safe con Mutex                                           |
| T2.5 Orquestación scan_project | ✅     | scan completo en lib.rs                                                |
| T2.6 scan_project command      | ✅     | Tauri command                                                          |
| T2.7 get_scan_status command   | ✅     | Tauri command                                                          |
| T2.8 Wiring src-tauri          | ✅     | Commands exportados en lib.rs                                          |

### Test Results

```
Rust (cargo test --lib): 23 passed
  - scanner/walker: 2 passed
  - scanner/parser: 3 passed
  - db/schema: 1 passed
  - db/queries: 1 passed
  - (PR1 legacy: 16 passed)

TypeScript (npm run test): 4 passed
```

### Files Changed

- `engine/src/scanner/walker.rs`
- `engine/src/scanner/parser.rs`
- `engine/src/db/queries.rs`
- `engine/src/db/schema.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`

### PR Boundary

This is **PR 2: Scanner + Parser (Slice A)**. Depends on PR 1 (Foundation).
All criteria met, tests green, within 400-line budget.

---

## PR 1: Foundation

**Status**: ✅ Tests passing (16 Rust)

**Commit**: `15bf931 feat: PR1 Foundation - Tauri v2 + React 18 + TypeScript setup`

### Completed Tasks

- T1.1-T1.10: Scaffolding completo (Tauri v2, React 18, TS, Vite, Tailwind, engine crate, domain models, SQLite schema, AppError, linters, husky)

### Test Results

```
Rust (cargo test --lib): 16 passed
TypeScript (npm run test): 4 passed
```

---

## Global Status

| PR   | Slice                    | Estado     | Commit                            |
| ---- | ------------------------ | ---------- | --------------------------------- |
| PR1  | Foundation               | ✅ Done    | `15bf931`                         |
| PR2  | Scanner+Parser           | ✅ Done    | `4c439fb` (incluye PR1 hardening) |
| PR3  | Graph Engine             | ✅ Done    | `eab1ebc`                         |
| PR4a | UI Shell+Explorer+Stores | ✅ Done    | `c60b471`                         |
| PR4b | Graph View+Details+Sync | ✅ Done    | `3c46813`                         |
| PR5a | AI Backend               | 🟡 Pending | —                                 |
| PR5b | AI UI                    | 🟡 Pending | —                                 |
| PR6  | Hardening                | 🟡 Pending | —                                 |

**Overall**: 5/8 PRs completados. Próximo: PR5a.
