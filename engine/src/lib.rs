//! CodeAtlas Engine — v1 MVP
//! Clean Architecture: Domain → Application → Infrastructure
//!
//! Public API re-exports only the public surface.

pub mod models;
pub mod scanner;
pub mod graph;
pub mod ai;
pub mod db;

pub use models::{ScanResult, FileInfo, GraphData, NodeType, ScanStatus};
pub use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Scan timeout: processed {files_processed}/{total_files} files")]
    ScanTimeout {
        files_processed: usize,
        total_files: usize,
    },

    #[error("Database error: {0}")]
    Database(String),

    #[error("AI unavailable: {0}")]
    AIUnavailable(String),

    #[error("AI rate limited")]
    AIRateLimited,

    #[error("AI token limit exceeded")]
    AITokenLimit,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}