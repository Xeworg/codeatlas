# PR-B Review — Multi-Language Code-Intelligence IR

**Branch:** `feat/parser-ir-pr-b`
**Review date:** 2026-06-04
**Reviewer:** el Gentleman (fresh-context review subagent)

---

## Status

**PASS with notes** — functionally correct, all 158 tests green. Two non-blocking findings and one minor gap noted below. The fmt gate failures are cosmetic and confined to PR-B files (not pre-existing as the worker claimed).

---

## Executive Summary

PR-B introduces the code-intelligence IR types (`LexicalValueKind`, `ReferenceKind`, `Range`, `Reference`) and extends `ParseResult` with `lexical_kind` and `references` fields. It wires the new IR hooks (`lexical_kind_for`, `extract_references`) into the TypeScript and Rust parsers within their existing single-pass `parse_all` walks, and adds a Python stub demonstrating the add-a-language contract. The implementation is additive, backward-compatible, and well-covered by focused tests.

The core logic — TypeScript priority-based lexical classification, TypeScript import/export reference extraction, and Rust conservative `use`-declaration reference emission — is correct. The single-pass wiring is clean with no double-extraction bugs.

---

## Confirmed Good

### 1. IR type definitions (`engine/src/models/file.rs`)

- `LexicalValueKind` enum with `Const` (default), `ArrowFunction`, `Function` — correctly annotated `#[serde(rename_all = "snake_case")]`.
- `ReferenceKind` enum with `Import`, `Export`, `Call`, `TypeRef` — correct snake_case serialization.
- `Range` struct with all six fields — correct `#[serde(rename_all = "camelCase")]`.
- `Reference` struct with `file_id` (design decision #6), `kind`, `target_name`, `range` — correct camelCase.
- `ParseResult` extended with `lexical_kind` and `references`, both `#[serde(default)]` for back-compat. The `#[derive(Serialize)]` is newly added to `ParseResult`, which is additive and enables future serialization paths.

### 2. TypeScript `lexical_kind_for` (`engine/src/scanner/parser/typescript.rs:289-311`)

- Correctly classifies `function_declaration` → `Function`, `lexical_declaration` → `ArrowFunction`/`Const` based on arrow detection, `class_declaration` → `ArrowFunction`/`Const` based on arrow field detection.
- `export_statement` correctly unwraps to inner declaration via `find_declaration_child`.
- Fallback `_ => Const` is safe.

### 3. TypeScript `extract_references` (`typescript.rs:313-400`)

- Correctly handles `import_statement` for default imports, named imports (with alias support via `child_by_field_name("alias")`), and namespace imports.
- Correctly handles `export_statement` by extracting the declaration name.
- `References` get `file_id` filled by `parse_all` (trait hook returns empty string intentionally).

### 4. TypeScript `parse_all` wiring (`typescript.rs:499-511` ff.)

- Import references are extracted in the existing `import_statement` block; the `continue` prevents double-extraction at the bottom guard.
- Export references are extracted at the bottom guard (lines ~477-483), correctly reached after symbol/outline/lexical-kind processing.
- Lexical kind priority logic (`ArrowFunction > Function > Const`) is correct: starts at default `Const`, upgrades to `Function` if any function declaration, upgrades to `ArrowFunction` if any arrow-valued binding/class.
- Arrow-field symbols (`SymbolKind::ArrowFunction`) are emitted via `collect_arrow_field_symbols` for bare `class_declaration` nodes.

### 5. Rust `lexical_kind_for` (`rust.rs:43-48`)

- Clean: `function_item` → `Function`, everything else → `Const`. No `ArrowFunction` for Rust (correct, Rust has no arrow functions).

### 6. Rust `extract_references` (`rust.rs:50-94`)

- Conservative emission for `use_declaration`: extracts last segment as `target_name`.
- Special handling for `self`, `super`, `crate` → empty `target_name` (v1 conservative).
- Glob (`*`) → empty `target_name`.
- `file_id` filled in `parse_all`, trait hook returns empty string.

### 7. Rust `parse_all` wiring (`rust.rs:95-180`)

- Reference extraction for `use_declaration` is inline in the existing block, correct.
- Lexical kind upgrade from `Const` to `Function` via `if matches!` at bottom of loop, correct.

### 8. Trait defaults (`traits.rs:27-41`)

- `lexical_kind_for` default returns `Function` — the most permissive fallback.
- `extract_references` default returns empty `Vec`.
- Both are invocable via `&dyn LanguageParser` (verified by `trait_tests.rs`).

### 9. Python stub (`python_stub.rs`)

- Implements the four core methods (`language_id`, `extensions`, `parse_all`, inherits `supports`).
- Inherits default `lexical_kind_for` and `extract_references` — zero IR work needed.
- Cleanly demonstrates the add-a-language contract.

### 10. Test coverage

- **153 unit tests pass**, **5 integration tests pass** (`add_a_language.rs`) — total 158 green.
- IR type serialization tests in `ir_tests.rs` — covers all four IR types.
- `ParseResult` integration tests in `parse_result_tests.rs` — field exposure, defaults, roundtrip, legacy JSON back-compat.
- Trait default tests in `trait_tests.rs` — dyn dispatch, minimal parser, override observation.
- TypeScript PR-B tests: lexical kind detection, import/export reference emission, single-pass population.
- Rust PR-B tests: last-segment extraction, glob/self/super/crate conservative, lexical kind.
- Python stub integration tests in `add_a_language.rs`.
- **Backward compatibility verified:** legacy JSON `{"symbols":[],"imports":[],"outline":[]}` deserializes cleanly with `references` defaulting to empty and `lexical_kind` defaulting to `Const`.

### 11. No regressions in existing consumers

- `CodeParser::parse_file_all` → `ParserRegistry::parse_file` → returns `ParseResult` with new fields; existing consumers only access `.outline`, `.symbols`, `.imports`.
- Tauri commands (`commands.rs`) only use `.outline` from the result — unaffected by new fields.
- `#[serde(default)]` ensures old JSON in SQLite/any persistence layer deserializes without error.

---

## Findings Now

### Finding 1 (Note): fmt gate failures are in PR-B files, not pre-existing

The worker reported that `cargo fmt --check` "fails on pre-existing unrelated files: ir_tests.rs, parse_result_tests.rs, python_stub.rs." This is incorrect — all three files are **new additions in PR-B** (verified via `git diff --stat`). The formatting issues are:

| File                        | Issue                                            |
| --------------------------- | ------------------------------------------------ |
| `ir_tests.rs:59`            | Long `assert!` line needs multi-line formatting  |
| `parse_result_tests.rs:46`  | Long `assert!` line needs multi-line formatting  |
| `parse_result_tests.rs:101` | Long `let parsed: ParseResult = ...` line        |
| `python_stub.rs:84-90`      | Long function type annotation needs reformatting |

All are cosmetic only — no functional impact. A single `cargo fmt` run resolves them.

### Finding 2 (Gap): Exported classes with arrow fields miss `SymbolKind::ArrowFunction` symbols

In `typescript.rs` `parse_all`, the `collect_arrow_field_symbols` call is gated on:

```rust
if kind == "class_declaration" {
    let field_symbols = Self::collect_arrow_field_symbols(&node, bytes, file_id);
    ...
}
```

For `export class Svc { handler = (req) => req.body }`, the top-level node kind is `export_statement`, not `class_declaration`. The `collect_arrow_field_symbols` block is never entered. The `handler` field symbol is **not emitted** for exported classes.

**Impact:** The `lexical_kind` classification still works correctly (via `lexical_kind_for` unwrapping `export_statement` → `class_declaration`), and the class itself gets its symbol. Only the individual arrow-field sub-symbols are missed for exported classes.

**Test coverage gap:** The existing test `lexical_kind_arrow_class_method_field_emits_arrow_function_symbol` uses fixture `arrow_field.ts` (bare `class Svc`) — it does not test the exported case.

**Severity:** Minor. The arrow-field symbol emission is a nice-to-have enhancement that the AI layer might use for finer-grained reasoning. The primary IR goal (`lexical_kind`) is unaffected. This can be addressed in a follow-up without blocking PR merge.

### Finding 3 (Note): Re-exports not captured by `extract_references`

`export { foo } from './bar'` (re-export syntax) and `export { foo }` (bare re-export) are not captured as `ReferenceKind::Export`. The `find_declaration_child` helper returns the first named child, which for re-exports is `export_clause` — which has no direct name field that `ts_declaration_name` can extract. This is acceptable for v1 conservative emission. V2 or a later PR can add explicit `export_clause` → `export_specifier` traversal.

---

## Fmt Gate Assessment

**Verdict: Cosmetic, not a blocker.**

The four formatting diffs are in three PR-B files (`ir_tests.rs`, `parse_result_tests.rs`, `python_stub.rs`). All are line-length violations that `cargo fmt` resolves automatically. They do not affect:

- Test correctness or coverage
- Runtime behavior
- API contracts
- Backward compatibility

The worker's characterization as "pre-existing unrelated" is wrong — these files are new in PR-B — but the failure itself is authentic and should be fixed before merge for repo hygiene.

**Fix:** `cd engine && cargo fmt` (one command, no manual edits needed).

---

## Recommended Next Step

1. **Run `cd engine && cargo fmt`** to resolve the four formatting diffs.
2. **Optionally**, add a test for exported class arrow-field symbol emission if the gap is considered blocking. Otherwise, file a follow-up issue.
3. **Optionally**, note the re-export gap as a v2 enhancement.
4. After fmt + optional gap resolution, PR-B is functionally ready for merge.

---

## Residual Risks

| Risk                                       | Likelihood                          | Impact                                       | Mitigation                        |
| ------------------------------------------ | ----------------------------------- | -------------------------------------------- | --------------------------------- |
| Exported class arrow-field symbols missing | Already present                     | Low — `lexical_kind` still correct           | File follow-up issue              |
| Re-export references missing               | Already present                     | Low — v1 scope accepts conservative emission | v2 enhancement                    |
| `ParseResult` now derives `Serialize`      | None — additive change              | None — no existing serialization path        | N/A                               |
| Old JSON consumers break on new fields     | None — `#[serde(default)]` verified | None                                         | Tested in `parse_result_tests.rs` |

---

## Skill Resolution

- **paths-injected:** none (parent did not provide skill paths for this review).
- **fallback-registry:** none — no project skills needed for this review scope.
- **Final:** `none` — no project/user `SKILL.md` files were loaded. The review was conducted via direct file inspection and test execution.
