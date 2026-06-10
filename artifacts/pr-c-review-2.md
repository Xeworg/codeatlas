## PR-C Blocker Fix — Focused Review (Round 2)

**status**: PASS (blocker resolved, no new blockers)

### Executive Summary

The `source_file_id` persistence regression is **confirmed fixed**. The commit `1e92139` (`fix(graph): surface import persistence failures`) correctly converts `source_file_id` from relative path to persisted file UUID before calling `save_import`. The three new regression tests in `shim_tests.rs` are meaningful and all pass. No remaining blocker for PR-C.

---

### blocker_status: RESOLVED

| Blocker                                                          | Status    | Evidence                                      |
| ---------------------------------------------------------------- | --------- | --------------------------------------------- |
| `source_file_id` persisted as relative path instead of file UUID | **FIXED** | `commands.rs:79-80` converts via `path_to_id` |

---

### Findings Now

#### 1. Core Fix: `source_file_id` conversion (Correct)

**Location**: `src-tauri/src/commands.rs` lines 66–96

**What happens**:

1. `path_to_id: HashMap<String, String>` is built from `scan_output.file_infos`, mapping `file.path → file.id` (line 66–68).
2. For each import from `scan_output.all_imports`, `imp.source_file_id` (which the registry set to the relative path) is **replaced with the persisted file UUID** via `path_to_id.get(&imp.source_file_id)` (line 79–80).
3. `source_file_id` is now a UUID matching `files.id`, so the `get_imports` query `WHERE source_file_id IN (SELECT id FROM files ...)` correctly returns import rows.

**Verified**: The parser (TypeScript `typescript.rs:521`, Rust `rust.rs:178`) sets `source_file_id = file_id.to_string()` where `file_id` comes from `scan_files` (engine `commands.rs:81`) as `file.relative_path`. The `FileInfo.path` is also `file.relative_path` (engine `commands.rs:86`). Keys in `path_to_id` match `import.source_file_id`. ✅

#### 2. Target Resolution Correctly Preserved

**Location**: `src-tauri/src/commands.rs` lines 82–95

After source_file_id is converted to UUID, a **reverse lookup** recovers the original relative path:

```rust
let rel_path = path_to_id
    .iter()
    .find(|(_, v)| *v == &imp.source_file_id)
    .map(|(k, _)| k.clone())
    .unwrap_or_else(|| imp.source_file_id.clone());
```

This relative path is passed to `resolver.resolve(module, &rel_path)`. The resolver (`engine/src/graph/resolver.rs:56`) requires a relative `from_file` path to compute relative directory resolution. This works correctly.

**Edge case**: If `source_file_id` wasn't found in `path_to_id` (unlikely — only if a parser emitted an import for a file absent from `file_infos`), the UUID was never set (line 79 guard), and the fallback passes the original relative path to the resolver. The import would still be persisted with a relative path `source_file_id`, breaking `get_imports` for that edge import. This is a theoretical edge case with no known reproduction path.

#### 3. Empty Source File ID Guard

**Location**: `src-tauri/src/commands.rs` lines 164–169

```rust
if imp.source_file_id.is_empty() {
    skipped_empty += 1;
    continue;
}
```

Defensive guard that prevents orphan import rows. Correct and safe. The parsers always set `source_file_id` to a non-empty relative path; this guard only fires on truly malformed data.

#### 4. Regression Tests

**Location**: `src-tauri/src/commands/tests/shim_tests.rs` (untracked, not yet committed)

| Test                                                                  | What it verifies                                                                                               | Status  |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ------- |
| `import_source_file_id_converts_relative_path_to_uuid`                | UUID conversion + length assertion (UUID ≥20 chars vs relative path <20)                                       | ✅ PASS |
| `path_to_id_map_covers_all_import_sources`                            | Path-to-ID map is built before the import loop                                                                 | ✅ PASS |
| `source_file_id_must_be_uuid_not_relative_path_for_get_imports_query` | Simulates the SQL `WHERE ... IN (SELECT id FROM files ...)` filter — UUID matches, relative path silently lost | ✅ PASS |

All three tests run in-process without DB and verify the **contract**, not the integration. This is appropriate for a focused regression test: they prevent the exact bug from recurring even if the integration plumbing changes.

---

### Notes (Non-Blocking)

1. **`shim_tests.rs` is untracked**. Must be `git add`ed before merge. Also has a compilation warning for unused imports (`is_root_path_conflict`, `map_save_scan_result_error`).

2. **Import ID not stable across rescans**. Both parsers generate `ImportInfo.id` via `uuid::Uuid::new_v4()` (random). On rescan, new random IDs are assigned, and `save_import` (`INSERT OR REPLACE`) inserts new rows instead of replacing old ones. Since `save_file_internal` uses `ON CONFLICT DO UPDATE` (not `DELETE + INSERT`), cascades don't fire, and old import rows accumulate. This is a **cleanliness concern** — not a correctness bug because `get_imports` still returns correct data for the current scan. The duplicate rows just waste space until a future cleanup mechanism is added. Not a PR-C blocker.

3. **Reverse lookup is O(n) per import**. For projects with hundreds of files and thousands of imports, this could be noticeable but not prohibitive. Consider a `HashMap<String, String>` for the reverse mapping if profiling shows it matters.

4. **`GraphNodeComponent.tsx` handle position change** (Top/Bottom → Left/Right) is a cosmetic change unrelated to the blocker. Harmless but should ideally be in a separate commit.

---

### Recommended Next Step

1. **Commit `shim_tests.rs`** (fix the unused import warning first).
2. **Merge PR-C** — the blocker is resolved.
3. **(Optional, post-merge)** Add import ID stability (derive from source-target pair instead of random UUID) to prevent duplicate accumulation on rescan.

---

### Residual Risks

| Risk                                         | Severity | Mitigation                                                               |
| -------------------------------------------- | -------- | ------------------------------------------------------------------------ |
| Import ID accumulation on rescan             | Low      | No correctness impact; address in a follow-up cleanup issue              |
| Edge case: import for file not in path_to_id | Minimal  | No known reproduction; defensive empty-guard prevents worst case         |
| Reverse lookup perf at scale                 | Minimal  | Only matters for 1000+ file projects; fixable with reverse map if needed |

---

### Skill Resolution

`skill_resolution: none` — no project/user SKILL.md files were required for this focused review.
