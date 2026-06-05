//! Integration test for the add-a-language contract.
//!
//! Verifies that registering a stub `PythonParser` (a new language) requires
//! NO changes to `ParseResult`, `Reference`, `LexicalValueKind`,
//! `ParserRegistry`, or `CodeParser`. The parser plugs in via trait + register
//! and the registry dispatches to it on its declared extension.
//!
//! This is the canonical example for the spec scenario
//! "Stub de cuarto lenguaje se registra sin tocar IR".

use std::fs;
use std::path::PathBuf;

use engine::models::{LexicalValueKind, ParseResult, Reference};
use engine::scanner::parser::python_stub::PythonParser;
use engine::scanner::parser::{LanguageParser, ParserRegistry};

fn fixture_path() -> PathBuf {
    // engine/tests/add_a_language.rs is compiled with CARGO_MANIFEST_DIR =
    // engine, so the fixture path is `<engine>/tests/fixtures/python/hello.py`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("python")
        .join("hello.py")
}

#[test]
fn python_stub_dispatches_without_ir_changes() {
    // Step 1: read the fixture so the test exercises the real byte path.
    let path = fixture_path();
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e));

    // Step 2: build a fresh registry (no Python pre-registered) and add the stub.
    let mut registry = ParserRegistry::new();
    registry.register(PythonParser::new());

    // Step 3: dispatch via the registry by extension — the contract is that
    // the registry resolves the parser by its declared `extensions` list.
    let result = registry.parse_file(path.to_str().unwrap(), &source, "py", "file-py");

    // Step 4: the IR shape must be stable and round-trippable.
    assert_eq!(result.symbols.len(), 0, "stub emits no symbols");
    assert_eq!(result.imports.len(), 0, "stub emits no imports");
    assert_eq!(result.outline.len(), 0, "stub emits no outline");
    assert_eq!(
        result.lexical_kind,
        LexicalValueKind::Const,
        "stub's ParseResult::default() lexical_kind is Const"
    );
    assert_eq!(
        result.references.len(),
        0,
        "stub emits no references (default extract_references returns Vec::new())"
    );
}

#[test]
fn python_stub_is_registerable_alongside_existing_parsers() {
    // The registry must support multiple languages simultaneously — the stub
    // is additive, not a replacement.
    let mut registry = ParserRegistry::new();
    registry.register(PythonParser::new());

    // Existing parsers still resolve.
    assert!(registry.parser_for_extension("ts").is_some());
    assert!(registry.parser_for_extension("rs").is_some());
    // The new one is too.
    let py = registry
        .parser_for_extension("py")
        .expect("python parser must be discoverable after registration");
    assert_eq!(py.language_id(), "python");
}

#[test]
fn python_stub_parse_result_roundtrips_through_serde() {
    // The IR shape (including the default fields the stub leaves empty)
    // must serialize and deserialize cleanly — the contract says new
    // languages get the same `ParseResult` shape for free.
    let result = PythonParser::new().parse_all("import os\n", "stub.py", "file-py");
    let json = serde_json::to_string(&result).expect("serialize");
    let parsed: ParseResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.lexical_kind, LexicalValueKind::Const);
    assert!(parsed.references.is_empty());
}

#[test]
fn unknown_extension_does_not_panic_with_python_registered() {
    // Adding a parser to the registry MUST NOT change behavior for
    // unsupported extensions — the contract is additive.
    let mut registry = ParserRegistry::new();
    registry.register(PythonParser::new());
    let result = registry.parse_file("foo.xyz", "x = 1", "xyz", "file-xyz");
    assert!(result.symbols.is_empty());
    assert!(result.imports.is_empty());
    assert!(result.outline.is_empty());
}

#[test]
fn reference_constructor_roundtrip_works_with_stub() {
    // The stub itself never emits `Reference`s, but the type must remain
    // constructible so the AI layer (and future PRs) can build them. This
    // test pins the public surface — a future change that breaks the
    // `Reference` constructor will fail here.
    let reference = Reference {
        file_id: "file-py".to_string(),
        kind: engine::models::ReferenceKind::Import,
        target_name: "os".to_string(),
        range: engine::models::Range {
            start_byte: 7,
            end_byte: 9,
            start_line: 1,
            start_col: 7,
            end_line: 1,
            end_col: 9,
        },
    };
    let json = serde_json::to_string(&reference).expect("serialize");
    let parsed: Reference = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.file_id, "file-py");
    assert_eq!(parsed.target_name, "os");
}
