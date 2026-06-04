//! ParserRegistry — extension-to-parser dispatch with safe fallback.

use super::{
    python_stub::PythonParser, rust::RustParser, traits::LanguageParser,
    typescript::TypeScriptParser,
};
use crate::models::ParseResult;

/// Registry of language parsers with fallback for unsupported extensions.
pub struct ParserRegistry {
    parsers: Vec<Box<dyn LanguageParser>>,
}

impl ParserRegistry {
    /// Create a registry pre-populated with all supported language parsers.
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: Vec::new(),
        };
        registry.register(TypeScriptParser::new());
        registry.register(RustParser::new());
        registry.register(PythonParser::new());
        registry
    }

    /// Add a parser to this registry.
    pub fn register(&mut self, parser: impl LanguageParser + 'static) {
        self.parsers.push(Box::new(parser));
    }

    /// Returns the parser for a given extension, if any.
    pub fn parser_for_extension(&self, extension: &str) -> Option<&dyn LanguageParser> {
        self.parsers
            .iter()
            .find(|p| p.supports(extension))
            .map(|p| p.as_ref())
    }

    /// Parse a file using the appropriate parser, or return an empty result.
    pub fn parse_file(
        &self,
        path: &str,
        source: &str,
        extension: &str,
        file_id: &str,
    ) -> ParseResult {
        self.parser_for_extension(extension)
            .map(|p| p.parse_all(source, path, file_id))
            .unwrap_or_default()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_extension_returns_empty_result() {
        let registry = ParserRegistry::default();
        let result = registry.parse_file("test.xyz", "some code", "xyz", "file-xyz");
        assert!(result.symbols.is_empty());
        assert!(result.imports.is_empty());
        assert!(result.outline.is_empty());
    }

    #[test]
    fn parser_for_extension_returns_supported_parsers() {
        let registry = ParserRegistry::default();
        assert!(registry.parser_for_extension("ts").is_some());
        assert!(registry.parser_for_extension("tsx").is_some());
        assert!(registry.parser_for_extension("rs").is_some());
        assert!(registry.parser_for_extension("xyz").is_none());
        assert!(registry.parser_for_extension("").is_none());
    }
}
