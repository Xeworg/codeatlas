# Tasks — outline-parser-abstraction

> Generated from: `proposal.md`, `specs/project-understanding/spec.md`, `design.md`.
> Language: español (convención proyecto). Strict TDD activo para apply/verify.

---

## Review Workload Forecast

| Field                   | Value                                     |
| ----------------------- | ----------------------------------------- |
| Estimated changed lines | ~700–1320                                 |
| 600-line budget risk    | High if implemented as one PR             |
| Chained PRs recommended | Yes                                       |
| Suggested split         | PR1 → PR2 → PR3 → PR4                     |
| Delivery strategy       | auto-chain                                |
| Chain strategy          | stacked-to-main or separate review slices |

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
600-line budget risk: High
```

**Justificación**: el cambio toca parser backend, modelos compartidos, migración SQLite, comandos Tauri, UI React y contexto IA. Aunque cada parte es razonable, combinarlas en un único PR aumentaría el riesgo de revisión y regresiones.

---

## Dependency Graph

```text
PR1 Semantic Parser Foundation
  └─► PR2 Persistence + Tauri API
        ├─► PR3 Outline UI Panel
        └─► PR4 Semantic AI Context
```

**Critical path**: PR1 → PR2 → PR4

PR3 puede avanzar después de PR2. PR4 depende de outline recuperable por backend.

---

## Test Strategy by Slice

| Slice | Backend checks                       | Frontend checks               | Evidence command                     |
| ----- | ------------------------------------ | ----------------------------- | ------------------------------------ |
| PR1   | parser fixtures, model serialization | typecheck if TS types touched | `cargo test`                         |
| PR2   | migration/query/command tests        | API typecheck                 | `cargo test` + `npm run typecheck`   |
| PR3   | n/a or contract smoke                | component tests               | `npm run test` + `npm run typecheck` |
| PR4   | context builder tests                | n/a initially                 | `cargo test`                         |

Full gate before final verify:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `npm run lint`
- `npm run test`
- `npm run typecheck`

---

# PR1 — Semantic Parser Foundation

**Objetivo**: Crear la base de contratos y parsers Tree-sitter sin cambiar comportamiento visible. Mantener compatibilidad de `CodeParser::parse_file()`.

**Changed files estimate**: ~250–450 líneas

## Tareas

- [x] **T1.1 RED — Fixtures parser TypeScript/Rust**
  - Agregar fixtures o tests inline para:
    - TypeScript/TSX: clase con métodos + interface/type + function.
    - Rust: struct + impl method + function/module.
  - Tests esperados inicialmente fallan porque no existe outline.

- [x] **T1.2 Modelos Rust de outline**
  - En `engine/src/models/file.rs`, agregar:
    - `OutlineItemKind`
    - `OutlineItem`
    - `ParseResult`
  - Re-exportar si hace falta desde `engine/src/models/mod.rs`.
  - Agregar test de serialización camelCase/snake_case.

- [x] **T1.3 Contratos parser**
  - Crear módulo parser incremental:
    - `engine/src/scanner/parser/traits.rs`
    - `engine/src/scanner/parser/registry.rs`
    - `engine/src/scanner/parser/typescript.rs`
    - `engine/src/scanner/parser/rust.rs`
  - Definir `LanguageParser` con `parse_all()`.
  - Definir `ParserRegistry` con fallback `ParseResult::default()`.

- [x] **T1.4 Compat facade**
  - Mantener `CodeParser::parse_file()` devolviendo `(Vec<SymbolInfo>, Vec<ImportInfo>)`.
  - Agregar `CodeParser::parse_file_all(path, content, extension, file_id) -> ParseResult`.
  - Asegurar que tests existentes de símbolos/imports sigan pasando.

- [x] **T1.5 Outline TypeScript/TSX básico**
  - Mapear nodos iniciales:
    - `class_declaration` → `class`
    - `method_definition` → `method`
    - `function_declaration` → `function`
    - `interface_declaration` → `interface`
    - `type_alias_declaration` → `type`
    - `enum_declaration` → `enum`
    - `lexical_declaration` → `const`/`variable`
  - Capturar hijos de clase cuando Tree-sitter expone jerarquía.
  - Poblar rangos línea/columna.

- [x] **T1.6 Outline Rust básico**
  - Mapear nodos iniciales:
    - `struct_item` → `struct`
    - `enum_item` → `enum`
    - `function_item` → `function`
    - `impl_item` → `impl`
    - `mod_item` → `module`
    - type alias según nodo real (`type_item` / `type_alias_item`).
  - Capturar métodos dentro de `impl_item` cuando sea posible.
  - Poblar rangos línea/columna.

- [x] **T1.7 IDs estables de outline**
  - Implementar helper:
    - `outline:<file_id>:<kind>:<line_start>:<line_end>:<name>`
  - Evitar UUID para outline salvo que haya razón explícita.

- [x] **T1.8 GREEN / TRIANGULATE**
  - Ejecutar `cargo test`.
  - Agregar al menos un caso extra si un parser pasa por accidente pero no valida hijos.

## Dependencies

Ninguna. Primer slice.

## Acceptance

- `CodeParser::parse_file()` preserva compatibilidad.
- `parse_file_all()` produce `ParseResult` con outline para TS/TSX y Rust.
- Unsupported extension devuelve resultado vacío sin crash.
- Fixtures validan top-level y nested outline items.
- `cargo test` verde.

---

# PR2 — Outline Persistence + Tauri API

**Objetivo**: Persistir outline por archivo y exponerlo al frontend con `get_node_outline(file_id)`.

**Changed files estimate**: ~180–320 líneas

## Tareas

- [x] **T2.1 RED — Roundtrip de persistencia outline**
  - Agregar test que intenta guardar y recuperar `Vec<OutlineItem>` anidado.
  - Debe fallar antes de queries/migración.

- [x] **T2.2 Migration 007**
  - Crear `engine/migrations/007_outline_items.sql`:
    - `outline_items(file_id TEXT PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE, outline_json TEXT NOT NULL, generated_at TEXT NOT NULL DEFAULT (datetime('now')))`
    - índice por `generated_at` si se mantiene.
  - Actualizar `engine/src/db/migrations.rs`:
    - `CURRENT_SCHEMA_VERSION = 7`
    - registrar migración file-backed.

- [x] **T2.3 Queries outline**
  - En `engine/src/db/queries.rs`, agregar:
    - `save_outline_items(file_id, items)`
    - `get_outline_items(file_id) -> Vec<OutlineItem>`
  - Usar `serde_json` para `outline_json`.
  - Decisión: archivo conocido sin outline → `Vec::new()`; id inválido puede ser error o vacío según patrón existente.

- [x] **T2.4 Integrar `scan_project`**
  - Migrar parsing en `src-tauri/src/commands.rs` hacia `parse_file_all()` donde sea seguro.
  - Mantener invariantes recientes:
    - archivos/proyecto existen antes de persistir datos dependientes;
    - `file_id` de outline debe ser UUID real de DB, no path.
  - Persistir outline después de tener `file_id` autoritativo.

- [x] **T2.5 Comando Tauri**
  - Agregar `get_node_outline(file_id, state)` en `src-tauri/src/commands.rs`.
  - Registrar comando en `src-tauri/src/lib.rs`.
  - Mapear errores a `String` sin panics.

- [x] **T2.6 Tipos TS + wrapper**
  - En `src/lib/types.ts`, agregar:
    - `OutlineItemKind`
    - `OutlineItem`
  - En `src/lib/tauri-api.ts`, agregar:
    - `getNodeOutline(fileId: string): Promise<OutlineItem[]>`

- [x] **T2.7 GREEN / Regression checks**
  - Ejecutar `cargo test`.
  - Ejecutar `npm run typecheck`.
  - Si existe test de scan/imports, confirmar que sigue verde.

## Dependencies

- PR1 completo.

## Acceptance

- Migración 007 es aditiva.
- Outline se guarda/recupera por `file_id`.
- `get_node_outline` devuelve árbol serializable.
- `scan_project`, imports y graph no regresan.
- `cargo test` y `npm run typecheck` verdes.

---

# PR3 — Outline UI Panel

**Objetivo**: Mostrar outline jerárquico en el panel lateral sin saturar el grafo.

**Changed files estimate**: ~150–300 líneas

## Tareas

- [x] **T3.1 RED — OutlineView component test**
  - Test de render de árbol con padre/hijo.
  - Test de empty state.
  - Test de collapse/expand si se implementa desde el inicio.

- [x] **T3.2 Crear `OutlineView.tsx`**
  - Render recursivo.
  - Mostrar kind, name, line range.
  - Indentar hijos.
  - Estado vacío: `No symbols detected` o equivalente según idioma UI existente.
  - Mantener estilos consistentes con panel actual.

- [x] **T3.3 Integrar en `DetailPanel.tsx`**
  - Cargar `getNodeDetails(selectedNodeId)` y `getNodeOutline(selectedNodeId)`.
  - Manejar estados independientes:
    - `outlineLoading`
    - `outlineError`
    - `outline`
  - Renderizar outline antes de `SymbolList` o reemplazar `SymbolList` cuando haya outline.

- [x] **T3.4 Mantener graph node compacto**
  - No agregar árbol completo a `GraphNodeComponent.tsx`.
  - Opcional: mantener solo conteo/resumen existente.

- [x] **T3.5 Error/loading UX**
  - Si falla outline, mostrar warning local y conservar detalles base.
  - Si está cargando, mostrar spinner/texto local.
  - Si vacío, mostrar estado vacío claro.

- [x] **T3.6 GREEN**
  - Ejecutar `npm run test`.
  - Ejecutar `npm run typecheck`.
  - Ejecutar `npm run lint` si el slice cambia UI suficiente.

## Dependencies

- PR2 completo.

## Acceptance

- Al seleccionar nodo, el panel lateral muestra outline si existe.
- Empty/error/loading no rompen detalles ni grafo.
- El grafo sigue compacto.
- Tests frontend/typecheck verdes.

---

# PR4 — Semantic AI Context

**Objetivo**: Usar outline/imports/dependencias para contexto IA antes de recurrir a fuente truncada.

**Changed files estimate**: ~120–250 líneas

## Tareas

- [x] **T4.1 RED — Tests de contexto semántico**
  - Test: contexto con outline incluye jerarquía y relaciones.
  - Test: contexto respeta `MAX_CONTEXT_BYTES`.
  - Test: outline vacío cae al comportamiento previo.
  - Test: extracción por rango respeta `lineStart`/`lineEnd` si se implementa en este slice.

- [x] **T4.2 Extender `ContextBuilder`**
  - En `engine/src/ai/context.rs`, agregar API compatible:
    - `AiContextMode` (`Summary`, `Focused`, `Full`) o equivalente.
    - `build_node_context_with_outline(...)`.
  - Mantener `build_node_context()` existente como fallback.

- [x] **T4.3 Render semántico bounded**
  - Incluir:
    - archivo/tipo/símbolos;
    - outline depth-first con límite;
    - dependencias;
    - dependientes;
    - indicador `(...más símbolos...)` si se trunca.
  - No superar `MAX_CONTEXT_BYTES`.

- [x] **T4.4 Extractos por rango**
  - Agregar helper para extraer líneas por rango desde `file_content`.
  - Usarlo solo en modo `Focused` o cuando haya símbolo relevante.
  - Si no se puede determinar relevancia todavía, dejar helper testeado y no forzar uso amplio.

- [x] **T4.5 Integrar `explain_node`**
  - En `src-tauri/src/commands.rs`, intentar cargar outline para el nodo.
  - Si hay outline, usar `build_node_context_with_outline`.
  - Si no hay outline o falla recuperación, usar `build_node_context` actual.
  - No expandir `chat` en este slice salvo refactor mínimo necesario.

- [x] **T4.6 GREEN**
  - Ejecutar `cargo test`.
  - Verificar que tests previos de contexto siguen verdes.

## Dependencies

- PR2 completo.
- PR3 no es estrictamente requerido para backend IA, pero ayuda a validar manualmente outline.

## Acceptance

- `explain_node` usa outline cuando está disponible.
- Fallback anterior sigue funcionando.
- Contexto permanece acotado.
- No se introduce dependencia de IA para extraer símbolos.
- `cargo test` verde.

---

## Final Verify Checklist

Antes de considerar completo `outline-parser-abstraction`:

- [ ] `cargo fmt --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo test`
- [ ] `npm run lint`
- [ ] `npm run test`
- [ ] `npm run typecheck`
- [ ] Scan manual de proyecto pequeño muestra graph funcional.
- [ ] Seleccionar nodo muestra outline o empty state.
- [ ] Explicación IA de nodo incluye outline semántico cuando hay datos.
- [ ] `docs/PLAN_OUTLINE_TREE_SITTER_Y_PARSERS.md` sigue alineado con implementación real o se actualiza.

---

## Out of Scope Backlog

Diferir a futuros cambios SDD:

- búsqueda global de símbolos;
- navegación a editor/línea real;
- método-level dependency graph;
- soporte Python/Go completo;
- normalización relacional de todos los outline items;
- ranking semántico avanzado para chat de proyecto completo;
- outline expandido dentro de cada nodo del grafo.
