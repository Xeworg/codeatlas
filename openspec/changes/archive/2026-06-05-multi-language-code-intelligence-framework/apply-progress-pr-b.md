# PR-B Apply Progress

> **Branch**: `feat/parser-ir-pr-b`
> **Strict TDD**: RED → GREEN → TRIANGULATE → REFACTOR for every task.

## TDD Cycle Evidence

| Task                  | RED Evidence                                                                                                                                     | GREEN Evidence                                                                                                                                               | Notes                                                                                                                            |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| B.1 (TS lexical kind) | `cargo test --lib typescript::tests::lexical_kind` failed with 4/5 tests (only `object_literal_is_const` passed because `Const` is the default). | After B.2 implementation, all 5 lexical_kind tests pass.                                                                                                     | Required bug fix: parser was using `LANGUAGE_TYPESCRIPT` for `.tsx` files; switched to `LANGUAGE_TSX` for `.tsx`/`.jsx`.         |
| B.3 (TS references)   | `cargo test --lib typescript::tests::reference` failed 2/5 — `import { foo }` and `import React from` produced empty `references`.               | After `extract_references` override + wiring into `parse_all` (and routing through the `continue;` branch for import_statement), all 5 reference tests pass. | `parse_all` had an early `continue;` for `import_statement`; reference emission had to be inlined before it.                     |
| B.4 (single-pass)     | n/a — test added together with B.3 implementation since the contract is "all categories populated from one `parse_all` call".                    | `single_pass_populates_all_ir_categories` passes: a single TS source populates imports, symbols, outline, references, and lexical_kind.                      | Implementation iterates the root children once; both `lexical_kind_for` and `extract_references` are called inline at each step. |
| B.5 (Rust references) | `cargo test --lib rust::tests::reference` failed 1/4 — `use self::foo;` produced `target_name="foo"` instead of `""`.                            | After detecting first-segment special keywords (`self`/`super`/`crate`) and emitting empty `target_name`, all 4 reference tests pass.                        | All 2 lexical_kind tests for Rust also pass: function_item → Function, struct_item → Const.                                      |

## Completed Tasks

| Task | Status  | Files Touched                                                                                                                                                                                |
| ---- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B.1  | ✅ DONE | `engine/tests/fixtures/typescript/{arrow_field.ts, object_literal.ts, react_const_arrow.tsx}` (new); 5 RED tests in `engine/src/scanner/parser/typescript.rs`                                |
| B.2  | ✅ DONE | `engine/src/scanner/parser/typescript.rs` — `lexical_kind_for` override; arrow field symbol collection; TSX grammar selection.                                                               |
| B.3  | ✅ DONE | `engine/src/scanner/parser/typescript.rs` — `extract_references` override; import/export specifier walking; `push_spec_reference` helper; wiring into `parse_all`.                           |
| B.4  | ✅ DONE | `engine/src/scanner/parser/typescript.rs` — `single_pass_populates_all_ir_categories` test.                                                                                                  |
| B.5  | ✅ DONE | `engine/tests/fixtures/rust/struct_impl_trait.rs` (new); `engine/src/scanner/parser/rust.rs` — `lexical_kind_for` + `extract_references` overrides; wiring into `parse_all`; 7 inline tests. |

## Files Changed

| File                                                     | Action   | Lines                                                        |
| -------------------------------------------------------- | -------- | ------------------------------------------------------------ |
| `engine/src/scanner/parser/typescript.rs`                | Modified | +~250 (lexical_kind_for, extract_references, helpers, tests) |
| `engine/src/scanner/parser/rust.rs`                      | Modified | +~120 (lexical_kind_for, extract_references, tests)          |
| `engine/tests/fixtures/typescript/arrow_field.ts`        | New      | 1                                                            |
| `engine/tests/fixtures/typescript/object_literal.ts`     | New      | 1                                                            |
| `engine/tests/fixtures/typescript/react_const_arrow.tsx` | New      | 1                                                            |
| `engine/tests/fixtures/rust/struct_impl_trait.rs`        | New      | 3                                                            |

## Commands Run

| Command                                                        | Result                                                                    |
| -------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `cargo test --lib typescript::tests::lexical_kind` (B.1 RED)   | 1/5 passed (object_literal_is_const was a no-change baseline); 4/5 failed |
| `cargo test --lib typescript::tests::lexical_kind` (B.2 GREEN) | 5/5 passed                                                                |
| `cargo test --lib typescript::tests::reference` (B.3 RED)      | 3/5 passed; 2/5 failed (imports empty)                                    |
| `cargo test --lib typescript::tests::reference` (B.3 GREEN)    | 5/5 passed                                                                |
| `cargo test --lib typescript` (B.4 GREEN)                      | 29/29 passed                                                              |
| `cargo test --lib rust::tests::reference` (B.5 RED)            | 3/4 passed; 1/4 failed (`use self::foo` produced `"foo"`)                 |
| `cargo test --lib rust::tests::reference` (B.5 GREEN)          | 4/4 passed                                                                |
| `cargo test --lib rust` (full Rust)                            | 12/12 passed                                                              |
| `cargo test --lib` (full engine)                               | 153/153 passed                                                            |
| `cargo test` (all targets)                                     | 163/163 passed (153 lib + 5 add_a_language + 3 bench + 2 wal)             |
| `cargo clippy -- -D warnings`                                  | Clean (lib only)                                                          |
| `cargo fmt --check` on typescript.rs + rust.rs                 | Clean                                                                     |
| `cargo test --test add_a_language`                             | 5/5 passed (Python stub integration intact)                               |

## Deviations from Design

1. **TSX grammar selection**: `TypeScriptParser::new()` previously hard-coded `LANGUAGE_TYPESCRIPT` for all extensions, which caused `.tsx` files to fail tree-sitter parsing (the JSX `<div/>` was tokenized as `type_arguments` and `<` regex). Fixed by storing both grammars and selecting based on the file path (`.tsx`/`.jsx` → `LANGUAGE_TSX`, else `LANGUAGE_TYPESCRIPT`). This was a pre-existing bug required for the `react_const_arrow.tsx` fixture to parse.

2. **Class field arrow symbol collection**: The design called for `SymbolKind::ArrowFunction` differentiation for the class-field arrow case (`handler = (req) => req.body`). The current parser did not extract class fields as `SymbolInfo`, so we added `collect_arrow_field_symbols` (a new helper) that walks the class body and pushes a `SymbolInfo { kind: ArrowFunction, ... }` per arrow field. This is consistent with the design's "SymbolKind differentiation" requirement.

3. **`export_statement` lexical_kind unwrap**: `lexical_kind_for` now recurses through `export_statement` to classify the inner `lexical_declaration` (necessary because `export const Card = () => ...` is an `export_statement` wrapping a `lexical_declaration`, not a bare `lexical_declaration`).

4. **Reference emission for import_statement**: The existing `parse_all` had an early `continue;` for `import_statement` to keep the legacy import format. Reference emission had to be inlined before the `continue;` to maintain the single-pass invariant.

5. **Rust `use` first-segment detection**: Initial implementation took the last `::` segment, which produced `target_name="foo"` for `use self::foo;`. Fixed by detecting the first `::` segment — if it is `self`/`super`/`crate`/`*`/empty, emit empty `target_name` (per the conservative emission spec).

## Remaining Tasks (per PR-B)

None — all 5 tasks (B.1..B.5) complete.

## Workload / PR Boundary

PR-B is well within the 800-line budget:

- 5 fixtures (~7 lines)
- 2 production files (~370 lines)
- 7 inline test functions

No chained PRs needed.
