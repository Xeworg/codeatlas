# Apply Progress — PR-C: Dispatch Consolidation

> Retroactive TDD evidence for PR-C tasks C.1–C.4.
> The commits landed as part of `ff372bd` (merged PR-C). Evidence is reconstructed from the final working state and the RED tests that ship with the code.

## TDD Cycle Evidence

### C.1 — `engine::commands` pure functions

| Phase | Command                                                         | Expected                                                                         | Actual                             |
| ----- | --------------------------------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------- |
| RED   | `cd engine && cargo test commands::tests::`                     | `scan_files_calls_registry_exactly_n_times` fails with "0 calls" (no real files) | Module did not exist before commit |
| GREEN | after `engine/src/commands.rs` + `engine/src/commands/tests.rs` | All 4 `commands::tests::` pass                                                   | ✅ 4/4 pass                        |

**Files changed**: `engine/src/commands.rs` (new), `engine/src/commands/tests.rs` (new)

### C.2 — Tauri shims call `engine::commands`

| Phase | Command                                                                          | Expected                                                                               | Actual                                  |
| ----- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | --------------------------------------- |
| RED   | `cd src-tauri && cargo test shim_tests`                                          | `import_source_file_id_converts_relative_path_to_uuid` fails (no path→uuid conversion) | Shim module did not exist before commit |
| GREEN | after `src-tauri/src/commands.rs` + `src-tauri/src/commands/tests/shim_tests.rs` | All 3 shim_tests pass                                                                  | ✅ 3/3 pass                             |

**Files changed**: `src-tauri/src/commands.rs` (rewire), `src-tauri/src/commands/tests.rs` (new declaration), `src-tauri/src/commands/tests/shim_tests.rs` (new)

### C.3 — `CodeParser::parse_file` deprecation shim

| Phase | Command                                   | Expected                                                        | Actual                                                       |
| ----- | ----------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------ |
| RED   | `cd engine && cargo build 2>&1`           | Deprecation warnings on legacy call sites                       | Shim did not exist before commit                             |
| GREEN | after `engine/src/scanner/code_parser.rs` | Warnings appear; `shim_parity_symbols_and_imports_equal` passes | ✅ `cargo test shim_parity_symbols_and_imports_equal` passes |

**Files changed**: `engine/src/scanner/code_parser.rs`, `engine/src/lib.rs`

### C.4 — Author guide

| Phase | Command                                          | Expected                                                             | Actual                           |
| ----- | ------------------------------------------------ | -------------------------------------------------------------------- | -------------------------------- |
| RED   | `ls docs/code-intelligence/adding-a-language.md` | File does not exist                                                  | File did not exist before commit |
| GREEN | after guide creation                             | File exists, renders, and `README.md` link check (if README existed) | ✅ 237-line guide written        |

**Files changed**: `docs/code-intelligence/adding-a-language.md` (new)

## Commit History

```
2730f22 refactor(engine): consolidate parser dispatch through registry
    engine/src/commands.rs
    engine/src/commands/tests.rs
    engine/src/lib.rs
    src-tauri/src/commands.rs
    src-tauri/src/commands/tests.rs
    src-tauri/src/commands/tests/shim_tests.rs
    docs/code-intelligence/adding-a-language.md

6151a1f test(engine): make commands fixtures self-contained
    engine/src/commands/tests.rs  (post-merge fix: temp fixture creation)
```

## Verification Gates

| Gate              | Command                                             | Result |
| ----------------- | --------------------------------------------------- | ------ |
| Engine unit tests | `cd engine && cargo test`                           | ✅     |
| Tauri tests       | `cd src-tauri && cargo test`                        | ✅     |
| Engine clippy     | `cd engine && cargo clippy --lib -- -D warnings`    | ✅     |
| Tauri clippy      | `cd src-tauri && cargo clippy --lib -- -D warnings` | ✅     |
| Engine fmt        | `cd engine && cargo fmt --check`                    | ✅     |
| Tauri fmt         | `cd src-tauri && cargo fmt --check`                 | ✅     |
