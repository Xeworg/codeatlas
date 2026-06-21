//! Domain models — NO external dependencies except serde.
//! These are the canonical types shared between Rust backend and TypeScript frontend.

mod ai;
mod file;
mod graph;
mod node_ref;
mod project;
mod workspace;

pub use ai::*;
pub use file::*;
pub use graph::*;
pub use node_ref::*;
pub use project::*;
pub use workspace::*;
