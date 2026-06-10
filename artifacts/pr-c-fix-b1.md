# PR-C Blocker Fix: B1 — Import Source File ID Regression

## Summary

Fixed the confirmed PR-C blocker: `ImportInfo.source_file_id` was left as a relative
path instead of being converted to the persisted file UUID before `save_import`,
causing `get_imports(project_id)` to return zero imports and graph edges to be empty.

## What Was Wrong

In `scan_project`, the import resolution loop consumed `all_imports` from
`scan_files` where each `ImportInfo.source_file_id` held the file's **relative_path**
(because `scan_files` passes `file.relative_path` as `file_id` to the registry so
the parser can use it for `ImportInfo.source_file_id`).

The resolver correctly used this relative path for `target_module` resolution, but
**never converted `source_file_id` from relative_path to UUID** before calling
`save_import`. The SQL query in `get_imports` filters by:

```sql
WHERE source_file_id IN (SELECT id FROM files WHERE project_id = ?1)
```

`files.id` is a UUID, but `source_file_id` was `"src/service.ts"` — a relative path
that never matches the UUID filter. All import edges silently disappear from the graph.

## Fix Applied

In `src-tauri/src/commands.rs`, import resolution loop (around line 72):

1. **Convert `source_file_id` from relative_path to persisted UUID** using `path_to_id`
   before any other processing. This is the critical fix.

2. **Reverse-lookup relative_path from UUID** for `PathResolver::resolve` — since the
   resolver needs the relative path (it resolves `module` against the source file's
   location), we look up the relative path from the UUID via `path_to_id.iter().find(...)`
   so the resolver still works correctly.

```rust
for mut imp in scan_output.all_imports {
    // Convert source: relative_path → persisted UUID
    if let Some(uuid) = path_to_id.get(&imp.source_file_id) {
        imp.source_file_id = uuid.clone();
    }
    if let Some(ref module) = imp.target_module {
        // Reverse-lookup: UUID → relative_path for resolver
        let rel_path = path_to_id
            .iter()
            .find(|(_, v)| *v == &imp.source_file_id)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| imp.source_file_id.clone());
        let res = resolver.resolve(module, &rel_path);
        // ... target resolution unchanged
    }
}
```

## Files Changed

| File                                         | Action   | Description                                                        |
| -------------------------------------------- | -------- | ------------------------------------------------------------------ |
| `src-tauri/src/commands.rs`                  | Modified | Added source_file_id UUID conversion + reverse-lookup for resolver |
| `src-tauri/src/commands/tests/shim_tests.rs` | New      | 3 regression tests covering the source_file_id contract            |
| `src-tauri/src/commands/tests.rs`            | Modified | Added `mod shim_tests;` declaration                                |

## Commands Run

| Command                                          | Result    | Summary              |
| ------------------------------------------------ | --------- | -------------------- |
| `cd src-tauri && cargo check`                    | ✅ passed | Clean compilation    |
| `cd src-tauri && cargo test --lib`               | ✅ passed | 31 passed; 0 failed  |
| `cd engine && cargo test --lib`                  | ✅ passed | 159 passed; 0 failed |
| `cd engine && cargo clippy --lib -- -D warnings` | ✅ passed | Clean                |
| `cd src-tauri && cargo clippy -- -D warnings`    | ✅ passed | Clean                |

## Tests Added

- `src-tauri/src/commands/tests/shim_tests.rs`
  - `import_source_file_id_converts_relative_path_to_uuid` — verifies conversion produces valid UUIDs
  - `path_to_id_map_covers_all_import_sources` — verifies lookup map has all file paths
  - `source_file_id_must_be_uuid_not_relative_path_for_get_imports_query` — regression proof that relative paths are silently lost in `get_imports` filter

## Residual Risks

| Risk                                                                                                                                              | Mitigation                                                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `path_to_id` reverse-lookup iterates all entries (O(n) per import)                                                                                | n is the number of files in the project; n is small (<1000 typical) so this is acceptable for v1. If perf becomes an issue, build a reverse map `id→path` in O(n) once.                                                 |
| Import with source_file_id not in `path_to_id` gets UUID from `get(&imp.source_file_id)` returning `None` — source_file_id stays as relative path | The `continue` path keeps behavior consistent with pre-PR-C (no broken query, just no edge). Pre-PR-C used `.unwrap_or_default()` which would insert empty UUID. The new code preserves the relative path in this case. |

## Notes

- The behavior was behavior-preserving for **target resolution** (PathResolver already
  received relative_path). Only **persistence** was broken.
- No changes to `.pi/`, `artifacts/`, or OpenSpec scratch files.
- Phase 3 outline loop in `scan_project` was already correct (uses `path_to_id.get(&file.relative_path)` to resolve `file_id` from the same lookup table).
- No architectural rewrite was needed; the fix is entirely within the existing
  import resolution loop in `scan_project`.
