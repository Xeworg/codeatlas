# Adding a Language to CodeAtlas

> How to implement a new `LanguageParser` and wire it into the dispatch registry.

This guide covers the complete process for adding a new language (e.g., Go, Java, Python, Ruby) to CodeAtlas's parser framework. The framework uses tree-sitter for parsing and exposes results via a language-neutral `ParseResult`.

## Overview

```
LanguageParser trait (engine/src/scanner/parser/traits.rs)
    │
    ├── TypeScriptParser ── supports: ts, tsx, js, jsx
    ├── RustParser ──────── supports: rs
    └── PythonParser (stub) ── supports: py

ParserRegistry (engine/src/scanner/parser/registry.rs)
    ├── new() → registers all parsers
    ├── parse_file(path,src,ext,file_id) → ParseResult
    └── impl ParseFile for ParserRegistry   (single-dispatch contract)
```

Every file is parsed by **exactly one** call to `ParserRegistry::parse_file`. The registry dispatches to the matching parser. All parsers produce the same `ParseResult` shape.

---

## Checklist

### 1. Implement `LanguageParser`

Create `engine/src/scanner/parser/<lang>.rs` with `pub struct <Lang>Parser` implementing the trait.

```rust
use super::traits::LanguageParser;
use crate::models::{ParseResult, LexicalValueKind, Reference};

pub struct GoParser;

impl LanguageParser for GoParser {
    fn language_id(&self) -> &'static str { "go" }
    fn extensions(&self) -> &'static [&'static str] { &["go"] }

    fn parse_all(&self, source: &str, path: &str, file_id: &str) -> ParseResult {
        // 1. Parse with tree-sitter-go.
        // 2. Walk the AST once and collect:
        //    - symbols: Vec<SymbolInfo>   (top-level declarations)
        //    - imports: Vec<ImportInfo>    (import / require statements)
        //    - outline: Vec<OutlineItem>   (hierarchical: parents + children)
        //    - lexical_kind (optional override)
        //    - references  (optional override)
        ParseResult::default() // replace with real extraction
    }
}
```

**Required methods:**

| Method          | Returns                   | Notes                                         |
| --------------- | ------------------------- | --------------------------------------------- |
| `language_id()` | `&'static str`            | Unique ID, e.g., `"typescript"`               |
| `extensions()`  | `&'static [&'static str]` | File extensions, e.g., `&["ts", "tsx"]`       |
| `parse_all()`   | `ParseResult`             | Walk the AST **once**; populate all IR fields |

**Optional overrides (defaults provided):**

| Method                          | Default return               | When to override                                           |
| ------------------------------- | ---------------------------- | ---------------------------------------------------------- |
| `lexical_kind_for(node, src)`   | `LexicalValueKind::Function` | Distinguish `const`/`let` from `function` declarations     |
| `extract_references(node, src)` | `vec![]`                     | Emit `Reference { Import, target_name, range }` per import |

### 2. Register the parser

In `engine/src/scanner/parser/registry.rs` → `ParserRegistry::new()`:

```rust
registry.register(GoParser::new());
```

### 3. Add a fixture for testing

```
engine/tests/fixtures/
├── go/
│   ├── hello.go        # minimal Go source
│   └── imports.go     # import statements for reference extraction
```

### 4. Verify with tests

```bash
cd engine && cargo test --lib
```

**Key assertions:**

- `registry.parser_for_extension("go")` returns `Some(...)`
- `registry.parse_file("test.go", src, "go", id)` returns a `ParseResult` with non-empty symbols or outline
- `ParseResult` field counts are stable across calls

### 5. Add the grammar dependency

In `engine/Cargo.toml`, add the tree-sitter grammar:

```toml
tree-sitter-go = "0.20"   # check the latest version
```

---

## IR Contract (ParseResult)

Every parser must produce `ParseResult` with these fields:

```
ParseResult {
    symbols: Vec<SymbolInfo>      - top-level declarations (name, kind)
    imports: Vec<ImportInfo>      - import statements (source, target_module)
    outline: Vec<OutlineItem>     - hierarchical: parents with children
    lexical_kind: LexicalValueKind (default: Const)    - const vs function
    references: Vec<Reference>   (default: empty)       - import/export edges
}
```

### SymbolInfo fields

- `name: String` — declaration name
- `kind: SymbolKind` — `Class | Function | Method | Interface | TypeAlias | Enum | Variable | Const | Struct | Impl`
- `file_id: String` — use the file's relative path (not absolute path) for cross-file resolution
- `line_start`, `line_end` — 1-based line numbers

### ImportInfo fields

- `source_file_id: String` — must be the file's relative path (matches `FileInfo.id` → `path_to_id` lookup in Tauri commands)
- `target_module: Option<String>` — module path as a string (`"react"`, `"fmt"`)
- `imports: Vec<String>` — imported names from specifiers
- `is_default`, `is_type` — flags for default/type imports

### OutlineItem fields

- `id` — use `OutlineItem::stable_id(file_id, kind, line_start, line_end, name)`
- `file_id: String` — DB-assigned UUID
- `name`, `kind: OutlineItemKind`, `line_start`, `line_end`
- `children: Vec<OutlineItem>` — hierarchical members

---

## Important Gotchas

### 1. `source_file_id` must be the relative path

The Tauri `scan_project` command builds a `path_to_id` lookup from `FileInfo.id = UUID` mapped to `FileInfo.path = relative_path`. Import resolution in Phase 2 uses that map. If `source_file_id = absolute_path` instead of `relative_path`, the resolver will miss all internal imports.

### 2. One AST pass only

Implement all extraction inside a **single tree walk** in `parse_all`. If you call tree-sitter or walk the AST multiple times, the `single_pass` contract is violated.

### 3. `#[allow(dead_code)]` for unused helpers

If you extract helper methods (e.g., `ts_symbol_kind`, `extract_ts_symbols`) that are only used in tests, mark them `#[allow(dead_code)]` to keep clippy clean between the stub phase and when the real implementation ships.

### 4. `.tsx` and `.jsx` need the TSX grammar

The TypeScript parser stores two grammars and selects based on the path:

```rust
fn language_for_path(&self, path: &str) -> &Language {
    if path.ends_with(".tsx") || path.ends_with(".jsx") {
        &self.tsx_language
    } else {
        &self.language
    }
}
```

If your language has variants (e.g., `.h` vs `.c` for C), propagate the full path, not just the extension.

### 5. Rust `use` items: conservative emission

Rust `use` declarations may resolve to crate-internal paths that can't be resolved at parse time. Emit what's observable in the source text; cross-file resolution is a v2 concern.

### 6. `file_id` vs `file_id` consistency

- `symbols[i].file_id = relative_path` — used for display and grouping
- `outline[j].file_id = DB UUID` — used for persistence and outline retrieval
- `ImportInfo.source_file_id = relative_path` — must match `path_to_id` keys

---

## Running Tests

```bash
# Engine unit tests
cd engine && cargo test --lib

# Engine with clippy
cd engine && cargo clippy --lib -- -D warnings

# Tauri (backend integration)
cd src-tauri && cargo test && cargo clippy -- -D warnings

# Frontend
npm run test && npm run lint && npm run typecheck
```

---

## File Layout for a New Language

```
engine/src/scanner/parser/
├── mod.rs          ← add: pub mod go; (sorted alphabetically)
├── traits.rs       ← LanguageParser trait (no changes needed)
├── registry.rs     ← add: registry.register(GoParser::new());
├── go.rs           ← NEW: GoParser { language, tsx_language? }

engine/tests/fixtures/
├── go/
│   ├── hello.go    ← NEW: minimal Go with common node kinds
│   └── imports.go  ← NEW: import statements for reference tests

engine/src/scanner/code_parser.rs
  (no changes needed after C.3 — CodeParser::parse_file is deprecated)
```

---

## Related Files

| File                                                           | Purpose                           |
| -------------------------------------------------------------- | --------------------------------- |
| `engine/src/scanner/parser/traits.rs`                          | `LanguageParser` trait definition |
| `engine/src/scanner/parser/registry.rs`                        | Parser dispatch                   |
| `engine/src/scanner/parser/typescript.rs`                      | Reference implementation (TS)     |
| `engine/src/scanner/parser/rust.rs`                            | Reference implementation (Rust)   |
| `engine/src/scanner/parser/python_stub.rs`                     | Minimal stub example              |
| `engine/src/scanner/code_parser.rs`                            | Deprecated legacy shim            |
| `engine/src/commands.rs`                                       | Pure scan orchestration           |
| `openspec/changes/multi-language-code-intelligence-framework/` | Full design docs                  |
