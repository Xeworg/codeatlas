//! Tests for the code-intelligence IR — language-neutral types every
//! `LanguageParser` emits.
//!
//! This file covers the IR type definitions in isolation (A.1):
//! - `LexicalValueKind` serializes as snake_case
//! - `ReferenceKind` serializes as snake_case
//! - `Range` serializes as camelCase
//! - `Reference` round-trips preserving `file_id`, `kind`, `target_name`, `range`
//!
//! Cross-module tests (ParseResult integration, trait defaults, add-a-language
//! contract) live in `parse_result_tests.rs` and `trait_tests.rs` (added in
//! A.2 / A.3 / A.4).

use crate::models::{LexicalValueKind, Range, Reference, ReferenceKind};

// ─────────────────────────────────────────────────────────────────────────────
// IR shape — type-level tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn lexical_value_kind_serializes_snake_case() {
    let cases = [
        (LexicalValueKind::Const, "\"const\""),
        (LexicalValueKind::ArrowFunction, "\"arrow_function\""),
        (LexicalValueKind::Function, "\"function\""),
    ];
    for (kind, expected_json) in cases {
        let json = serde_json::to_string(&kind).expect("serialize LexicalValueKind");
        assert_eq!(json, expected_json, "LexicalValueKind {:?}", kind);
    }
}

#[test]
fn reference_kind_serializes_snake_case() {
    let cases = [
        (ReferenceKind::Import, "\"import\""),
        (ReferenceKind::Export, "\"export\""),
        (ReferenceKind::Call, "\"call\""),
        (ReferenceKind::TypeRef, "\"type_ref\""),
    ];
    for (kind, expected_json) in cases {
        let json = serde_json::to_string(&kind).expect("serialize ReferenceKind");
        assert_eq!(json, expected_json, "ReferenceKind {:?}", kind);
    }
}

#[test]
fn range_serializes_camel_case() {
    let range = Range {
        start_byte: 10,
        end_byte: 42,
        start_line: 2,
        start_col: 4,
        end_line: 5,
        end_col: 8,
    };
    let json = serde_json::to_string(&range).expect("serialize Range");
    assert!(json.contains("\"startByte\":10"), "got: {}", json);
    assert!(json.contains("\"endByte\":42"), "got: {}", json);
    assert!(json.contains("\"startLine\":2"), "got: {}", json);
    assert!(json.contains("\"endCol\":8"), "got: {}", json);
    assert!(
        !json.contains("start_byte"),
        "must not have snake_case: {}",
        json
    );
}

#[test]
fn reference_roundtrip_preserves_file_id_kind_name_and_range() {
    let reference = Reference {
        file_id: "file-7".to_string(),
        kind: ReferenceKind::Import,
        target_name: "useState".to_string(),
        range: Range {
            start_byte: 0,
            end_byte: 25,
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 25,
        },
    };
    let json = serde_json::to_string(&reference).expect("serialize Reference");
    let parsed: Reference = serde_json::from_str(&json).expect("deserialize Reference");

    assert_eq!(parsed.file_id, "file-7");
    assert_eq!(parsed.kind, ReferenceKind::Import);
    assert_eq!(parsed.target_name, "useState");
    assert_eq!(parsed.range.start_byte, 0);
    assert_eq!(parsed.range.end_byte, 25);
    assert_eq!(parsed.range.start_line, 1);
    assert_eq!(parsed.range.end_col, 25);
}
