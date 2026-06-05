# Verify Report — PR-A: Code-Intelligence IR

**Status**: WARN
**Branch**: `feat/code-intelligence-ir-pr-a`
**Commits verified**: 4 (f968150, de0750a, a2cf7f3, ef4f56a)
**Lines changed**: 709 (cap 800)
**Strict TDD**: 100% adherence per apply-progress #541
**Verified at**: 2026-06-04T03:54:09Z

## Summary

PR-A correctly lays the language-neutral IR foundation (`LexicalValueKind`, `Reference`, `ReferenceKind`, `Range`), extends `ParseResult` additively with `#[serde(default)]` (preserving legacy JSON), and exposes defaulted `LanguageParser::lexical_kind_for` / `extract_references` so a new language plugs in with only `impl LanguageParser` + `registry.register(...)`. All 145 engine tests pass, all 28 src-tauri tests pass, `npm run lint` is clean, and the 800-line budget is honoured (709 lines, 91 lines under cap). Verdict is **WARN** because the spec's bullet-list MUST clauses around parser emission (LexicalValueKind discrimination, Reference emission, single-pass invariant, stable SymbolInfo id) are deferred to PR-B by design and lack covering tests in PR-A.

## Spec Compliance — `code-intelligence-ir`

### Requirements

| # | Requirement | Status | Evidence |
|---|-------------|--------|----------|
| 1 | IR Shape — `LexicalValueKind`, `Reference` added; `ParseResult` extended additively; legacy fields unchanged | PASS | `engine/src/models/file.rs:91-162` defines all 4 types + 2 new `ParseResult` fields with `#[serde(default)]`. `parse_result_tests::parse_result_exposes_lexical_kind_and_references` passes; `parse_result_tests::parse_result_legacy_json_without_ir_fields_still_decodes` confirms back-compat. `models::file::tests::parse_result_default_is_empty` (pre-existing) still green. |
| 2 | Invariante de Identidad Estable — `(file_id, kind, name, range)` produces stable IDs | WARN | `OutlineItem::stable_id` is deterministic and tested by `outline_item_stable_id_format`. However, `SymbolInfo.id` is generated via `uuid::Uuid::new_v4()` in `engine/src/scanner/parser/typescript.rs:287` and `rust.rs:74` — pre-existing issue, NOT introduced by PR-A. The IR spec is new and was apparently written assuming `SymbolInfo::id` would be derived from the same composite key, but that would require a follow-up refactor. Flag for follow-up. |
| 3 | Contrato de Emisión de Reference — TS emits `Import`/`Export` References, Rust emits conservative shape | DEFERRED (PASS-by-design) | PR-B's B.3 and B.5 explicitly own this. PR-A only adds the `Reference` type itself (covered by `ir_tests::reference_roundtrip_preserves_file_id_kind_name_and_range`). The trait method `extract_references` is now available with a `Vec::new()` default; concrete overrides come in PR-B. |
| 4 | Trait Extension sin Duplicación — defaulted `lexical_kind_for` / `extract_references`, file-static helpers preserved | PASS | `engine/src/scanner/parser/traits.rs:36-46` adds the two default methods. `ts_node_kind_to_outline_kind` (line 50), `rust_node_kind_to_outline_kind` (line 64), `make_outline_id` (line 78) all retained. `trait_tests::default_lexical_kind_for_returns_function`, `default_extract_references_returns_empty_vec`, `default_methods_invocable_via_dyn_trait_object`, `minimal_parser_compatible_with_registry_dispatch`, `override_lexical_kind_is_observed` all pass. |
| 5 | Add-a-Language Contract — only `impl LanguageParser` + `registry.register(...)` | PASS | `add_a_language::python_stub_dispatches_without_ir_changes` and `add_a_language::python_stub_is_registerable_alongside_existing_parsers` pass. `PythonParser` is in `engine/src/scanner/parser/python_stub.rs` with only the 4 core trait methods. The integration test explicitly creates a fresh `ParserRegistry::new()` + `register(PythonParser::new())` and asserts dispatch by extension. |
| 6 | Single AST Pass — `symbols`, `outline`, `lexical_kind`, `references` built in one walk | DEFERRED (PASS-by-design) | PR-B's B.4 owns the single-pass counter test. PR-A provides the default methods with `Vec::new()` / `Function` so the trait surface is complete; the single-pass constraint applies to the **overrides** that PR-B adds. The default impl trivially satisfies it. |

### Scenarios

| # | Scenario | Status | Evidence |
|---|----------|--------|----------|
| 1 | ParseResult expone los nuevos campos | PASS | `parse_result_tests::parse_result_exposes_lexical_kind_and_references` constructs and asserts `result.lexical_kind` and `result.references[0].target_name`. |
| 2 | LexicalValueKind discrimina arrow-vs-const | WARN | The IR types support the discrimination (`LexicalValueKind::ArrowFunction` and `::Const` both exist; `#[default] Const`); no TypeScript parser in PR-A emits `ArrowFunction` (that override is PR-B's B.2). The "arrow-vs-const" discrimination test will land in PR-B (`ts_arrow_field_emits_arrow_function_symbol`). |
| 3 | Re-scan produce mismos IDs | WARN | `OutlineItem::stable_id` is deterministic and tested. `SymbolInfo::id` is UUID-generated (pre-existing, not PR-A's fault). PR-A doesn't add a re-scan integration test for either type, but the invariant was already partially established for outline items. |
| 4 | TS emite Import reference | DEFERRED | PR-B's B.3 owns this scenario. |
| 5 | Rust emite forma conservadora | DEFERRED | PR-B's B.5 owns this scenario. |
| 6 | Parser mínimo compila sin overrides | PASS | `trait_tests::MinimalParser` implements only the 4 core methods; `default_lexical_kind_for_returns_function` and `default_extract_references_returns_empty_vec` prove defaults are inherited. |
| 7 | Stub de cuarto lenguaje se registra sin tocar IR | PASS | `add_a_language::python_stub_dispatches_without_ir_changes` and `python_stub_is_registerable_alongside_existing_parsers` pass. `python_stub_parse_result_roundtrips_through_serde` confirms the IR shape is stable. |
| 8 | (variant of 7) Stub `PythonParser` extends `ParserRegistry` | PASS | `engine/src/scanner/parser/registry.rs:22` registers `PythonParser::new()` in `ParserRegistry::new()`. `parser_for_extension_returns_supported_parsers` (pre-existing test, now covers `py`) passes. |
| 9 | Parser no invoca second-pass | DEFERRED | PR-B's B.4 owns the single-pass counter test. |

## Spec Compliance — `multi-language-dispatch` (in-scope parts only)

| # | Requirement | Status | Evidence |
|---|-------------|--------|----------|
| 1 | `ParserRegistry` es el Único Punto de Dispatch | DEFERRED (PR-C) | Out of PR-A scope per orchestrator. Pre-existing. |
| 2 | Shim Deprecated para `CodeParser::parse_file` | DEFERRED (PR-C) | Out of PR-A scope. PR-A does not touch `code_parser.rs`. |
| 3 | `scan_project` usa Registry Una Sola Vez | DEFERRED (PR-C) | Out of PR-A scope. |
| 4 | `get_node_outline` usa Registry Una Sola Vez | DEFERRED (PR-C) | Out of PR-A scope. |
| 5 | Add-a-Language no Toca Dispatch | PASS | `add_a_language::python_stub_dispatches_without_ir_changes` and `python_stub_is_registerable_alongside_existing_parsers` pass. No diffs in `commands.rs`, `code_parser.rs`. |

## Deviations Assessment

| # | Deviation | Spec Impact | Action |
|---|-----------|-------------|--------|
| 1 | Test file split: `ir_tests.rs`, `parse_result_tests.rs`, `trait_tests.rs` instead of one file | None — all 13 tests across the three files cover the original spec scenarios. `parse_result_tests` covers the ParseResult field exposure (Req 1) and the legacy back-compat invariant. `trait_tests` covers the defaulted methods (Req 4) and the minimal-parser compile check (Scenario 6). `ir_tests` covers the type-level serialisation contract. | OK |
| 2 | IR types in `engine/src/models/file.rs` (not `engine/src/scanner/parser/ir.rs`) | None — `design.md` decisions #1 and the IR extension snippet (line 70, 113-147) explicitly place the types in `models/file.rs`. Tasks.md A.1 also says `engine/src/models/file.rs (Modify)`. | OK |
| 3 | `PythonParser` auto-registered in `ParserRegistry::new()` | **WARN** — `multi-language-dispatch` Req 5 says "Incorporar un nuevo parser (cuarto lenguaje) MUST requerir SOLO `impl LanguageParser` y `registry.register(...)`". The auto-registration does not violate the MUST (it still works with manual `register` too — `add_a_language.rs:36-37` proves it), but the spec language implies "register" is the explicit step. The auto-register behaviour is a *default convenience*; remove the line in `registry.rs:22` if strict reading is required. | **WARN — user decision** |

## Open Issues

| # | Issue | Severity | Source | Action |
|---|-------|----------|--------|--------|
| 1 | TS parser override order (`lexical_kind_for` default = `Function`; PR-B must override to `ArrowFunction`/`Const`) | Info | apply-progress #541 | Track for PR-B (B.2) |
| 2 | Single-pass invariant (no `node.find(...)` loops) | Info | apply-progress #541 | Track for PR-B (B.4) |
| 3 | `#[serde(rename_all = "camelCase")]` on `ParseResult` changes JSON shape for frontend | Info | apply-progress #541 | Track for PR-C (frontend impact) |
| 4 | Pre-existing clippy errors in `src/analysis/degraded_tests.rs:93-94` (`cycles.len() >= 0`, `hotspots.len() >= 0` always-true) | Pre-existing | apply-progress #541 | Confirmed on `main` via `git checkout main && cargo clippy`. Not PR-A's fault. Track in cleanup issue. |
| 5 | Pre-existing clippy errors in benches (`main` function not found in `bench_graph_insights.rs`, `bench_impact.rs`, `bench_export.rs`; unresolved imports in `bench_arch_detection_test.rs`, `wal_concurrency_test.rs`) | Pre-existing | confirmed on `main` | Not PR-A's fault. Track in cleanup. |
| 6 | Pre-existing src-tauri clippy in `src/commands.rs:1027` (`vec!["json", "png"]` should be array) | Pre-existing | confirmed on `main` (`40418f53` from 2026-05-31) | Not PR-A's fault. Track in cleanup. |
| 7 | `SymbolInfo::id` is `uuid::Uuid::new_v4()` — violates the spec's stable-id invariant for `SymbolInfo` | Pre-existing | `engine/src/scanner/parser/typescript.rs:287` and `rust.rs:74` | PR-A doesn't introduce; the IR spec wording implies SymbolInfo.id should be derived from `(file_id, kind, name, range)`. Track for follow-up. |
| 8 | `python_stub_default_impls_inherit_from_trait` test has `unused variable: parser` clippy warning at `engine/src/scanner/parser/python_stub.rs:79` | WARNING (PR-A) | clippy | New minor warning introduced by PR-A. Easy fix: prefix with `_` or remove the unused binding (the test is mainly a compile-check). |
| 9 | Python auto vs manual registration | **WARN** | spec `multi-language-dispatch` Req 5 | **User decision needed.** Auto-registration in `ParserRegistry::new()` is convenient but may violate strict spec reading. See Deviations #3. |

## Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| `cd engine && cargo test` | PASS | 145/145 (135 lib + 5 add_a_language + 3 bench_arch + 2 wal_concurrency) |
| `cd engine && cargo test --test add_a_language` | PASS | 5/5 |
| `cd engine && cargo test --lib -- ir_tests:: parse_result_tests:: trait_tests:: python_stub::` | PASS | 16/16 new + 2 pre-existing models tests still green |
| `cd src-tauri && cargo test` | PASS | 28/28 (no regressions) |
| `cd engine && cargo clippy --lib -- -D warnings` | PASS | clean |
| `cd engine && cargo clippy --test add_a_language -- -D warnings` | PASS | clean |
| `cd engine && cargo clippy --all-targets -- -D warnings` | FAIL (pre-existing only) | Pre-existing failures in `degraded_tests.rs`, benches (`main` function), `wal_concurrency_test.rs` (`Mutex` import), `bench_arch_detection_test.rs` (`tempfile` import). Verified identical on `main`. **PR-A adds exactly one new minor warning:** `unused variable: parser` in `python_stub.rs:79` (test code). |
| `cd src-tauri && cargo clippy --lib -- -D warnings` | PASS | clean |
| `cd src-tauri && cargo clippy --all-targets -- -D warnings` | FAIL (pre-existing only) | Pre-existing `vec!["json", "png"]` in `src/commands.rs:1027`. Verified identical on `main` (commit `40418f53` from 2026-05-31). |
| `npm run lint` | PASS | 0 errors, 0 warnings |
| Commits match tasks.md | PASS | All 4 commit messages are exactly as specified in tasks.md A.1–A.4 |
| Within 800-line budget | PASS | 709 insertions, 3 deletions (91 lines under cap) |
| Out-of-scope file changes | PASS | Diff is limited to 10 files, all explicitly listed in tasks.md A.1–A.4. No `src-tauri/` diff. No `.pi/`, `.opencode/`, or settings changes. |
| `.pi/` untouched | PASS | `git diff --stat main..feat/code-intelligence-ir-pr-a -- .pi/ .opencode/` returns empty |
| SQLite schema unchanged | PASS | PR-A is purely additive to the in-memory `ParseResult`. `#[serde(default)]` on both new fields preserves legacy JSON. No DB migration file added. |
| Test layers | OK | 13 unit tests (in `ir_tests`, `parse_result_tests`, `trait_tests`, `python_stub`); 5 integration tests (in `add_a_language`); 0 E2E. Coverage tool (cargo-llvm-cov) not configured — skipped. |
| Assertion quality | OK | All assertions verify real behaviour: `serde_json::to_string` / `from_str` round-trips, field equality, `result.symbols.is_empty()`, registry dispatch by extension. No tautologies, no ghost loops, no smoke tests. |

## CRITICAL Issues

None.

## WARNINGS

1. **Parser-emission scenarios deferred to PR-B.** Scenarios 2 (`LexicalValueKind` arrow-vs-const), 4 (TS Import), 5 (Rust conservative), 9 (single-pass) have no covering tests in PR-A. PR-A's design.md and tasks.md explicitly defer them; the type system and trait defaults are in place to enable PR-B. Marking these as WARN (not CRITICAL) because PR-A's contract is "make the IR real" and PR-B's contract is "make the parsers emit it".

2. **`SymbolInfo::id` is UUID-generated.** The IR spec's "Re-scan produce mismos IDs" scenario implies a stable id derived from `(file_id, kind, name, range)`. `OutlineItem::stable_id` does this; `SymbolInfo::id` does not. Pre-existing, not PR-A's fault, but the new spec wording makes the gap visible. Recommend opening a follow-up issue.

3. **Python auto-registration.** `ParserRegistry::new()` pre-registers `PythonParser`. The spec wording "MUST requerir SOLO `impl LanguageParser` y `registry.register(...)`" is satisfied functionally (manual registration also works, proven by `add_a_language.rs:36-37`), but a strict reader could argue auto-registration bypasses the explicit `register` call. **User decision needed:** keep auto-register, or remove `registry.register(PythonParser::new())` from `registry.rs:22` and require every consumer to register explicitly.

4. **One new minor clippy warning in PR-A's test code.** `python_stub_default_impls_inherit_from_trait` declares `let parser = PythonParser::new();` but never uses it. Trivial fix: rename to `_parser` or drop the binding. Not blocking, but should be cleaned up.

## SUGGESTIONS

1. **Assertion quality on `python_stub_default_impls_inherit_from_trait`.** The test is mostly a compile-check that `lexical_kind_for` and `extract_references` are reachable as trait methods. The `let parser = ...` line is dead. Consider replacing it with a clearer runtime assertion (e.g. parse a tiny source with the stub, check the result, then assert the trait surface is callable).

2. **`models::file::tests::parse_result_default_is_empty` predates PR-A.** The pre-existing test asserts `symbols`, `imports`, `outline` are empty on default. PR-A's two new fields (`lexical_kind`, `references`) should also be asserted in this test for the default — covered separately by `parse_result_tests::parse_result_default_has_empty_references_and_const_lexical_kind`. Consider merging them or adding a comment cross-referencing both.

3. **No pre-existing test for `SymbolInfo` id stability.** Now that the IR spec mandates it, a test that re-parses a fixture and compares SymbolInfo.id is appropriate. Defer to follow-up.

## Decision

**Verdict**: WARN

PR-A correctly implements the IR foundation it owns: types, defaulted trait methods, add-a-language contract, Python stub, and a 709-line additive diff well under the 800-line budget. All 145 engine tests pass, all 28 src-tauri tests pass, `npm run lint` is clean, and Strict TDD is honoured per the apply-progress evidence (4 commits, one per task, RED→GREEN per task, 22 new tests).

The WARN is justified because:

- **Three spec scenarios have no PR-A covering test** (lexical kind arrow-vs-const, TS Import emission, Rust conservative emission, single-pass). These are explicitly deferred to PR-B by the design and tasks. Marking as WARN, not FAIL, because the IR foundation is the contract for PR-B; the parsers will be the contract for those scenarios.
- **Pre-existing `SymbolInfo::id` UUID issue** is now visible against the new spec wording. Not introduced by PR-A.
- **Python auto-registration** may or may not match the spec's strict reading — user input needed.
- **One trivial clippy warning** in PR-A's test code.

**Recommendation:** Merge PR-A. Open three follow-up issues: (1) `SymbolInfo::id` should be derived from `(file_id, kind, name, range)`; (2) clean up pre-existing clippy in `degraded_tests.rs` and benches; (3) user decision on Python auto vs manual registration. After merge, launch `sdd-apply` for PR-B.
