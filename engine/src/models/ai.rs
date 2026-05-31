//! AI domain models

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeExplanation {
    pub node_id: String,
    pub summary: String,
    pub details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies_note: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for ChatRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatRole::User => write!(f, "user"),
            ChatRole::Assistant => write!(f, "assistant"),
            ChatRole::System => write!(f, "system"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referenced_nodes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_serialization() {
        let msg = ChatMessage {
            id: "msg-1".into(),
            role: ChatRole::User,
            content: "¿Qué hace este archivo?".into(),
            timestamp: "2026-05-31T12:00:00Z".into(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("archivo"));
    }

    #[test]
    fn node_explanation_with_role() {
        let exp = NodeExplanation {
            node_id: "file-1".into(),
            summary: "Maneja autenticación de usuarios".into(),
            details: "## Detalles\n\nEsta clase...".into(),
            dependencies_note: Some("Depende de UserRepository".into()),
            role: "Service Layer".into(),
        };

        let json = serde_json::to_string(&exp).unwrap();
        assert!(json.contains("Service Layer"));
    }
}