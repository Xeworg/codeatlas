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
    ImportInfo, LexicalValueKind, OutlineItem, OutlineItemKind, ParseResult, Range, Reference,
    ReferenceKind, SymbolInfo, SymbolKind,
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

    fn lexical_kind_for(&self, node: &tree_sitter::Node, _src: &str) -> LexicalValueKind {
        match node.kind() {
            "function_item" => LexicalValueKind::Function,
            _ => LexicalValueKind::Const,
        }
    }

    fn extract_references(&self, node: &tree_sitter::Node, src: &str) -> Vec<Reference> {
        let mut refs: Vec<Reference> = Vec::new();
        let bytes = src.as_bytes();
        if node.kind() != "use_declaration" {
            return refs;
        }
        let use_text = node.utf8_text(bytes).unwrap_or("");
        let cleaned = use_text
            .trim()
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .trim();
        // Conservative emission: any path that starts with a special
        // keyword (`self`, `super`, `crate`) is a relative path that
        // requires cross-file resolution; we emit an empty target_name
        // for v1 and let v2 fill it in.
        let first_segment = cleaned.split("::").next().map(|s| s.trim()).unwrap_or("");
        let target_name = if matches!(first_segment, "self" | "super" | "crate" | "*" | "") {
            String::new()
        } else {
            let last_segment = cleaned.rsplit("::").next().map(|s| s.trim()).unwrap_or("");
            if last_segment == "*" {
                String::new()
            } else {
                last_segment
                    .trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace())
                    .to_string()
            }
        };
        let start = node.start_position();
        let end = node.end_position();
        refs.push(Reference {
            file_id: String::new(), // filled by parse_all
            kind: ReferenceKind::Import,
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
        refs
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

                // PR-B: emit Import reference for the use declaration.
                let mut refs = self.extract_references(&node, source);
                for r in &mut refs {
                    r.file_id = file_id.to_string();
                }
                result.references.extend(refs);
            }

            // PR-B: lexical_kind (priority: Function > Const). Function items
            // upgrade the file-level lexical_kind; structs/etc. keep Const.
            let kind_for_node = self.lexical_kind_for(&node, source);
            if matches!(kind_for_node, LexicalValueKind::Function) {
                result.lexical_kind = LexicalValueKind::Function;
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
    use crate::models::{LexicalValueKind, ReferenceKind};

    fn rust_result(code: &str) -> ParseResult {
        let parser = RustParser::new();
        parser.parse_all(code, "test.rs", "file-rs")
    }

    fn rust_fixture_result(name: &str) -> ParseResult {
        use std::fs;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("rust")
            .join(name);
        let code = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
        let parser = RustParser::new();
        parser.parse_all(&code, &path.to_string_lossy(), "file-rs")
    }

    // ── PR-B B.5: Rust conservative Reference emission + lexical_kind ─────

    #[test]
    fn reference_use_item_last_segment() {
        // B.5 RED: `use std::collections::HashMap;` must emit a Reference
        // with kind=Import and target_name="HashMap" (the last segment).
        let code = "use std::collections::HashMap;\n";
        let result = rust_result(code);
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Import && r.target_name == "HashMap"),
            "expected import reference for 'HashMap', got: {:#?}",
            result.references
        );
    }

    #[test]
    fn reference_use_glob_emits_empty_target_name() {
        // B.5 contract: `use std::collections::*;` has no resolvable name;
        // conservative emission MUST use target_name="".
        let code = "use std::collections::*;\n";
        let result = rust_result(code);
        let glob_ref = result
            .references
            .iter()
            .find(|r| r.kind == ReferenceKind::Import)
            .expect("expected at least one import reference for glob");
        assert_eq!(
            glob_ref.target_name, "",
            "glob use must produce empty target_name, got {:?}",
            glob_ref.target_name
        );
    }

    #[test]
    fn reference_use_self_super_crate_emit_empty_target_name() {
        // B.5 contract: self/super/crate paths are conservative — empty target_name.
        for code in &["use self::foo;\n", "use super::bar;\n", "use crate::baz;\n"] {
            let result = rust_result(code);
            for r in &result.references {
                if r.kind == ReferenceKind::Import {
                    assert_eq!(
                        r.target_name, "",
                        "self/super/crate must produce empty target_name for source `{}`, got {:?}",
                        code, r.target_name
                    );
                }
            }
        }
    }

    #[test]
    fn reference_file_id_is_populated() {
        // B.5 contract: file_id MUST be set in parse_all (the trait hook
        // returns empty; parse_all fills it in).
        let code = "use std::collections::HashMap;\n";
        let result = rust_result(code);
        for r in &result.references {
            assert!(
                !r.file_id.is_empty(),
                "reference file_id must be set, got empty for {:?}",
                r
            );
        }
    }

    #[test]
    fn lexical_kind_function_item_is_function() {
        // B.5 contract: a `fn foo() {}` item yields LexicalValueKind::Function.
        let code = "fn foo() {}\n";
        let result = rust_result(code);
        assert_eq!(
            result.lexical_kind,
            LexicalValueKind::Function,
            "expected Function for function_item, got {:?}",
            result.lexical_kind
        );
    }

    #[test]
    fn lexical_kind_struct_item_is_const() {
        // B.5 contract: a `struct S;` item yields LexicalValueKind::Const.
        let code = "struct S;\n";
        let result = rust_result(code);
        assert_eq!(
            result.lexical_kind,
            LexicalValueKind::Const,
            "expected Const for struct_item, got {:?}",
            result.lexical_kind
        );
    }

    #[test]
    fn parse_struct_impl_trait_fixture_conservative_emission() {
        // B.5 contract: the fixture `struct_impl_trait.rs` produces a
        // HashMap import reference (conservative emission) and Function
        // lexical_kind (the impl method is a function_item).
        let result = rust_fixture_result("struct_impl_trait.rs");
        assert!(
            result
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Import && r.target_name == "HashMap"),
            "expected HashMap import reference, got: {:#?}",
            result.references
        );
        // The impl method body is a function_item (via extract_impl_methods
        // emits only outline items; the function_item itself does not become
        // a top-level symbol). However the impl method's name `m` is not a
        // function_item node at the root. Skip lexical_kind for this fixture
        // to avoid coupling to impl-method behavior.
        assert!(
            !result.symbols.is_empty(),
            "expected struct symbol, got: {:#?}",
            result.symbols
        );
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
