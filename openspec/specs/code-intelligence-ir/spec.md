# Code-Intelligence IR Specification

## Purpose

IR language-neutral que toda implementación de `LanguageParser` produce. Extiende `ParseResult` con `lexical_kind: LexicalValueKind`, `references: Vec<Reference>`, rangos estables e invariante de identidad `(file_id, kind, name, range)` que la capa IA consume en v1 sin resolución cross-file. Validado con TS/TSX y Rust; no rompe la persistencia SQLite.

## Requirements

### Requirement: IR Shape — LexicalValueKind y Reference

El sistema MUST extender `ParseResult` con `lexical_kind: LexicalValueKind` y `references: Vec<Reference>`. `LexicalValueKind` MUST discriminar `Const`, `ArrowFunction` y `Function`. `Reference` MUST contener `file_id`, `kind: ReferenceKind` (`Import | Export | Call | TypeRef`), `target_name: String` y `range: Range`. Los nuevos tipos MUST ser `serde::Serialize + Deserialize`. Los campos existentes (`symbols`, `imports`, `outline`) MUST permanecer sin breaking changes.

#### Scenario: ParseResult expone los nuevos campos

- GIVEN un `ParseResult` producido por un `LanguageParser` registrado
- WHEN un consumidor accede a `result.lexical_kind` y `result.references`
- THEN el sistema MUST retornar los valores poblados
- AND el compilador MUST aceptar el acceso como público

#### Scenario: LexicalValueKind discrimina arrow-vs-const

- GIVEN una declaración TS `const Component = () => <div/>`
- WHEN el parser evalúa la lexical declaration
- THEN el sistema MUST emitir `LexicalValueKind::ArrowFunction`
- AND para `const CONFIG = {...}` MUST emitir `LexicalValueKind::Const`

### Requirement: Invariante de Identidad Estable

El sistema MUST garantizar que `(file_id, kind, name, range)` produce IDs estables entre re-escaneos del mismo archivo sin modificaciones. El sistema MUST NOT depender de orden de recorrido, direcciones de memoria ni contadores incrementales.

#### Scenario: Re-scan produce mismos IDs

- GIVEN un fixture TS o Rust parseado dos veces consecutivas sin modificación
- WHEN se comparan `SymbolInfo.id` y `OutlineItem.id` resultantes
- THEN el sistema MUST producir IDs idénticos para los mismos rangos y nombres

### Requirement: Contrato de Emisión de Reference

El parser de TypeScript MUST emitir una `Reference` por cada `import_statement` (`kind=Import`) y por cada `export_statement` (`kind=Export`). El parser de Rust MUST emitir la misma forma con defaults conservadores (`target_name=""` cuando el nombre no sea resoluble). El sistema MUST emitir todas las `Reference` en el mismo recorrido AST que `symbols` y `outline`.

#### Scenario: TS emite Import reference

- GIVEN un archivo TS con `import { foo } from "./bar"`
- WHEN el parser completa el recorrido
- THEN `result.references` MUST contener `Reference { kind: Import, target_name: "foo" }`

#### Scenario: Rust emite forma conservadora

- GIVEN un archivo Rust con `use std::collections::HashMap`
- WHEN el parser completa el recorrido
- THEN `result.references` MUST contener `Reference { kind: Import, target_name: "HashMap" }`
- AND si no resuelve un nombre, MUST emitir `target_name: ""`

### Requirement: Trait Extension sin Duplicación

El trait `LanguageParser` MUST exponer default methods: `parse_symbols(&ParseResult) -> &[SymbolInfo]` (passthrough) y `lexical_value_kind(node) -> LexicalValueKind`. Cada parser concreto MAY override solo el hook de clasificación. Helpers file-static (`ts_node_kind_to_outline_kind`, `rust_node_kind_to_outline_kind`, `make_outline_id`) MUST permanecer como utilidades reutilizables.

#### Scenario: Parser mínimo compila sin overrides

- GIVEN un parser que implementa solo `language_id`, `extensions`, `parse_all` y `supports`
- WHEN se compila contra el trait extendido
- THEN el compilador MUST aceptar el impl sin overrides
- AND el parser MUST heredar el default de `parse_symbols` y `lexical_value_kind`

### Requirement: Add-a-Language Contract

Añadir un nuevo lenguaje MUST requerir SOLO: (1) `impl LanguageParser` con los métodos del trait, y (2) `registry.register(...)`. El sistema MUST NOT requerir cambios a `ParseResult`, `Reference`, `LexicalValueKind`, `ParserRegistry` ni `CodeParser` para incorporar un cuarto parser.

#### Scenario: Stub de cuarto lenguaje se registra sin tocar IR

- GIVEN un trait y registry extendidos
- WHEN se añade un stub `PythonParser` y se llama `registry.register(...)`
- THEN el sistema MUST despachar archivos `.py` al nuevo parser
- AND MUST NOT requerir diffs en `engine/src/models/file.rs`, `engine/src/scanner/parser/traits.rs` ni `engine/src/scanner/parser/registry.rs`

### Requirement: Single AST Pass

El sistema MUST construir `symbols`, `outline`, `lexical_kind` y `references` en un único recorrido del AST por archivo. El sistema MUST NOT invocar `node.find(...)` ni loops adicionales sobre el árbol.

#### Scenario: Parser no invoca second-pass

- GIVEN un fixture TS o Rust arbitrario
- WHEN el parser completa `parse_all`
- THEN el sistema MUST haber realizado un único recorrido lineal
- AND un bench comparativo MUST evidenciar coste asintótico no superior al heredado

## Out of Scope

- Parsers para Python, Go, Java u otros lenguajes más allá de TS/TSX y Rust.
- Resolución cross-file de `Reference` (imports/calls/type-refs): el IR v1 expone hechos crudos, no la resolución.
- Inferencia de tipos, genéricos, trait/impl resolution y control-flow graphs.
- Cambios al schema SQLite o migraciones: los nuevos campos viven solo en memoria en v1.
- Consumidores en frontend o prompts IA para los nuevos campos del IR.
- Servidor LSP, integraciones IDE y semantic diffing.
