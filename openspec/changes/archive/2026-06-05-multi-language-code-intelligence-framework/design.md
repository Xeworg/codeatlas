# Design: multi-language-code-intelligence-framework

## Technical Approach

Consolidate the dual parse path into a single `ParserRegistry` dispatch and extend
the IR (in `engine/src/models/file.rs`) additively with `LexicalValueKind` and
`Reference { kind, target_name, range, file_id }` so every `LanguageParser` impl
emits the same shape. `CodeParser::parse_file` survives as a `#[deprecated]`
**narrowing shim** that calls the registry and extracts `(symbols, imports)` from
the full `ParseResult` (it does NOT change its tuple return type, per the
exploration). The Tauri shim in `src-tauri/src/commands.rs` is rewritten so
`scan_project` parses each file **once** via the registry and threads the same
`ParseResult` through symbols / imports / outline persistence. New language
adapters (Python, Go, Java) plug in via `impl LanguageParser` +
`registry.register(...)` with zero IR or dispatch changes.

## Architecture Decisions

| # | Choice | Alternative | Decision rationale |
|---|--------|-------------|--------------------|
| 1 | Extend `ParseResult` additively (new fields, `#[serde(default)]`) | New parallel `EnrichedParseResult` | Preserves SQLite, frontend reads keep compiling, minimal blast radius. |
| 2 | `LanguageParser` trait grows **default methods** `lexical_kind_for` and `extract_references` | New sub-trait `ReferenceExtractor` | Defaults keep the add-a-language contract trivial (parser may omit them entirely). |
| 3 | `ReferenceKind` enum keeps `Call` / `TypeRef` as **stub variants** even though v1 never emits them | Emit only `Import` / `Export` | AI layer can wire to the full enum now; no schema churn when v2 lands. |
| 4 | `CodeParser::parse_file` shim keeps its `(Vec<SymbolInfo>, Vec<ImportInfo>)` tuple return, internally calls registry | Change return type to `ParseResult` | Tuple signature is consumed at 2 sites today; widening it would force a bigger PR-C diff and break the spec's "same ParseResult observable" wording — we test the fields, not the struct. |
| 5 | New `engine::commands` module holds pure orchestration (`scan_files`, `outline_for_file`); Tauri commands are thin wrappers | Keep orchestration inline in Tauri commands | Pure functions are testable from `cargo test` without `tauri::State`; reusable for the future CLI consumer. |
| 6 | `Reference` carries `file_id` even though it lives inside a per-file `ParseResult` | Omit `file_id` (implied by scope) | Spec scenario #3 lists `file_id` as a required field; explicit beats implicit for a public IR contract. |
| 7 | Stacked-to-main chained PRs (PR-A → PR-B → PR-C), each ≤ 800 lines | Single ~900-line PR | Forecast shows PR-B (TS + Rust overrides) is the heavy slice; splitting isolates risk and gives reviewers a clean IR diff before parser work. |
| 8 | Conservative Rust emission: `target_name = ""` for unresolvable `use` items; v2 fills them | Best-effort name resolution in v1 | Spec scenario explicitly permits empty target_name; deferring resolution matches the "no cross-file work" boundary. |

## Data Flow

### Current (drift) — 3 parses/file in `scan_project`

```
src-tauri::scan_project
  ├─ loop A: CodeParser::parse_file         (FLAT, returns symbols)
  ├─ loop B: CodeParser::parse_file         (FLAT, returns imports)
  └─ loop C: CodeParser::parse_file_all     (= ParserRegistry::default().parse_file)
src-tauri::get_node_outline
  └─ CodeParser::parse_file_all             (registry, OK)
```

### Target (single path) — 1 parse/file

```
src-tauri::scan_project (Tauri shim, AppState + tracing + DB)
  └─ engine::commands::scan_files(registry, files)        (PURE)
       └─ ParserRegistry::parse_file(path, src, ext, id)  (single registry call)
            └─ LanguageParser::parse_all(source, path, id)  (TS or Rust)
                 ├─ symbols + outline
                 ├─ LexicalValueKind via lexical_kind_for(node, src)
                 └─ references via extract_references(node, src)  (same AST walk)
       -> ScanFilesOutput { file_infos, all_imports, parse_ms }
  -> persistence (save_scan_result, save_import, save_outline_items)
  -> single-parsed data drives all three sinks
```

```
src-tauri::get_node_outline (Tauri shim)
  └─ engine::commands::outline_for_file(registry, id, path, src, ext)  (PURE)
       └─ ParserRegistry::parse_file(...) -> result.outline
```

## File Changes

### PR-A — IR + trait + reference stub + Python stub

| File | Action | Description |
|------|--------|-------------|
| `engine/src/models/file.rs` | Modify | Add `LexicalValueKind` (Const / ArrowFunction / Function), `ReferenceKind` (Import / Export / Call* / TypeRef*), `Reference`, `Range`; extend `ParseResult` with `lexical_kind: LexicalValueKind` + `references: Vec<Reference>` (both `#[serde(default)]`). |
| `engine/src/models/file.rs` | Modify | Tests: `lexical_value_kind_serializes_snake_case`, `parse_result_default_has_empty_references`, `reference_roundtrip_preserves_file_id`. |
| `engine/src/scanner/parser/ir_tests.rs` | New | RED tests for IR shape, identity invariant, default methods, and the add-a-language contract. ~120 lines. |
| `engine/src/scanner/parser/mod.rs` | Modify | Re-export `ir::*` types so `crate::models` is the single import path. |
| `engine/src/scanner/parser/traits.rs` | Modify | Add default `lexical_kind_for` (returns `LexicalValueKind::Function`) and `extract_references` (returns `vec![]`); keep file-static helpers. |
| `engine/src/scanner/parser/python_stub.rs` | New | Minimal `PythonParser` implementing `LanguageParser` for `.py` — returns `ParseResult::default()`; demonstrates the contract without a real grammar. ~40 lines. |
| `engine/src/scanner/parser/registry.rs` | Modify | Register `PythonParser` in `ParserRegistry::new()`. |
| `engine/tests/add_a_language.rs` | New | Integration test: create a Python stub, register, scan a `.py` fixture, assert dispatch without IR changes. ~90 lines. |
| `engine/src/scanner/parser/python_stub.rs` | New | Tests for the stub: `python_stub_compiles_with_minimal_trait_impl`, `python_stub_returns_empty_parse_result`. |

**Estimate: ~280 lines.** RED tests must compile-fail before adding the IR fields; GREEN once `file.rs` exports them.

### PR-B — TS arrow detection + Rust `Reference` emission

| File | Action | Description |
|------|--------|-------------|
| `engine/src/scanner/parser/typescript.rs` | Modify | Override `lexical_kind_for`: when a `lexical_declaration` contains a `variable_declarator` whose value child is `arrow_function`, return `LexicalValueKind::ArrowFunction`; for object/array/primitive initialisers keep `Const`; for `function_declaration` return `Function`. Override `extract_references`: emit one `Reference { kind: Import, target_name: spec.name, range: spec.range, file_id }` per `import_specifier` and one `Reference { kind: Export, target_name: declaration.name, range, file_id }` per `export_statement` whose inner is a recognised declaration. **Single pass** — collected inside the existing `parse_all` walk (line 230), no `node.find(...)`. |
| `engine/src/scanner/parser/typescript.rs` | Modify | Wire `SymbolKind::ArrowFunction` in `ts_symbol_kind` for the arrow case; tests in `typescript.rs::tests`: `ts_arrow_field_emits_arrow_function_symbol`, `ts_const_object_keeps_const_symbol`, `ts_import_emits_reference_with_target_name`, `ts_export_emits_reference_with_target_name`, `ts_arrow_and_const_share_single_pass` (asserts no `node.find` calls via a counter injected in a test-only refactor). |
| `engine/tests/fixtures/typescript/arrow_field.ts` | New | `class Svc { handler = (req) => req.body; }` |
| `engine/tests/fixtures/typescript/object_literal.ts` | New | `export const CONFIG = { a: 1, b: () => 2 };` (object literal top-level; verify `Const`, not `ArrowFunction`). |
| `engine/tests/fixtures/typescript/react_const_arrow.tsx` | New | `export const Card = ({title}) => <div>{title}</div>;` |
| `engine/src/scanner/parser/rust.rs` | Modify | Override `extract_references`: for each `use_declaration`, emit one `Reference { kind: Import, target_name: last_path_segment or "" when unresolvable, range: use_declaration range, file_id }`. Override `lexical_kind_for` to return `LexicalValueKind::Function` for `function_item`, `Const` for everything else. |
| `engine/src/scanner/parser/rust.rs` | Modify | Tests: `rust_use_emits_reference_with_last_segment`, `rust_use_glob_emits_empty_target_name`, `rust_function_emits_function_lexical_kind`. |
| `engine/tests/fixtures/rust/struct_impl_trait.rs` | New | `struct S; impl S { fn m(&self) {} }` plus `use std::collections::HashMap;` — verifies conservative emission for `use` while leaving impl-trait method resolution for v2. |

**Estimate: ~380 lines.** Largest slice; the 800-line cap is the binding constraint. Fixtures are 5-15 lines each.

### PR-C — Dispatch consolidation + shim

| File | Action | Description |
|------|--------|-------------|
| `engine/src/commands.rs` | New | Pure orchestration: `pub fn scan_files(registry: &ParserRegistry, files: &[DiscoveredFile], root: &Path) -> ScanFilesOutput` reads each file once, calls `registry.parse_file(...)` once, returns `{ file_infos, all_imports, parse_ms, files_read, files_failed }`. `pub fn outline_for_file(registry: &ParserRegistry, file_id: &str, path: &Path, ext: &str) -> Vec<OutlineItem>` returns the outline from a single registry call. ~100 lines. |
| `engine/src/commands/tests.rs` | New | RED tests with a **mock registry** (counts `parse_file` invocations) asserting `scan_files` calls registry **exactly N** times for N files; `outline_for_file` calls it exactly once. ~80 lines. |
| `src-tauri/src/commands.rs` | Modify | `scan_project`: replace the 3 loops with one call to `engine::commands::scan_files(&ParserRegistry::default(), &discovered, Path::new(&path))`; thread `file_infos` and `all_imports` through the existing persistence block. `get_node_outline`: replace `CodeParser::parse_file_all(...)` with `engine::commands::outline_for_file(&ParserRegistry::default(), &node_id, &abs_path, &file_info.extension)`. Net diff: ~120 lines (the 3 loops collapse to 1; the persistence/tracing/error mapping blocks stay). |
| `engine/src/scanner/code_parser.rs` | Modify | `parse_file` becomes a `#[deprecated(note = "use ParserRegistry::parse_file_all or engine::commands::* instead")] pub fn` whose body calls `ParserRegistry::default().parse_file(path, content, extension, "")` and returns `(result.symbols, result.imports)`. The legacy `extract_ts_symbols` and `extract_rust_symbols` private methods **stay** but are no longer called from the public path — they remain available until the follow-up change removes them. ~25-line diff. |
| `engine/src/scanner/parser/registry.rs` | Modify | `ParserRegistry::parse_file` is unchanged; no API change here. |
| `src-tauri/src/commands.rs` | Modify | Remove `use engine::scanner::CodeParser;` once the 2 tuple call sites are gone; keep it only for the `parse_file_all` call site that the new `outline_for_file` replaces. After PR-C, `CodeParser` is no longer imported in Tauri commands. |

**Estimate: ~340 lines.** The biggest risk in this PR is the `scan_project` rewrite (touching ~250 lines of existing orchestration); mitigation is to keep AppState + tracing + persistence wiring in the Tauri shim and only move the parse loop to `engine::commands`.

## Interfaces / Contracts

```rust
// engine/src/models/file.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LexicalValueKind { #[default] Const, ArrowFunction, Function }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind { Import, Export, Call, TypeRef }   // Call/TypeRef stubs in v1

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub file_id: String,
    pub kind: ReferenceKind,
    pub target_name: String,
    pub range: Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start_byte: usize, pub end_byte: usize,
    pub start_line: u32, pub start_col: u32,
    pub end_line:   u32, pub end_col:   u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseResult {
    pub symbols: Vec<SymbolInfo>,
    pub imports: Vec<ImportInfo>,
    pub outline: Vec<OutlineItem>,
    #[serde(default)] pub lexical_kind: LexicalValueKind,
    #[serde(default)] pub references:    Vec<Reference>,
}
```

```rust
// engine/src/scanner/parser/traits.rs
pub trait LanguageParser: Send + Sync {
    fn language_id(&self) -> &'static str;
    fn extensions(&self)  -> &'static [&'static str];
    fn parse_all(&self, source: &str, path: &str, file_id: &str) -> ParseResult;

    // NEW defaults — parsers may override.
    fn lexical_kind_for(&self, _node: &tree_sitter::Node, _src: &str) -> LexicalValueKind {
        LexicalValueKind::Function
    }
    fn extract_references(&self, _node: &tree_sitter::Node, _src: &str) -> Vec<Reference> {
        Vec::new()
    }
}
```

```rust
// engine/src/scanner/code_parser.rs
#[deprecated(note = "use ParserRegistry::parse_file_all or engine::commands::* instead")]
pub fn parse_file(path: &str, content: &str, extension: &str)
    -> (Vec<SymbolInfo>, Vec<ImportInfo>)
{
    let result = ParserRegistry::default().parse_file(path, content, extension, "");
    (result.symbols, result.imports)
}
```

## Testing Strategy

| Layer | PR | Test | Fixture / Mock |
|-------|----|------|----------------|
| Unit (Rust) | A | `ir_tests::lexical_value_kind_serializes_snake_case` | inline |
| Unit | A | `ir_tests::parse_result_default_has_empty_references` | inline |
| Unit | A | `ir_tests::language_parser_defaults_emit_empty_references` | inline mini-parser |
| Unit | A | `ir_tests::reference_roundtrip_preserves_file_id_and_range` | inline |
| Integration | A | `add_a_language::python_stub_dispatches_without_ir_changes` | `engine/tests/fixtures/python/hello.py` |
| Unit | A | `python_stub::python_stub_returns_empty_parse_result` | inline |
| Unit | B | `typescript::ts_arrow_field_emits_arrow_function_symbol` | `arrow_field.ts` |
| Unit | B | `typescript::ts_const_object_keeps_const_symbol` | `object_literal.ts` |
| Unit | B | `typescript::ts_react_const_arrow_emits_arrow_function` | `react_const_arrow.tsx` |
| Unit | B | `typescript::ts_import_emits_reference_with_target_name` | inline |
| Unit | B | `typescript::ts_export_emits_reference_with_target_name` | inline |
| Unit | B | `rust::rust_use_emits_reference_with_last_segment` | inline |
| Unit | B | `rust::rust_use_glob_emits_empty_target_name` | inline |
| Unit | B | `rust::rust_function_emits_function_lexical_kind` | `struct_impl_trait.rs` |
| Unit | C | `commands::scan_files_calls_registry_exactly_n_times` | mock `ParserRegistry` (counts calls) |
| Unit | C | `commands::outline_for_file_calls_registry_exactly_once` | mock registry |
| Unit | C | `commands::scan_files_threads_lexical_kind_and_references_through` | inline TS source |
| Integration | C | `src-tauri/commands::scan_project_does_not_invoke_parse_file_legacy` | vitest + `invoke` mock, asserts call-site text |
| Gate | A,B,C | `cargo test` (engine + src-tauri), `cargo clippy -- -D warnings`, `cargo fmt --check` | — |
| Bench | B | `engine/benches/scan_benchmarks.rs::parse_ts_fixture_1000_files` | `engine/fixtures/benchmark_ts_1000` (existing) |

## Performance

- **Single AST pass**: `lexical_kind_for` and `extract_references` are invoked from the existing `parse_all` walk (line 230 of `typescript.rs` and line 56 of `rust.rs`). No new `node.find(...)` calls. The `parse_ts_fixture_1000_files` bench in `engine/benches/scan_benchmarks.rs` must show ≤ 1.2× the pre-change median (NFR: same order of magnitude per the proposal).
- **Memory**: `Reference` is `Vec<Reference>` on the result; parsers that do not override `extract_references` allocate nothing (default returns `Vec::new()`). `LexicalValueKind` is a `Copy` enum — zero allocation.
- **Dispatch cost**: `ParserRegistry::parse_file` is a `Vec.iter().find(...)` — O(P) where P = number of registered parsers (currently 3 after PR-A). Constant time in practice.

## Risks (per PR)

| PR | Risk | Likelihood | Mitigation |
|----|------|------------|------------|
| A | Existing `impl LanguageParser` sites break because of trait signature change | Low | All new methods are **defaulted**; no existing override is affected. `cargo build` is the only gate needed. |
| A | IR additions change JSON output shape for downstream consumers (frontend) | Low | Both new fields are `#[serde(default)]` and skipped when empty; frontend keeps parsing unchanged `ParseResult` objects. |
| B | Tree-sitter `node.start_position()` / `end_position()` API differs across `tree-sitter 0.24` patch versions | Low | Pin `tree-sitter = "0.24"` in `engine/Cargo.toml` (line 38); add a regression fixture per grammar. |
| B | `arrow_function` detection misses nested `() => x` inside object initialisers | Med | RED test `ts_object_literal_keeps_const_symbol` asserts top-level object is `Const` even if it contains an arrow method (the inner method would be a child outline item, not the lexical declaration's value). |
| B | `extract_references` accidentally re-walks children (second pass) | Med | Add a counter injected via a `cfg(test)` refactor of the parser walk; test asserts the counter is exactly 1 for a known fixture. |
| C | The `scan_project` rewrite accidentally changes persistence order (FK errors) | Med | Keep the persistence block byte-identical: only the parse loop moves to `engine::commands::scan_files`; the `save_scan_result` / `save_import` / `save_outline_items` calls stay in the same order in `src-tauri/src/commands.rs`. |
| C | `CodeParser::parse_file` shim returns different fields than the legacy flat extractor | Low | Test `code_parser::shim_legacy_parse_file_matches_registry_symbols_and_imports` parses a TS fixture via both paths and asserts `symbols` and `imports` field-by-field equality (range, kind, name, exports). |
| C | `engine::commands` module duplicates AppState concerns | Low | `scan_files` takes `&ParserRegistry` and `&[DiscoveredFile]`; **no** `tauri::State`, no DB, no tracing — those stay in the Tauri shim. The pure function is testable with a mock registry. |

## Rollback

| PR | Strategy |
|----|----------|
| A | `git revert` of the merge commit. `ParseResult` loses `lexical_kind` / `references`; both are `#[serde(default)]`, so old code keeps working. `LanguageParser` trait loses the default methods. `PythonParser` and its registration are removed. No DB migration to undo. |
| B | `git revert` of the merge commit. TS / Rust parsers lose the overrides and fall back to `LanguageParser` defaults (which return `Function` / empty `Vec`). All existing tests continue to pass because the override is additive. |
| C | `git revert` of the merge commit. `scan_project` / `get_node_outline` revert to calling `CodeParser::parse_file` and `CodeParser::parse_file_all` (legacy paths restored). The `engine::commands` module is removed. The shim is removed. The deprecation attribute is removed from `CodeParser::parse_file`. |

## Migration / Sequencing

```
PR-A (IR + trait + Python stub)             ── independent, lands first
        │
        ▼
PR-B (TS arrow + Rust Reference emission)   ── depends on A (overrides trait)
        │
        ▼
PR-C (Dispatch consolidation + shim)        ── depends on A + B (parsers must emit
                                              the new shape before dispatch moves)
```

Total: ~1,000 lines spread across 3 PRs, each PR strictly under the 800-line
review budget. Each PR merges to `main` in order (stacked-to-main). No feature
flag needed; SQLite is untouched, frontend is untouched, AI layer untouched.

## Open Questions

None blocking. The proposal and both delta specs are tight; the only judgement
calls (shim return type, `engine::commands` extraction) are documented above as
decisions with rationale. Two follow-up items deferred (not open questions for
this change):

- v2: cross-file resolution of `Reference::target_name` for Rust `use` paths.
- v2: real Python / Go / Java grammars following the `PythonStub` template.
