# Apply Progress — outline-parser-abstraction

## Current slice

PR4 — Semantic AI Context

## Status

Completed with PR1+PR2+PR3+PR4 applied and fresh-reviewed. Reviewer verdict: `ACCEPTABLE_FOR_VERIFY: YES`.

## RED / GREEN evidence

### RED

- PR1 introduced parser/model tests before semantic parser foundation existed.
- PR2 introduced outline persistence expectations; fresh review later caught missing DB tests and a lost `#[test]` annotation.
- PR3 introduced outline UI component tests before `OutlineView` and `DetailPanel` integration existed.
- PR4 introduced 5 semantic-context tests before outline-aware context construction existed.

### GREEN

Commands executed during/after slices:

```bash
cd engine && cargo fmt --check && cargo clippy -- -D warnings && cargo test
npm run typecheck
npm run lint
npm run test -- src/components/panel/OutlineView.test.tsx
```

Latest verified results:

- `engine`: `cargo fmt --check` clean, `cargo clippy -- -D warnings` clean, `cargo test` 103 passed.
- Frontend: `npm run typecheck` clean, `npm run lint` clean.
- `OutlineView.test.tsx`: 8 passed.
- Full `npm run test`: 210 passed with 10 pre-existing Tauri-integration failures caused by missing Tauri runtime in Vitest.
- `src-tauri` formatting was applied after PR4 review.

## Implemented slices

### PR1 — Semantic Parser Foundation

- Added Rust semantic parser models:
  - `OutlineItemKind`
  - `OutlineItem`
  - `ParseResult`
- Added stable outline id pattern:
  - `outline:<file_id>:<kind>:<line_start>:<line_end>:<name>`
- Introduced parser abstraction:
  - `LanguageParser`
  - `ParserRegistry`
  - `TypeScriptParser`
  - `RustParser`
- Pre-populated `ParserRegistry::default()` with TypeScript and Rust parsers.
- Preserved `CodeParser::parse_file()` compatibility.
- Added `CodeParser::parse_file_all(...) -> ParseResult`.

### PR2 — Outline Persistence + Tauri API

- Added migration `engine/migrations/007_outline_items.sql`.
- Updated migration registry to schema version 7.
- Added `ProjectRepository::save_outline_items` and `get_outline_items`.
- Added 4 DB persistence tests:
  - save/retrieve hierarchy;
  - empty unknown file;
  - replace on resave;
  - cascade delete.
- Restored `#[test]` on `snapshot_diff_same_snapshot_zero_delta` after review caught regression.
- Integrated outline persistence into `scan_project` using authoritative file UUIDs.
- Added `get_node_outline` command and Tauri registration.
- Added `OutlineItemKind`/`OutlineItem` TS contracts and `getNodeOutline` wrapper.

### PR3 — Outline UI Panel

- Added `src/components/panel/OutlineView.tsx` with recursive tree rendering.
- Integrated outline loading into `DetailPanel.tsx` with independent loading/error/empty state.
- Kept graph node cards compact.
- Added `OutlineView.test.tsx` with 8 tests, including collapse/expand behavior.

### PR4 — Semantic AI Context

- Extended `engine/src/ai/context.rs` with outline-aware semantic context.
- Added bounded outline rendering with hard item limit and final byte cap.
- Added line-range extraction helper.
- Kept `build_node_context()` as fallback.
- Integrated `explain_node` to load outline from DB and use semantic context only when available.
- Preserved fallback to existing source-truncation behavior when outline is empty or unavailable.
- Did not change project chat/global search.

## Changed files of interest

- `docs/PLAN_OUTLINE_TREE_SITTER_Y_PARSERS.md`
- `openspec/changes/outline-parser-abstraction/*`
- `engine/src/models/file.rs`
- `engine/src/scanner/code_parser.rs`
- `engine/src/scanner/parser/*`
- `engine/migrations/007_outline_items.sql`
- `engine/src/db/migrations.rs`
- `engine/src/db/queries.rs`
- `engine/src/ai/context.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src/lib/types.ts`
- `src/lib/tauri-api.ts`
- `src/components/panel/DetailPanel.tsx`
- `src/components/panel/OutlineView.tsx`
- `src/components/panel/OutlineView.test.tsx`

## Risks / notes

- `CodeParser::parse_file()` still uses the legacy inline path while `parse_file_all()` uses the registry, so scan can parse files twice. This is intentional deferral and should be optimized later.
- `build_node_context` and `build_node_context_with_outline` share dependency/dependent collection logic. A follow-up refactor could extract private helpers.
- Full frontend test suite still has pre-existing Tauri runtime failures unrelated to outline-parser-abstraction.

## Next recommended

Run SDD verify for `outline-parser-abstraction` against proposal/spec/design/tasks and recorded test evidence.
