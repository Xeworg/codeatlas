//! IPC error conversion for the Tauri presentation layer.
//!
//! This module is the single, canonical point at which `engine::AppError`
//! values are converted to the JSON string sent across the Tauri IPC
//! boundary. The shape of that JSON is the `IpcErrorPayload` struct
//! defined in `engine::lib`, produced by the `Serialize for AppError`
//! impl. The conversion re-uses that impl so the wire format stays in
//! one place: any future change to `IpcErrorPayload` automatically
//! propagates here.
//!
//! STRICT POLICY: Tauri commands MUST call `to_ipc_error(e)` to translate
//! `AppError` values into the IPC return string. They MUST NOT call
//! `e.to_string()` directly: that path emits the `thiserror::Display`
//! message and discards the structured code/details that the frontend
//! error parser (`src/lib/tauri-api.ts`) relies on.

use engine::AppError;

/// Converts any error type to `AppError` for IPC serialization.
///
/// This trait allows `to_ipc_error` to accept both `AppError` (the common case
/// from service-layer errors) and `PoisonError` (from mutex lock operations)
/// through a single interface.
pub(crate) trait ToAppError {
    fn to_app_error(self) -> AppError;
}

impl ToAppError for AppError {
    fn to_app_error(self) -> AppError {
        self
    }
}

impl<T> ToAppError for std::sync::PoisonError<T> {
    fn to_app_error(self) -> AppError {
        AppError::Internal(self.to_string())
    }
}

/// Serialize an `AppError` into the canonical `IpcErrorPayload` JSON
/// string for transport across the Tauri IPC boundary.
///
/// Falls back to a `"INTERNAL"` payload if serialization fails, so the
/// presentation layer can always return a `String` error to Tauri
/// without panicking on a serialization bug. The fallback is observable
/// in tests as a last-resort guard.
pub(crate) fn to_ipc_error<E: ToAppError>(e: E) -> String {
    let app_error = e.to_app_error();
    match serde_json::to_string(&app_error) {
        Ok(s) => s,
        Err(_) => serde_json::json!({
            "code": "INTERNAL",
            "message": format!("Failed to serialize AppError: {}", app_error),
            "details": null,
        })
        .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::AppError;
    use serde_json::Value;

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).expect("to_ipc_error must produce valid JSON")
    }

    #[test]
    fn file_not_found_carries_path_in_details() {
        let payload = to_ipc_error(AppError::FileNotFound("src/main.rs".to_string()));
        let v = parse(&payload);
        assert_eq!(v["code"], "FILE_NOT_FOUND");
        assert_eq!(v["message"], "File not found: src/main.rs");
        assert_eq!(v["details"]["path"], "src/main.rs");
    }

    #[test]
    fn ai_unavailable_carries_reason_in_details() {
        let payload = to_ipc_error(AppError::AIUnavailable(
            "no provider configured".to_string(),
        ));
        let v = parse(&payload);
        assert_eq!(v["code"], "AI_UNAVAILABLE");
        assert_eq!(v["message"], "AI unavailable: no provider configured");
        assert_eq!(v["details"]["reason"], "no provider configured");
    }

    #[test]
    fn database_error_carries_reason_in_details() {
        let payload = to_ipc_error(AppError::Database("UNIQUE constraint failed".to_string()));
        let v = parse(&payload);
        assert_eq!(v["code"], "DATABASE");
        assert_eq!(v["message"], "Database error: UNIQUE constraint failed");
        assert_eq!(v["details"]["reason"], "UNIQUE constraint failed");
    }

    #[test]
    fn unit_variants_have_no_details_field() {
        let payload = to_ipc_error(AppError::AIRateLimited);
        let v = parse(&payload);
        assert_eq!(v["code"], "AI_RATE_LIMITED");
        assert!(v.get("details").is_none() || v["details"].is_null());
    }

    #[test]
    fn project_not_found_is_distinct_from_file_not_found() {
        let p = to_ipc_error(AppError::ProjectNotFound("/repo".to_string()));
        let f = to_ipc_error(AppError::FileNotFound("src/main.rs".to_string()));
        let pv = parse(&p);
        let fv = parse(&f);
        assert_eq!(pv["code"], "PROJECT_NOT_FOUND");
        assert_eq!(fv["code"], "FILE_NOT_FOUND");
        assert_ne!(pv["code"], fv["code"]);
    }
}
