# PR-C Implementation Surface Map

## Status

**Complete.** All 7 key files inspected, 5 mismatches flagged. Ready for `sdd-apply` phase.

## Executive Summary

PR-C has 4 tasks that collapse the current 3-loops-per-file scan into a single `ParserRegistry` dispatch. The implementation surface is well-understood:

- **C.1** creates `engine/src/commands.rs` (pure functions, no Tauri/DB dependency)
- **C.2** rewrites `scan_project` (the biggest diff ~250 lines) and `get_node_outline` in `src-tauri/src/commands.rs`
- **C.3** deprecates `CodeParser::parse_file` in `engine/src/scanner/code_parser.rs`
- **C.4** creates `docs/code-intelligence/adding-a-language.md`

The total diff is ~340 lines. The biggest risk is C.2's `scan_project` rewrite — currently 3 separate loops (Phase 1 symbols, Phase 2 imports, Phase 3 outline). The consolidation preserves `AppState`, tracing, DB persistence, and error mapping intact.

5 mismatches between the design/spec artifacts and the actual repo were identified and documented below.

## Files to Touch

### New files (C.1, C.4)

| #   | File                                          | Action  | Reason                                                                            |
| --- | --------------------------------------------- | ------- | --------------------------------------------------------------------------------- |
| 1   | `engine/src/commands.rs`                      | **New** | Pure `scan_files()` and `outline_for_file()` functions + `ScanFilesOutput` struct |
| 2   | `engine/src/commands/tests.rs`                | **New** | RED tests: mock registry counting `parse_file` invocations                        |
| 3   | `docs/code-intelligence/adding-a-language.md` | **New** | Author guide with checklist and Python stub example                               |

### Modified files (C.1–C.3)

| #   | File                                | Action     | Reason                                                                                                  |
| --- | ----------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------- |
| 4   | `engine/src/lib.rs`                 | **Modify** | Add `pub mod commands;`                                                                                 |
| 5   | `engine/src/scanner/mod.rs`         | **Modify** | Export `DiscoveredFile` from `walker` (currently not public)                                            |
| 6   | `src-tauri/src/commands.rs`         | **Modify** | Rewrite `scan_project` (3 loops → 1 call) and `get_node_outline` (replace `CodeParser::parse_file_all`) |
| 7   | `engine/src/scanner/code_parser.rs` | **Modify** | Add `#[deprecated]` to `parse_file`, delegate to `ParserRegistry::parse_file`                           |

### Test-only files (C.2)

| #   | File                                         | Action     | Reason                                                                   |
| --- | -------------------------------------------- | ---------- | ------------------------------------------------------------------------ |
| 8   | `src-tauri/src/commands/tests/shim_tests.rs` | **New**    | Vitest/integration test asserting shim does NOT call legacy `parse_file` |
| 9   | `src-tauri/src/commands/tests.rs`            | **Modify** | Add `mod shim_tests;`                                                    |

### Indirectly affected (C.1–C.3 interaction)

| #   | File                                    | Action        | Reason                                                     |
| --- | --------------------------------------- | ------------- | ---------------------------------------------------------- |
| 10  | `engine/src/scanner/parser/registry.rs` | **No change** | `parse_file` method already exists; no API change needed   |
| 11  | `src-tauri/src/lib.rs`                  | **No change** | `invoke_handler` stays the same; AppState wiring unchanged |

## Task-by-Task Notes

### C.1 — `engine::commands` pure functions (RED→GREEN)

**What to build:**

- `engine/src/commands.rs` — two pure functions:
  - `scan_files(registry, files) -> ScanFilesOutput`
  - `outline_for_file(registry, file_id, path, src, ext) -> Vec<OutlineItem>`
- `engine/src/commands/tests.rs` — mock registry tests

**Key design constraints:**

- **NO** `tauri::State`, no DB, no tracing
- Takes `&ParserRegistry` and `&[DiscoveredFile]`
- `scan_files` calls `registry.parse_file(...)` exactly once per file
- Returns `ScanFilesOutput { file_infos, all_imports, parse_ms, files_read, files_failed }`

**Mismatch 1 — `DiscoveredFile` not exported:**
`DiscoveredFile` is defined in `engine/src/scanner/walker.rs` (line 23) but is NOT re-exported from `scanner/mod.rs` or `engine/lib.rs`. The design signature requires it as an input parameter.
**Fix:** Add `pub use walker::DiscoveredFile` to `engine/src/scanner/mod.rs`.

**Mismatch 2 — `ScanFilesOutput` doesn't exist:**
This struct needs to be defined in `engine/src/commands.rs`. Fields per design: `file_infos: Vec<FileInfo>`, `all_imports: Vec<ImportInfo>`, `parse_ms: u64`, `files_read: usize`, `files_failed: usize`.

**Mismatch 3 — `root: &Path` parameter is redundant:**
The design signature includes `root: &Path` but `DiscoveredFile.path` is already absolute (set in `walker.rs` line 71). The `scan_files` function reads content from `file.path` directly. No root resolution needed.
**Recommendation:** Drop `root` from the signature, or keep it as an `Option<&Path>` for future use.

**Mock approach for tests:**
Since `ParserRegistry` owns `Vec<Box<dyn LanguageParser>>`, mocking requires either:

1. A test-only `CountingParser` that implements `LanguageParser` and increments a `Cell<usize>` counter — registered in a test-only `ParserRegistry::with_parsers(vec![CountingParser::new()])`.
2. Or expose a `ParserRegistry::with_parsers(parsers: Vec<Box<dyn LanguageParser>>)` constructor for test injection.

Option 1 is cleaner. The `CountingParser` returns `ParseResult::default()` and counts `parse_all` calls.

### C.2 — Tauri shims call `engine::commands` (RED→GREEN)

**Current state in `scan_project` (lines 38-260 of `src-tauri/src/commands.rs`):**

```
Phase 1 loop (lines 50-69):   for each file: parse_file(symbols), build FileInfo
Phase 2 loop (lines 85-120):  for each file: parse_file(imports), resolve paths, extend all_imports
Phase 3 loop (lines 201-227): for each file: parse_file_all(outline), persist outline
```

**Target state:**

```
Single call: engine::commands::scan_files(&registry, &discovered) -> ScanFilesOutput
Then: thread file_infos and all_imports through existing persistence block
Then: Phase 3 outline persistence loop stays but uses ParseResult from scan_files output
```

Wait — the design says `scan_files` produces outline too through `ParseResult.outline`. But the current Phase 3 reads files again. The target could:

- Option A: have `scan_files` collect outlines and return them in the output (add `outlines: Vec<(String, Vec<OutlineItem>)>` to `ScanFilesOutput`).
- Option B: keep Phase 3 but use `outline_for_file` instead of `CodeParser::parse_file_all`.

The tasks.md says `scan_files` returns `ScanFilesOutput` and separately `outline_for_file` exists. The design says "single registry call" and threading "single-parsed data drives all three sinks." This suggests Option A — the `ScanFilesOutput` should include outlines.

**Recommendation:** Include outlines in `ScanFilesOutput` to truly achieve single-parse-per-file. The structure:

```rust
pub struct ScanFilesOutput {
    pub file_infos: Vec<FileInfo>,
    pub all_imports: Vec<ImportInfo>,
    pub outlines: Vec<(String, Vec<OutlineItem>)>,  // (file_id, outline)
    pub parse_ms: u64,
    pub files_read: usize,
    pub files_failed: usize,
}
```

**What changes in `scan_project`:**

1. Remove `CodeParser` import
2. Replace lines 42-120 (discover + Phase 1 + Phase 2) with single call
3. Replace Phase 3 loop (lines 201-227) to use `ScanFilesOutput.outlines`
4. Keep persistence/tracing byte-identical

**What changes in `get_node_outline`:**

- Line 486: replace `CodeParser::parse_file_all(...)` with `engine::commands::outline_for_file(...)`
- Remove `CodeParser` import (once it's no longer used anywhere)

**Mismatch 4 — Vitest test infrastructure is greenfield:**
C.2 says "add a vitest + invoke mock asserting shim does NOT call legacy `parse_file`." There are zero existing vitest tests that test Tauri commands. The `vitest.config.ts` exists but only has React/alias config. No `@tauri-apps/api` mock setup, no `invoke` test infrastructure.

**Fix options:**

- Add a Rust-level test in `src-tauri/src/commands/tests/` that checks the shim by inspecting the source code or call graph (simpler, but limited).
- Add a TypeScript test in `src/` that mocks `@tauri-apps/api` and verifies the expected command names/handlers (more complete, but requires setting up mock infrastructure first).

**Recommendation:** Use a Rust-level `#[cfg(test)]` test in `src-tauri/src/commands/tests/shim_tests.rs` that creates the `AppState` with an in-memory SQLite DB and tests `scan_project` end-to-end — verifying the `ScanFilesOutput` flows through persistence correctly. This is more valuable than a vitest mock.

Alternatively, the task's vitest requirement can be met with a minimal smoke test that verifies the Tauri command handler is registered (which it always is since `invoke_handler!` macro is compile-time).

### C.3 — `CodeParser::parse_file` deprecation shim

**Current `parse_file` (lines 15-38 in `code_parser.rs`):**

```rust
pub fn parse_file(path, content, extension) -> (Vec<SymbolInfo>, Vec<ImportInfo>) {
    // Own tree-sitter dispatch — NOT using ParserRegistry
    let language_fn = match extension { ... };
    let mut parser = Parser::new();
    parser.set_language(&language);
    let tree = parser.parse(content, None);
    // Own extraction: extract_ts_symbols / extract_rust_symbols
    (symbols, imports)
}
```

**Target:**

```rust
#[deprecated(note = "use ParserRegistry::parse_file or engine::commands::* instead")]
pub fn parse_file(path, content, extension) -> (Vec<SymbolInfo>, Vec<ImportInfo>) {
    let result = ParserRegistry::default().parse_file(path, content, extension, "");
    (result.symbols, result.imports)
}
```

**Mismatch 5 — Deprecation note references wrong method name:**
The design and spec say the note should mention `ParserRegistry::parse_file_all` but the actual method on `ParserRegistry` is named `parse_file` (line 31 of `registry.rs`). The `parse_file_all` name lives on `CodeParser`, not on `ParserRegistry`.
**Fix:** Change deprecation note to say `ParserRegistry::parse_file` or `engine::commands::*`.

**Key concern:** The legacy `parse_file` has its own tree-sitter dispatch and extraction logic (extract_ts_symbols, extract_rust_symbols) that the ParserRegistry path may produce subtly different results for. The design says a test should assert field-by-field equality between the shim and direct registry paths (design line 219). This test is critical.

**Test needed:** `code_parser::tests::shim_legacy_parse_file_matches_registry_symbols_and_imports` — parse a TS fixture via both `CodeParser::parse_file` (new shim) and `ParserRegistry::parse_file` (direct) and assert symbols/imports are identical. This should be added to `code_parser.rs`'s test module.

**Private methods to keep:** `extract_ts_symbols`, `extract_rust_symbols`, `ts_symbol_kind`, `ts_declaration_name` — these stay (marked `#[allow(dead_code)]` or kept accessible) until the follow-up change removes them.

### C.4 — Author guide docs

**Target file:** `docs/code-intelligence/adding-a-language.md`

**Content per tasks.md:**

- Checklist
- Python stub example (from `engine/src/scanner/parser/python_stub.rs`)
- Per-language gotchas
- Link from `README.md` Documentation section

**Actual Python stub location:** `engine/src/scanner/parser/python_stub.rs` — returns `ParseResult::default()`, implements `LanguageParser` for `.py` files. This is the reference example for the guide.

**README.md modification:** Add link under Documentation section.

## Validation Plan

### Per-task validation

| Task | RED Command                                                                                                   | GREEN Command                                                                 | Notes                                                                    |
| ---- | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| C.1  | `cd engine && cargo test commands::tests::scan_files_calls_registry_exactly_n_times` (FAIL: module not found) | Same command PASSES                                                           | Create test module first                                                 |
| C.2  | `cd src-tauri && cargo test` (FAIL until engine tests are green)                                              | `cd src-tauri && cargo test` green                                            | Also `cargo check` clean                                                 |
| C.3  | `cd engine && cargo build` (warnings appear at legacy call sites)                                             | Warnings appear; `cargo clippy -- -D warnings` in src-tauri/ does NOT regress | The warnings ARE the "green" — this is a deprecation, not a test failure |
| C.4  | N/A (docs)                                                                                                    | `README.md` link resolves; doc renders                                        | Manual verification                                                      |

### Gate commands (before C.x merge)

```bash
cd engine && cargo test                          # All engine tests
cd engine && cargo clippy -- -D warnings         # No lint regressions
cd src-tauri && cargo test                       # Tauri integration tests
cd src-tauri && cargo clippy -- -D warnings      # No lint regressions
npm run test                                      # Frontend vitest
npm run lint                                      # Frontend eslint
npm run typecheck                                 # TypeScript check
```

Note: `npm run build` is only needed as a final gate after all PRs merge, not per-PR.

### Specific test inventory for C.1–C.3

| Test                                                     | Module                                   | What it verifies                                                             |
| -------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------- |
| `scan_files_calls_registry_exactly_n_times`              | `engine::commands::tests`                | Mock registry counts `parse_file` calls == N files                           |
| `outline_for_file_calls_registry_exactly_once`           | `engine::commands::tests`                | Mock registry counts 1 call                                                  |
| `scan_files_threads_lexical_kind_and_references_through` | `engine::commands::tests`                | ParseResult fields flow through ScanFilesOutput                              |
| `shim_legacy_parse_file_matches_registry`                | `engine::scanner::code_parser::tests`    | `parse_file` shim == direct `ParserRegistry::parse_file` for symbols/imports |
| `scan_project_single_parse_per_file`                     | `src-tauri::commands::tests::shim_tests` | Integration: scan_project uses engine::commands, NOT legacy parse_file       |
| `get_node_outline_uses_outline_for_file`                 | `src-tauri::commands::tests::shim_tests` | Integration: outline comes from engine::commands::outline_for_file           |

## Risks

| Risk                                                                                                                                   | Likelihood | Severity | Mitigation                                                                                                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `scan_project` rewrite changes persistence order                                                                                       | Medium     | High     | Keep persistence block byte-identical; only the parse loop moves to `engine::commands`. The `file_infos`, `all_imports`, and outlines should be produced in the same order.                       |
| `CodeParser::parse_file` shim returns different symbols/imports than legacy                                                            | Low        | High     | Add field-by-field equality test comparing shim vs direct registry output for a known TS fixture.                                                                                                 |
| `DiscoveredFile` path resolution: the consolidated function needs absolute paths to read files but `FileInfo.path` uses relative paths | Low        | Medium   | `DiscoveredFile` already has both `path` (absolute) and `relative_path`. Use `path` for reading, `relative_path` for `FileInfo.path`.                                                             |
| Extending `ScanFilesOutput` to include outlines changes the design-surface API                                                         | Low        | Low      | The design is ambiguous about whether `scan_files` or `outline_for_file` produces outlines. Clarify during implementation: include outlines in `ScanFilesOutput` for true single-parse-per-file.  |
| Mock `ParserRegistry` for C.1 tests needs `new()` with injected parsers                                                                | Low        | Low      | Add a test-only constructor or use `register()` on a default instance with a `CountingParser`.                                                                                                    |
| `Deprecated` attribute on `parse_file` causes build warnings in CI when `-D warnings` is used                                          | Medium     | Medium   | After C.2 removes the Tauri call sites of `parse_file`, the only remaining callers are test code. Allow dead_code/deprecated in test modules or add `#[allow(deprecated)]` on the test call site. |

## PR Boundary Note

**Critical:** PR-C depends on PR-A and PR-B being complete and working. The current worktree state (`feat/parser-ir-pr-b`) has uncommitted PR-B changes.

**Before starting PR-C work:**

1. Commit or stash all PR-B changes
2. Verify `cargo test` passes on both `engine/` and `src-tauri/` on `feat/parser-ir-pr-b`
3. Create a new branch `feat/parser-ir-pr-c` from `feat/parser-ir-pr-b` (stacked-to-main strategy)

**Why this matters:**

- C.1's `scan_files` calls `registry.parse_file(...)` which now returns IR with `lexical_kind` and `references` — these fields must exist from PR-A
- C.2 replaces `CodeParser::parse_file` calls that in PR-B already demonstrate the 3-loop structure
- C.3 deprecates `parse_file` which is still called in the Tauri commands — C.2 must land first (or simultaneously) to avoid warnings

**Stacked-to-main sequence:**

```
main ← feat/parser-ir-pr-a (merged first)
    ← feat/parser-ir-pr-b (merged second, on top of pr-a)
        ← feat/parser-ir-pr-c (merged third, on top of pr-b)
```

If PR-B is not yet merged to main, PR-C can be developed on a branch based on PR-B. The merge order is enforced: C depends on A+B.

## Skill Resolution

| Field      | Value                                                                                                                                                                                                                                                                                |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Resolution | `none`                                                                                                                                                                                                                                                                               |
| Notes      | No project skills matched. The `.atl/skill-registry.md` was not found during scout. The generic SDD phase artifacts (`tasks.md`, `design.md`, `spec.md`) provided all needed context. Strict TDD mode confirmed from `openspec/config.yaml` (runners: `cargo test`, `npm run test`). |
