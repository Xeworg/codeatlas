# PR-C Fresh-Context Review

**Branch:** `feat/parser-ir-pr-c`
**Date:** 2026-06-04
**Reviewer:** subagent-reviewer (fresh context, no parent assumptions)

## Status

**BLOCKED** — one critical regression in import persistence (`source_file_id` mismatch) that breaks graph edges. All other changes are correct.

## Executive Summary

The PR-C changes move `scan_project` and `get_node_outline` to a pure engine orchestration layer (`engine/src/commands.rs`) with a `ParseFile` trait-based single-dispatch pattern. The deprecated `CodeParser::parse_file` shim delegates to the registry and a parity test verifies equivalence. A comprehensive language-adding guide is provided. All tests pass and both crates compile clean. However, a critical bug was introduced in import persistence: `ImportInfo.source_file_id` is now the file's relative path instead of the file UUID, causing `get_imports` to return zero imports and graph edges to be empty.

---

## Confirmed Good

### 1. Pure orchestration layer (`engine/src/commands.rs`)

- `scan_files` and `outline_for_file` are well-designed pure functions with no Tauri/DB/tracing dependencies. ✅
- `ParseFile` trait abstracts the registry dispatch, enabling test injection via `TrackingRegistry`. ✅
- `ScanFilesOutput` carries all metadata (`file_infos`, `all_imports`, `parse_ms`, `registry_call_count`, `files_failed`, `files_read`). ✅
- Single-dispatch contract is enforced: each call path invokes the registry exactly once per file. ✅

### 2. Tests (`engine/src/commands/tests.rs`)

- `scan_files_calls_registry_exactly_n_times` — verifies registry called N times for N files. ✅
- `scan_files_propagates_symbols_and_imports` — verifies output shape. ✅
- `outline_for_file_calls_registry_exactly_once` — verifies single dispatch for outline. ✅
- `commands_module_types_exist` — compile-time type existence check. ✅
- All 4 commands tests pass. ✅

### 3. Deprecated shim (`engine/src/scanner/code_parser.rs`)

- `CodeParser::parse_file` now delegates to `ParserRegistry::default().parse_file(..., "")` instead of duplicating tree-sitter logic. ✅
- `#[deprecated]` attribute with clear migration path. ✅
- `#[allow(deprecated)]` on test call sites so tests pass without warnings. ✅
- `#[allow(dead_code)]` on orphaned helper methods (`extract_ts_symbols`, `ts_symbol_kind`, `ts_declaration_name`, `extract_rust_symbols`) since they are only used in old code paths. ✅
- New `shim_parity_symbols_and_imports_equal` test validates that `parse_file` and `parse_file_all` produce the same symbols/imports from the same registry-backed `ParseResult`. ✅

### 4. Tauri commands (`src-tauri/src/commands.rs`)

- `scan_project` Phase 1+2 consolidated into single `scan_files` call — eliminates the old per-file `CodeParser::parse_file` duplication. ✅
- `get_node_outline` switched from `CodeParser::parse_file_all` to `outline_for_file`. ✅
- Imports are resolved using `imp.source_file_id` (now correctly the relative path, used for path resolution) and `path_to_id` lookup for `target_file_id`. ✅
- Outline persistence uses `outline_for_file` correctly with DB UUID as `file_id`. ✅
- `imports_count` correctly set to `persisted_count` after import persistence loop. ✅

### 5. Documentation (`docs/code-intelligence/adding-a-language.md`)

- Comprehensive guide covering trait implementation, registry registration, fixtures, verification, and gotchas. ✅
- Correctly references the IR contract and relevant files. ✅
- Includes important gotcha about `source_file_id` needing to be relative_path for cross-file resolution in Phase 2. ✅

### 6. Module wiring (`engine/src/lib.rs`)

- Clean addition of `pub mod commands;`. ✅

### 7. Compilation & tests

- `cargo check --lib` (engine): ✅ clean
- `cargo check` (src-tauri): ✅ clean
- `cargo clippy --lib` (engine): ✅ clean (no warnings)
- `cargo test --lib` (engine): ✅ 159 passed, 0 failed
- `cargo test` (src-tauri): ✅ 28 passed, 0 failed

---

## Blocker

### B1: `ImportInfo.source_file_id` changed from UUID to relative_path — breaks `get_imports` and graph edges

**Location:**

- `src-tauri/src/commands.rs` lines 72–84 (new import resolution loop)
- `engine/src/scanner/parser/typescript.rs:521` — `let source_file_id = file_id.to_string();`
- `engine/src/scanner/parser/rust.rs:178` — `let source_file_id = file_id.to_string();`
- `engine/src/commands.rs:89` — `registry.parse_file(&file.path, &source, &file.extension, &file.relative_path)`

**Evidence chain:**

1. `scan_files` calls `registry.parse_file(..., &file.relative_path)` — passing **relative path** as `file_id`.
2. TypeScript parser (line 521) and Rust parser (line 178) use `file_id` directly as `ImportInfo.source_file_id` — resulting in `"src/service.ts"` instead of a UUID.
3. The new import resolution loop (line 72–84) resolves `target_file_id` via `path_to_id` lookup, but **never converts `source_file_id` from relative_path to UUID**.
4. The old code explicitly did this conversion:
   ```rust
   // OLD (removed in this diff):
   let source_id = path_to_id.get(&file.relative_path).cloned().unwrap_or_default();
   imp.source_file_id = source_id.clone(); // UUID
   ```
5. `save_import(imp)` writes `source_file_id = "src/service.ts"` to the `imports` table.
6. `get_imports(project_id)` executes:
   ```sql
   SELECT ... FROM imports WHERE source_file_id IN
       (SELECT id FROM files WHERE project_id = ?1)
   ```
   `files.id` is always a UUID → `source_file_id = "src/service.ts"` never matches → **returns 0 imports**.
7. `get_graph_data` calls `repo.get_imports(project_id)` → receives empty vec → `GraphBuilder::build` produces **a graph with no edges**.

**Impact:** The dependency graph in the frontend will have no edges between files. All resolved imports are silently lost after a scan.

**Fix:** After the import resolution loop (line ~84), or within it, transform `imp.source_file_id` from relative_path to UUID using `path_to_id`. For example:

```rust
// After resolving target, fix source:
if let Some(src_id) = path_to_id.get(&imp.source_file_id) {
    imp.source_file_id = src_id.clone();
}
```

**Severity:** Blocker — must be fixed before PR-C is functionally ready.

---

## Notes

### N1: Fragile test fixtures — hardcoded `/tmp` paths

**Location:** `engine/src/commands/tests.rs` lines 29–45, 105–117

The tests reference hardcoded paths like `/tmp/engine_cmd_test/a.ts` without creating those files. `scan_files` reads files from disk via `std::fs::read_to_string`. These files happen to exist on the current machine (created at 10:28 by a previous run), but tests will fail on a clean system.

**Recommendation:** Use `tempfile::TempDir` (already a dependency via walker tests) to create temporary files with known content before invoking `scan_files`.

### N2: Misleading "single-parse-per-file across all 3 phases" comment

**Location:** `src-tauri/src/commands.rs` lines 165–168

The comment claims "the full scan achieves single-parse-per-file across all 3 phases." In reality, Phase 3 creates a fresh `ParserRegistry` and calls `outline_for_file` — which invokes `parse_file` — so each file is parsed **twice** (once in `scan_files`, once in `outline_for_file`). This is pre-existing behavior (the old code called `parse_file` in Phase 1–2 and `parse_file_all` in Phase 3), so it is not a regression. Consider updating the comment for accuracy.

### N3: Trivial warnings from test compilation

- `commands/tests.rs:141`: field `inner` never read in `CountingRegistry` — the mock intentionally uses `calls` counter only. Could add `#[allow(dead_code)]`.
- `commands/tests.rs:127`: `output.parse_ms >= 0` is always true (`u64`). Consider `assert!(output.parse_ms >= 0)` → no-op, harmless.

### N4: Scope — docs addition is safe

`docs/code-intelligence/adding-a-language.md` is a standalone guide. It describes the post-PR-C architecture correctly and does not reference implementation details that might change. No risk of docs-code drift within PR-C scope.

---

## PR Boundary Assessment

| Criterion                       | Assessment                                                             |
| ------------------------------- | ---------------------------------------------------------------------- |
| **Implements PR-C intent**      | ✅ `scan_project` and `get_node_outline` moved to pure engine dispatch |
| **Single-dispatch contract**    | ✅ Each call path uses registry exactly once per file                  |
| **Deprecated shim correctness** | ✅ `parse_file` delegates to registry; parity test passes              |
| **Persistence regression**      | ❌ `source_file_id` mismatch breaks graph edges (Blocker B1)           |
| **Resolver behavior preserved** | ✅ `target_file_id` resolution uses same `PathResolver` logic          |
| **Scope overreach**             | ✅ No unexpected changes; docs addition is bounded and relevant        |
| **Test coverage**               | ✅ 4 new tests + 1 parity test; all existing tests pass                |
| **Compilation**                 | ✅ Both crates compile clean; clippy passes                            |

---

## Recommended Next Step

1. **Fix Blocker B1**: Add `source_file_id` → UUID conversion in the import resolution loop of `scan_project`.
2. **Fix Note N1** (optional, but recommended for CI): Make tests use `tempfile` instead of hardcoded `/tmp` paths.
3. **Fix Note N2** (optional): Update the misleading comment.
4. After fixes, run the full test suite and verify graph edges appear in a manual scan.

---

## Residual Risks

| Risk                                                                                                                     | Likelihood | Impact                                                       |
| ------------------------------------------------------------------------------------------------------------------------ | ---------- | ------------------------------------------------------------ |
| Additional callers of `CodeParser::parse_file` outside the repo (e.g., external plugins) may not see deprecation warning | Low        | Low — no known external consumers                            |
| `file_id` semantic conflict (relative_path vs UUID) may reappear in future parser additions                              | Medium     | Low — documentation in `adding-a-language.md` addresses this |
| Phase 3 double-parse may cause performance regressions on large projects                                                 | Low        | Medium — pre-existing, but worth tracking as tech debt       |

---

## Skill Resolution

- `paths-injected`: No project/user skills were injected by the parent (review-only task).
- `fallback-registry`: Not applicable.
- `none`: No project/user skills loaded.
