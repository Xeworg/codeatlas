# Delta for Project Understanding MVP — outline-parser-abstraction

## ADDED Requirements

### Requirement: Tree-sitter Semantic Parser Contracts

The system MUST expose a parser abstraction that can produce symbols, imports, and outline data from the same language-specific parsing contract.

#### Scenario: Parser registry selects supported language parser

- GIVEN a discovered source file with extension `ts`, `tsx`, or `rs`
- WHEN the scanner requests parser support for that file
- THEN the parser registry MUST return the matching language parser
- AND the scanner MUST NOT need language-specific extraction details

#### Scenario: Unsupported extension falls back safely

- GIVEN a discovered file with no registered language parser
- WHEN the scanner processes the file
- THEN the system MUST continue scanning without crashing
- AND the file MUST produce empty symbols, imports, and outline data unless another fallback exists

#### Scenario: Parse result keeps extraction coherent

- GIVEN a supported source file
- WHEN the language parser parses the file
- THEN it MUST return a `ParseResult` containing `symbols`, `imports`, and `outline`
- AND the returned outline MUST be derived from Tree-sitter structure, not AI inference

### Requirement: Hierarchical Outline Model

The system MUST represent file structure as a serializable hierarchical `OutlineItem` model shared by backend and frontend contracts.

#### Scenario: Outline item includes navigation range

- GIVEN a parsed symbol in a supported file
- WHEN the parser emits an outline item
- THEN the item MUST include `id`, `fileId`, `name`, `kind`, `lineStart`, `lineEnd`, and `children`
- AND it SHOULD include column boundaries when available

#### Scenario: TypeScript outline captures common hierarchy

- GIVEN a TypeScript or TSX file containing a class with methods and an interface or type
- WHEN outline extraction runs
- THEN the outline MUST include top-level class/interface/type/function items
- AND class methods MUST appear as children when Tree-sitter exposes that hierarchy

#### Scenario: Rust outline captures common hierarchy

- GIVEN a Rust file containing structs, enums, functions, modules, or impl methods
- WHEN outline extraction runs
- THEN the outline MUST include supported top-level Rust items
- AND impl/module children MUST appear hierarchically when Tree-sitter exposes that hierarchy

#### Scenario: Outline kind remains UI/IA oriented

- GIVEN existing `SymbolKind` values and new outline kinds differ
- WHEN contracts are defined
- THEN `OutlineItemKind` MUST remain a separate contract from `SymbolKind`
- AND any mapping between them MUST be explicit and tested

### Requirement: Outline Persistence and Retrieval API

The system MUST persist or retrieve outline data per file without breaking existing scan, graph, symbol, and import flows.

#### Scenario: Scan persists outline for supported file

- GIVEN `scan_project` processes a supported TypeScript/TSX or Rust file
- WHEN parsing succeeds
- THEN the system MUST persist the file's outline data associated with its `file_id`
- AND existing file, symbol, and import persistence MUST remain functional

#### Scenario: Outline storage is additive

- GIVEN an existing database with prior schema versions
- WHEN the outline migration is applied
- THEN it MUST add outline storage without destructive changes to existing tables
- AND existing projects MUST remain readable after migration

#### Scenario: Node outline command returns tree

- GIVEN a scanned file with persisted outline data
- WHEN the frontend invokes `get_node_outline(file_id)`
- THEN the backend MUST return a serializable `OutlineItem[]` tree
- AND the response MUST preserve parent/child nesting

#### Scenario: Missing outline returns empty state

- GIVEN a valid file with no persisted or generated outline
- WHEN `get_node_outline(file_id)` is invoked
- THEN the backend MUST return an empty outline result rather than failing unexpectedly
- AND the frontend MUST be able to render a clear empty state

### Requirement: Outline UI in Detail Panel

The system MUST show a VS Code-like outline in the node detail panel without overloading graph node cards.

#### Scenario: Selecting node loads outline

- GIVEN the user selects a graph node for a scanned file
- WHEN the detail panel opens
- THEN the frontend MUST request outline data for that node/file
- AND it MUST render the hierarchy if outline items exist

#### Scenario: Outline view displays semantic metadata

- GIVEN outline items are available
- WHEN the outline view renders
- THEN each visible item MUST show at least kind, name, and line range
- AND nested items MUST be visually distinguishable from parent items

#### Scenario: Empty and error states are visible

- GIVEN outline loading is pending, empty, or fails
- WHEN the detail panel renders
- THEN the UI MUST show an appropriate loading, empty, or error state
- AND the graph canvas MUST remain usable

#### Scenario: Graph node remains compact

- GIVEN a file has many outline items
- WHEN the graph is rendered
- THEN the graph node card MUST remain compact by default
- AND full outline rendering MUST stay in the panel or another deliberate expanded surface

### Requirement: Semantic AI Context from Outline

The system MUST use Tree-sitter outline data to build compact AI context before falling back to full or truncated source text.

#### Scenario: Node explanation includes semantic summary

- GIVEN a user requests an AI explanation for a scanned node with outline data
- WHEN the backend builds node context
- THEN the context MUST include a semantic summary with file identity, outline, imports, and relevant graph relationships
- AND the context SHOULD avoid sending full source content unless needed as fallback or targeted excerpt

#### Scenario: Targeted symbol excerpts are possible

- GIVEN an outline item has line range metadata
- WHEN AI context needs source detail for a relevant symbol
- THEN the system SHOULD be able to extract source text for that item's line range
- AND this extraction SHOULD be preferred over reading the entire file for focused questions

#### Scenario: Fallback preserves current AI behavior

- GIVEN outline data is unavailable, empty, or cannot be parsed
- WHEN AI context is requested
- THEN the system MUST continue to provide a usable fallback based on existing file content behavior
- AND the failure MUST NOT break node explanation or chat flows

#### Scenario: Context modes stay bounded

- GIVEN a large file with many outline items
- WHEN AI context is generated
- THEN the context builder MUST be able to produce a bounded summary rather than unbounded outline output
- AND future `summary`, `focused`, and `full` modes MUST remain compatible with this contract

### Requirement: Verification and Scope Protection

The system MUST verify outline behavior with tests or fixtures and protect the change from expanding beyond the approved first slice.

#### Scenario: Language fixtures validate hierarchy

- GIVEN fixture files for TypeScript/TSX and Rust
- WHEN parser tests run
- THEN tests MUST verify representative top-level and nested outline items
- AND they MUST verify line ranges are populated

#### Scenario: Existing scan and graph behavior remains compatible

- GIVEN existing scan and graph tests or smoke flows
- WHEN outline support is added
- THEN existing symbols/imports/graph behavior MUST remain compatible
- AND no existing public command may require a breaking rename for this change

#### Scenario: Review workload triggers slicing

- GIVEN implementation planning estimates more than 600 changed lines
- WHEN tasks are created
- THEN the work MUST be split into reviewable slices or chained PRs
- AND the first slice MUST preserve a working product state

#### Scenario: Out-of-scope capability is deferred

- GIVEN requests for full IDE editing, all-language support, global symbol search, or method-level flow analysis
- WHEN evaluated against this change
- THEN they MUST be marked out of scope for the initial implementation
- AND deferred to a future approved change
