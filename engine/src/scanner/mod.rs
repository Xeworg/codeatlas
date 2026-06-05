//! Scanner module — Infrastructure layer
//! File discovery and traversal.

pub mod code_parser;
pub mod parser;
pub mod walker;

pub use code_parser::CodeParser;
pub use parser::{LanguageParser, OutlineItem, ParseResult, ParserRegistry};
pub use walker::FileWalker;
