# Next Session Plan: Logging First, Then Tree-sitter Adaptation

## Goal

Improve CodeAtlas diagnosability before making deeper parser changes, then adapt the TypeScript/TSX Tree-sitter integration so it recognizes more method-like constructs reliably.

## Why This Order

Recent failures were hard to diagnose because frontend errors rendered as `[object Object]` and backend scan failures were not surfaced with enough structured context. Before changing parser behavior again, add logging that makes scan/open-project failures, DB persistence errors, and parser misses visible.

## Phase 1 — Add Observability / Logger

### Objectives

- Avoid silent or opaque errors during project open, scanning, DB persistence, graph load, and parser extraction.
- Make backend failures visible with operation, project path/id, file path when relevant, and error detail.
- Make frontend API errors render readable messages instead of `[object Object]`.

### Candidate Work Items

1. **Frontend error normalization**
   - Add a shared helper that converts unknown thrown values into readable strings.
   - Handle `Error`, `{ message }`, `{ code, message }`, strings, and fallback JSON/string conversion.
   - Use it in `App.tsx`, graph/detail hooks, and other API catch paths that currently use `String(err)`.

2. **API error class or structured helper**
   - Convert `src/lib/tauri-api.ts` plain `{ code, message }` throws into an actual `Error` subclass, or expose a robust `getErrorMessage()` helper.
   - Keep the existing `ApiError` shape compatible with TypeScript types.

3. **Backend scan logging**
   - Add structured `tracing` logs around:
     - project scan start/end;
     - file discovery count;
     - parser result counts per language;
     - DB save failures;
     - import persistence failures;
     - outline persistence failures;
     - graph load failures.

4. **DB conflict logging**
   - Log project id, root path, status, and SQL error when `save_scan_result()` fails.
   - Specifically make `projects.root_path` uniqueness conflicts obvious.

5. **Optional parser miss logging**
   - Add debug-level logs for TypeScript/TSX node kinds encountered but not captured as symbols/outline items.
   - Keep this behind debug logging to avoid noisy normal runs.

### Acceptance Criteria

- Opening a project never renders `[object Object]`; it renders a readable message.
- If DB save fails, logs identify the failing operation and root path/project id.
- If graph load fails, UI message includes the actual backend error message.
- Logging does not spam normal release output at info level.

### Suggested Validation

```bash
npm test -- error-handling
npm run typecheck # or npx tsc --noEmit
cd src-tauri && cargo check
cd engine && cargo test save_scan_result
```

## Phase 2 — Improve Tree-sitter TypeScript/TSX Recognition

### Problem

The current TypeScript parser still relies on ad-hoc child walking. It handles some exported declarations but still misses many method-like constructs because Tree-sitter exposes different syntax forms for class methods, interface signatures, object literal methods, fields, arrow functions, private/static/async methods, and TSX patterns.

### Direction

Move from manual node-kind branching toward language-specific Tree-sitter queries or a query-like capture layer.

### Candidate Captures

Use semantic captures such as:

- `@symbol.class`
- `@symbol.function`
- `@symbol.method`
- `@symbol.field`
- `@symbol.interface`
- `@symbol.type`
- `@symbol.enum`
- `@outline.method`
- `@outline.field`

### TypeScript/TSX Forms To Cover

Add fixtures and AST dumps for:

1. `class Service { method() {} }`
2. `class Service { private method() {} }`
3. `class Service { static async method() {} }`
4. `class Service { field = () => {} }`
5. `interface Service { method(): void }`
6. `type Service = { method(): void }`
7. `const service = { method() {}, arrow: () => {} }`
8. `export class Service { method() {} }`
9. `export default class Service { method() {} }`
10. `export const Component = () => <div />`
11. decorated classes/methods if supported by the grammar
12. nested object/module patterns only if needed for outline UX

### Implementation Shape

1. Create small parser fixtures for each syntax form.
2. Add an AST dump command/test helper to inspect actual node kinds.
3. Define Tree-sitter query patterns or a declarative capture table.
4. Convert captures into:
   - flat `SymbolInfo` for top-level/project metrics;
   - hierarchical `OutlineItem` for detail panel navigation.
5. Keep symbols and outline rules separate:
   - top-level exported declarations are symbols and outline roots;
   - class/interface/object members are outline children;
   - only promote nested constructs to symbols if product semantics require it.

### Acceptance Criteria

- Parser tests prove recognition for class methods, interface method signatures, object methods, private/static/async methods, exported/default declarations, and arrow-function fields.
- Outline hierarchy preserves methods under their owning class/interface/object where possible.
- Existing import parsing still passes.
- No regression in Rust parser behavior.

### Suggested Validation

```bash
cd engine && cargo test typescript
cd engine && cargo test parser
npm test -- OutlineView
```

## Notes From Current Session

- The live app DB was moved to: `~/.local/share/com.codeatlas.app/db-backup-20260602-221105/`.
- Current uncommitted work includes DB cascade/upsert fixes and TypeScript export parsing changes.
- A robust backend fix may still be needed for `projects.root_path UNIQUE` conflicts if project ids remain regenerated on every scan.
