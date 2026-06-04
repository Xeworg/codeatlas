//! Python language parser — stub implementation.
//!
//! This is the canonical "fourth language" example demonstrating the
//! add-a-language contract:
//!
//! - Implements `LanguageParser` with only the four core methods.
//! - Inherits the default `lexical_kind_for` and `extract_references`.
//! - Returns a stable `ParseResult` shape so the registry and consumers
//!   (Tauri shim, persistence, AI layer) treat Python files uniformly.
//!
//! A real Python grammar (tree-sitter-python) will be plugged in by a
//! follow-up change; for PR-A the stub proves the contract works.

use super::traits::LanguageParser;
use crate::models::ParseResult;

/// Minimal Python language parser (stub).
pub struct PythonParser;

impl PythonParser {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageParser for PythonParser {
    fn language_id(&self) -> &'static str {
        "python"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["py"]
    }

    fn parse_all(&self, _source: &str, _path: &str, _file_id: &str) -> ParseResult {
        // Stub: return a default ParseResult. A real Python parser (PR beyond
        // PR-C) will populate symbols/imports/outline from tree-sitter-python
        // and call `self.lexical_kind_for` and `self.extract_references`
        // during its single AST pass.
        ParseResult::default()
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LexicalValueKind;

    #[test]
    fn python_stub_reports_python_metadata() {
        let parser = PythonParser::new();
        assert_eq!(parser.language_id(), "python");
        assert!(parser.supports("py"));
        assert!(!parser.supports("ts"));
        assert!(!parser.supports(""));
    }

    #[test]
    fn python_stub_returns_empty_parse_result() {
        let parser = PythonParser::new();
        let result = parser.parse_all("import os\n", "stub.py", "file-py");
        assert!(result.symbols.is_empty());
        assert!(result.imports.is_empty());
        assert!(result.outline.is_empty());
        assert_eq!(result.lexical_kind, LexicalValueKind::Const);
        assert!(result.references.is_empty());
    }

    #[test]
    fn python_stub_default_impls_inherit_from_trait() {
        // The stub does not override `lexical_kind_for` or `extract_references`,
        // so it inherits the trait defaults: `Function` and empty `Vec`.
        let parser = PythonParser::new();
        // The trait defaults are observable: a parser that doesn't override
        // them gets Function + empty. The stub's `parse_all` doesn't call
        // them, but the trait surface is fully usable.
        let node_placeholder: Option<tree_sitter::Node> = None;
        // We can't easily construct a Node here, so we just compile-check.
        let _: fn(&PythonParser, &tree_sitter::Node, &str) -> LexicalValueKind =
            |p, n, s| p.lexical_kind_for(n, s);
        let _: fn(
            &PythonParser,
            &tree_sitter::Node,
            &str,
        ) -> Vec<crate::models::Reference> = |p, n, s| p.extract_references(n, s);
        // Ensure the unused binding doesn't get optimized out.
        let _ = node_placeholder;
    }
}
