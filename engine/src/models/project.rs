//! Project domain models

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ScanStatus {
    #[default]
    Idle,
    Scanning,
    BuildingGraph,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub project_id: String,
    pub project_name: String,
    pub root_path: String,
    pub files_count: usize,
    pub symbols_count: usize,
    pub imports_count: usize,
    pub files: Vec<super::FileInfo>,
    pub scan_duration_ms: u64,
    pub status: ScanStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_result_serializes_correctly() {
        let result = ScanResult {
            project_id: "test-123".into(),
            project_name: "TestProject".into(),
            root_path: "/tmp/test".into(),
            files_count: 10,
            symbols_count: 50,
            imports_count: 30,
            files: vec![],
            scan_duration_ms: 1500,
            status: ScanStatus::Ready,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("ready"));
    }

    #[test]
    fn scan_status_default_is_idle() {
        assert_eq!(ScanStatus::default(), ScanStatus::Idle);
    }
}
