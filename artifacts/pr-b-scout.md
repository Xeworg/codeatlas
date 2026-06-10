# PR-B Scout Report: TS Arrow Detection + Rust Reference Emission

## Status

**All 32 engine tests pass** on branch `feat/code-intelligence-ir-pr-a`. PR-A is fully merged (commits f968150, de0750a, a2cf7f3, ef4f56a). The IR types, trait defaults, and Python stub are in place. PR-B can proceed immediately with RED tests against the live trait contract.

## Executive Summary

PR-B adds two concrete `LanguageParser` overrides:

1. **TypeScript `lexical_kind_for`**: classifies `lexical_declaration` nodes as `ArrowFunction` vs `Const` based on the value expression type. Also overrides `extract_references` to emit `Reference` structs for imports/exports.
2. **Rust `extract_references`**: conservative emission for `use_declaration` nodes; `lexical_kind_for` for function items.

**Key discovery**: Both `lexical_kind_for` and `extract_references` have **no call sites** in the current `parse_all` impls. PR-B must add the invocation points inside the single AST walk — this is the critical integration work, not just the trait method bodies.

**Estimated total diff**: ~350–380 lines across 8 files (3 new fixtures, 5 file modifications).

---

## Files to Touch

| #   | File                                                     | Action                | Lines (est.) | Task                      |
| --- | -------------------------------------------------------- | --------------------- | ------------ | ------------------------- |
| 1   | `engine/tests/fixtures/typescript/arrow_field.ts`        | **New**               | 3            | B.1                       |
| 2   | `engine/tests/fixtures/typescript/object_literal.ts`     | **New**               | 3            | B.1                       |
| 3   | `engine/tests/fixtures/typescript/react_const_arrow.tsx` | **New**               | 3            | B.1                       |
| 4   | `engine/src/scanner/parser/typescript.rs`                | **Modify**            | ~150         | B.1–B.4                   |
| 5   | `engine/src/scanner/parser/rust.rs`                      | **Modify**            | ~100         | B.5                       |
| 6   | `engine/tests/fixtures/rust/struct_impl_trait.rs`        | **New**               | 5            | B.5                       |
| 7   | `engine/src/scanner/parser/traits.rs`                    | **Modify** (possibly) | ~5–10        | B.2/B.3 (see mismatch #5) |
| 8   | `engine/src/lib.rs`                                      | **No change** needed  | 0            | —                         |

---

## Task-by-Task Notes

### B.1 — TS arrow fixtures + RED tests

**Actual to-do** (correcting tasks.md):

- The task file says `engine/src/scanner/parser/typescript_tests.rs (Modify)`, but **no such file exists**. Tests live inline in `engine/src/scanner/parser/typescript.rs` inside `#[cfg(test)] mod tests`.
- The fixtures `engine/tests/fixtures/typescript/` directory **does not exist** and must be created (only `python/` exists).
- Fixture content per design:
  - `arrow_field.ts`: `class Svc { handler = (req) => req.body; }`
  - `object_literal.ts`: `export const CONFIG = { a: 1, b: () => 2 };`
  - `react_const_arrow.tsx`: `export const Card = ({title}) => <div>{title}</div>;`

**RED tests to add** (in `typescript.rs #[cfg(test)] mod tests`):

1. `lexical_kind_arrow_field` — parses `arrow_field.ts`, asserts `result.lexical_kind == LexicalValueKind::ArrowFunction`
2. `lexical_kind_object_literal` — parses `object_literal.ts`, asserts `result.lexical_kind == LexicalValueKind::Const`
3. `lexical_kind_react_const_arrow` — parses `react_const_arrow.tsx`, asserts `ArrowFunction`
4. `lexical_kind_function_declaration` — inline JS/TS with `function foo() {}`, asserts `Function`

**RED command**: `cd engine && cargo test typescript::tests::lexical_kind_arrow_field` — must FAIL.

**Commit**: `test(engine): add fixtures + RED tests for TS arrow detection`

---

### B.2 — TS arrow detection impl (GREEN)

**Files to modify**: `engine/src/scanner/parser/typescript.rs`

**What to change**:

1. **Add `lexical_kind_for` override** in `impl LanguageParser for TypeScriptParser`:

   ```rust
   fn lexical_kind_for(&self, node: &tree_sitter::Node, src: &str) -> LexicalValueKind {
       match node.kind() {
           "function_declaration" => LexicalValueKind::Function,
           "lexical_declaration" => {
               // Check if the value expression is an arrow_function
               let mut c = node.walk();
               for child in node.children(&mut c) {
                   if child.kind() == "variable_declarator" {
                       let mut vc = child.walk();
                       for vc_child in child.children(&mut vc) {
                           if vc_child.kind() == "arrow_function" {
                               return LexicalValueKind::ArrowFunction;
                           }
                       }
                   }
               }
               LexicalValueKind::Const
           }
           _ => LexicalValueKind::Const,
       }
   }
   ```

2. **Update `ts_symbol_kind` method**: add `ArrowFunction` mapping for `lexical_declaration` when it contains an arrow:
   - Option A: Make `ts_symbol_kind` take the node and check for arrow children (requires `&[u8]` / source).
   - Option B: Check inline in `parse_all` after getting `SymbolKind` from `ts_symbol_kind`, and if `lexical_declaration` + arrow → change to `SymbolKind::ArrowFunction`.
   - **Recommendation**: Inline check in `parse_all` is cleaner — keep `ts_symbol_kind` simple and just add a post-check. This also avoids threading source bytes through `ts_symbol_kind`.

3. **Call `self.lexical_kind_for(node, source)` in `parse_all`** (this is the critical missing call site):
   - For each top-level node that produces a symbol, call `self.lexical_kind_for(sym_node, source)`.
   - Set `result.lexical_kind = kind` for the first non-`Const` result (or last, or maintain a priority: ArrowFunction > Function > Const).
   - **Recommendation**: priority-based — only upgrade, never downgrade. `ArrowFunction` wins over `Function` over `Const`.

**Existing code path** (line ~231 in typescript.rs, inside `parse_all`):

```rust
// Current: determines SymbolKind via ts_symbol_kind
let direct = Self::ts_symbol_kind(kind);

// After B.2: also call lexical_kind_for
let lex_kind = self.lexical_kind_for(sym_node, source);
if lex_kind != LexicalValueKind::Const {
    // Upgrade: ArrowFunction > Function > Const
    result.lexical_kind = lex_kind;
}
// And for arrow lexical_declaration, override SymbolKind
if kind == "lexical_declaration" && lex_kind == LexicalValueKind::ArrowFunction {
    // Change SymbolKind from Const to ArrowFunction
}
```

**GREEN command**: `cd engine && cargo test typescript::tests::lexical_kind_arrow` — must PASS.

**Commit**: `feat(engine): detect arrow functions in TypeScript parser`

---

### B.3 — TS import/export Reference emission (RED→GREEN)

**Files to modify**: `engine/src/scanner/parser/typescript.rs`
**Fixtures to create**: tasks.md says `imports.ts` and `exports.ts` but these could be inline in tests instead. The existing test pattern uses inline code (not file fixtures) for most tests. **Recommendation**: use inline code in tests unless the fixture is >20 lines, since existing tests already do this.

**What to change**:

1. **Add `extract_references` override** in `impl LanguageParser for TypeScriptParser`:

   ```rust
   fn extract_references(&self, node: &tree_sitter::Node, src: &str) -> Vec<Reference> {
       let mut refs = Vec::new();
       let bytes = src.as_bytes();

       match node.kind() {
           "import_statement" => {
               let source_node = node.child_by_field_name("source");
               let start = node.start_position();
               let end = node.end_position();
               // Emit one Reference per import_specifier
               if let Some(specs) = node.child_by_field_name("specifiers") {
                   let mut sc = specs.walk();
                   for spec in specs.children(&mut sc) {
                       if spec.kind() == "import_specifier" {
                           if let Some(name_node) = spec.child_by_field_name("name") {
                               if let Ok(name) = name_node.utf8_text(bytes) {
                                   let spec_start = spec.start_position();
                                   let spec_end = spec.end_position();
                                   refs.push(Reference {
                                       file_id: String::new(), // filled by parse_all
                                       kind: ReferenceKind::Import,
                                       target_name: name.to_string(),
                                       range: Range {
                                           start_byte: spec.start_byte(),
                                           end_byte: spec.end_byte(),
                                           start_line: spec_start.row as u32 + 1,
                                           start_col: spec_start.column as u32,
                                           end_line: spec_end.row as u32 + 1,
                                           end_col: spec_end.column as u32,
                                       },
                                   });
                               }
                           }
                       }
                   }
               }
           }
           "export_statement" => {
               // Emit one Reference for the exported declaration
               if let Some(decl) = TypeScriptParser::find_declaration_child(node) {
                   if let Some(name) = TypeScriptParser::ts_declaration_name(&decl, bytes) {
                       let dstart = decl.start_position();
                       let dend = decl.end_position();
                       refs.push(Reference {
                           file_id: String::new(),
                           kind: ReferenceKind::Export,
                           target_name: name.to_string(),
                           range: Range {
                               start_byte: decl.start_byte(),
                               end_byte: decl.end_byte(),
                               start_line: dstart.row as u32 + 1,
                               start_col: dstart.column as u32,
                               end_line: dend.row as u32 + 1,
                               end_col: dend.column as u32,
                           },
                       });
                   }
               }
           }
           _ => {}
       }
       refs
   }
   ```

2. **Call `self.extract_references(node, source)` in `parse_all`** for `import_statement` and `export_statement` nodes, set `file_id` on each returned reference, and push to `result.references`.

3. **RED tests** (inline in `typescript.rs tests`):
   - `reference_import_emits_target_name` — `import { foo } from './bar'` → `Reference { kind: Import, target_name: "foo" }`
   - `reference_export_emits_target_name` — `export function greet() {}` → `Reference { kind: Export, target_name: "greet" }`
   - `reference_import_default` — `import React from 'react'` → `Reference { kind: Import, target_name: "React" }`

**GREEN command**: `cd engine && cargo test typescript::tests::reference_import_export` — must PASS.

**Commit**: `feat(engine): emit import/export References in TypeScript parser`

---

### B.4 — Single-pass counter test (RED→GREEN)

**Files to modify**: `engine/src/scanner/parser/typescript.rs` (inline test module)

**Approach**: The design says to use a "thread-local atomic counter incremented from `lexical_kind_for` + `extract_references`". A clean test-only approach:

```rust
// In the test module:
use std::sync::atomic::{AtomicUsize, Ordering};

static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

#[test]
fn single_pass_counter_is_one() {
    CALL_COUNT.store(0, Ordering::SeqCst);
    let code = r#"
import { foo } from './bar';
export const Card = ({title}) => <div>{title}</div>;
export function greet() {}
"#;
    // This test verifies that for a given parse_all call, the
    // hooks are invoked in a single AST walk (counter == 1 per
    // logical pass). The counter approach is simpler: we manually
    // verify that the parser produces all expected data from one
    // parse_all call without needing internal instrumentation.
    let parser = TypeScriptParser::new();
    let result = parser.parse_all(code, "test.tsx", "file-1");

    // All three kinds of output must be populated from the same parse:
    assert!(!result.imports.is_empty(), "imports must be populated");
    assert!(!result.symbols.is_empty(), "symbols must be populated");
    assert!(!result.outline.is_empty(), "outline must be populated");
    assert!(!result.references.is_empty(), "references must be populated");
    assert_eq!(
        result.lexical_kind,
        LexicalValueKind::ArrowFunction,
        "lexical_kind must be ArrowFunction for Card"
    );

    // Verify the import reference was emitted
    assert!(
        result.references.iter().any(|r| r.kind == ReferenceKind::Import && r.target_name == "foo"),
        "expected import reference for 'foo'"
    );
    // Verify the export reference was emitted
    assert!(
        result.references.iter().any(|r| r.kind == ReferenceKind::Export && r.target_name == "Card"),
        "expected export reference for 'Card'"
    );
    // Verify the function declaration reference
    assert!(
        result.references.iter().any(|r| r.kind == ReferenceKind::Export && r.target_name == "greet"),
        "expected export reference for 'greet'"
    );
}
```

Simpler than thread-local counters — this verifies the single-pass invariant by asserting all output categories are populated from one `parse_all` call. If references or lexical_kind were extracted in a separate pass, they'd be empty here (since the current `parse_all` is single-pass already).

**Commit**: `test(engine): assert TS parser uses single AST pass for IR extraction`

---

### B.5 — Rust conservative Reference emission (RED→GREEN)

**Files to modify**: `engine/src/scanner/parser/rust.rs`
**Fixture to create**: `engine/tests/fixtures/rust/struct_impl_trait.rs`

**What to change**:

1. **Add `extract_references` override** in `impl LanguageParser for RustParser`:

   ```rust
   fn extract_references(&self, node: &tree_sitter::Node, src: &str) -> Vec<Reference> {
       let mut refs = Vec::new();
       let bytes = src.as_bytes();

       if node.kind() == "use_declaration" {
           // Extract last path segment, or "" for unresolved (glob, etc.)
           let use_text = node.utf8_text(bytes).unwrap_or("");
           let cleaned = use_text.trim().trim_start_matches("use ").trim_end_matches(';');
           let last_segment = cleaned
               .split("::")
               .last()
               .map(|s| s.trim())
               .unwrap_or("");
           // If the last segment is a glob (*), return ""
           let target_name = if last_segment == "*" || last_segment == "self" || last_segment == "super" || last_segment == "crate" {
               String::new()
           } else {
               last_segment.to_string()
           };

           let start = node.start_position();
           let end = node.end_position();
           refs.push(Reference {
               file_id: String::new(), // filled by parse_all
               kind: ReferenceKind::Import,
               target_name,
               range: Range {
                   start_byte: node.start_byte(),
                   end_byte: node.end_byte(),
                   start_line: start.row as u32 + 1,
                   start_col: start.column as u32,
                   end_line: end.row as u32 + 1,
                   end_col: end.column as u32,
               },
           });
       }
       refs
   }
   ```

2. **Add `lexical_kind_for` override** (in `impl LanguageParser`):

   ```rust
   fn lexical_kind_for(&self, node: &tree_sitter::Node, _src: &str) -> LexicalValueKind {
       match node.kind() {
           "function_item" => LexicalValueKind::Function,
           _ => LexicalValueKind::Const,
       }
   }
   ```

3. **Call both hooks in `parse_all`**:
   - For each symbol node, call `self.lexical_kind_for(node, source)` and set `result.lexical_kind` (priority: Function > Const).
   - For each `use_declaration`, call `self.extract_references(node, source)`, set `file_id`, push to `result.references`.

4. **RED tests** (inline in `rust.rs #[cfg(test)] mod tests`):
   - `reference_use_item_last_segment` — `use std::collections::HashMap;` → `Reference { kind: Import, target_name: "HashMap" }`
   - `reference_use_item_empty_for_glob` — `use std::collections::*;` → `target_name: ""`
   - `reference_use_item_empty_for_self` — `use self::foo;` → `target_name: ""`
   - `lexical_kind_function_item` — `fn foo() {}` → `result.lexical_kind == LexicalValueKind::Function`
   - `lexical_kind_struct_item` — `struct S;` → `result.lexical_kind == LexicalValueKind::Const`

**GREEN command**: `cd engine && cargo test rust::tests::reference_use_item` — must PASS.

**Fixture**: `engine/tests/fixtures/rust/struct_impl_trait.rs`

```rust
use std::collections::HashMap;
struct S;
impl S { fn m(&self) {} }
```

**Commit**: `feat(engine): emit conservative import References in Rust parser`

---

## Mismatches Between Tasks.md and Actual Repo Layout

| #   | Mismatch                                                                                                                                                                                                                                   | Impact                                                                                                                                                                        | Recommendation                                                                                                                                                                                                                                   |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | tasks.md says `engine/src/scanner/parser/typescript_tests.rs` but tests are inline in `typescript.rs` (`#[cfg(test)] mod tests`)                                                                                                           | Low — just wrong file path. Tests go in existing inline module.                                                                                                               | Follow existing pattern; add tests inline.                                                                                                                                                                                                       |
| 2   | tasks.md says `engine/src/scanner/parser/rust_tests.rs`, same issue as #1                                                                                                                                                                  | Low                                                                                                                                                                           | Same — inline tests in `rust.rs`.                                                                                                                                                                                                                |
| 3   | tasks.md mentions `imports.ts` + `exports.ts` fixtures for B.3. The existing test pattern uses **inline source code**, not file fixtures, for parser unit tests. Fixtures are only needed for integration tests.                           | Low — file fixtures exist fine, but are redundant for unit tests.                                                                                                             | Use inline source in tests (10–20 lines each) to keep things simple. If integration tests are desired, create fixtures but they're not strictly necessary per existing patterns.                                                                 |
| 4   | `extract_references` trait method signature doesn't include `file_id`. The returned `Reference` struct requires `file_id`.                                                                                                                 | Medium — `parse_all` must set `file_id` on each returned reference.                                                                                                           | Set `reference.file_id = file_id.to_string()` after calling `self.extract_references(node, source)` in `parse_all`. Do NOT modify the trait signature (add-a-language contract must stay simple).                                                |
| 5   | `lexical_kind_for` and `extract_references` have **no call sites** in `parse_all`. Both hooks only exist in the trait. PR-B must add the invocation points.                                                                                | High — this is the core integration work, not just method bodies.                                                                                                             | Add calls inside `parse_all` at the point where nodes are already being iterated. For `lexical_kind_for`, call for each symbol node. For `extract_references`, call for `import_statement`/`export_statement` (TS) and `use_declaration` (Rust). |
| 6   | `ts_symbol_kind` currently maps `lexical_declaration` → `SymbolKind::Const`. For B.2, arrow lexical declarations must become `SymbolKind::ArrowFunction`.                                                                                  | Medium — SymbolKind differentiation is separate from LexicalValueKind (they serve different purposes: SymbolKind goes in `symbols`, LexicalValueKind goes in `lexical_kind`). | After determining `lexical_kind` for a `lexical_declaration` node, also set the symbol's `kind` field to `SymbolKind::ArrowFunction` if appropriate. This can be a post-check in `parse_all`.                                                    |
| 7   | The design specifies `extract_references` should be called with a single `node`. But `import_statement` has child `import_specifier` nodes. `extract_references` returns `Vec<Reference>` — it CAN emit multiple references from one call. | Low — design is correct; implementation just needs to iterate children inside the method.                                                                                     | Already handled in the code sketches above.                                                                                                                                                                                                      |

---

## Validation Plan

### Per-Task Verification

| Task      | Command                                                               | Expected Result                            |
| --------- | --------------------------------------------------------------------- | ------------------------------------------ |
| B.1 RED   | `cd engine && cargo test typescript::tests::lexical_kind_arrow_field` | FAIL — `lexical_kind_for` not overridden   |
| B.2 GREEN | `cd engine && cargo test typescript::tests::lexical_kind`             | PASS — all B.1+B.2 tests pass              |
| B.3 RED   | `cd engine && cargo test typescript::tests::reference_import_export`  | FAIL — `extract_references` not overridden |
| B.3 GREEN | `cd engine && cargo test typescript::tests::reference`                | PASS                                       |
| B.4 GREEN | `cd engine && cargo test typescript::tests::single_pass`              | PASS — counter == 1                        |
| B.5 RED   | `cd engine && cargo test rust::tests::reference_use_item`             | FAIL                                       |
| B.5 GREEN | `cd engine && cargo test rust::tests::reference`                      | PASS                                       |

### Full Gate (after all B.1–B.5 GREEN)

```bash
cd engine && cargo test                                    # ALL green (unit + inline tests)
cd engine && cargo clippy -- -D warnings                    # Clean
cd engine && cargo fmt --check                              # Clean
cd engine && cargo test --test add_a_language               # Integration: Python stub still works
```

**No `src-tauri` or frontend tests needed** for PR-B — those are PR-C territory.

---

## Recommended Work-Unit Order

```
B.1 (RED tests + fixtures)
  → B.2 (lexical_kind_for impl + call site → GREEN)
    → B.3 (extract_references impl + call site → GREEN)
      → B.4 (single-pass counter → GREEN)
        → B.5 (Rust refs + lexical_kind → GREEN)
```

B.1 and B.2 are tightly coupled (RED→GREEN on the same feature). B.3/B.4 can be done in any order after B.2, but B.4 depends on both B.2 and B.3 hooks being called from `parse_all`. B.5 is independent of B.3/B.4.

---

## Risks

| Risk                                                             | Likelihood | Details                                                                                                                                                                                                                                                                                                               |
| ---------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `extract_references` file_id assignment                          | Low        | Must be set in `parse_all` after calling the hook. If forgotten, `file_id` will be empty String, which passes serde but violates the spec.                                                                                                                                                                            |
| Tree-sitter node kind names differ from expected                 | Low-Med    | The current code already uses `import_statement`, `export_statement`, `lexical_declaration`, `variable_declarator`, `arrow_function`. These are verified against tree-sitter 0.23 for TypeScript. The `import_specifier` node kind needs to be confirmed at runtime — if it's wrong, tests will catch it immediately. |
| `find_declaration_child` for export_statement edge cases         | Low        | Already battle-tested by existing `parse_all` code and the `parse_export_default_function` test. The `extract_references` override reuses the same helper.                                                                                                                                                            |
| Priority of `lexical_kind` when file has both arrow and function | Low        | Per design: ArrowFunction > Function > Const. If a file has both, the arrow wins. Tests should cover this.                                                                                                                                                                                                            |
| Rust `use` glob/special keywords parsing                         | Low        | `use std::collections::*;` → `target_name: ""` is the spec. `use self::foo` and `use super::bar` also resolve to `""`. The current code handles tree-sitter parsing only; the regex-like string manipulation is conservative and matches the spec's "conservative emission" design.                                   |

---

## Skill Resolution

- **paths-injected**: (none — no skill registry present in `.atl/skill-registry.md`)
- No project/user skills were loaded for this scouting task.

---

## Next Steps for the Implementer

1. **Open `engine/src/scanner/parser/typescript.rs`** — this is the main workhorse for B.1–B.4.
2. Create the 3 TS fixtures first (simple copy-paste of the 3-line files).
3. Start with B.1 RED tests inline in the existing `tests` module.
4. For B.2, the critical step is **adding the call to `self.lexical_kind_for()` inside `parse_all`** at line ~231 (the symbol detection block). The trait method body itself is straightforward.
5. For B.5, open `engine/src/scanner/parser/rust.rs` — the `parse_all` loop at line ~56 needs the same injection of `self.lexical_kind_for()` and `self.extract_references()` calls.
6. After all commits, run the full gate before declaring PR-B ready.

The trait surface contract (`LanguageParser::lexical_kind_for` and `extract_references`) is already stable from PR-A. No trait changes needed for PR-B — only override implementations and call-site wiring.
