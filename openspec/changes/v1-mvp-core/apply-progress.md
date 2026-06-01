# SDD Apply Progress — v1-mvp-core / PR 2: Scanner + Parser

**Status**: ✅ Tests passing (23 Rust + 4 TS)

**Test runner**: `cd engine && cargo test && npm run test`

**TDD mode**: Strict TDD — RED → GREEN → TRIANGULATE → REFACTOR

---

## TDD Cycle Evidence

| Cycle | Phase | Evidence                                                                                          |
| ----- | ----- | ------------------------------------------------------------------------------------------------- |
| 1     | GREEN | walker tests: `walker_excludes_node_modules`, `walker_only_finds_supported_extensions` — 2 passed |
| 2     | GREEN | parser tests: `parse_typescript_function`, `parse_rust_struct`, `parse_imports` — 3 passed        |
| 3     | GREEN | db queries tests: `save_and_retrieve_project` — passed                                            |
| 4     | GREEN | schema tests: `schema_initializes_without_error` — passed                                         |
| 5     | GREEN | All 23 engine tests + 4 TS tests passing — full suite green                                       |

---

## Completed Tasks

| Task                           | Status | Notes                                                                                                                                             |
| ------------------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| T2.1 Walker                    | ✅     | `ignore::WalkBuilder`, exclusiones: `node_modules`, `.git`, `dist`, `build`, `target`, `.next`, `.nuxt`, `.svelte-kit`, `coverage`, `__pycache__` |
| T2.2 Tree-sitter               | ✅     | `tree-sitter-typescript`, `tree-sitter-javascript`, `tree-sitter-rust`                                                                            |
| T2.3 Extracción símbolos       | ✅     | Imports, exports, funciones, classes, structs, impl, interfaces, enums                                                                            |
| T2.4 SQLite queries            | ✅     | `DbPool` (thread-safe con Mutex), `ProjectRepository`, schema 6 tablas                                                                            |
| T2.5 Orquestación scan_project | ✅     | `scan_project` en `src-tauri/src/commands.rs`                                                                                                     |
| T2.6 scan_project command      | ✅     | Tauri command registrado                                                                                                                          |
| T2.7 get_scan_status command   | ✅     | Tauri command registrado                                                                                                                          |
| T2.8 Wiring src-tauri          | ✅     | Commands exportados en `src-tauri/src/lib.rs`                                                                                                     |

## Test Results

```
Rust (cargo test --lib): 23 passed
  - scanner/walker: 2 passed
  - scanner/parser: 3 passed
  - db/schema: 1 passed
  - db/queries: 1 passed
  - (resto de PR1: 16 passed)

TypeScript (npm run test): 4 passed
```

## Files Changed

- `engine/src/scanner/walker.rs` — walker con exclusiones
- `engine/src/scanner/parser.rs` — Tree-sitter TS/JS/Rust
- `engine/src/db/queries.rs` — DbPool thread-safe + repos
- `engine/src/db/schema.rs` — 6 tablas
- `src-tauri/src/commands.rs` — scan_project, get_scan_status
- `src-tauri/src/lib.rs` — registro de comandos

## PR Boundary

This is **PR 2: Scanner + Parser (Slice A)**. Depends on PR 1 (Foundation).
All criteria met, tests green, within 400-line budget.
