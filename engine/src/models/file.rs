//! File, Symbol, and Import domain models

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// SymbolKind — used for metrics and symbol-level analysis
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SymbolKind {
    Class,
    Function,
    ArrowFunction,
    Method,
    Interface,
    TypeAlias,
    Enum,
    Variable,
    Const,
    Struct,
    Impl,
    #[default]
    Unknown,
}

// ─────────────────────────────────────────────────────────────────────────────
// OutlineItemKind — UI/IA-oriented, separate from SymbolKind
// ─────────────────────────────────────────────────────────────────────────────
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

// ─────────────────────────────────────────────────────────────────────────────
// OutlineItem — hierarchical tree-sitter-derived structure for UI/IA
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineItem {
    pub id: String,
    pub file_id: String,
    pub name: String,
    pub kind: OutlineItemKind,
    pub line_start: u32,
    pub line_end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_end: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<OutlineItem>,
}

impl OutlineItem {
    /// Build a stable ID for this outline item based on file, kind, and range.
    /// This is more stable across rescans than a UUID and good enough for UI keys.
    pub fn stable_id(
        file_id: &str,
        kind: OutlineItemKind,
        line_start: u32,
        line_end: u32,
        name: &str,
    ) -> String {
        format!(
            "outline:{}:{:?}:{}:{}:{}",
            file_id, kind, line_start, line_end, name
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LexicalValueKind — classification of a lexical/binding declaration.
// Language-neutral: every `LanguageParser` emits one of these variants per
// top-level binding so the AI layer can distinguish `const`/`let` values,
// arrow functions, and `function` declarations without grammar-specific logic.
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LexicalValueKind {
    #[default]
    Const,
    ArrowFunction,
    Function,
}

// ─────────────────────────────────────────────────────────────────────────────
// ReferenceKind — kinds of cross-symbol references the IR v1 exposes.
// `Import` and `Export` are the v1 emission targets; `Call` and `TypeRef` are
// stub variants kept in the enum so the AI layer can wire to the full shape
// now without a schema churn when v2 adds resolution.
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    Import,
    Export,
    Call,
    TypeRef,
}

// ─────────────────────────────────────────────────────────────────────────────
// Range — half-open byte range plus 1-based line/column for UI/IA displays.
// All fields are required; legacy consumers that don't track column data must
// pass `0` (which is also what tree-sitter returns for unknown columns).
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Reference — a single cross-symbol edge observed during a single AST pass.
// Carries `file_id` explicitly (per design decision #6) so the AI layer can
// group references by source file without consulting the parent ParseResult.
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub file_id: String,
    pub kind: ReferenceKind,
    pub target_name: String,
    pub range: Range,
}

// ─────────────────────────────────────────────────────────────────────────────
// ParseResult — aggregated output from a single parse pass
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    pub symbols: Vec<SymbolInfo>,
    pub imports: Vec<ImportInfo>,
    pub outline: Vec<OutlineItem>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SymbolInfo — flat symbol record used for metrics/analysis
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolInfo {
    pub id: String,
    pub name: String,
    pub kind: SymbolKind,
    pub file_id: String,
    pub line_start: u32,
    pub line_end: u32,
    pub exports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub symbols: Vec<SymbolInfo>,
    pub lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    pub id: String,
    pub source_file_id: String,
    pub target_file_id: Option<String>,
    pub target_module: Option<String>,
    pub imports: Vec<String>,
    pub is_default: bool,
    pub is_type: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — file models
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_item_kind_serialization_is_snake_case() {
        let kinds = [
            (OutlineItemKind::Class, "class"),
            (OutlineItemKind::Function, "function"),
            (OutlineItemKind::Method, "method"),
            (OutlineItemKind::Interface, "interface"),
            (OutlineItemKind::Type, "type"),
            (OutlineItemKind::Enum, "enum"),
            (OutlineItemKind::Const, "const"),
            (OutlineItemKind::Variable, "variable"),
            (OutlineItemKind::Module, "module"),
            (OutlineItemKind::Field, "field"),
            (OutlineItemKind::Struct, "struct"),
            (OutlineItemKind::Impl, "impl"),
            (OutlineItemKind::Unknown, "unknown"),
        ];
        for (kind, expected) in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", expected), "kind {:?}", kind);
        }
    }

    #[test]
    fn outline_item_serialization_is_camel_case() {
        let item = OutlineItem {
            id: "outline:test:class:10:50:UserService".into(),
            file_id: "file-1".into(),
            name: "UserService".into(),
            kind: OutlineItemKind::Class,
            line_start: 10,
            line_end: 50,
            column_start: Some(0),
            column_end: Some(20),
            children: vec![],
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(
            json.contains("\"fileId\""),
            "expected camelCase fileId, got: {}",
            json
        );
        assert!(
            json.contains("\"lineStart\""),
            "expected camelCase lineStart, got: {}",
            json
        );
        assert!(
            !json.contains("file_id"),
            "should not have snake_case: {}",
            json
        );
    }

    #[test]
    fn outline_item_roundtrip_hierarchy() {
        let item = OutlineItem {
            id: "outline:file-1:class:10:50:UserService".into(),
            file_id: "file-1".into(),
            name: "UserService".into(),
            kind: OutlineItemKind::Class,
            line_start: 10,
            line_end: 50,
            column_start: None,
            column_end: None,
            children: vec![OutlineItem {
                id: "outline:file-1:method:12:18:getUser".into(),
                file_id: "file-1".into(),
                name: "getUser".into(),
                kind: OutlineItemKind::Method,
                line_start: 12,
                line_end: 18,
                column_start: None,
                column_end: None,
                children: vec![],
            }],
        };
        let json = serde_json::to_string(&item).unwrap();
        let parsed: OutlineItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "UserService");
        assert_eq!(parsed.kind, OutlineItemKind::Class);
        assert_eq!(parsed.children.len(), 1);
        assert_eq!(parsed.children[0].name, "getUser");
        assert_eq!(parsed.children[0].kind, OutlineItemKind::Method);
    }

    #[test]
    fn outline_item_stable_id_format() {
        let id = OutlineItem::stable_id("file-1", OutlineItemKind::Class, 10, 50, "UserService");
        assert!(id.starts_with("outline:file-1:Class:10:50:UserService"));
    }

    #[test]
    fn parse_result_default_is_empty() {
        let result = ParseResult::default();
        assert!(result.symbols.is_empty());
        assert!(result.imports.is_empty());
        assert!(result.outline.is_empty());
    }

    #[test]
    fn symbol_info_serialization_roundtrip() {
        let symbol = SymbolInfo {
            id: "sym-1".into(),
            name: "UserService".into(),
            kind: SymbolKind::Class,
            file_id: "file-1".into(),
            line_start: 10,
            line_end: 50,
            exports: true,
        };
        let json = serde_json::to_string(&symbol).unwrap();
        let parsed: SymbolInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "UserService");
        assert_eq!(parsed.kind, SymbolKind::Class);
    }

    #[test]
    fn import_info_handles_external_module() {
        let imp = ImportInfo {
            id: "imp-1".into(),
            source_file_id: "file-1".into(),
            target_file_id: None,
            target_module: Some("react".into()),
            imports: vec!["useState".into()],
            is_default: false,
            is_type: true,
        };
        let json = serde_json::to_string(&imp).unwrap();
        assert!(json.contains("react"));
        assert!(json.contains("useState"));
    }
}
