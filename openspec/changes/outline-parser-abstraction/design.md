# Design — outline-parser-abstraction

## 1) Executive design summary

`outline-parser-abstraction` se implementa como una capa semántica Tree-sitter-first. La UI de outline es el primer consumidor visible, pero el diseño debe servir también para contexto IA compacto: primero estructura, imports y relaciones; después extractos de código por rango si hacen falta.

El cambio se divide en 4 slices para proteger revisión:

1. **Semantic parser foundation** — contratos, modelos, registry, parsers y fixtures.
2. **Persistence/API** — migración `007`, queries, integración con `scan_project`, comando Tauri.
3. **Outline UI** — `OutlineView` en `DetailPanel` con estados claros.
4. **AI semantic context** — `ContextBuilder` usa outline antes que fuente truncada.

Si la estimación final supera 600 líneas cambiadas, estos slices deben aplicarse como PRs encadenados.

---

## 2) Current architecture baseline

### Backend

| Área     | Estado actual                                                                     | Cambio requerido                                        |
| -------- | --------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Parser   | `engine/src/scanner/parser.rs` contiene `CodeParser::parse_file()` monolítico     | Extraer contratos y parsers por lenguaje                |
| Modelos  | `engine/src/models/file.rs` tiene `SymbolInfo`, `ImportInfo`, `SymbolKind`        | Agregar `OutlineItem`, `OutlineItemKind`, `ParseResult` |
| DB       | schema/migrations sin outline                                                     | Agregar `outline_items` por `file_id`                   |
| Commands | `src-tauri/src/commands.rs` tiene `scan_project`, `get_graph`, `get_node_details` | Agregar `get_node_outline` e integrar outline al scan   |
| IA       | `engine/src/ai/context.rs` trunca fuente por bytes                                | Priorizar outline/imports/deps y extractos por rango    |

### Frontend

| Área       | Estado actual                                                     | Cambio requerido                                |
| ---------- | ----------------------------------------------------------------- | ----------------------------------------------- |
| Types      | `src/lib/types.ts` espeja modelos Rust                            | Agregar tipos de outline                        |
| API        | `src/lib/tauri-api.ts` envuelve comandos Tauri                    | Agregar `getNodeOutline(fileId)`                |
| Panel      | `DetailPanel.tsx` carga `getNodeDetails` y renderiza `SymbolList` | Cargar outline y renderizar `OutlineView`       |
| Graph node | `GraphNodeComponent.tsx` muestra resumen compacto                 | Mantener compacto; no renderizar árbol completo |

---

## 3) Backend design

### 3.1 Modelos de dominio

Agregar en `engine/src/models/file.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutlineItemKind {
    Class,
    Function,
    Method,
    Interface,
    Type,
    Enum,
    Const,
    Variable,
    Module,
    Field,
    Struct,
    Impl,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineItem {
    pub id: String,
    pub file_id: String,
    pub name: String,
    pub kind: OutlineItemKind,
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: Option<u32>,
    pub column_end: Option<u32>,
    pub children: Vec<OutlineItem>,
}

#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    pub symbols: Vec<SymbolInfo>,
    pub imports: Vec<ImportInfo>,
    pub outline: Vec<OutlineItem>,
}
```

Design rule: `OutlineItemKind` queda separado de `SymbolKind`. Puede haber helpers de mapeo, pero no deben ser el mismo contrato.

### 3.2 Parser module layout

Migrar desde `engine/src/scanner/parser.rs` hacia módulo incremental:

```text
engine/src/scanner/
├─ parser.rs                 # compat facade temporal o mod redirect
└─ parser/
   ├─ mod.rs
   ├─ traits.rs              # LanguageParser + helpers comunes
   ├─ registry.rs            # extension -> parser
   ├─ typescript.rs          # TS/TSX/JS/JSX si aplica
   └─ rust.rs                # Rust parser
```

Para evitar un refactor riesgoso en un solo paso, `CodeParser::parse_file()` puede mantenerse como facade temporal:

```rust
impl CodeParser {
    pub fn parse_file(path: &str, content: &str, extension: &str) -> (Vec<SymbolInfo>, Vec<ImportInfo>) {
        let result = ParserRegistry::default().parse_file(path, content, extension, path);
        (result.symbols, result.imports)
    }

    pub fn parse_file_all(path: &str, content: &str, extension: &str, file_id: &str) -> ParseResult {
        ParserRegistry::default().parse_file(path, content, extension, file_id)
    }
}
```

La facade protege tests y flujos existentes mientras se migra `scan_project` a `parse_file_all`.

### 3.3 Parser trait

```rust
pub trait LanguageParser {
    fn language_id(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn parse_all(&self, source: &str, path: &str, file_id: &str) -> ParseResult;
}
```

`ParserRegistry`:

```rust
pub struct ParserRegistry {
    parsers: Vec<Box<dyn LanguageParser + Send + Sync>>,
}

impl ParserRegistry {
    pub fn default() -> Self;
    pub fn parser_for_extension(&self, extension: &str) -> Option<&dyn LanguageParser>;
    pub fn parse_file(&self, path: &str, source: &str, extension: &str, file_id: &str) -> ParseResult;
}
```

Unsupported extension returns `ParseResult::default()`.

### 3.4 Outline extraction rules

Use Tree-sitter node traversal with recursive helpers. Each parser maps language-specific nodes to `OutlineItemKind`.

#### TypeScript/TSX initial mapping

| Tree-sitter node         | Outline kind         | Notes                                          |
| ------------------------ | -------------------- | ---------------------------------------------- |
| `class_declaration`      | `class`              | children from class body methods/fields        |
| `method_definition`      | `method`             | child of class when nested                     |
| `function_declaration`   | `function`           | top-level or nested if found                   |
| `interface_declaration`  | `interface`          | children from members if reliable              |
| `type_alias_declaration` | `type`               | top-level                                      |
| `enum_declaration`       | `enum`               | top-level                                      |
| `lexical_declaration`    | `const` / `variable` | classify by declaration keyword when available |

#### Rust initial mapping

| Tree-sitter node                | Outline kind | Notes                                         |
| ------------------------------- | ------------ | --------------------------------------------- |
| `struct_item`                   | `struct`     | top-level/module child                        |
| `enum_item`                     | `enum`       | top-level/module child                        |
| `function_item`                 | `function`   | top-level/module child                        |
| `impl_item`                     | `impl`       | children are methods/functions when available |
| `mod_item`                      | `module`     | children from body if inline module           |
| `type_item` / `type_alias_item` | `type`       | depending grammar node observed               |

### 3.5 Stable IDs

Preferred initial strategy:

```text
outline:<file_id>:<kind>:<line_start>:<line_end>:<name>
```

This is more stable than UUID across scans when lines do not move, and good enough for UI keys and source-range lookup. If later cross-scan symbol identity matters, add a content/path hash.

### 3.6 Persistence

Add `engine/migrations/007_outline_items.sql`:

```sql
CREATE TABLE IF NOT EXISTS outline_items (
    file_id TEXT PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    outline_json TEXT NOT NULL,
    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_outline_items_generated_at
ON outline_items(generated_at);
```

Update migration registry:

- `engine/src/db/migrations.rs`: bump `CURRENT_SCHEMA_VERSION` from `6` to `7`.
- Add migration file loading for `007_outline_items.sql` following existing file-backed migrations.

Queries in `engine/src/db/queries.rs`:

```rust
pub fn save_outline_items(&self, file_id: &str, items: &[OutlineItem]) -> Result<()>;
pub fn get_outline_items(&self, file_id: &str) -> Result<Vec<OutlineItem>>;
pub fn delete_outline_items_for_project(&self, project_id: &str) -> Result<()>; // optional if scan replacement needs cleanup
```

Use `INSERT OR REPLACE` by `file_id`.

### 3.7 scan_project integration

Current scan has two important invariants from recent fixes:

1. files/projects must exist before imports are persisted;
2. import `source_file_id` must be real DB file UUID, not path.

Design rule: outline persistence must follow the same file UUID discipline.

Recommended flow:

```text
scan_project
  1. discover files
  2. assign file UUIDs and collect content metadata
  3. parse each supported file with file_id UUID
  4. save project/files/symbols
  5. resolve/import persist imports
  6. save outline_items by file_id
```

If current code parses before final file IDs exist, adapt outline persistence to use the same path -> UUID map used for import fix.

### 3.8 Tauri command

Add to `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub async fn get_node_outline(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<OutlineItem>, String> {
    let repo = ProjectRepository::new(state.db.clone());
    repo.get_outline_items(&file_id).map_err(|e| e.to_string())
}
```

Register in `src-tauri/src/lib.rs` inside `generate_handler!`.

Potential fallback for missing persisted outline: return `Ok(vec![])` in first version. Reparse-on-demand can be a later enhancement if needed.

---

## 4) Frontend design

### 4.1 Type contracts

Add to `src/lib/types.ts`:

```ts
export type OutlineItemKind =
  | 'class'
  | 'function'
  | 'method'
  | 'interface'
  | 'type'
  | 'enum'
  | 'const'
  | 'variable'
  | 'module'
  | 'field'
  | 'struct'
  | 'impl'
  | 'unknown'

export interface OutlineItem {
  id: string
  fileId: string
  name: string
  kind: OutlineItemKind
  lineStart: number
  lineEnd: number
  columnStart?: number | null
  columnEnd?: number | null
  children: OutlineItem[]
}
```

### 4.2 Tauri API wrapper

Add to `src/lib/tauri-api.ts`:

```ts
export async function getNodeOutline(fileId: string): Promise<OutlineItem[]> {
  return await invoke<OutlineItem[]>('get_node_outline', { fileId })
}
```

### 4.3 OutlineView component

Create `src/components/panel/OutlineView.tsx`.

Responsibilities:

- recursive tree render;
- kind badge/icon;
- name;
- line range;
- indentation by depth;
- local collapse state by item id;
- empty state.

No editor navigation in first version. A click may later emit `{ fileId, lineStart, lineEnd }`.

### 4.4 DetailPanel integration

`DetailPanel.tsx` should fetch details and outline independently:

```text
selectedNodeId changes
  -> getNodeDetails(selectedNodeId)
  -> getNodeOutline(selectedNodeId)
```

State can be local initially:

```ts
const [outline, setOutline] = useState<OutlineItem[]>([])
const [outlineLoading, setOutlineLoading] = useState(false)
const [outlineError, setOutlineError] = useState<string | null>(null)
```

Render order:

1. header/metadata;
2. outline section;
3. existing `SymbolList` as fallback/debug during transition.

Do not add outline state to Zustand in first version unless multiple components need it.

---

## 5) AI semantic context design

### 5.1 ContextBuilder API

Extend `engine/src/ai/context.rs` without breaking existing callers:

```rust
pub enum AiContextMode {
    Summary,
    Focused,
    Full,
}

pub struct NodeSemanticContext<'a> {
    pub file_content: &'a str,
    pub file_path: &'a str,
    pub graph: &'a GraphData,
    pub node_id: &'a str,
    pub outline: &'a [OutlineItem],
    pub mode: AiContextMode,
}

pub fn build_node_context_with_outline(input: NodeSemanticContext<'_>) -> String;
```

Keep existing `build_node_context()` as fallback wrapper.

### 5.2 Output shape

Semantic context should look like:

````text
**Archivo:** src/services/UserService.ts
**Tipo:** Service
**Símbolos:** 12

**Outline:**
- class UserService (10-95)
  - method constructor (12-18)
  - method getUser (20-42)
  - method saveUser (44-70)

**Dependencias:**
- UserRepository (...)

**Dependientes:**
- UserController (...)

**Extractos dirigidos:**
```text
// optional focused snippets by line range
````

```

### 5.3 Bounded context rules

- cap total context to existing `MAX_CONTEXT_BYTES`;
- cap outline rendered items, e.g. first 80 items depth-first;
- include `(...más símbolos...)` when truncated;
- extract code by line range only for focused mode or selected relevant symbols;
- fallback to current first-lines truncation when outline is empty/unavailable.

### 5.4 Commands affected

Initial target: `explain_node` only.

`chat` can adopt project-level semantic index later because it needs broader ranking/search. Do not expand this change into global symbol search.

---

## 6) Test strategy

### Backend unit tests

Add parser fixture tests near parser modules:

- TypeScript class with methods + interface/type.
- TSX component function + hook imports.
- Rust struct + impl method + module/function.
- Unsupported extension returns empty `ParseResult`.

Assertions:

- top-level items exist;
- nested items exist where expected;
- line ranges are > 0 and `lineEnd >= lineStart`;
- `OutlineItemKind` serialization uses snake_case/camelCase correctly through serde contract.

### DB/migration tests

- migration 007 applies on existing schema;
- `save_outline_items` then `get_outline_items` roundtrips nested JSON;
- missing file outline returns empty vector or not-found mapped to empty command response.

### Tauri/command tests

If command-level tests exist:

- `get_node_outline` returns persisted outline for known file;
- invalid/missing file id returns safe error or empty according to implementation decision.

### Frontend tests

- `OutlineView` renders nested items;
- empty state appears for empty array;
- error/loading states in `DetailPanel` do not block base details.

### AI context tests

- context with outline includes symbol hierarchy;
- context remains under byte cap;
- empty outline falls back to existing source truncation behavior;
- focused extraction respects line ranges.

---

## 7) Rollout plan

### Slice 1 — Semantic parser foundation

Files likely touched:

- `engine/src/models/file.rs`
- `engine/src/models/mod.rs`
- `engine/src/scanner/parser.rs`
- `engine/src/scanner/parser/*`
- parser tests/fixtures

Exit criteria:

- `CodeParser::parse_file()` compatibility preserved;
- `parse_file_all()` or registry returns outline;
- TS/Rust fixtures pass.

### Slice 2 — Persistence/API

Files likely touched:

- `engine/migrations/007_outline_items.sql`
- `engine/src/db/migrations.rs`
- `engine/src/db/queries.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src/lib/types.ts`
- `src/lib/tauri-api.ts`

Exit criteria:

- outline persisted during scan;
- `get_node_outline` returns tree;
- existing graph/import behavior unchanged.

### Slice 3 — UI outline panel

Files likely touched:

- `src/components/panel/OutlineView.tsx`
- `src/components/panel/DetailPanel.tsx`
- maybe `src/components/panel/SymbolList.tsx`

Exit criteria:

- selected node shows outline when available;
- empty/loading/error states are clear;
- graph node cards remain compact.

### Slice 4 — AI semantic context

Files likely touched:

- `engine/src/ai/context.rs`
- `src-tauri/src/commands.rs` for `explain_node` integration
- tests for context builder

Exit criteria:

- `explain_node` context includes outline summary when available;
- fallback remains working;
- context cap respected.

---

## 8) Review workload forecast

Estimated changed lines by slice:

| Slice | Estimate | Recommendation |
| --- | ---: | --- |
| Semantic parser foundation | 250–450 | Reviewable alone |
| Persistence/API | 180–320 | Reviewable alone |
| UI outline panel | 150–300 | Reviewable alone |
| AI semantic context | 120–250 | Reviewable alone |

Combined estimate likely exceeds 600 lines. Therefore implementation SHOULD be planned as chained PRs or at least separate commits/slices. The first slice should leave product behavior unchanged except for internal parser capability and tests.

---

## 9) Non-goals and guardrails

Do not include in initial implementation:

- full IDE/editor navigation;
- global symbol search UI;
- method-level dependency graph;
- all-language support;
- large graph-node inline outline;
- AI-driven symbol extraction;
- breaking rename/removal of existing commands.

Any of these should become a future SDD change.

---

## 10) Open decisions for tasks phase

1. Whether JS/JSX ships in Slice 1 with TypeScript parser or follows after TS/TSX.
2. Whether `get_node_outline` returns empty for unknown `file_id` or a typed error. Recommended: unknown id error, known file with no outline empty.
3. Exact fixture location for parser tests.
4. Whether outline persistence happens before or after import resolution in current `scan_project`; recommended after files are persisted and file UUIDs are authoritative.
5. Whether `SymbolList` remains visible below outline during transition or is replaced when outline exists.
```
