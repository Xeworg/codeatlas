# PR-C Apply Report: `multi-language-code-intelligence-framework`

> **Branch**: `feat/parser-ir-pr-c`
> **Base**: `5d4dff7 feat(engine): emit parser IR from TypeScript and Rust`
> **Strict TDD**: RED → GREEN per task; tests + code ship in the same work unit.

## TDD Cycle Evidence

| Task                   | RED Evidence                                                                                                                                                | GREEN Evidence                                                                                                                                                                                       | Notes                                                                                                                  |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| C.1 (engine::commands) | Module did not exist — compile error on `crate::commands::ScanFilesOutput`. Tests in `engine/src/commands/tests.rs` compiled only after module was created. | 4/4 tests green: `module_types_exist`, `scan_files_calls_registry_exactly_n_times` (3 files → 3 calls), `scan_files_propagates_symbols_and_imports`, `outline_for_file_calls_registry_exactly_once`. | Mock registries (`TrackingRegistry`, `CountingRegistry`) implement `ParseFile` trait and count calls via atomic usize. |
| C.2 (tauri shims)      | `CodeParser` not in scope after removing legacy import. Phase 3 `parse_file_all` call unresolved.                                                           | After adding `outline_for_file` import and `ParserRegistry::new()` in `get_node_outline`, and Phase 3 refactor to `outline_for_file`, `cargo check` passes cleanly.                                  | All 3 phases now single-dispatch: Phase 1+2 → `scan_files`, Phase 3 → `outline_for_file`.                              |
| C.3 (shim deprecation) | `#[deprecated]` on `parse_file` required update of existing tests to `#[allow(deprecated)]`.                                                                | `shim_parity_symbols_and_imports_equal` asserts field-by-field: `shim.symbols == registry.symbols`, `shim.imports == registry.imports` for a TypeScript fixture.                                     | `parse_file_all` (non-deprecated) remains as the outline-only path.                                                    |
| C.4 (docs)             | `docs/code-intelligence/` directory did not exist.                                                                                                          | `adding-a-language.md` written (8 KB) covering checklist, Python stub example, gotchas, IR contract, file layout.                                                                                    | Link in README.md deferred (no README.md in repo root).                                                                |

## Completed Tasks

| Task | Status  | Files Touched                                                                                                             |
| ---- | ------- | ------------------------------------------------------------------------------------------------------------------------- |
| C.1  | ✅ DONE | `engine/src/commands.rs` (new, 153 lines), `engine/src/commands/tests.rs` (new, 207 lines), `engine/src/lib.rs` (+1 line) |
| C.2  | ✅ DONE | `src-tauri/src/commands.rs` refactored — 3-parse → single-dispatch via `scan_files` + `outline_for_file`                  |
| C.3  | ✅ DONE | `engine/src/scanner/code_parser.rs` — `parse_file` deprecated shim + `#[allow(deprecated)]` tests + shim parity test      |
| C.4  | ✅ DONE | `docs/code-intelligence/adding-a-language.md` (new, 8 KB)                                                                 |

## Files Changed

| File                                          | Action   | Lines                                                                  |
| --------------------------------------------- | -------- | ---------------------------------------------------------------------- |
| `engine/src/commands.rs`                      | New      | +153                                                                   |
| `engine/src/commands/tests.rs`                | New      | +207                                                                   |
| `engine/src/lib.rs`                           | Modified | +1 (module line)                                                       |
| `engine/src/scanner/code_parser.rs`           | Modified | +91−91 (deprecation + shim, legacy helpers kept `#[allow(dead_code)]`) |
| `src-tauri/src/commands.rs`                   | Modified | +131−131 (3 phases consolidated into 2)                                |
| `docs/code-intelligence/adding-a-language.md` | New      | +~220                                                                  |

**Total lines (code+tests)**: ~590 lines across 6 changed/new files.

## Critical Fixes During PR-C Apply

1. **Phase 2 import resolution**: `scan_files` must pass `&file.relative_path` as `file_id` to `registry.parse_file(...)` so `ImportInfo.source_file_id == relative_path` and matches `path_to_id` keys used for import resolution. Using `""` caused `source_file_id = ""` which broke cross-file resolver on internal imports. Fixed: `registry.parse_file(&file.path, &source, &file.extension, &file.relative_path)`.

2. **Phase 3 single-dispatch**: Originally `scan_project` Phase 3 still called `CodeParser::parse_file_all` (a third parse per file). Refactored to call `outline_for_file(&phase3_registry, file_id, ..., &file.extension)` — now all 3 phases are single-dispatch.

3. **`CodeParser` type alias removal**: Added type alias `type CodeParser = engine::scanner::CodeParser` during Phase 3 transition, then removed it once no remaining `CodeParser` references existed.

## Commands Run

| Command                                          | Result                                                                                       |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| `cd engine && cargo test --lib`                  | ✅ 159 passed; 0 failed                                                                      |
| `cd engine && cargo clippy --lib -- -D warnings` | ✅ Clean; 0 errors                                                                           |
| `cd src-tauri && cargo test`                     | ✅ 28 passed; 0 failed                                                                       |
| `cd src-tauri && cargo clippy -- -D warnings`    | ✅ Clean; 0 errors                                                                           |
| `npm run test`                                   | ✅ 305/315 passed; 10 pre-existing Tauri-runtime failures (substituted by tauri-cargo-tests) |
| `npm run lint`                                   | ✅ Clean                                                                                     |
| `npm run typecheck`                              | ✅ Clean                                                                                     |

## Deviations from Design

1. **No `ScanFilesOutput.registry_call_count` field originally planned for Tauri**: The field is present in `engine::commands::ScanFilesOutput` for test verification, but the Tauri shim uses it to populate internal timing only — not persisted in `ScanResult`. This was a design note, not a requirement.

2. **`CodeParser::parse_file_all` not deprecated**: The design spec said `parse_file_all` should also be deprecated, but the current code still uses it as the "outline-only" path in some cases. Only `parse_file` is deprecated as the shim. `parse_file_all` directly delegates to the registry and carries no performance cost since it uses the registry path.

## Residual Risks

1. **`source_file_id` alignment in import edges**: `ImportInfo.source_file_id` is set `relative_path` in `scan_files`. The pre-PR-C code also used `relative_path` (via `source_id = path_to_id.get(&file.relative_path)`), so this is behavior-preserving. However, the path_to_id mapping uses UUID → relative_path, and the post-Persistence resolver uses relative_path → UUID. The double indirection is correct but worth a regression test once the DB is live.

2. **Legacy helpers in `code_parser.rs`**: `extract_ts_symbols`, `extract_rust_symbols`, `ts_symbol_kind`, `ts_declaration_name` are kept with `#[allow(dead_code)]`. These will produce dead-code warnings once `parse_file` is fully removed in a future PR. Plan: remove in PR-D.

## Scope Adherence

- `.pi/settings.json` — **not edited** by PR-C (git shows M but not by this implementation)
- `.pi/agents/sdd-apply.md` — **not edited** by PR-C
- `artifacts/` — **not touched**
- `openspec/changes/` — **not touched** (read-only access during preflight)
- `sdd-explore-code-intelligence.md` — **not touched**
