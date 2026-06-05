# Explore — `multi-language-code-intelligence-framework`

Status: explore
Date: 2026-06-04
Owner: codeatlas
Session preflight: interactive / OpenSpec + Engram / auto-forecast / 800 changed lines

## 1. Phase envelope

- status: completed
- executive_summary: CodeAtlas already has a per-language parser trait and a working
  TypeScript/Rust registry, but extraction is fragmented. There are two parse paths
  (`CodeParser::parse_file` legacy flat, `CodeParser::parse_file_all` registry-based)
  and the registry parsers do not yet cover what an AI code-analysis layer actually
  needs: lexical declaration discrimination, object-literal and class-field arrow
  methods, richer symbol metadata, ranges/spans, and explicit relationship data
  (call/import/export edges, type references). The first implementation slice
  should be a language-neutral intermediate representation (IR) plus a single
  dispatch path, validated against TypeScript/TSX and Rust fixtures, kept within
  an 800-line review budget.
- artifacts: this file plus the parent-facing summary at `sdd-explore-code-intelligence.md`.
- skill_resolution: none (no project/user skill matched this exact parser
  architecture task from `.atl/skill-registry.md`).

## 2. Problem boundaries

In scope for this SDD change:

- The single language-neutral IR that all `LanguageParser` implementations feed.
- The `LanguageParser` trait contract extensions required to feed the IR.
- A single dispatch path used by `src-tauri/src/commands.rs` instead of the
  current dual path.
- TypeScript/TSX and Rust as the two validating languages for the IR.
- Fixtures and RED tests that lock the IR shape across both languages.
- The minimum parser changes needed to keep AI-context output stable
  (symbols, outline, imports, spans) while exposing hooks for relationships.

Out of scope (defer to later changes):

- Building the actual call/import-resolution graph and persistence layer.
- Implementing cross-file resolution, type inference, control-flow graphs, or
  any analysis that requires resolving IDs across `files`.
- Adding new language grammars beyond TypeScript/TSX and Rust. The framework
  should make them easy, but no Python/Go/Java/Python parsers are part of this
  change.
- Replacing the SQLite schema or migration machinery.
- Replacing the React/TS frontend or the Anthropic provider.
- NFR/performance work beyond keeping the registry path within the same order
  of magnitude as the legacy path.

## 3. AI-analysis needs (target shape of the IR)

The framework must hand an AI layer everything it needs to:

1. Identify the kinds of code constructs that exist in a file: classes,
   functions, methods, fields, properties, arrow functions, components, hooks,
   types/interfaces/enums, modules, impls, structs, constants, and so on.
2. Walk the file in a hierarchy: parent (class/impl/module) → children
   (methods/fields/variants).
3. Distinguish arrow/function-typed lexical declarations from plain constants,
   because React components, custom hooks, and `const sum = (a,b) => a+b` are
   not the same code intelligence as `const CONFIG = {...}`.
4. Carry accurate ranges (line/column start and end) for every symbol and
   outline item, since UI breadcrumb and explain-node flows rely on them.
5. Carry import/export edges with their module and specifier information.
6. Carry discriminated call/usage references where the parser can already
   see them, even if the resolver is out of scope for this change.
7. Carry a stable identifier (file id + kind + name + range) so re-scans do
   not break UI keys.
8. Carry optional language-specific metadata as an open extension point
   (TSX JSX, Rust attrs, decorators, generics), without forcing the IR to
   bloat for one language.
9. Be language-neutral enough that adding a third parser (e.g. Python) only
   requires a `LanguageParser` implementation, not IR changes.

## 4. Current code map (what exists today)

Files inspected:

- `engine/src/scanner/parser/traits.rs`: defines `LanguageParser` with
  `language_id`, `extensions`, `parse_all`, `supports`. Provides small helpers
  `ts_node_kind_to_outline_kind`, `rust_node_kind_to_outline_kind`,
  `make_outline_id`. The helpers are file-static functions, not on the trait,
  so each language is responsible for its own mapping.
- `engine/src/scanner/parser/registry.rs`: holds `Box<dyn LanguageParser>`,
  registers `TypeScriptParser` and `RustParser`, dispatches by extension via
  `parser_for_extension`. No fallback language.
- `engine/src/scanner/parser/typescript.rs`: produces `ParseResult` from a
  single tree walk. Recognises `class_declaration`, `function_declaration`,
  `method_definition`, `interface_declaration`, `type_alias_declaration`,
  `enum_declaration`, and any `export_statement` wrapping those. Treats every
  `lexical_declaration` as `Const`, which loses the
  `const Component = () => <div/>` and `const hook = () => {...}` cases. Reads
  `property_declaration` and `method_definition` from `class_body` for
  children. Detects imports via `import_statement`.
- `engine/src/scanner/parser/rust.rs`: similar shape, extracts
  `struct_item`, `impl_item`, `function_item`, `enum_item`,
  `type_alias_item`, plus methods inside `impl_item`. `use_declaration` is
  turned into an import with the leading path component as `target_module`.
- `engine/src/scanner/code_parser.rs`: legacy flat extractor. Owns its own
  `extract_ts_symbols` and `extract_rust_symbols`. Still exports
  `parse_file(path, content, extension)` and `parse_file_all(...)`. The two
  methods produce overlapping but not identical output.
- `engine/src/models/file.rs`: `SymbolKind` (Class, Function, ArrowFunction,
  Method, Interface, TypeAlias, Enum, Variable, Const, Struct, Impl, Unknown),
  `OutlineItemKind` (adds Field, Module), `OutlineItem` with stable id, and
  `ParseResult { symbols, imports, outline }`. There is no
  discriminated call/reference vector.
- `src-tauri/src/commands.rs`: `scan_project` calls
  `CodeParser::parse_file` twice (once for symbols, once for imports) and
  `CodeParser::parse_file_all` once for outline. `get_node_outline` calls
  `CodeParser::parse_file_all` directly. `get_graph` and `explain_node` rely
  on the persisted symbols/imports/outline. Two callers means the IR has to
  remain drop-in compatible with both flows during the migration, or the
  calls must move atomically to the registry.
- `openspec/config.yaml`: `strict_tdd: true`, `review_budget_changed_lines:
400` (overridden to 800 for this session by the SDD preflight choice).
- The `outline-parser-abstraction` change is already archived, so the trait
  and registry are not net-new architecture. This change is the next layer
  on top of them.

## 5. Assumptions

- The existing `LanguageParser` trait and `ParserRegistry` are good enough as
  the dispatch surface; we are extending them, not replacing them.
- `ParseResult`, `OutlineItem`, `SymbolInfo`, `ImportInfo` are the canonical
  types that frontend, Tauri commands, AI, and SQLite persistence all read.
  Adding fields is safe; renaming or removing fields is not.
- The SQLite schema does not need a migration in this change because all
  added IR fields are computed in memory and exposed only through new
  methods/commands.
- TypeScript/TSX and Rust fixtures are sufficient validation; we are not
  promising support for a third grammar.
- The AI provider in use is `Anthropic`; the IR must stay JSON-serializable
  so it can travel through `serde_json` and Tauri commands.
- `strict_tdd: true` is active; RED tests must be written first for the new
  IR fields and the dispatch consolidation.
- Review budget for this change is 800 changed lines, per the session
  preflight. The slice must be planned to fit.

## 6. Risks

- **Dual-path drift**: today the two parse paths can produce different
  results. Consolidating the dispatch without freezing the legacy path can
  cause silent regressions in the persisted DB. Mitigation: keep the legacy
  `CodeParser::parse_file` for one extra change as a thin shim that calls
  into the registry, marked deprecated, and remove only in a follow-up.
- **Behavioural change in symbol/outline counts**: any move from
  `SymbolKind::Const` to discriminated kinds can change persisted counts and
  the React UI. Mitigation: keep `SymbolKind::Const` for object/array/JSON
  initialisers, add `SymbolKind::ArrowFunction` only for function-typed
  values, document the difference in `explore.md` and in the next phase's
  proposal.
- **IR bloat**: trying to satisfy every AI-analysis need now risks turning
  `ParseResult` into a kitchen-sink type. Mitigation: add only
  discriminated lexical-declaration handling, ranges/spans (already
  partial), and an explicit `references` field stub. Defer call/type
  resolution to a follow-up change.
- **Parser performance**: walking the AST once per file is the current
  contract. Any new IR work must not require a second walk. Mitigation:
  collect everything in the same pass; no `find()` loops over the tree.
- **Tree-sitter version drift**: `tree_sitter_typescript::LANGUAGE_TYPESCRIPT`
  is used for `.ts`, `.tsx`, `.js`, `.jsx`. Lexical-declaration shape
  changes across tree-sitter versions could break tests. Mitigation: pin
  versions in `Cargo.toml` and add a regression fixture per grammar.
- **OpenSpec config drift**: `openspec/config.yaml` still says
  `review_budget_changed_lines: 400` while this session is 800. Mitigation:
  document the override in the proposal and ask the user before opening a
  PR; do not edit the config file from this change.
- **Workload on the user**: a full framework slice with IR, trait extension,
  and migration shim can approach 800 lines. Mitigation: split into two PRs
  if the apply-phase forecast exceeds the budget — IR + trait first,
  TypeScript+Rust lexer migration second.

## 7. Decision points (defer to proposal/spec/design)

- IR shape: extend `ParseResult` with a discriminated `lexical_kind` enum
  vs. introduce a separate `LexicalDecl` type. Recommendation: keep
  extending `ParseResult` and add `SymbolKind::ArrowFunction` rather than
  a parallel type, to keep the persistence layer untouched.
- Where to put the IR helpers: free functions in `traits.rs` vs.
  `impl LanguageParser`. Recommendation: extend the trait with default
  methods so language parsers do not duplicate.
- Whether to make the registry a singleton or an injectable dependency for
  the Tauri command. Recommendation: keep the registry as the default
  factory but accept an injected registry for tests; the command stays on
  the default.
- Whether to keep the legacy `CodeParser::parse_file` after the migration.
  Recommendation: shim it through the registry in this change; remove in
  the next change.
- Whether to add a `references: Vec<Reference>` field now or defer.
  Recommendation: add a typed stub that returns `[]` for every parser this
  change, so the AI layer can already wire to it without breaking.

## 8. Recommended first implementation slice

Smallest safe slice that supports all of the AI needs and fits the 800-line
budget:

1. Introduce `SymbolKind::ArrowFunction` and a small `lexical_value_kind`
   helper on `LanguageParser` that classifies
   `lexical_declaration`/`variable_declarator` into
   `Const`/`ArrowFunction`/`Function`. TypeScript and Rust both get the
   helper; TypeScript uses it, Rust returns `Const` for now.
2. Extend `LanguageParser` with two default methods:
   `parse_symbols(&ParseResult) -> &Vec<SymbolInfo>` (passthrough) and
   `lexical_value_kind(...)` returning a language-neutral
   `LexicalValueKind`. Concrete parsers override only the latter.
3. Add `ParseResult::references: Vec<Reference>` (empty by default) and a
   `Reference` struct with `file_id, kind (Import|Export|Call|TypeRef),
target_name, range`. TypeScript parser emits one `Reference` per
   `import_statement` (kind=Import) and per `export_statement` (kind=Export)
   so the AI layer can already see them.
4. Replace the dual parse path in `src-tauri/src/commands.rs` so
   `scan_project` and `get_node_outline` call the registry once and derive
   symbols, imports, outline, and references from a single `ParseResult`.
   `CodeParser::parse_file` becomes a thin shim that calls the registry.
5. Add RED tests in `engine/src/scanner/parser/typescript.rs`,
   `engine/src/scanner/parser/rust.rs`, and a new
   `engine/src/scanner/parser/ir_tests.rs` for the IR shape, the
   arrow-vs-const discrimination, and the `Reference` emission.
6. Add fixtures under `fixtures/` for both languages: minimal class with
   arrow-field method, object literal with methods, React component via
   const arrow, Rust struct + impl + trait method.

If this slice exceeds 800 lines during the apply phase, split into:

- PR-A: IR + trait + reference stub (no parser changes).
- PR-B: TypeScript arrow detection + Rust stub.
- PR-C: dispatch consolidation + `CodeParser::parse_file` shim.

## 9. Validation approach

- `cargo test` must pass for `engine` and `src-tauri`.
- New RED tests must compile-fail before the slice, then pass.
- Fixture parsing must produce a stable `ParseResult` shape across rescans
  (same `id`s for the same ranges).
- A representative `cargo test parser::ir_tests` run must show the IR
  contract works for both languages with the same shape.
- A manual scan of the current project must still produce non-empty
  `outline`, `imports`, and `symbols` from the registry path.
- No new warnings from `cargo clippy -- -D warnings` or `npm run lint`.

## 10. Review workload guidance (800 lines)

- The IR additions should be ≤ 150 lines including new types and helpers.
- The trait extension should be ≤ 50 lines.
- Each language's RED tests should be ≤ 120 lines; GREEN changes ≤ 200.
- The dispatch consolidation in `commands.rs` should be ≤ 50 lines of
  net diff.
- The `CodeParser::parse_file` shim should be ≤ 30 lines.
- If any of these grow beyond the budget, fall back to the chained-PR
  plan in section 8.

## 11. Out of scope (explicit non-goals)

- New language grammars.
- Cross-file call/import resolution.
- Type inference, generics, trait/impl resolution.
- Persistence schema changes.
- Frontend changes to use the new IR fields.
- AI prompt changes to use the new IR fields.

## 12. Next recommended phase

- Move to **proposal** for `multi-language-code-intelligence-framework`.
- The proposal should lock: IR shape, `LanguageParser` additions, reference
  stub, dual-path consolidation strategy, and the chained-PR fallback.
- After proposal: spec → design → tasks → apply (with strict TDD) → verify
  → archive.
