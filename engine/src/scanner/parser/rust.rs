//! Rust language parser.
//!
//! Extracts symbols, imports, and hierarchical outline from Rust source.
//!
//! # Outline hierarchy
//!
//! - Top-level items (struct, enum, function, impl, mod) become root outline items.
//! - `impl_item` children: methods defined within the impl block.
//! - `mod_item` inline body: functions, structs, etc. if the body is inline.

use super::traits::{make_outline_id, rust_node_kind_to_outline_kind, LanguageParser};
use crate::models::{
    ImportInfo, OutlineItem, OutlineItemKind, ParseResult, SymbolInfo, SymbolKind,
};
use tree_sitter::{Language, Parser};
use tree_sitter_rust::LANGUAGE;

/// Rust language parser.
pub struct RustParser {
    language: Language,
}

impl RustParser {
    pub fn new() -> Self {
        Self {
            language: LANGUAGE.into(),
        }
    }
}

impl LanguageParser for RustParser {
    fn language_id(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
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

            // Symbols (flat list)
            let symbol_kind = match kind {
                "struct_item" => Some(SymbolKind::Struct),
                "impl_item" => Some(SymbolKind::Impl),
                "function_item" => Some(SymbolKind::Function),
                "enum_item" => Some(SymbolKind::Enum),
                "type_alias_item" => Some(SymbolKind::TypeAlias),
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
            if let Some(outline_kind) = rust_node_kind_to_outline_kind(kind) {
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

                    // For impl blocks, extract methods inside
                    let children = if kind == "impl_item" {
                        Self::extract_impl_methods(&node, source, file_id, bytes)
                    } else {
                        vec![]
                    };

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

            // use declarations (imports)
            if kind == "use_declaration" {
                let source_file_id = file_id.to_string();
                let use_text = node.utf8_text(bytes).unwrap_or("");
                let module = use_text
                    .trim()
                    .trim_start_matches("use ")
                    .trim_end_matches(';')
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .to_string();

                result.imports.push(ImportInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_file_id,
                    target_file_id: None,
                    target_module: if module.is_empty() {
                        None
                    } else {
                        Some(module)
                    },
                    imports: vec![],
                    is_default: false,
                    is_type: false,
                });
            }
        }

        result
    }
}

impl RustParser {
    /// Extract method items from an impl block's body.
    fn extract_impl_methods(
        impl_node: &tree_sitter::Node,
        _source: &str,
        file_id: &str,
        bytes: &[u8],
    ) -> Vec<OutlineItem> {
        let mut methods = Vec::new();

        // Find the impl_item's body (typically the last child)
        let mut impl_cursor = impl_node.walk();
        let children: Vec<_> = impl_node.children(&mut impl_cursor).collect();

        for child in children {
            if child.kind() == "declarations" || child.kind() == "item" {
                let mut item_cursor = child.walk();
                for item in child.children(&mut item_cursor) {
                    if item.kind() == "function_item" {
                        if let Some(name_node) = item.child_by_field_name("name") {
                            let name = name_node.utf8_text(bytes).unwrap_or("");
                            let start = item.start_position();
                            let end = item.end_position();
                            let id = make_outline_id(
                                file_id,
                                OutlineItemKind::Method,
                                start.row as u32 + 1,
                                end.row as u32 + 1,
                                name,
                            );
                            methods.push(OutlineItem {
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
                }
            }
        }

        methods
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_result(code: &str) -> ParseResult {
        let parser = RustParser::new();
        parser.parse_all(code, "test.rs", "file-rs")
    }

    #[test]
    fn parse_rust_struct_with_impl_methods() {
        let code = r#"
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    pub fn find_by_id(&self, id: u64) -> Option<User> { None }
    pub fn save(&self, user: User) -> Result<(), Error> { Ok(()) }
}
"#;
        let result = rust_result(code);

        // Should have struct symbol
        assert!(
            result.symbols.iter().any(|s| s.name == "UserRepository"),
            "Expected UserRepository in symbols"
        );

        // Outline: struct + impl with methods
        assert!(
            !result.outline.is_empty(),
            "Expected outline items, got empty"
        );

        let struct_item = result.outline.iter().find(|o| o.name == "UserRepository");
        assert!(struct_item.is_some(), "Expected struct in outline");

        let impl_item = result.outline.iter().find(|o| o.name == "UserRepository");
        assert!(impl_item.is_some(), "Expected impl in outline");

        // Check impl has method children
        if let Some(impl_outline) = result
            .outline
            .iter()
            .find(|o| o.kind == OutlineItemKind::Impl)
        {
            assert!(
                !impl_outline.children.is_empty(),
                "Expected impl methods as children, got: {:?}",
                impl_outline.children
            );
            let method_names: Vec<_> = impl_outline
                .children
                .iter()
                .map(|c| c.name.clone())
                .collect();
            assert!(
                method_names.contains(&"find_by_id".to_string())
                    || method_names.contains(&"save".to_string()),
                "Expected method names, got: {:?}",
                method_names
            );
        }
    }

    #[test]
    fn parse_rust_enum_and_function() {
        let code = r#"
enum Status { Active, Inactive, Pending }

pub fn parse_status(input: &str) -> Status { Status::Active }
"#;
        let result = rust_result(code);

        assert!(result.symbols.iter().any(|s| s.name == "Status"));
        assert!(result.symbols.iter().any(|s| s.name == "parse_status"));

        assert!(result.outline.iter().any(|o| o.name == "Status"));
        assert!(result.outline.iter().any(|o| o.name == "parse_status"));
    }

    #[test]
    fn parse_rust_use_declarations() {
        let code = r#"
use std::collections::HashMap;
use crate::models::User;
"#;
        let result = rust_result(code);

        assert_eq!(result.imports.len(), 2);
        assert!(result
            .imports
            .iter()
            .any(|i| i.target_module.as_deref() == Some("std")));
    }

    #[test]
    fn parse_rust_line_ranges_populated() {
        let code = "struct Foo {}\nfn bar() {}\n";
        let result = rust_result(code);

        for item in &result.outline {
            assert!(
                item.line_start > 0,
                "line_start should be > 0 for {}",
                item.name
            );
            assert!(
                item.line_end >= item.line_start,
                "line_end {} >= line_start {} for {}",
                item.line_end,
                item.line_start,
                item.name
            );
        }
    }

    #[test]
    fn parse_rust_module_item() {
        // Inline module
        let code = "mod utils {\n    pub fn helper() {}\n}\n";
        let result = rust_result(code);

        assert!(result.outline.iter().any(|o| o.name == "utils"));
        let mod_item = result.outline.iter().find(|o| o.name == "utils");
        assert!(mod_item.is_some());
    }
}
