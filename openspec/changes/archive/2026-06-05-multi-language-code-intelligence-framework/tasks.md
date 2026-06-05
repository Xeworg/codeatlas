# Tasks: Multi-Language Code-Intelligence Framework

> **Chain strategy (locked from preflight)**: stacked-to-main, each PR ≤ 800 lines.
> **Strict TDD**: RED → GREEN per task; tests + code ship in the same commit.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Total estimated lines | ~1,000 across 3 PRs |
| 400-line budget risk | N/A (session preflight budget = 800 lines) |
| Chained PRs recommended | Yes (user accepted in preflight) |
| Suggested split | PR-A (~280) → PR-B (~380) → PR-C (~340) |
| Delivery strategy | ask-on-risk (resolved at preflight) |
| Chain strategy | stacked-to-main |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Low

## PR-A — IR + Trait Defaults + Python Stub (~280 lines, 4 commits)

### Task A.1: Add IR types to `engine/src/models/file.rs` (RED→GREEN)
- **Files**: `engine/src/models/file.rs` (Modify — add `LexicalValueKind`, `ReferenceKind`, `Reference`, `Range` with `#[serde(rename_all)]`), `engine/src/scanner/parser/ir_tests.rs` (New — RED tests).
- **RED**: `cd engine && cargo test ir_tests::` — must FAIL with "cannot find type `LexicalValueKind`".
- **GREEN**: types compile, tests PASS.
- **Verify**: `cargo test ir_tests::` PASS; `cargo fmt --check` clean.
- **Commit**: `feat(engine): add neutral code-intelligence IR types (RED→GREEN)`
- **Lines**: ~80
- **Status**: ✅ DONE (commit `f968150`, 4 tests in `ir_tests.rs`, types in `file.rs`)

### Task A.2: Extend `ParseResult` additively (RED→GREEN)
- **Files**: `engine/src/models/file.rs` (Modify — add `lexical_kind: LexicalValueKind` and `references: Vec<Reference>` with `#[serde(default)]`); extend `ir_tests.rs` with roundtrip + back-compat test.
- **RED**: `cd engine && cargo test parse_result_roundtrip` — must FAIL.
- **GREEN**: tests PASS; legacy JSON fixture still decodes (read existing `parse_result_default_is_empty` test, must pass).
- **Verify**: `cargo test` (engine) green; SQLite consumers untouched.
- **Commit**: `feat(engine): extend ParseResult with IR fields (back-compat via serde default)`
- **Lines**: ~60
- **Status**: ✅ DONE (commit `de0750a`, 4 tests in `parse_result_tests.rs` + 1 in `file.rs`)

### Task A.3: Add defaulted trait methods on `LanguageParser` (RED→GREEN)
- **Files**: `engine/src/scanner/parser/traits.rs` (Modify — add `lexical_kind_for(_,_)` default `Function` and `extract_references(_,_)` default `vec![]`); `engine/src/scanner/parser/ir_tests.rs` (add `language_parser_defaults_emit_empty_references` test with a minimal in-test impl).
- **RED**: `cd engine && cargo test trait_default_lexical_kind` — must FAIL with "method not found".
- **GREEN**: tests PASS; existing `impl LanguageParser` sites (TS, Rust) still compile.
- **Verify**: `cargo check` in `engine/` AND `src-tauri/` clean.
- **Commit**: `feat(engine): add defaulted trait methods for IR extraction`
- **Lines**: ~40
- **Status**: ✅ DONE (commit `a2cf7f3`, 5 tests in `trait_tests.rs`)

### Task A.4: Python stub + add-a-language integration test (RED→GREEN)
- **Files**: `engine/src/scanner/parser/python_stub.rs` (New — `PythonParser` impl, `extensions=["py"]`, `parse_all` returns `ParseResult::default()`); `engine/src/scanner/parser/mod.rs` (Modify — `pub mod python_stub`); `engine/src/scanner/parser/registry.rs` (Modify — `registry.register(PythonParser::new())`); `engine/tests/fixtures/python/hello.py` (New — `import os\nCONST = 1`); `engine/tests/add_a_language.rs` (New — integration test).
- **RED**: `cd engine && cargo test --test add_a_language` — must FAIL with "module `python_stub` not found".
- **GREEN**: tests PASS; Python dispatch produces stable IR.
- **Verify**: `cargo test --test add_a_language` PASS; `cargo check` (src-tauri) still clean.
- **Commit**: `feat(engine): add Python stub demonstrating add-a-language contract`
- **Lines**: ~100
- **Status**: ✅ DONE (commit `ef4f56a`, 5 integration tests in `add_a_language.rs` + 3 unit tests in `python_stub.rs`)

## PR-B — TS Arrow Detection + Rust `Reference` Emission (~380 lines, 5 commits)

### Task B.1: TS arrow fixtures + RED tests
- **Files**: `engine/tests/fixtures/typescript/arrow_field.ts` (New — `class Svc { handler = (req) => req.body; }`), `engine/tests/fixtures/typescript/object_literal.ts` (New — `export const CONFIG = { a: 1, b: () => 2 };`), `engine/tests/fixtures/typescript/react_const_arrow.tsx` (New — `export const Card = ({title}) => <div>{title}</div>;`); `engine/src/scanner/parser/typescript_tests.rs` (Modify — add RED tests).
- **RED**: `cd engine && cargo test typescript::tests::lexical_kind_arrow_field` — must FAIL.
- **GREEN**: deferred to B.2.
- **Verify**: tests must FAIL at this commit.
- **Commit**: `test(engine): add fixtures + RED tests for TS arrow detection`
- **Lines**: ~80

### Task B.2: TS arrow detection impl (GREEN)
- **Files**: `engine/src/scanner/parser/typescript.rs` (Modify — override `lexical_kind_for`; `lexical_declaration` with `arrow_function` value → `ArrowFunction`; `function_declaration` → `Function`; everything else → `Const`; single pass inside existing `parse_all`).
- **RED→GREEN**: `cd engine && cargo test typescript::tests::lexical_kind_arrow` — must now PASS.
- **Verify**: all B.1 tests PASS; `cargo test` (engine) green.
- **Commit**: `feat(engine): detect arrow functions in TypeScript parser`
- **Lines**: ~60

### Task B.3: TS import/export `Reference` emission (RED→GREEN)
- **Files**: `engine/tests/fixtures/typescript/imports.ts` (New), `engine/tests/fixtures/typescript/exports.ts` (New); `engine/src/scanner/parser/typescript.rs` (Modify — override `extract_references`).
- **RED**: `cd engine && cargo test typescript::tests::reference_import_export` — must FAIL.
- **GREEN**: tests PASS.
- **Verify**: `cargo test typescript::tests::reference` PASS; single-pass invariant (see B.4).
- **Commit**: `feat(engine): emit import/export References in TypeScript parser`
- **Lines**: ~100

### Task B.4: Single-pass counter test
- **Files**: `engine/src/scanner/parser/typescript_tests.rs` (Modify — add `single_pass` test with thread-local atomic counter incremented from `lexical_kind_for` + `extract_references`, asserts counter == 1 per `parse_all` call).
- **RED**: `cd engine && cargo test typescript::tests::single_pass` — must FAIL (counter is 0).
- **GREEN**: refactor if needed; tests PASS.
- **Verify**: counter == 1; existing tests still green.
- **Commit**: `test(engine): assert TS parser uses single AST pass for IR extraction`
- **Lines**: ~30

### Task B.5: Rust conservative `Reference` emission (RED→GREEN)
- **Files**: `engine/tests/fixtures/rust/struct_impl_trait.rs` (New — `use std::collections::HashMap; struct S; impl S { fn m(&self) {} }`); `engine/src/scanner/parser/rust.rs` (Modify — override `extract_references` for `use_declaration` nodes; last path segment or `""`); `engine/src/scanner/parser/rust_tests.rs` (Modify — RED tests).
- **RED**: `cd engine && cargo test rust::tests::reference_use_item` — must FAIL.
- **GREEN**: tests PASS.
- **Verify**: `cargo test rust::tests::reference` PASS.
- **Commit**: `feat(engine): emit conservative import References in Rust parser`
- **Lines**: ~110

## PR-C — Dispatch Consolidation + Shim (~340 lines, 4 commits)

### Task C.1: `engine::commands` pure functions (RED→GREEN)
- **Files**: `engine/src/commands.rs` (New — `pub fn scan_files(registry, paths) -> ScanFilesOutput` and `pub fn outline_for_file(registry, file_id, path, src, ext) -> Vec<OutlineItem>`; NO `tauri::State`, NO DB); `engine/src/commands/tests.rs` (New — mock registry counting calls).
- **RED**: `cd engine && cargo test commands::tests::scan_files_calls_registry_exactly_n_times` — must FAIL with "module `commands` not found".
- **GREEN**: tests PASS; mock registry hit count == file count.
- **Verify**: `cargo test commands::tests::` green.
- **Commit**: `feat(engine): add pure engine::commands orchestration (RED→GREEN)`
- **Lines**: ~130

### Task C.2: Tauri shims call `engine::commands` (RED→GREEN)
- **Files**: `src-tauri/src/commands.rs` (Modify — `scan_project` calls `engine::commands::scan_files` once; `get_node_outline` calls `engine::commands::outline_for_file` once; persistence/tracing block byte-identical); add a vitest + invoke mock asserting shim does NOT call legacy `parse_file`.
- **RED**: `cd src-tauri && cargo test` must FAIL until engine tests are green; vitest must FAIL on legacy call-site text.
- **GREEN**: both green; persistence order preserved.
- **Verify**: `cargo check` in `src-tauri/` clean; `npm run test` green; legacy DB writes unchanged.
- **Commit**: `refactor(tauri): replace 3-loop scan_project with single engine::commands call`
- **Lines**: ~120

### Task C.3: `CodeParser::parse_file` deprecation shim
- **Files**: `engine/src/scanner/code_parser.rs` (Modify — `parse_file` becomes `#[deprecated(note = "use ParserRegistry::parse_file or engine::commands::* instead")]` thin wrapper delegating to `ParserRegistry::default().parse_file(...)` and returning `(result.symbols, result.imports)`).
- **RED**: `cd engine && cargo build` — must show deprecation warnings at legacy call sites in `src-tauri/src/commands.rs`.
- **GREEN**: warnings appear (this is the new contract; no test failure).
- **Verify**: `cargo clippy -- -D warnings` in `src-tauri/` does NOT regress; existing call sites still compile.
- **Commit**: `feat(engine): deprecate CodeParser::parse_file in favor of ParserRegistry`
- **Lines**: ~40

### Task C.4: Author guide — `docs/code-intelligence/adding-a-language.md`
- **Files**: `docs/code-intelligence/adding-a-language.md` (New — checklist, Python stub example, per-language gotchas); link from `README.md` Documentation section.
- **Verify**: link in `README.md` resolves; doc renders.
- **Commit**: `docs(code-intelligence): add language-author guide`
- **Lines**: ~50

## Sequencing & Dependencies

```
PR-A:  A.1 → A.2 → A.3 → A.4   (independent; lands first)
              │
PR-B:  B.1 → B.2 → B.3 → B.4 → B.5   (depends on A.2 + A.3 trait contract)
              │
PR-C:  C.1 → C.2 → C.3 → C.4   (depends on A + B; C.3 is independent of C.1/C.2)
```

Cross-PR edges: A.2 → B.2/B.3 (IR fields exist before parser override); A.3 → B.2/B.3/B.5 (trait method exists); B.2/B.3/B.5 → C.1 (parsers emit full IR before orchestration moves); C.1 → C.2 (pure fn exists before shim calls it); C.3 → C.4 (docs after deprecation is live).

## Per-PR Verification (gate before merge)

- `cd engine && cargo test` (all green)
- `cd engine && cargo clippy -- -D warnings`
- `cd src-tauri && cargo test`
- `cd src-tauri && cargo clippy -- -D warnings`
- `npm run test` (frontend)
- `npm run lint`
- `npm run build`
- `cd engine && cargo bench` — only after PR-C, perf ≤ 1.2× pre-change median

## Per-PR Rollback

Each PR = one merge commit (or 4–5 work-unit commits merged together). `git revert <merge-commit>` restores prior state. The IR extensions are `#[serde(default)]` so reverts cannot corrupt persisted data.

## Total Summary

| PR | Tasks | Estimated lines | Cap |
|----|-------|-----------------|-----|
| PR-A | 4 (A.1–A.4) | 280 | 800 |
| PR-B | 5 (B.1–B.5) | 380 | 800 |
| PR-C | 4 (C.1–C.4) | 340 | 800 |
| **Total** | **13 tasks** | **~1,000** | — |

## Review Workload Forecast (final)

- Chained PRs recommended: **yes** (user preflight accepted)
- 400-line budget risk: **N/A** (session budget = 800)
- Decision needed before apply: **No** (`stacked-to-main` is locked)
