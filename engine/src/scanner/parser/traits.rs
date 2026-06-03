//! LanguageParser trait — contract for per-language parse implementations.
//!
//! Each parser can extract symbols, imports, and outline from a single pass
//! via `parse_all()`. This avoids traversing the AST multiple times per file.

use crate::models::ParseResult;

/// Language parser that produces symbols, imports, and outline from a single parse.
pub trait LanguageParser: Send + Sync {
    /// Unique identifier for this language (e.g. "typescript", "rust").
    fn language_id(&self) -> &'static str;

    /// File extensions supported by this parser (e.g. ["ts", "tsx"]).
    fn extensions(&self) -> &'static [&'static str];

    /// Parse a source file and return all extraction results.
    ///
    /// The implementation should traverse the AST once and populate symbols,
    /// imports, and outline items from the same tree walk.
    fn parse_all(&self, source: &str, path: &str, file_id: &str) -> ParseResult;

    /// Returns true if this parser supports the given extension.
    fn supports(&self, extension: &str) -> bool {
        self.extensions().contains(&extension)
    }
}

/// Helper to extract OutlineItemKind from a tree-sitter node kind string for TypeScript.
pub fn ts_node_kind_to_outline_kind(node_kind: &str) -> Option<crate::models::OutlineItemKind> {
    match node_kind {
        "class_declaration" => Some(crate::models::OutlineItemKind::Class),
        "method_definition" => Some(crate::models::OutlineItemKind::Method),
        "function_declaration" => Some(crate::models::OutlineItemKind::Function),
        "interface_declaration" => Some(crate::models::OutlineItemKind::Interface),
        "type_alias_declaration" => Some(crate::models::OutlineItemKind::Type),
        "enum_declaration" => Some(crate::models::OutlineItemKind::Enum),
        "lexical_declaration" => Some(crate::models::OutlineItemKind::Const),
        _ => None,
    }
}

/// Helper to extract OutlineItemKind from a tree-sitter node kind string for Rust.
pub fn rust_node_kind_to_outline_kind(node_kind: &str) -> Option<crate::models::OutlineItemKind> {
    match node_kind {
        "struct_item" => Some(crate::models::OutlineItemKind::Struct),
        "enum_item" => Some(crate::models::OutlineItemKind::Enum),
        "function_item" => Some(crate::models::OutlineItemKind::Function),
        "impl_item" => Some(crate::models::OutlineItemKind::Impl),
        "mod_item" => Some(crate::models::OutlineItemKind::Module),
        "type_item" | "type_alias_item" => Some(crate::models::OutlineItemKind::Type),
        _ => None,
    }
}

/// Helper: build a stable outline item ID from its components.
/// Format: `outline:<file_id>:<kind>:<line_start>:<line_end>:<name>`
pub fn make_outline_id(
    file_id: &str,
    kind: crate::models::OutlineItemKind,
    line_start: u32,
    line_end: u32,
    name: &str,
) -> String {
    crate::models::OutlineItem::stable_id(file_id, kind, line_start, line_end, name)
}
