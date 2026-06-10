# Explore summary — `multi-language-code-intelligence-framework`

Parent-facing copy of `openspec/changes/multi-language-code-intelligence-framework/explore.md`.

## Phase envelope

- status: completed
- executive_summary: CodeAtlas already has a per-language `LanguageParser` trait
  and a working TypeScript/Rust registry, but extraction is fragmented across a
  legacy flat path (`CodeParser::parse_file`) and a registry path
  (`CodeParser::parse_file_all`). The framework extension needs a
  language-neutral IR that an AI code-analysis layer can rely on: discriminated
  lexical declarations, ranges/spans, a typed `Reference` vector, and a single
  dispatch path. First implementation slice: extend `ParseResult` and the
  `LanguageParser` trait, keep TypeScript/TSX and Rust as the validating
  languages, ship inside an 800-line review budget, and split into chained PRs
  if the apply phase outgrows it.
- artifacts:
  - `openspec/changes/multi-language-code-intelligence-framework/explore.md`
  - this file
- skill_resolution: none (no project/user skill matched this exact parser
  architecture task from `.atl/skill-registry.md`).

## Key decisions / discoveries

- `LanguageParser` + `ParserRegistry` are already in place; the change is the
  next layer, not net-new architecture.
- `SymbolKind` and `OutlineItemKind` already include `ArrowFunction`, `Field`,
  and `Module`; the gap is the parser does not emit them. The IR shape is
  ready, the wiring is not.
- `CodeParser::parse_file` is still the symbols/imports path used by
  `src-tauri/src/commands.rs::scan_project`, while outline goes through
  `CodeParser::parse_file_all`. This is the main drift risk.
- The OpenSpec `review_budget_changed_lines` is 400, but the session preflight
  set 800 for this SDD. Treat the session value as authoritative for planning
  and ask the user before opening a PR.
- `openspec/config.yaml` has `strict_tdd: true`; RED tests must precede
  parser changes.

## AI-analysis needs (the IR must support)

1. Discriminated lexical-declaration kinds (arrow vs const vs function).
2. Hierarchical outline: parent (class/impl/module) → children
   (methods/fields/variants).
3. Stable identifiers and accurate line/column ranges for every symbol and
   outline item.
4. Imports with module, specifiers, default flag, and type flag.
5. A typed `Reference` vector so the AI layer can already see import/export
   edges even before cross-file resolution is built.
6. Language-specific metadata as an open extension point (decorators,
   generics, attributes).
7. JSON-serializable shape so it can travel through Tauri commands and
   `serde_json`.

## Current code map (where the work will land)

- `engine/src/scanner/parser/traits.rs` — extend with default IR helpers.
- `engine/src/scanner/parser/registry.rs` — keep as the dispatch surface.
- `engine/src/scanner/parser/typescript.rs` — emit `ArrowFunction`,
  `Reference`s, and discriminated `LexicalValueKind`.
- `engine/src/scanner/parser/rust.rs` — same IR shape, conservative content.
- `engine/src/scanner/code_parser.rs` — turn `parse_file` into a shim that
  calls the registry.
- `engine/src/models/file.rs` — add `Reference` and `LexicalValueKind` types.
- `src-tauri/src/commands.rs` — single dispatch in `scan_project` and
  `get_node_outline`.

## Recommended first slice (≤ 800 lines)

1. Add `SymbolKind::ArrowFunction` (already present) wired up in TypeScript.
2. Add `LexicalValueKind` and `Reference` to `ParseResult`.
3. Extend `LanguageParser` with default helpers and one override hook.
4. TypeScript parser emits `Reference` for imports and exports; Rust emits
   the same shape with empty import names where appropriate.
5. `CodeParser::parse_file` becomes a shim around the registry.
6. `scan_project` and `get_node_outline` use the registry once.
7. RED tests in `typescript.rs`, `rust.rs`, and a new `ir_tests.rs` for the
   shared contract.
8. Fixtures under `fixtures/` for both languages.

If the slice exceeds 800 lines during apply, fall back to chained PRs:

- PR-A: IR + trait + reference stub.
- PR-B: TypeScript arrow detection + Rust stub.
- PR-C: dispatch consolidation + `CodeParser::parse_file` shim.

## Risks

- Behavioural change in symbol/outline counts after discriminated
  lexical-declaration handling. Mitigation: document the difference and
  gate by tests.
- Dual-path drift if the shim is removed too early. Mitigation: keep the
  legacy shim through this change, remove in a follow-up.
- IR bloat from trying to satisfy every AI need now. Mitigation: ship
  `Reference` as a typed stub with empty default, defer call/type
  resolution.
- Tree-sitter grammar drift breaking fixtures. Mitigation: pin versions and
  keep one regression fixture per grammar.
- `review_budget_changed_lines` drift between OpenSpec config and session.
  Mitigation: keep the session value for planning; ask the user before
  opening a PR.

## Validation

- `cargo test` for `engine` and `src-tauri` must pass.
- RED tests in this change must compile-fail before the slice, then pass.
- `cargo test parser::ir_tests` must show the IR contract working for both
  languages.
- A scan of the current project must still produce non-empty `outline`,
  `imports`, and `symbols` from the registry path.
- `cargo clippy -- -D warnings` and `npm run lint` must stay green.

## Out of scope (non-goals)

- New language grammars (Python, Go, Java, etc.).
- Cross-file call/import resolution.
- Type inference and trait/impl resolution.
- Persistence schema changes.
- Frontend or AI-prompt changes to consume the new IR fields.

## Next recommended phase

- Move to **proposal** for `multi-language-code-intelligence-framework`.
- The proposal must lock: IR shape, `LanguageParser` additions, `Reference`
  stub, dispatch consolidation strategy, and the chained-PR fallback.
- After proposal: spec → design → tasks → apply (strict TDD) → verify →
  archive.
