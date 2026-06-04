//! Tests for the defaulted `LanguageParser` trait methods
//! (`lexical_kind_for`, `extract_references`) — A.3 of the multi-language
//! code-intelligence framework.
//!
//! Contract:
//! - A parser that implements only the four core methods (`language_id`,
//!   `extensions`, `parse_all`, `supports`) MUST compile and inherit the
//!   default `lexical_kind_for` (returns `LexicalValueKind::Function`) and
//!   default `extract_references` (returns empty `Vec`).
//! - The default methods MUST be invokable via a `&dyn LanguageParser` so
//!   concrete parsers can dispatch through a registry without re-allocating.

use super::LanguageParser;
use crate::models::{LexicalValueKind, ParseResult, Reference, ReferenceKind};
use tree_sitter::Parser;

/// Minimal in-test `LanguageParser` impl that only provides the four core
/// methods. This is the canonical example for the add-a-language contract:
/// a stub that drops in with zero IR work.
struct MinimalParser;

impl LanguageParser for MinimalParser {
    fn language_id(&self) -> &'static str {
        "minimal"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["min"]
    }
    fn parse_all(&self, _source: &str, _path: &str, _file_id: &str) -> ParseResult {
        ParseResult::default()
    }
}

/// Parse a tiny TS source and return the owned source + the parsed tree.
/// We return a tuple that owns both so callers can borrow the node freely.
struct TsFixture {
    source: String,
    tree: tree_sitter::Tree,
}

impl TsFixture {
    fn new(source: &str) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .expect("set typescript language");
        let tree = parser.parse(source, None).expect("parse source");
        Self {
            source: source.to_string(),
            tree,
        }
    }
    fn root(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }
    fn source(&self) -> &str {
        &self.source
    }
}

#[test]
fn default_lexical_kind_for_returns_function() {
    // A minimal parser that does NOT override `lexical_kind_for` must inherit
    // the default value `LexicalValueKind::Function` (per design decision #3
    // and the spec's "MUST expose default methods" requirement).
    let fix = TsFixture::new("const x = 1;");
    let node = fix.root();
    let src = fix.source();
    let parser = MinimalParser;
    let kind = parser.lexical_kind_for(&node, src);
    assert_eq!(
        kind,
        LexicalValueKind::Function,
        "default lexical_kind_for must return Function"
    );
}

#[test]
fn default_extract_references_returns_empty_vec() {
    // A minimal parser that does NOT override `extract_references` must
    // inherit the default value (empty `Vec<Reference>`), so a stub cannot
    // accidentally emit references it does not understand.
    let fix = TsFixture::new("import { foo } from './bar';");
    let node = fix.root();
    let src = fix.source();
    let parser = MinimalParser;
    let references = parser.extract_references(&node, src);
    assert!(
        references.is_empty(),
        "default extract_references must return empty vec, got: {:#?}",
        references
    );
}

#[test]
fn default_methods_invocable_via_dyn_trait_object() {
    // The registry stores `Box<dyn LanguageParser>`. The defaulted methods
    // must be invokable through a trait object so concrete parsers can
    // dispatch hooks polymorphically.
    let fix = TsFixture::new("const x = 1;");
    let node = fix.root();
    let src = fix.source();
    let parser: Box<dyn LanguageParser> = Box::new(MinimalParser);
    let kind = parser.lexical_kind_for(&node, src);
    let references = parser.extract_references(&node, src);
    assert_eq!(kind, LexicalValueKind::Function);
    assert!(references.is_empty());
}

#[test]
fn minimal_parser_compatible_with_registry_dispatch() {
    // A stub parser must be register-able in a `ParserRegistry` and dispatch
    // on its declared extension. This is the add-a-language contract.
    use crate::scanner::parser::ParserRegistry;

    let mut registry = ParserRegistry::new();
    registry.register(MinimalParser);
    let result = registry.parse_file("hello.min", "any source", "min", "file-min");
    assert!(result.symbols.is_empty());
    assert!(result.imports.is_empty());
    assert!(result.outline.is_empty());
    assert_eq!(result.lexical_kind, LexicalValueKind::Const);
    assert!(result.references.is_empty());
}

/// Helper for `LanguageParser` implementers: a concrete parser can override
/// `lexical_kind_for` to return `ArrowFunction` for arrow nodes. This test
/// demonstrates the override is honored via trait dispatch.
struct ArrowAwareParser;

impl LanguageParser for ArrowAwareParser {
    fn language_id(&self) -> &'static str {
        "arrow-aware"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["awa"]
    }
    fn parse_all(&self, _source: &str, _path: &str, _file_id: &str) -> ParseResult {
        ParseResult {
            lexical_kind: LexicalValueKind::ArrowFunction,
            ..ParseResult::default()
        }
    }
    fn lexical_kind_for(&self, _node: &tree_sitter::Node, _src: &str) -> LexicalValueKind {
        LexicalValueKind::ArrowFunction
    }
}

#[test]
fn override_lexical_kind_is_observed() {
    // When a parser overrides the default, its override must be observed.
    let fix = TsFixture::new("const C = () => 1;");
    let node = fix.root();
    let src = fix.source();
    let parser = ArrowAwareParser;
    let kind = parser.lexical_kind_for(&node, src);
    assert_eq!(kind, LexicalValueKind::ArrowFunction);
    // And the parser's `parse_all` produces a result with the same kind.
    let result = parser.parse_all("const C = () => 1;", "c.awa", "file-awa");
    assert_eq!(result.lexical_kind, LexicalValueKind::ArrowFunction);
    // `Reference` types are concrete in the public surface — sanity check
    // the path is wired all the way through.
    let _import_ref = Reference {
        file_id: "file-awa".to_string(),
        kind: ReferenceKind::Import,
        target_name: "x".to_string(),
        range: crate::models::Range {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 1,
        },
    };
}
