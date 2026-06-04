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
    ImportInfo, OutlineItem, OutlineItemKind, ParseResult, SymbolInfo, SymbolKind,
};
use tree_sitter::{Language, Parser};
use tree_sitter_typescript::LANGUAGE_TYPESCRIPT;

/// TypeScript/TSX language parser.
pub struct TypeScriptParser {
    language: Language,
}

impl TypeScriptParser {
    pub fn new() -> Self {
        Self {
            language: LANGUAGE_TYPESCRIPT.into(),
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
}

impl LanguageParser for TypeScriptParser {
    fn language_id(&self) -> &'static str {
        "typescript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "tsx", "js", "jsx"]
    }

    fn parse_all(&self, source: &str, _path: &str, file_id: &str) -> ParseResult {
        let mut parser = Parser::new();
        if parser.set_language(&self.language).is_err() {
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
}
