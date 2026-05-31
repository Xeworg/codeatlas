//! Scanner module — Infrastructure layer
//! File discovery and traversal.

pub mod parser;
pub mod walker;

pub use parser::CodeParser;
pub use walker::FileWalker;
