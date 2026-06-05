# Proposal: multi-language-code-intelligence-framework

## Intent

Resolver la divergencia de dispatch entre `CodeParser::parse_file` (ruta heredada plana usada por `src-tauri/src/commands.rs::scan_project` para symbols/imports) y `CodeParser::parse_file_all` (ruta registry usada para outline). Construir un framework reutilizable y multi-lenguaje de code intelligence para análisis orientado a IA. Target v1: TypeScript/TSX + Rust; el framework debe aceptar nuevos lenguajes sin tocar IR ni dispatch.

## Scope

### In Scope
- IR neutral: `LexicalValueKind` (Const/ArrowFunction/Function) + `Reference { kind: Import|Export|Call|TypeRef, target_name, range }` extendiendo `ParseResult` en `engine/src/models/file.rs`
- Wiring de `SymbolKind::ArrowFunction` en parser TypeScript; Rust devuelve `Const` por ahora
- Default methods en `LanguageParser` (`parse_symbols` passthrough, `lexical_value_kind`); cada lenguaje override solo el hook de clasificación
- Consolidación de dispatch: `CodeParser::parse_file` → shim que delega al registry; `commands.rs::scan_project` y `get_node_outline` usan una sola llamada registry
- Fixtures TS/TSX y Rust (clase con arrow-field, object-literal con métodos, React component via const arrow; Rust struct+impl+trait method)
- RED tests en `engine/src/scanner/parser/ir_tests.rs`, `typescript.rs`, `rust.rs`
- Documentación: `docs/code-intelligence/adding-a-language.md`

### Out of Scope
- Nuevos lenguajes (Python/Go/Java) — diferidos a cambios posteriores
- Resolución cross-file de imports/calls/type-refs
- Inferencia de tipos, genéricos, trait/impl resolution
- Cambios al schema SQLite o migraciones
- Cambios al frontend o prompts IA para consumir nuevos campos del IR
- Servidor LSP, integraciones IDE, semantic diffing
- NFR/performance más allá de mantener la ruta registry dentro del mismo orden de magnitud que la ruta heredada

## Capabilities

> Investigado `openspec/specs/`: solo existe `project-understanding/spec.md` (v1/v2/v3/robust-logging). Este cambio es una capa aditiva — no toca requisitos existentes.

### New Capabilities
- `code-intelligence-ir`: IR language-neutral que toda implementación de `LanguageParser` produce. Define `LexicalValueKind`, `Reference`, discriminación arrow-vs-const, rangos estables e invariantes de identidad (`file_id + kind + name + range`) consumibles por la capa IA sin requerir resolución cross-file.
- `multi-language-dispatch`: ruta única de dispatch a través de `ParserRegistry`; `CodeParser::parse_file` queda como shim deprecado que delega al registry. Cierra el doble recorrido de `scan_project` y define el contrato que futuros parsers (Python/Go/Java) deben respetar.

### Modified Capabilities
- None — `project-understanding` mantiene su shape (v1/v2/v3/robust-logging intactos); el cambio es una capa aditiva sobre el spec base.

## Approach

- Extender `ParseResult` con `lexical_kind` y `references: Vec<Reference>` en lugar de crear un tipo paralelo (mantiene la persistencia SQLite intacta).
- `scan_project` y `get_node_outline` llaman al registry una sola vez y derivan symbols/imports/outline/references del mismo `ParseResult`.
- `CodeParser::parse_file` sobrevive como shim deprecado (`#[deprecated(note = "...")]`) que delega al registry; la remoción queda para un follow-up change.
- TypeScript parser emite `Reference` por cada `import_statement` (kind=Import) y `export_statement` (kind=Export); Rust emite la misma forma con defaults conservadores (nombres vacíos donde aplique).
- Cubrir con fixtures TS/TSX y Rust + RED→GREEN tests en `engine/` (cargo test rápido y self-contained; evita el problema del mock Tauri bridge en vitest).
- Documentar el flujo "añadir nuevo lenguaje" para que la próxima PR (Python/Go/Java) requiera SOLO `impl LanguageParser` + `registry.register(...)`.
- Si durante apply la estimación supera 800 líneas, activar fallback de chained PRs (PR-A: IR+trait+reference stub; PR-B: TS arrow detection + Rust stub; PR-C: consolidación de dispatch + shim) — strategy ya documentada en `explore.md` §8.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `engine/src/scanner/parser/traits.rs` | Modified | Default methods `parse_symbols`, `lexical_value_kind`; helpers file-static permanecen |
| `engine/src/scanner/parser/registry.rs` | Unchanged shape | Sigue siendo la superficie de dispatch |
| `engine/src/scanner/parser/typescript.rs` | Modified | Emite `ArrowFunction`, `Reference` (Import/Export), `LexicalValueKind` |
| `engine/src/scanner/parser/rust.rs` | Modified | Misma forma IR, contenido conservador (Const por ahora) |
| `engine/src/scanner/code_parser.rs` | Modified | `parse_file` → shim deprecado que delega al registry |
| `engine/src/models/file.rs` | Modified | Nuevos `LexicalValueKind`, `Reference`; `ParseResult::references` |
| `src-tauri/src/commands.rs` | Modified | `scan_project` y `get_node_outline` usan una sola llamada registry |
| `engine/src/scanner/parser/ir_tests.rs` | New | RED→GREEN tests del contrato IR compartido entre lenguajes |
| `engine/tests/fixtures/typescript/` | New | Fixtures TS/TSX (clase con arrow-field, object-literal, React const arrow) |
| `engine/tests/fixtures/rust/` | New | Fixtures Rust (struct + impl + trait method) |
| `docs/code-intelligence/adding-a-language.md` | New | Pasos: `impl LanguageParser`, registrar, sin tocar IR ni dispatch |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Drift de doble ruta durante la transición | Med | `parse_file` queda como shim deprecado este cambio; remover en follow-up |
| Drift de versión tree-sitter rompe fixtures | Low | Pin 0.24 + tree-sitter-typescript/rust; fixture de regresión por gramática |
| Bloat del IR si se añaden todos los hooks IA ahora | Med | `Reference` queda como stub tipado con default `[]`; diferir call/type-ref resolution |
| Performance por doble recorrido AST | Med | Single pass; sin `find()` loops; benchmark en `engine/benches/` |
| Strict TDD para código Tauri-touching | Med | Lógica IR/trait en `engine/` (cargo test rápido); mock Tauri bridge en vitest para command-level |
| Desviación config 400 vs session 800 líneas | Low | Session value es autoritativa para planning; pedir confirmación al usuario antes de abrir PR |

## Rollback Plan

`git revert` del commit de consolidación restaura la ruta `parse_file` previa desde git history. La extensión del trait y los tipos IR son aditivos (`pub` con default `[]`) — no rompen consumidores existentes. Marcar `parse_file` como `#[deprecated(note = "use ParserRegistry::parse_file_all instead")]` para señalizar la migración sin forzar remoción en este cambio.

## Dependencies

- tree-sitter 0.24, tree-sitter-typescript, tree-sitter-rust (ya pineados en `engine/Cargo.toml`)
- `LanguageParser` + `ParserRegistry` (base de `outline-parser-abstraction`, archivado — no es arquitectura nueva)
- `ParseResult`, `OutlineItem`, `SymbolInfo`, `ImportInfo` (en `engine/src/models/file.rs`; añadir campos es seguro, renombrar/remover no)
- Anthropic provider — el IR debe ser JSON-serializable para `serde_json` + Tauri commands

## Success Criteria

- [ ] `parse_file` y `parse_file_all` comparten una sola ruta registry (sin doble recorrido)
- [ ] `commands.rs::scan_project` y `get_node_outline` llaman al registry una sola vez
- [ ] `LexicalValueKind` y `Reference` definidos y emitidos por parser TS (Rust misma forma con defaults conservadores)
- [ ] RED tests en `ir_tests.rs`/`typescript.rs`/`rust.rs` compilan-fail antes, pasan después
- [ ] Fixtures TS/TSX y Rust producen `ParseResult` estable (mismos IDs en rescans)
- [ ] Parser stub de un cuarto lenguaje (ej. Python) requiere SOLO: `impl LanguageParser` + `registry.register(...)`; sin cambios a IR/dispatch/`CodeParser`
- [ ] `cargo test` (engine + src-tauri), `cargo clippy -- -D warnings`, `npm run lint` verdes sin warnings nuevos
- [ ] Presupuesto 800 líneas honrado O `size:exception` explícito solicitado antes de apply
