//! Graph module — Application layer
//! Builds dependency graphs from file metadata.

pub mod builder;
pub mod resolver;

pub use builder::GraphBuilder;
pub use resolver::PathResolver;
