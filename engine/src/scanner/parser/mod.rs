//! Parser module — Language-specific parse implementations with shared contracts.
//!
//! # Structure
//!
//! - `traits.rs` — `LanguageParser` trait, kind helpers, and stable ID builder.
//! - `registry.rs` — `ParserRegistry` with extension-to-parser dispatch.
//! - `typescript.rs` — TypeScript/TSX/JS/JSX parser with hierarchical outline.
//! - `rust.rs` — Rust parser with hierarchical outline.
//!
//! The registry is pre-populated with all supported parsers.

pub mod python_stub;
pub mod registry;
pub mod rust;
pub mod traits;
pub mod typescript;

#[cfg(test)]
mod ir_tests;
#[cfg(test)]
mod parse_result_tests;
#[cfg(test)]
mod trait_tests;

pub use crate::models::{OutlineItem, ParseResult};
pub use registry::ParserRegistry;
pub use traits::LanguageParser;
