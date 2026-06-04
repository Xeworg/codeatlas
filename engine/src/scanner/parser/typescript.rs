//! TypeScript / TSX language parser.
//!
//! Extracts symbols, imports, and hierarchical outline from TypeScript source.
//!
//! # Outline hierarchy
//!
//! - Top-level declarations become root outline items.
//! - `class_declaration` children: methods, fields if Tree-sitter exposes them.
//! - `interface_declaration` children: properties, methods if Tree-sitter exposes them.
//! - Handles `export_statement` wrappers: `export function`, `export class`, etc.

use super::traits::{make_outline_id, LanguageParser};
use crate::models::{
    ImportInfo, LexicalValueKind, OutlineItem, OutlineItemKind, ParseResult, Range, Reference,
    ReferenceKind, SymbolInfo, SymbolKind,
};
use tree_sitter::{Language, Parser};
use tree_sitter_typescript::{LANGUAGE_TSX, LANGUAGE_TYPESCRIPT};

/// TypeScript/TSX language parser.
pub struct TypeScriptParser {
    /// Non-JSX grammar (used for `.ts`/`.js`).
    language: Language,
    /// JSX grammar (used for `.tsx`/`.jsx`).
    tsx_language: Language,
}

impl TypeScriptParser {
    pub fn new() -> Self {
        Self {
            language: LANGUAGE_TYPESCRIPT.into(),
            tsx_language: LANGUAGE_TSX.into(),
        }
    }

    /// Pick the right grammar for the file extension. PR-B requires the TSX
    /// grammar for files containing JSX (e.g. `react_const_arrow.tsx`).
    fn language_for_path(&self, path: &str) -> &Language {
        if path.ends_with(".tsx") || path.ends_with(".jsx") {
            &self.tsx_language
        } else {
            &self.language
        }
    }

    /// Maps tree-sitter node kinds to SymbolKind.
    fn ts_symbol_kind(kind: &str) -> Option<SymbolKind> {
        match kind {
            "class_declaration" => Some(SymbolKind::Class),
            "function_declaration" => Some(SymbolKind::Function),
            "method_definition" => Some(SymbolKind::Method),
            "interface_declaration" => Some(SymbolKind::Interface),
            "type_alias_declaration" => Some(SymbolKind::TypeAlias),
            "enum_declaration" => Some(SymbolKind::Enum),
            "lexical_declaration" => Some(SymbolKind::Const),
            _ => None,
        }
    }

    /// Maps SymbolKind to OutlineItemKind.
    fn ts_symbol_kind_to_outline_kind(kind: SymbolKind) -> Option<OutlineItemKind> {
        match kind {
            SymbolKind::Class => Some(OutlineItemKind::Class),
            SymbolKind::Function => Some(OutlineItemKind::Function),
            SymbolKind::Method => Some(OutlineItemKind::Method),
            SymbolKind::Interface => Some(OutlineItemKind::Interface),
            SymbolKind::TypeAlias => Some(OutlineItemKind::Type),
            SymbolKind::Enum => Some(OutlineItemKind::Enum),
            SymbolKind::Const => Some(OutlineItemKind::Const),
            _ => None,
        }
    }

    /// For export_statement nodes, find the first named child declaration.
    /// Skips keyword tokens (export, default, type) and comments.
    fn find_declaration_child<'a>(
        node: &'a tree_sitter::Node<'a>,
    ) -> Option<tree_sitter::Node<'a>> {
        let mut c = node.walk();
        for child in node.children(&mut c) {
            let kind = child.kind();
            if kind.starts_with("comment") {
                continue;
            }
            if !child.is_named() {
                continue;
            }
            return Some(child);
        }
        None
    }

    /// Extract declaration name from a node. Handles nested names for
    /// lexical_declaration (variable_declarator > name).
    fn ts_declaration_name<'a>(
        node: &'a tree_sitter::Node<'a>,
        bytes: &'a [u8],
    ) -> Option<&'a str> {
        if let Some(name_node) = node.child_by_field_name("name") {
            return name_node.utf8_text(bytes).ok();
        }
        // For lexical_declaration, walk children to find variable_declarator,
        // then its name field.  tree-sitter may not always set "name" field on
        // variable_declarator, so walk named children instead.
        if node.kind() == "lexical_declaration" {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "variable_declarator" {
                    // Try field access first
                    if let Some(n) = child.child_by_field_name("name") {
                        return n.utf8_text(bytes).ok();
                    }
                    // Fallback: first named child of variable_declarator that is not
                    // a value expression (arrow_function, object, array, etc.) is the name.
                    let mut vc = child.walk();
                    for vc_child in child.children(&mut vc) {
                        let vc_kind = vc_child.kind();
                        if vc_child.is_named()
                            && vc_kind != "arrow_function"
                            && vc_kind != "object"
                            && vc_kind != "array"
                            && vc_kind != "member"
                            && vc_kind != "binary"
                            && vc_kind != "call"
                            && vc_kind != "conditional"
                            && vc_kind != "sequence"
                            && vc_kind != "parenthesized"
                            && vc_kind != "template"
                        {
                            return vc_child.utf8_text(bytes).ok();
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract children outline items from a parent node (e.g. methods inside a class).
    fn extract_children(
        parent: &tree_sitter::Node,
        _source: &str,
        file_id: &str,
        bytes: &[u8],
    ) -> Vec<OutlineItem> {
        let mut children = Vec::new();
        let mut cursor = parent.walk();

        let body = parent
            .children(&mut cursor)
            .find(|n| n.kind() == "class_body");
        let nodes_to_scan = match body {
            Some(b) => {
                let mut bc = b.walk();
                b.children(&mut bc).collect::<Vec<_>>()
            }
            None => parent.children(&mut cursor).collect::<Vec<_>>(),
        };

        for node in nodes_to_scan {
            let kind = node.kind();

            if kind == "method_definition" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(bytes).unwrap_or("");
                    let start = node.start_position();
                    let end = node.end_position();
                    let id = make_outline_id(
                        file_id,
                        OutlineItemKind::Method,
                        start.row as u32 + 1,
                        end.row as u32 + 1,
                        name,
                    );
                    children.push(OutlineItem {
                        id,
                        file_id: file_id.to_string(),
                        name: name.to_string(),
                        kind: OutlineItemKind::Method,
                        line_start: start.row as u32 + 1,
                        line_end: end.row as u32 + 1,
                        column_start: Some(start.column as u32),
                        column_end: Some(end.column as u32),
                        children: vec![],
                    });
                }
            }

            if kind == "property_declaration" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(bytes).unwrap_or("");
                    let start = node.start_position();
                    let end = node.end_position();
                    let id = make_outline_id(
                        file_id,
                        OutlineItemKind::Field,
                        start.row as u32 + 1,
                        end.row as u32 + 1,
                        name,
                    );
                    children.push(OutlineItem {
                        id,
                        file_id: file_id.to_string(),
                        name: name.to_string(),
                        kind: OutlineItemKind::Field,
                        line_start: start.row as u32 + 1,
                        line_end: end.row as u32 + 1,
                        column_start: Some(start.column as u32),
                        column_end: Some(end.column as u32),
                        children: vec![],
                    });
                }
            }
        }

        children
    }

    /// Walk a `lexical_declaration` node and return true if any of its
    /// `variable_declarator` children has an `arrow_function` as its direct
    /// value (e.g. `const x = () => {}`).
    fn lexical_decl_has_arrow(node: &tree_sitter::Node) -> bool {
        let mut c = node.walk();
        for child in node.children(&mut c) {
            if child.kind() == "variable_declarator" {
                let mut vc = child.walk();
                for vc_child in child.children(&mut vc) {
                    if vc_child.kind() == "arrow_function" {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Walk a class body and return true if any `public_field_definition` /
    /// `field_definition` child has an `arrow_function` value.
    fn class_body_has_arrow_field(class_node: &tree_sitter::Node) -> bool {
        let mut c = class_node.walk();
        for child in class_node.children(&mut c) {
            if child.kind() == "class_body" {
                let mut bc = child.walk();
                for body_child in child.children(&mut bc) {
                    if body_child.kind() == "public_field_definition"
                        || body_child.kind() == "field_definition"
                    {
                        let mut fc = body_child.walk();
                        for fc_child in body_child.children(&mut fc) {
                            if fc_child.kind() == "arrow_function" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Append a `Reference` to `refs` using the given node's range. The
    /// `file_id` is left empty here and filled in by `parse_all`.
    fn push_spec_reference(
        refs: &mut Vec<Reference>,
        node: tree_sitter::Node,
        target_name: String,
        kind: ReferenceKind,
    ) {
        let start = node.start_position();
        let end = node.end_position();
        refs.push(Reference {
            file_id: String::new(), // filled by parse_all
            kind,
            target_name,
            range: Range {
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_line: start.row as u32 + 1,
                start_col: start.column as u32,
                end_line: end.row as u32 + 1,
                end_col: end.column as u32,
            },
        });
    }

    /// Collect `SymbolInfo { ArrowFunction }` for every arrow-valued class
    /// field within `class_node`. PR-B contract: the field symbol must be
    /// distinct from the class symbol so the AI layer can reason about both.
    fn collect_arrow_field_symbols(
        class_node: &tree_sitter::Node,
        bytes: &[u8],
        file_id: &str,
    ) -> Vec<SymbolInfo> {
        let mut symbols = Vec::new();
        let mut c = class_node.walk();
        for child in class_node.children(&mut c) {
            if child.kind() != "class_body" {
                continue;
            }
            let body_children: Vec<tree_sitter::Node> = {
                let mut bc = child.walk();
                child.children(&mut bc).collect()
            };
            for body_child in body_children {
                let kind = body_child.kind();
                if kind != "public_field_definition" && kind != "field_definition" {
                    continue;
                }
                let has_arrow = {
                    let mut fc = body_child.walk();
                    let x = body_child
                        .children(&mut fc)
                        .any(|cc| cc.kind() == "arrow_function");
                    x
                };
                if !has_arrow {
                    continue;
                }
                let name_node = match body_child.child_by_field_name("name") {
                    Some(n) => n,
                    None => continue,
                };
                let name = name_node.utf8_text(bytes).unwrap_or("");
                let start = body_child.start_position();
                let end = body_child.end_position();
                symbols.push(SymbolInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.to_string(),
                    kind: SymbolKind::ArrowFunction,
                    file_id: file_id.to_string(),
                    line_start: start.row as u32 + 1,
                    line_end: end.row as u32 + 1,
                    exports: true,
                });
            }
        }
        symbols
    }
}

impl LanguageParser for TypeScriptParser {
    fn language_id(&self) -> &'static str {
        "typescript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "tsx", "js", "jsx"]
    }

    fn lexical_kind_for(&self, node: &tree_sitter::Node, _src: &str) -> LexicalValueKind {
        match node.kind() {
            "function_declaration" => LexicalValueKind::Function,
            "lexical_declaration" => {
                if Self::lexical_decl_has_arrow(node) {
                    LexicalValueKind::ArrowFunction
                } else {
                    LexicalValueKind::Const
                }
            }
            "class_declaration" => {
                if Self::class_body_has_arrow_field(node) {
                    LexicalValueKind::ArrowFunction
                } else {
                    LexicalValueKind::Const
                }
            }
            "export_statement" => {
                // Unwrap: `export const X = () => {}` is a `lexical_declaration`
                // inside the export_statement; classify the inner declaration.
                if let Some(inner) = Self::find_declaration_child(node) {
                    self.lexical_kind_for(&inner, _src)
                } else {
                    LexicalValueKind::Const
                }
            }
            _ => LexicalValueKind::Const,
        }
    }

    fn extract_references(&self, node: &tree_sitter::Node, src: &str) -> Vec<Reference> {
        let mut refs: Vec<Reference> = Vec::new();
        let bytes = src.as_bytes();
        match node.kind() {
            "import_statement" => {
                // Walk the import_clause to find each import specifier.
                // TypeScript tree-sitter nests specifiers under:
                //   import_clause
                //     ├─ default_import        -> identifier  (`import React`)
                //     ├─ named_imports         -> {foo, bar}  (`import {foo, bar}`)
                //     └─ namespace_import      -> * as ns     (`import * as ns`)
                // Each leaf becomes a Reference with kind=Import.
                let clause: Option<tree_sitter::Node> = {
                    let mut c = node.walk();
                    let x = node.children(&mut c).find(|n| n.kind() == "import_clause");
                    x
                };
                if let Some(clause) = clause {
                    let leaves: Vec<tree_sitter::Node> = {
                        let mut cc = clause.walk();
                        let x = clause.children(&mut cc).collect::<Vec<_>>();
                        x
                    };
                    for leaf in leaves {
                        match leaf.kind() {
                            "default_import" | "identifier" => {
                                // `import React from 'react'` — React is the
                                // direct child identifier of default_import.
                                let name_node: Option<tree_sitter::Node> = if leaf.kind()
                                    == "default_import"
                                {
                                    let mut dc = leaf.walk();
                                    let x =
                                        leaf.children(&mut dc).find(|n| n.kind() == "identifier");
                                    x
                                } else {
                                    Some(leaf)
                                };
                                if let Some(nn) = name_node {
                                    if let Ok(name) = nn.utf8_text(bytes) {
                                        Self::push_spec_reference(
                                            &mut refs,
                                            leaf,
                                            name.to_string(),
                                            ReferenceKind::Import,
                                        );
                                    }
                                }
                            }
                            "named_imports" => {
                                let mut nc = leaf.walk();
                                for spec in leaf.children(&mut nc) {
                                    if spec.kind() != "import_specifier" {
                                        continue;
                                    }
                                    // `import { foo as bar }` -> use alias if present
                                    let name_node = spec
                                        .child_by_field_name("alias")
                                        .or_else(|| spec.child_by_field_name("name"));
                                    if let Some(nn) = name_node {
                                        if let Ok(name) = nn.utf8_text(bytes) {
                                            Self::push_spec_reference(
                                                &mut refs,
                                                spec,
                                                name.to_string(),
                                                ReferenceKind::Import,
                                            );
                                        }
                                    }
                                }
                            }
                            "namespace_import" => {
                                // `import * as ns from 'x'` -> "ns" is
                                // child identifier of namespace_import.
                                let mut nc = leaf.walk();
                                let id = leaf.children(&mut nc).find(|n| n.kind() == "identifier");
                                if let Some(nn) = id {
                                    if let Ok(name) = nn.utf8_text(bytes) {
                                        Self::push_spec_reference(
                                            &mut refs,
                                            leaf,
                                            name.to_string(),
                                            ReferenceKind::Import,
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "export_statement" => {
                // Emit one Reference for the exported declaration's name.
                if let Some(decl) = Self::find_declaration_child(node) {
                    if let Some(name) = Self::ts_declaration_name(&decl, bytes) {
                        let start = decl.start_position();
                        let end = decl.end_position();
                        refs.push(Reference {
                            file_id: String::new(), // filled by parse_all
                            kind: ReferenceKind::Export,
                            target_name: name.to_string(),
                            range: Range {
                                start_byte: decl.start_byte(),
                                end_byte: decl.end_byte(),
                                start_line: start.row as u32 + 1,
                                start_col: start.column as u32,
                                end_line: end.row as u32 + 1,
                                end_col: end.column as u32,
                            },
                        });
                    }
                }
            }
            _ => {}
        }
        refs
    }

    fn parse_all(&self, source: &str, path: &str, file_id: &str) -> ParseResult {
        let mut parser = Parser::new();
        let language = self.language_for_path(path);
        if parser.set_language(language).is_err() {
            return ParseResult::default();
        }

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return ParseResult::default(),
        };

        let mut result = ParseResult::default();
        let root = tree.root_node();
        let bytes = source.as_bytes();

        let mut cursor = root.walk();
        for node in root.children(&mut cursor) {
            let kind = node.kind();

            // ── Imports ────────────────────────────────────────────────────────
            if kind == "import_statement" {
                let source_file_id = file_id.to_string();
                let target_module = node
                    .child_by_field_name("source")
                    .and_then(|n| n.utf8_text(bytes).ok())
                    .map(|s| s.trim_matches(|c| c == '\'' || c == '"').to_string());

                let mut import_names: Vec<String> = vec![];
                if let Some(specs) = node.child_by_field_name("specifiers") {
                    let mut spec_cursor = specs.walk();
                    for spec in specs.children(&mut spec_cursor) {
                        if let Ok(name) = spec.utf8_text(bytes) {
                            import_names.push(name.to_string());
                        }
                    }
                }

                let is_default = import_names.iter().any(|n| n == "default");
                let is_type = source[node.start_byte()..node.end_byte()].contains("import type");

                result.imports.push(ImportInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_file_id,
                    target_file_id: None,
                    target_module,
                    imports: import_names,
                    is_default,
                    is_type,
                });
                // PR-B: emit Import references inline (single pass).
                let mut refs = self.extract_references(&node, source);
                for r in &mut refs {
                    r.file_id = file_id.to_string();
                }
                result.references.extend(refs);
                continue;
            }

            // ── Symbols (direct or wrapped in export_statement) ────────────────
            let direct = Self::ts_symbol_kind(kind);
            let declaration = if kind == "export_statement" {
                Self::find_declaration_child(&node)
            } else {
                None
            };

            let target_kind = direct.or_else(|| {
                declaration
                    .as_ref()
                    .and_then(|d| Self::ts_symbol_kind(d.kind()))
            });

            let sym_node = direct.map(|_| &node).or(declaration.as_ref());

            if let (Some(sk), Some(sym_node)) = (target_kind, sym_node) {
                if let Some(name) = Self::ts_declaration_name(sym_node, bytes) {
                    let start = sym_node.start_position();
                    let end = sym_node.end_position();
                    result.symbols.push(SymbolInfo {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.to_string(),
                        kind: sk,
                        file_id: file_id.to_string(),
                        line_start: start.row as u32 + 1,
                        line_end: end.row as u32 + 1,
                        exports: true,
                    });
                }
            }

            // ── Outline items (direct or wrapped in export_statement) ──────────
            let outline_kind = target_kind
                .and_then(Self::ts_symbol_kind_to_outline_kind)
                .or_else(|| {
                    declaration
                        .as_ref()
                        .and_then(|d| Self::ts_symbol_kind(d.kind()))
                        .and_then(Self::ts_symbol_kind_to_outline_kind)
                });

            if let (Some(ok), Some(on)) = (outline_kind, sym_node) {
                if let Some(name) = Self::ts_declaration_name(on, bytes) {
                    let start = on.start_position();
                    let end = on.end_position();
                    let id = make_outline_id(
                        file_id,
                        ok,
                        start.row as u32 + 1,
                        end.row as u32 + 1,
                        name,
                    );
                    let children = Self::extract_children(on, source, file_id, bytes);
                    result.outline.push(OutlineItem {
                        id,
                        file_id: file_id.to_string(),
                        name: name.to_string(),
                        kind: ok,
                        line_start: start.row as u32 + 1,
                        line_end: end.row as u32 + 1,
                        column_start: Some(start.column as u32),
                        column_end: Some(end.column as u32),
                        children,
                    });
                }
            }

            // ── PR-B: lexical kind (priority: ArrowFunction > Function > Const)
            let kind_for_node = self.lexical_kind_for(&node, source);
            match (result.lexical_kind, kind_for_node) {
                (_, LexicalValueKind::ArrowFunction) => {
                    result.lexical_kind = LexicalValueKind::ArrowFunction
                }
                (LexicalValueKind::Const, LexicalValueKind::Function) => {
                    result.lexical_kind = LexicalValueKind::Function
                }
                _ => {}
            }

            // ── PR-B: class field arrow symbol extraction ────────────────────
            // Cover both bare `class_declaration` and `export_statement`
            // wrapping a class so exported classes also emit ArrowFunction
            // sub-symbols for arrow-valued fields.
            if kind == "class_declaration" {
                let field_symbols = Self::collect_arrow_field_symbols(&node, bytes, file_id);
                for sym in field_symbols {
                    result.symbols.push(sym);
                }
            } else if kind == "export_statement" {
                if let Some(inner) = Self::find_declaration_child(&node) {
                    if inner.kind() == "class_declaration" {
                        let field_symbols =
                            Self::collect_arrow_field_symbols(&inner, bytes, file_id);
                        for sym in field_symbols {
                            result.symbols.push(sym);
                        }
                    }
                }
            }

            // ── PR-B: import/export reference extraction (single pass) ───────
            if kind == "import_statement" || kind == "export_statement" {
                let mut refs = self.extract_references(&node, source);
                for r in &mut refs {
                    r.file_id = file_id.to_string();
                }
                result.references.extend(refs);
            }
        }

        result
    }
}

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LexicalValueKind, ReferenceKind};

    fn ts_result(code: &str, extension: &str) -> ParseResult {
        let parser = TypeScriptParser::new();
        parser.parse_all(code, &format!("test.{extension}"), "file-ts")
    }

    // ── Export tests ─────────────────────────────────────────────────────────

    #[test]
    fn parse_export_function() {
        let code = r#"export function greet(name: string): string {
    return `Hello, ${name}`;
}"#;
        let result = ts_result(code, "ts");
        let has_greet = result.symbols.iter().any(|s| s.name == "greet")
            && result.outline.iter().any(|o| o.name == "greet");
        assert!(
            has_greet,
            "Expected 'greet' in symbols and outline, got symbols={:?}, outline={:?}",
            result.symbols.iter().map(|s| &s.name).collect::<Vec<_>>(),
            result.outline.iter().map(|o| &o.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn parse_export_class_with_methods() {
        let code = r#"export class OrderService {
    create(order: Order): void {}
    cancel(id: string): void {}
}"#;
        let result = ts_result(code, "ts");
        assert!(
            result.symbols.iter().any(|s| s.name == "OrderService"),
            "Expected 'OrderService' symbol, got {:#?}",
            result.symbols
        );
        let class_outline = result.outline.iter().find(|o| o.name == "OrderService");
        assert!(
            class_outline.is_some(),
            "Expected 'OrderService' in outline, got {:#?}",
            result.outline
        );
        let class_outline = class_outline.unwrap();
        assert!(
            !class_outline.children.is_empty(),
            "Expected methods in children, got {:#?}",
            class_outline.children
        );
        let method_names: Vec<_> = class_outline
            .children
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert!(
            method_names.contains(&"create".to_string())
                || method_names.contains(&"cancel".to_string()),
            "Expected 'create' or 'cancel' in methods, got {:?}",
            method_names
        );
    }

    #[test]
    fn parse_export_const_arrow_function() {
        // Plain TS (not TSX — TypeScriptParser uses tsx grammar which handles JSX)
        let code = r#"export const Component001 = (props: Props) => {
    return null;
}"#;
        let result = ts_result(code, "ts");
        assert!(
            result.symbols.iter().any(|s| s.name == "Component001"),
            "Expected 'Component001' symbol, got {:#?}",
            result.symbols
        );
        assert!(
            result.outline.iter().any(|o| o.name == "Component001"),
            "Expected 'Component001' outline item, got {:#?}",
            result.outline
        );
    }

    #[test]
    fn parse_export_const_object() {
        let code = r#"export const CONFIG = { key: 'value', debug: true }"#;
        let result = ts_result(code, "ts");
        assert!(
            result.symbols.iter().any(|s| s.name == "CONFIG"),
            "Expected 'CONFIG' symbol, got {:#?}",
            result.symbols
        );
    }

    #[test]
    fn parse_export_interface() {
        let code = r#"export interface UserProfile {
    id: string;
    name: string;
}"#;
        let result = ts_result(code, "ts");
        assert!(
            result.symbols.iter().any(|s| s.name == "UserProfile"),
            "Expected 'UserProfile' symbol, got {:#?}",
            result.symbols
        );
        assert!(
            result.outline.iter().any(|o| o.name == "UserProfile"),
            "Expected 'UserProfile' outline item, got {:#?}",
            result.outline
        );
    }

    #[test]
    fn parse_export_default_function() {
        let code = r#"export default function App() { return null; }"#;
        let result = ts_result(code, "tsx");
        let symbols_names: Vec<_> = result.symbols.iter().map(|s| s.name.clone()).collect();
        let outline_names: Vec<_> = result.outline.iter().map(|o| o.name.clone()).collect();
        assert!(
            symbols_names.contains(&"App".to_string()),
            "Expected 'App' in symbols, got {:#?}",
            symbols_names
        );
        assert!(
            outline_names.contains(&"App".to_string()),
            "Expected 'App' in outline, got {:#?}",
            outline_names
        );
    }

    #[test]
    fn parse_export_type_alias() {
        let code =
            r#"export type Result<T> = { data: T; error: null } | { data: null; error: string }"#;
        let result = ts_result(code, "ts");
        assert!(
            result.symbols.iter().any(|s| s.name == "Result"),
            "Expected 'Result' symbol, got {:#?}",
            result.symbols
        );
    }

    #[test]
    fn parse_mixed_exported_and_local() {
        let code = r#"interface BaseConfig { url: string }
export class Service {}
export function parse() {}"#;
        let result = ts_result(code, "ts");
        let names: Vec<_> = result.symbols.iter().map(|s| s.name.clone()).collect();
        assert!(
            names.len() >= 3,
            "Expected at least 3 symbols (BaseConfig, Service, parse), got {:#?}",
            names
        );
        assert!(
            names.contains(&"BaseConfig".to_string()),
            "Missing 'BaseConfig', got {:#?}",
            names
        );
        assert!(
            names.contains(&"Service".to_string()),
            "Missing 'Service', got {:#?}",
            names
        );
        assert!(
            names.contains(&"parse".to_string()),
            "Missing 'parse', got {:#?}",
            names
        );
    }

    #[test]
    fn parse_imports_still_work_after_export_handling() {
        let code = r#"import { useState } from 'react';
import type { User } from './types';"#;
        let result = ts_result(code, "ts");
        assert_eq!(
            result.imports.len(),
            2,
            "Expected 2 imports, got {:#?}",
            result.imports
        );
        assert!(
            result
                .imports
                .iter()
                .any(|i| i.target_module.as_deref() == Some("react")),
            "Expected 'react' import, got {:#?}",
            result.imports
        );
        assert!(
            result.imports.iter().any(|i| i.is_type),
            "Expected type import, got {:#?}",
            result.imports
        );
    }

    #[test]
    fn parse_tsx_with_import_and_export_default() {
        let code = r#"import React, { useState } from 'react';
export default function App() { return <div />; }"#;
        let result = ts_result(code, "tsx");
        assert!(
            !result.outline.is_empty() || !result.imports.is_empty(),
            "Expected non-empty outline or imports, got outline={:#?} imports={:#?}",
            result.outline,
            result.imports
        );
    }

    #[test]
    fn parse_ts_line_ranges_populated() {
        let code = "export class Foo { method() {} }\n";
        let result = ts_result(code, "ts");
        for item in &result.outline {
            assert!(item.line_start > 0, "line_start should be > 0");
            assert!(
                item.line_end >= item.line_start,
                "line_end {} >= line_start {}",
                item.line_end,
                item.line_start
            );
        }
    }

    // ── Original non-export tests (preserved) ───────────────────────────────

    #[test]
    fn parse_ts_class_with_methods() {
        let code = r#"
class UserService {
    constructor() {}
    getUser(id: string): User | null { return null; }
    saveUser(user: User): void {}
}
"#;
        let result = ts_result(code, "ts");
        assert!(result.symbols.iter().any(|s| s.name == "UserService"));
        let class_item = result.outline.iter().find(|o| o.name == "UserService");
        assert!(class_item.is_some(), "Expected class in outline");
        let class_item = class_item.unwrap();
        assert_eq!(class_item.kind, OutlineItemKind::Class);
        assert!(
            !class_item.children.is_empty(),
            "Expected method children, got {:?}",
            class_item.children
        );
    }

    #[test]
    fn parse_ts_interface_with_properties() {
        let code = r#"
interface UserDto {
    id: string;
    name: string;
    email: string;
}
"#;
        let result = ts_result(code, "ts");
        assert!(result.symbols.iter().any(|s| s.name == "UserDto"));
        let interface_item = result.outline.iter().find(|o| o.name == "UserDto");
        assert!(interface_item.is_some(), "Expected interface in outline");
    }

    #[test]
    fn parse_ts_function_and_enum() {
        let code = r#"
function parseUser(input: string): User {
    return {} as User;
}
enum Status { Active, Inactive, Pending }
"#;
        let result = ts_result(code, "ts");
        assert!(result.symbols.iter().any(|s| s.name == "parseUser"));
        assert!(result.symbols.iter().any(|s| s.name == "Status"));
        assert!(result.outline.iter().any(|o| o.name == "parseUser"));
        assert!(result.outline.iter().any(|o| o.name == "Status"));
    }

    #[test]
    fn parse_ts_imports() {
        let code = r#"import { useState } from "react";
import type { User } from "./types";"#;
        let result = ts_result(code, "ts");
        assert_eq!(result.imports.len(), 2);
        assert!(result
            .imports
            .iter()
            .any(|i| i.target_module.as_deref() == Some("react")));
        assert!(result.imports.iter().any(|i| i.is_type));
    }

    #[test]
    fn parse_ts_line_ranges_are_populated() {
        let code = "class Foo {\n    method() {}\n}\n";
        let result = ts_result(code, "ts");
        for item in &result.outline {
            assert!(item.line_start > 0, "line_start should be > 0");
            assert!(
                item.line_end >= item.line_start,
                "line_end {} should be >= line_start {}",
                item.line_end,
                item.line_start
            );
        }
    }

    #[test]
    fn parse_js_arrow_functions_not_in_outline() {
        // Arrow functions (const name = () => {}) produce lexical_declaration with
        // Const kind. Symbols are detected but OutlineItemKind::Const maps from
        // SymbolKind::Const, so arrow functions ARE in the outline.
        // This test checks that we handle both correctly.
        let code = "const fn = () => {};";
        let result = ts_result(code, "js");
        // Arrow const: should have symbol (name extracted from lexical_declaration)
        assert!(
            result.symbols.iter().any(|s| s.name == "fn"),
            "Expected 'fn' symbol from arrow const, got {:#?}",
            result.symbols
        );
        // Outline may or may not include const — depends on mapping
        // (we don't assert on outline emptiness here since Const kind IS mapped)
    }

    // ── PR-B: lexical kind detection (RED tests) ────────────────────────────

    fn fixture(name: &str) -> String {
        use std::fs;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("typescript")
            .join(name);
        fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e))
    }

    #[test]
    fn lexical_kind_arrow_field_is_arrow_function() {
        // B.1 RED: a class field with an arrow function value should yield
        // LexicalValueKind::ArrowFunction for the file (top-level binding).
        let code = fixture("arrow_field.ts");
        let result = ts_result(&code, "ts");
        assert_eq!(
            result.lexical_kind,
            LexicalValueKind::ArrowFunction,
            "expected ArrowFunction for arrow field, got {:?}",
            result.lexical_kind
        );
    }

    #[test]
    fn lexical_kind_object_literal_is_const() {
        // B.1 RED baseline: object literal must be Const, even when the object
        // contains a method-style arrow property.
        let code = fixture("object_literal.ts");
        let result = ts_result(&code, "ts");
        assert_eq!(
            result.lexical_kind,
            LexicalValueKind::Const,
            "expected Const for object literal, got {:?}",
            result.lexical_kind
        );
    }

    #[test]
    fn lexical_kind_react_const_arrow_is_arrow_function() {
        // B.1 RED: a `const Component = (...) => <div/>` lexical_declaration
        // with a JSX arrow body should be classified as ArrowFunction.
        let code = fixture("react_const_arrow.tsx");
        let result = ts_result(&code, "tsx");
        assert_eq!(
            result.lexical_kind,
            LexicalValueKind::ArrowFunction,
            "expected ArrowFunction for react const arrow, got {:?}",
            result.lexical_kind
        );
    }

    #[test]
    fn lexical_kind_function_declaration_is_function() {
        // B.1 RED: a `function foo() {}` declaration must yield Function,
        // not the default Const.
        let code = "function foo() {}\n";
        let result = ts_result(code, "ts");
        assert_eq!(
            result.lexical_kind,
            LexicalValueKind::Function,
            "expected Function for function_declaration, got {:?}",
            result.lexical_kind
        );
    }

    #[test]
    fn lexical_kind_arrow_class_method_field_emits_arrow_function_symbol() {
        // B.2 contract: the `handler` field inside a class with an arrow
        // initializer must produce a `SymbolKind::ArrowFunction` symbol.
        // This pins SymbolKind differentiation separately from LexicalValueKind.
        let code = fixture("arrow_field.ts");
        let result = ts_result(&code, "ts");
        let handler = result
            .symbols
            .iter()
            .find(|s| s.name == "handler")
            .expect("expected 'handler' symbol for class field arrow");
        assert_eq!(
            handler.kind,
            SymbolKind::ArrowFunction,
            "expected ArrowFunction symbol kind, got {:?}",
            handler.kind
        );
    }

    #[test]
    fn exported_class_with_arrow_field_emits_arrow_function_sub_symbol() {
        // Reviewer-noted minor gap: an `export class Svc { handler = (req) => ... }`
        // must also emit a `SymbolKind::ArrowFunction` sub-symbol for the
        // arrow-valued field, not just set the file's `lexical_kind`.
        let code = fixture("exported_class_arrow.ts");
        let result = ts_result(&code, "ts");
        let handler = result
            .symbols
            .iter()
            .find(|s| s.name == "handler")
            .expect("expected 'handler' symbol for exported class arrow field");
        assert_eq!(
            handler.kind,
            SymbolKind::ArrowFunction,
            "expected ArrowFunction symbol kind for exported class field, got {:?}",
            handler.kind
        );
        // Sanity: the class itself is still a Class symbol.
        let svc = result
            .symbols
            .iter()
            .find(|s| s.name == "Svc")
            .expect("expected 'Svc' class symbol");
        assert_eq!(
            svc.kind,
            SymbolKind::Class,
            "expected Class symbol kind for exported class, got {:?}",
            svc.kind
        );
    }

    // ── PR-B: import/export Reference emission (RED tests) ───────────────────

    #[test]
    fn reference_import_emits_target_name() {
        // B.3 RED: `import { foo } from './bar'` must emit a Reference
        // with kind=Import and target_name="foo".
        let code = r#"import { foo } from './bar';"#;
        let result = ts_result(code, "ts");
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Import && r.target_name == "foo"),
            "expected import reference for 'foo', got: {:#?}",
            result.references
        );
    }

    #[test]
    fn reference_import_default_emits_target_name() {
        // B.3 RED: `import React from 'react'` must emit a Reference
        // with kind=Import and target_name="React".
        let code = r#"import React from 'react';"#;
        let result = ts_result(code, "ts");
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Import && r.target_name == "React"),
            "expected import reference for 'React', got: {:#?}",
            result.references
        );
    }

    #[test]
    fn reference_export_emits_target_name() {
        // B.3 RED: `export function greet() {}` must emit a Reference
        // with kind=Export and target_name="greet".
        let code = "export function greet() {}\n";
        let result = ts_result(code, "ts");
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Export && r.target_name == "greet"),
            "expected export reference for 'greet', got: {:#?}",
            result.references
        );
    }

    #[test]
    fn reference_export_arrow_emits_target_name() {
        // B.3 RED: `export const Card = () => ...` must emit a Reference
        // with kind=Export and target_name="Card".
        let code = fixture("react_const_arrow.tsx");
        let result = ts_result(&code, "tsx");
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Export && r.target_name == "Card"),
            "expected export reference for 'Card', got: {:#?}",
            result.references
        );
    }

    #[test]
    fn reference_file_id_is_populated() {
        // B.3 contract: file_id MUST be set in parse_all (the trait hook
        // returns empty; parse_all fills it in).
        let code = r#"import { foo } from './bar';"#;
        let result = ts_result(code, "ts");
        for r in &result.references {
            assert!(
                !r.file_id.is_empty(),
                "reference file_id must be set, got empty for {:?}",
                r
            );
        }
    }

    #[test]
    fn single_pass_populates_all_ir_categories() {
        // B.4: a single parse_all call must populate symbols, outline,
        // lexical_kind, and references. If any category is missing the
        // parser has done a second AST pass (or skipped a category).
        let code = r#"
import { foo } from './bar';
export const Card = ({title}) => <div>{title}</div>;
export function greet() {}
"#;
        let result = ts_result(code, "tsx");
        assert!(
            !result.imports.is_empty(),
            "imports must be populated: {:#?}",
            result.imports
        );
        assert!(
            !result.symbols.is_empty(),
            "symbols must be populated: {:#?}",
            result.symbols
        );
        assert!(
            !result.outline.is_empty(),
            "outline must be populated: {:#?}",
            result.outline
        );
        assert!(
            !result.references.is_empty(),
            "references must be populated: {:#?}",
            result.references
        );
        assert_eq!(
            result.lexical_kind,
            LexicalValueKind::ArrowFunction,
            "lexical_kind must be ArrowFunction for const Card arrow"
        );
        // Spot-check that import / export reference targets are present.
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Import && r.target_name == "foo"),
            "expected import reference for 'foo'"
        );
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Export && r.target_name == "Card"),
            "expected export reference for 'Card'"
        );
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Export && r.target_name == "greet"),
            "expected export reference for 'greet'"
        );
    }
}
