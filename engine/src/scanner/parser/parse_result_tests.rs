//! Tests for `ParseResult` extension with the IR fields
//! (`lexical_kind`, `references`) — A.2 of the multi-language code-intelligence
//! framework.
//!
//! Contract:
//! - `ParseResult` MUST expose `lexical_kind: LexicalValueKind` and
//!   `references: Vec<Reference>` publicly.
//! - Both fields MUST be `#[serde(default)]` so legacy JSON without them still
//!   decodes (SQLite consumers untouched).
//! - `ParseResult::default()` MUST produce an empty `references` vec and the
//!   default `LexicalValueKind::Const`.

use crate::models::{LexicalValueKind, ParseResult, Range, Reference, ReferenceKind};

// ─────────────────────────────────────────────────────────────────────────────
// Field exposure
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn parse_result_exposes_lexical_kind_and_references() {
    let result = ParseResult {
        symbols: vec![],
        imports: vec![],
        outline: vec![],
        lexical_kind: LexicalValueKind::ArrowFunction,
        references: vec![Reference {
            file_id: "file-1".into(),
            kind: ReferenceKind::Export,
            target_name: "Component".into(),
            range: Range {
                start_byte: 0,
                end_byte: 10,
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 10,
            },
        }],
    };
    // Both fields must be publicly accessible.
    assert_eq!(result.lexical_kind, LexicalValueKind::ArrowFunction);
    assert_eq!(result.references.len(), 1);
    assert_eq!(result.references[0].target_name, "Component");
}

#[test]
fn parse_result_default_has_empty_references_and_const_lexical_kind() {
    let result = ParseResult::default();
    assert!(result.references.is_empty(), "references default must be empty");
    assert_eq!(
        result.lexical_kind,
        LexicalValueKind::Const,
        "default lexical_kind must be Const"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Roundtrip via serde
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn parse_result_roundtrip_preserves_ir_fields() {
    let reference = Reference {
        file_id: "file-1".into(),
        kind: ReferenceKind::Import,
        target_name: "useState".into(),
        range: Range {
            start_byte: 7,
            end_byte: 35,
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 25,
        },
    };
    let result = ParseResult {
        symbols: vec![],
        imports: vec![],
        outline: vec![],
        lexical_kind: LexicalValueKind::Function,
        references: vec![reference.clone()],
    };
    let json = serde_json::to_string(&result).expect("serialize ParseResult");
    let parsed: ParseResult = serde_json::from_str(&json).expect("deserialize ParseResult");

    assert_eq!(parsed.lexical_kind, LexicalValueKind::Function);
    assert_eq!(parsed.references.len(), 1);
    assert_eq!(parsed.references[0].file_id, "file-1");
    assert_eq!(parsed.references[0].kind, ReferenceKind::Import);
    assert_eq!(parsed.references[0].target_name, "useState");
    assert_eq!(parsed.references[0].range.start_byte, 7);
    assert_eq!(parsed.references[0].range.end_byte, 35);
}

// ─────────────────────────────────────────────────────────────────────────────
// Back-compat: legacy JSON without IR fields must still decode
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn parse_result_legacy_json_without_ir_fields_still_decodes() {
    // Legacy JSON shape (pre-IR): no `lexicalKind` or `references` field.
    // `#[serde(default)]` MUST ensure it deserializes cleanly.
    let legacy = r#"{"symbols":[],"imports":[],"outline":[]}"#;
    let parsed: ParseResult =
        serde_json::from_str(legacy).expect("legacy JSON must deserialize");
    assert!(parsed.symbols.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.outline.is_empty());
    assert!(
        parsed.references.is_empty(),
        "missing references must default to empty"
    );
    assert_eq!(
        parsed.lexical_kind,
        LexicalValueKind::Const,
        "missing lexicalKind must default to Const"
    );
}
