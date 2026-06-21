//! CodeAtlas Engine — v1 MVP
//! Clean Architecture: Domain → Application → Infrastructure
//!
//! Public API re-exports only the public surface.

pub mod ai;
pub mod analysis;
pub mod commands;
pub mod db;
pub mod graph;
pub mod models;
pub mod ports;
pub mod scanner;
pub mod services;

// C4View, ExecutiveSummary, SnapshotDiff re-exported from db::queries
// which now re-exports them from models::workspace (AD-005 task 7.2).
pub use db::queries::{C4View, ExecutiveSummary, SnapshotDiff};
pub use models::{FileInfo, GraphData, NodeType, ScanResult, ScanStatus};
pub use ports::hexagonal::{
    Clock, IdGenerator, MockClock, MockIdGen, MockStopwatch, RandomIdGen, Stopwatch,
    StopwatchHandle, SystemClock, SystemStopwatch,
};
pub use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Scan not found: {0}")]
    NotFound(String),

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

/// Structured error payload for IPC transport.
///
/// This JSON structure is serialized as a STRING when crossing the Tauri IPC
/// boundary because Tauri error channel is string-oriented.
///
/// Backend-to-frontend error code mapping:
/// - PROJECT_NOT_FOUND -> PATH_NOT_FOUND
/// - FILE_NOT_FOUND -> PATH_NOT_FOUND
/// - AI_UNAVAILABLE -> UNREACHABLE
/// - AI_RATE_LIMITED -> RATE_LIMITED
/// - AI_TOKEN_LIMIT -> TOKEN_LIMIT
/// - INVALID_API_KEY -> INVALID_KEY
/// - ACCESS_DENIED -> ACCESS_DENIED
/// - SCAN_TIMEOUT -> SCAN_TIMEOUT
/// - DATABASE -> INTERNAL
/// - INTERNAL -> INTERNAL
#[derive(Debug, serde::Serialize)]
pub struct IpcErrorPayload {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let payload = match self {
            AppError::ProjectNotFound(path) => IpcErrorPayload {
                code: "PROJECT_NOT_FOUND",
                message: format!("Project not found: {}", path),
                details: Some(serde_json::json!({ "path": path })),
            },
            AppError::FileNotFound(path) => IpcErrorPayload {
                code: "FILE_NOT_FOUND",
                message: format!("File not found: {}", path),
                details: Some(serde_json::json!({ "path": path })),
            },
            AppError::NotFound(id) => IpcErrorPayload {
                code: "NOT_FOUND",
                message: format!("Scan not found: {}", id),
                details: Some(serde_json::json!({ "id": id })),
            },
            AppError::ScanTimeout {
                files_processed,
                total_files,
            } => IpcErrorPayload {
                code: "SCAN_TIMEOUT",
                message: format!(
                    "Scan timeout: processed {}/{}",
                    files_processed, total_files
                ),
                details: Some(serde_json::json!({
                    "files_processed": files_processed,
                    "total_files": total_files
                })),
            },
            AppError::Database(msg) => IpcErrorPayload {
                code: "DATABASE",
                message: format!("Database error: {}", msg),
                details: Some(serde_json::json!({ "reason": msg })),
            },
            AppError::AIUnavailable(msg) => IpcErrorPayload {
                code: "AI_UNAVAILABLE",
                message: format!("AI unavailable: {}", msg),
                details: Some(serde_json::json!({ "reason": msg })),
            },
            AppError::AIRateLimited => IpcErrorPayload {
                code: "AI_RATE_LIMITED",
                message: self.to_string(),
                details: None,
            },
            AppError::AITokenLimit => IpcErrorPayload {
                code: "AI_TOKEN_LIMIT",
                message: self.to_string(),
                details: None,
            },
            AppError::InvalidApiKey => IpcErrorPayload {
                code: "INVALID_API_KEY",
                message: self.to_string(),
                details: None,
            },
            AppError::AccessDenied(resource) => IpcErrorPayload {
                code: "ACCESS_DENIED",
                message: format!("Access denied: {}", resource),
                details: Some(serde_json::json!({ "resource": resource })),
            },
            AppError::Internal(msg) => IpcErrorPayload {
                code: "INTERNAL",
                message: format!("Internal error: {}", msg),
                details: Some(serde_json::json!({ "reason": msg })),
            },
        };

        payload.serialize(serializer)
    }
}
