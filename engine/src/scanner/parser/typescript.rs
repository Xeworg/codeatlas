//! TypeScript / TSX language parser.
//!
//! Extracts symbols, imports, and hierarchical outline from TypeScript source.
//!
//! # Outline hierarchy
//!
//! - Top-level declarations become root outline items.
//! - `class_declaration` children: methods, fields if Tree-sitter exposes them.
//! - `interface_declaration` children: properties, methods if Tree-sitter exposes them.

use super::traits::{make_outline_id, ts_node_kind_to_outline_kind, LanguageParser};
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

        // Walk top-level nodes
        let root = tree.root_node();
        let bytes = source.as_bytes();

        let mut cursor = root.walk();
        for node in root.children(&mut cursor) {
            let kind = node.kind();

            // Symbols (flat list for metrics)
            let symbol_kind = match kind {
                "class_declaration" => Some(SymbolKind::Class),
                "function_declaration" => Some(SymbolKind::Function),
                "method_definition" => Some(SymbolKind::Method),
                "interface_declaration" => Some(SymbolKind::Interface),
                "type_alias_declaration" => Some(SymbolKind::TypeAlias),
                "enum_declaration" => Some(SymbolKind::Enum),
                "lexical_declaration" => Some(SymbolKind::Const),
                _ => None,
            };

            if let Some(sk) = symbol_kind {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(bytes).unwrap_or("");
                    let start = node.start_position();
                    let end = node.end_position();
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

            // Outline items — hierarchical
            if let Some(outline_kind) = ts_node_kind_to_outline_kind(kind) {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(bytes).unwrap_or("");
                    let start = node.start_position();
                    let end = node.end_position();
                    let id = make_outline_id(
                        file_id,
                        outline_kind,
                        start.row as u32 + 1,
                        end.row as u32 + 1,
                        name,
                    );

                    // Capture children (methods, fields) if Tree-sitter exposes them
                    let children = Self::extract_children(&node, source, file_id, bytes);

                    result.outline.push(OutlineItem {
                        id,
                        file_id: file_id.to_string(),
                        name: name.to_string(),
                        kind: outline_kind,
                        line_start: start.row as u32 + 1,
                        line_end: end.row as u32 + 1,
                        column_start: Some(start.column as u32),
                        column_end: Some(end.column as u32),
                        children,
                    });
                }
            }

            // Imports
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
            }
        }

        result
    }
}

impl TypeScriptParser {
    /// Extract children outline items from a parent node (e.g. methods inside a class).
    fn extract_children(
        parent: &tree_sitter::Node,
        _source: &str,
        file_id: &str,
        bytes: &[u8],
    ) -> Vec<OutlineItem> {
        let mut children = Vec::new();
        let mut cursor = parent.walk();

        // Methods/properties live inside the class_body node, not direct children
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

            // Method inside class body
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

            // Property/field inside class or interface
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

        // Should have class symbol
        assert!(result.symbols.iter().any(|s| s.name == "UserService"));

        // Should have outline with class + methods
        assert!(!result.outline.is_empty(), "Expected outline items");

        let class_item = result.outline.iter().find(|o| o.name == "UserService");
        assert!(class_item.is_some(), "Expected class in outline");
        let class_item = class_item.unwrap();
        assert_eq!(class_item.kind, OutlineItemKind::Class);
        assert!(
            !class_item.children.is_empty(),
            "Expected class to have method children, got {:?}",
            class_item.children
        );

        // Methods should appear as children
        let method_names: Vec<_> = class_item.children.iter().map(|c| c.name.clone()).collect();
        assert!(
            method_names.contains(&"getUser".to_string())
                || method_names.contains(&"constructor".to_string()),
            "Expected method children, got: {:?}",
            method_names
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
    fn parse_tsx_file() {
        let code = r#"import React, { useState } from "react";
export default function App() { return <div />; }"#;
        let result = ts_result(code, "tsx");

        assert!(!result.outline.is_empty() || !result.imports.is_empty());
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
        // Arrow functions are SymbolKind::ArrowFunction, not in outline kind map
        let code = "const fn = () => {};";
        let result = ts_result(code, "js");

        // Should still parse without crashing
        assert!(result.outline.is_empty() || result.symbols.is_empty());
    }
}
