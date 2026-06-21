//! Tests for structured error contract (PR-1)
//!
//! These tests verify that AppError serializes to an IPC-safe JSON string
//! with a stable contract: { "code": "...", "message": "...", "details": {...} }
//!
//! The JSON is transported as a STRING because Tauri IPC error channel is string-oriented.

use engine::AppError;

/// Verifies the contract: AppError serializes to valid JSON string
/// with required `code` and `message` fields.
fn assert_json_contract(error: &AppError, expected_code: &str) {
    use serde_json::Value;

    let json_str = serde_json::to_string(error).expect("AppError must serialize to JSON string");
    let value: Value =
        serde_json::from_str(&json_str).expect("Serialized value must be valid JSON");

    // Required fields
    let code = value
        .get("code")
        .expect("Serialized error must have 'code' field")
        .as_str()
        .expect("'code' must be a string");
    assert_eq!(code, expected_code, "Error code mismatch");

    let message = value
        .get("message")
        .expect("Serialized error must have 'message' field")
        .as_str()
        .expect("'message' must be a string");
    assert!(!message.is_empty(), "Error message must not be empty");

    // `details` is optional but must be an object if present
    if let Some(details) = value.get("details") {
        assert!(
            details.is_object() || details.is_null(),
            "'details' must be an object or null, got: {:?}",
            details
        );
    }
}

// MARK: Error Code Catalog Tests

#[test]
fn project_not_found_has_correct_code() {
    let err = AppError::ProjectNotFound("foo".to_string());
    assert_json_contract(&err, "PROJECT_NOT_FOUND");
}

#[test]
fn file_not_found_has_correct_code() {
    let err = AppError::FileNotFound("bar.rs".to_string());
    assert_json_contract(&err, "FILE_NOT_FOUND");
}

#[test]
fn scan_timeout_has_correct_code() {
    let err = AppError::ScanTimeout {
        files_processed: 42,
        total_files: 100,
    };
    assert_json_contract(&err, "SCAN_TIMEOUT");
}

#[test]
fn database_error_has_correct_code() {
    let err = AppError::Database("connection refused".to_string());
    assert_json_contract(&err, "DATABASE");
}

#[test]
fn ai_unavailable_has_correct_code() {
    let err = AppError::AIUnavailable("network error".to_string());
    assert_json_contract(&err, "AI_UNAVAILABLE");
}

#[test]
fn ai_rate_limited_has_correct_code() {
    let err = AppError::AIRateLimited;
    assert_json_contract(&err, "AI_RATE_LIMITED");
}

#[test]
fn ai_token_limit_has_correct_code() {
    let err = AppError::AITokenLimit;
    assert_json_contract(&err, "AI_TOKEN_LIMIT");
}

#[test]
fn invalid_api_key_has_correct_code() {
    let err = AppError::InvalidApiKey;
    assert_json_contract(&err, "INVALID_API_KEY");
}

#[test]
fn access_denied_has_correct_code() {
    let err = AppError::AccessDenied("forbidden".to_string());
    assert_json_contract(&err, "ACCESS_DENIED");
}

#[test]
fn internal_error_has_correct_code() {
    let err = AppError::Internal("something went wrong".to_string());
    assert_json_contract(&err, "INTERNAL");
}

// MARK: Message Preservation Tests

#[test]
fn message_is_human_readable() {
    let err = AppError::ProjectNotFound("my-project".to_string());
    let json_str = serde_json::to_string(&err).expect("Must serialize");
    let value: serde_json::Value = serde_json::from_str(&json_str).expect("Must be valid JSON");

    let message = value.get("message").unwrap().as_str().unwrap();
    assert!(
        message.contains("my-project"),
        "Message should contain the project name for debuggability, got: {}",
        message
    );
}

// MARK: Structured Details Tests

#[test]
fn errors_with_context_include_structured_details() {
    // ScanTimeout has contextual data (files_processed, total_files)
    let err = AppError::ScanTimeout {
        files_processed: 42,
        total_files: 100,
    };
    let json_str = serde_json::to_string(&err).expect("Must serialize");
    let value: serde_json::Value = serde_json::from_str(&json_str).expect("Must be valid JSON");

    let details = value
        .get("details")
        .expect("Errors with context should have 'details'")
        .as_object()
        .expect("'details' must be an object");

    assert_eq!(details.get("files_processed").unwrap().as_u64(), Some(42));
    assert_eq!(details.get("total_files").unwrap().as_u64(), Some(100));
}

#[test]
fn simple_errors_may_omit_details() {
    // Simple errors like AIRateLimited have no additional context
    let err = AppError::AIRateLimited;
    let json_str = serde_json::to_string(&err).expect("Must serialize");
    let value: serde_json::Value = serde_json::from_str(&json_str).expect("Must be valid JSON");

    // Details can be null or absent for simple errors
    let details = value.get("details");
    assert!(
        details.is_none() || details.unwrap().is_null(),
        "Simple errors should not have details or should have null details"
    );
}

// MARK: IPC-Safe Serialization Tests

#[test]
fn serialization_produces_valid_json_string() {
    use serde_json::Value;

    let err = AppError::ProjectNotFound("test-path".to_string());

    // Serialize to JSON string (what Tauri IPC will transport)
    let json_string = serde_json::to_string(&err).expect("Must serialize to JSON string");

    // The string itself must be valid JSON (parseable)
    let _parsed: Value =
        serde_json::from_str(&json_string).expect("IPC payload must be valid JSON string");

    // The JSON string should NOT be a plain string value like "error message"
    // It must be an object with structure
    assert!(
        json_string.starts_with('{'),
        "IPC payload must be a JSON object, got: {}",
        json_string
    );
}

#[test]
fn serialization_roundtrip_preserves_data() {
    let original = AppError::AccessDenied("resource X".to_string());

    let json_str = serde_json::to_string(&original).expect("Must serialize");
    let value: serde_json::Value = serde_json::from_str(&json_str).expect("Must deserialize");

    let code = value.get("code").unwrap().as_str().unwrap();
    let message = value.get("message").unwrap().as_str().unwrap();

    assert_eq!(code, "ACCESS_DENIED");
    assert!(message.contains("resource X"));
}

// MARK: AIServicePort::explain_node error contract

use engine::ai::AIProviderResolver;
use engine::ai::AIService;
use engine::ai::AIServicePort;
use engine::models::ScanStatus;
use engine::ports::hexagonal::{MockClock, MockIdGen};
use engine::ports::{GraphRepository, ScanRepository};

/// Minimal ScanRepository that always returns None (file not found).
struct MissingFileScanRepo;
impl ScanRepository for MissingFileScanRepo {
    fn save_scan_result(&self, _: &engine::models::ScanResult) -> engine::Result<()> {
        Ok(())
    }
    fn get_project_by_path(&self, _: &str) -> engine::Result<Option<engine::models::ProjectMeta>> {
        Ok(None)
    }
    fn get_project(&self, _: &str) -> engine::Result<Option<(String, String, i64)>> {
        Ok(None)
    }
    fn get_files(&self, _: &str) -> engine::Result<Vec<engine::models::FileInfo>> {
        Ok(vec![])
    }
    fn get_imports(&self, _: &str) -> engine::Result<Vec<engine::models::ImportInfo>> {
        Ok(vec![])
    }
    fn save_import(&self, _: &engine::models::ImportInfo) -> engine::Result<()> {
        Ok(())
    }
    fn get_file_by_id(&self, _: &str) -> engine::Result<Option<engine::models::FileInfo>> {
        // Always return None — simulates a missing file
        Ok(None)
    }
    fn save_outline_items(&self, _: &str, _: &[engine::models::OutlineItem]) -> engine::Result<()> {
        Ok(())
    }
    fn get_outline_items(&self, _: &str) -> engine::Result<Vec<engine::models::OutlineItem>> {
        Ok(vec![])
    }
    fn get_scan_status(&self, _: &str) -> engine::Result<Option<ScanStatus>> {
        Ok(None)
    }
    fn cancel(&self, _: &str) -> engine::Result<()> {
        Ok(())
    }
}

/// Minimal GraphRepository for the test.
struct EmptyGraphRepo;
impl GraphRepository for EmptyGraphRepo {
    fn save_graph_cache(&self, _: &str, _: &str) -> engine::Result<()> {
        Ok(())
    }
    fn get_graph_cache(&self, _: &str) -> engine::Result<Option<String>> {
        Ok(None)
    }
    fn search_files(
        &self,
        _: &str,
        _: &str,
        _: usize,
    ) -> engine::Result<Vec<engine::models::FileInfo>> {
        Ok(vec![])
    }
    fn get_project_root_for_file(&self, _: &str) -> engine::Result<Option<String>> {
        Ok(None)
    }
    fn save_outline_items(&self, _: &str, _: &[engine::models::OutlineItem]) -> engine::Result<()> {
        Ok(())
    }
    fn get_outline_items(&self, _: &str) -> engine::Result<Vec<engine::models::OutlineItem>> {
        Ok(vec![])
    }
    fn get_dependencies(&self, _: &str) -> engine::Result<Vec<engine::models::NodeRef>> {
        Ok(vec![])
    }
    fn get_dependents(&self, _: &str) -> engine::Result<Vec<engine::models::NodeRef>> {
        Ok(vec![])
    }
}

/// Stub provider required by AIProviderResolver bound; unreachable because
/// explain_node returns early when the file is not found (before resolver is called).
struct UnreachableProvider;
#[async_trait::async_trait]
impl engine::ai::AIProvider for UnreachableProvider {
    async fn explain_node(
        &self,
        _node_id: &str,
        _code_context: &str,
        _dependencies: &[String],
    ) -> engine::Result<engine::models::NodeExplanation> {
        unreachable!("provider must not be reached when file is missing")
    }
    async fn chat(
        &self,
        _messages: &[engine::models::ChatMessage],
        _context: &str,
    ) -> engine::Result<engine::models::ChatResponse> {
        unreachable!("provider must not be reached when file is missing")
    }
}

/// Regression test: `AIServicePort::explain_node` must return `AppError::FileNotFound`
/// when the file is absent from the scan repository — NOT `AppError::NotFound`.
///
/// The C3b spec mandates wire payload `{ "code": "FILE_NOT_FOUND", "details": { "path": <node_id> } }`.
/// Using `AppError::FileNotFound(node_id)` is required; `AppError::NotFound(...)` produces
/// `code: "NOT_FOUND"` which does not match the locked contract.
///
/// This test FAILS if the implementation uses:
///   `.ok_or_else(|| AppError::NotFound(format!("File not found: {}", node_id)))`
#[test]
fn explain_node_returns_file_not_found_when_file_missing() {
    struct DummyResolver;
    impl AIProviderResolver for DummyResolver {
        type Provider = UnreachableProvider;
        fn resolve(&self, _: &engine::models::AIConfig) -> engine::Result<UnreachableProvider> {
            Ok(UnreachableProvider)
        }
    }

    let service = AIService::new(
        DummyResolver,
        MockClock::new(chrono::Utc::now()),
        MockIdGen::new(),
        MissingFileScanRepo,
        EmptyGraphRepo,
    );
    let node_id = "unknown-node-uuid";
    let rt = tokio::runtime::Runtime::new().unwrap();

    let err = rt
        .block_on(service.explain_node(
            &engine::models::AIConfig {
                provider: "anthropic".to_string(),
                api_key: "x".to_string(),
                model: "test".to_string(),
                endpoint: None,
            },
            node_id,
            "proj-1",
            "/tmp",
        ))
        .unwrap_err();

    match err {
        AppError::FileNotFound(path) => {
            assert_eq!(path, node_id);
        }
        other => {
            panic!(
                "explain_node on missing file must return AppError::FileNotFound, got {:?}",
                other
            );
        }
    }
}

// MARK: Logging Compatibility

#[test]
fn error_displays_human_readably() {
    // The Display impl should remain human-readable for logs
    let err = AppError::FileNotFound("config.yaml".to_string());
    let display = err.to_string();

    assert!(
        display.contains("config.yaml"),
        "Display should be human-readable for logs, got: {}",
        display
    );
    assert!(
        display.contains("not found"),
        "Display should include the error nature, got: {}",
        display
    );
}
