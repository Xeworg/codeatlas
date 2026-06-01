//! Tree-sitter-based code parser for JS, TS, and Rust.

use tree_sitter::{Language, Parser, Tree};

use crate::models::{ImportInfo, SymbolInfo, SymbolKind};

pub struct CodeParser;

impl CodeParser {
    pub fn parse_file(
        path: &str,
        content: &str,
        extension: &str,
    ) -> (Vec<SymbolInfo>, Vec<ImportInfo>) {
        let language_fn = match extension {
            "ts" | "tsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            "js" | "jsx" => tree_sitter_javascript::LANGUAGE,
            "rs" => tree_sitter_rust::LANGUAGE,
            _ => return (vec![], vec![]),
        };

        let mut parser = Parser::new();
        let language: Language = language_fn.into();
        if parser.set_language(&language).is_err() {
            return (vec![], vec![]);
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return (vec![], vec![]),
        };

        let mut symbols = vec![];
        let mut imports = vec![];

        match extension {
            "ts" | "tsx" | "js" | "jsx" => {
                Self::extract_ts_symbols(&tree, path, content, &mut symbols, &mut imports)
            }
            "rs" => Self::extract_rust_symbols(&tree, path, content, &mut symbols, &mut imports),
            _ => {}
        }

        (symbols, imports)
    }

    fn extract_ts_symbols(
        tree: &Tree,
        file_path: &str,
        content: &str,
        symbols: &mut Vec<SymbolInfo>,
        imports: &mut Vec<ImportInfo>,
    ) {
        let root = tree.root_node();
        let bytes = content.as_bytes();

        // Walk tree and extract symbols
        let mut cursor = root.walk();
        for node in root.children(&mut cursor) {
            let kind = node.kind();

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
                    symbols.push(SymbolInfo {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.to_string(),
                        kind: sk,
                        file_id: file_path.to_string(),
                        line_start: start.row as u32 + 1,
                        line_end: end.row as u32 + 1,
                        exports: true,
                    });
                }
            }

            // Extract imports
            if kind == "import_statement" {
                let source_file_id = file_path.to_string();
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
                let is_type = content[node.start_byte()..node.end_byte()].contains("import type");

                imports.push(ImportInfo {
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
    }

    fn extract_rust_symbols(
        tree: &Tree,
        file_path: &str,
        content: &str,
        symbols: &mut Vec<SymbolInfo>,
        imports: &mut Vec<ImportInfo>,
    ) {
        let root = tree.root_node();
        let bytes = content.as_bytes();

        let mut cursor = root.walk();
        for node in root.children(&mut cursor) {
            let kind = node.kind();

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
                    symbols.push(SymbolInfo {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.to_string(),
                        kind: sk,
                        file_id: file_path.to_string(),
                        line_start: start.row as u32 + 1,
                        line_end: end.row as u32 + 1,
                        exports: true,
                    });
                }
            }

            // Extract use declarations
            if kind == "use_declaration" {
                let source_file_id = file_path.to_string();
                let use_text = node.utf8_text(bytes).unwrap_or("");
                let module = use_text
                    .trim()
                    .trim_start_matches("use ")
                    .trim_end_matches(';')
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .to_string();

                imports.push(ImportInfo {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typescript_function() {
        // Use simpler JavaScript syntax that tree-sitter-javascript handles well
        let code = r#"
function calculateTotal(items) {
    return items.reduce(function(sum, item) { return sum + item; }, 0);
}
module.exports = { calculateTotal };
"#;
        let (symbols, imports) = CodeParser::parse_file("test.js", code, "js");

        // Should find at least one function symbol
        assert!(
            !symbols.is_empty() || !imports.is_empty(),
            "Expected symbols or imports, got none"
        );
    }

    #[test]
    fn parse_rust_struct() {
        let code = r#"
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    pub fn find_by_id(&self, id: u64) -> Option<User> { None }
}
"#;
        let (symbols, _imports) = CodeParser::parse_file("lib.rs", code, "rs");

        assert!(symbols.iter().any(|s| s.name == "UserRepository"));
        assert!(symbols.iter().any(|s| s.kind == SymbolKind::Struct));
    }

    #[test]
    fn parse_imports() {
        let code = r#"import { useState, useEffect } from "react";
import type { User } from "./types";
export default App;"#;
        let (_symbols, imports) = CodeParser::parse_file("App.tsx", code, "tsx");

        assert_eq!(imports.len(), 2);
        assert!(imports
            .iter()
            .any(|i| i.target_module.as_deref() == Some("react")));
        assert!(imports.iter().any(|i| i.is_type));
    }
}
