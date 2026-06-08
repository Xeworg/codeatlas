//! Database module — Infrastructure layer
//! SQLite schema and queries.

pub mod error_mapping;
pub mod migrations;
pub mod queries;
pub mod schema;

pub use queries::{DbPool, ProjectRepository};
pub use schema::init_schema;
