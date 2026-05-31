//! File, Symbol, and Import domain models

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
