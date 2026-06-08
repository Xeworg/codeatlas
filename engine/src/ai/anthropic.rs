//! Anthropic AI provider implementation.

use crate::ai::AIProvider;
use crate::models::{AIConfig, ChatMessage, ChatResponse, ChatRole, NodeExplanation};
use crate::{AppError, Result};

const DEFAULT_ANTHROPIC_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

fn map_error_response(status: u16, body: &str) -> AppError {
    match status {
        401 | 403 => AppError::InvalidApiKey,
        429 => AppError::AIRateLimited,
        400 if body.contains("token") => AppError::AITokenLimit,
        _ => AppError::AIUnavailable(format!("HTTP {}", status)),
    }
}

#[derive(Debug)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    endpoint: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model: Option<&str>) -> Self {
        Self::with_endpoint(api_key, model, None)
    }

    pub fn with_endpoint(
        api_key: impl Into<String>,
        model: Option<&str>,
        endpoint: Option<&str>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.unwrap_or("minimax").to_string(),
            // Guard: empty string falls back to default instead of using an empty URL.
            endpoint: endpoint
                .filter(|s| !s.is_empty())
                .unwrap_or(DEFAULT_ANTHROPIC_ENDPOINT)
                .to_string(),
        }
    }

    pub fn from_config(config: &AIConfig) -> Self {
        Self::with_endpoint(
            config.api_key.clone(),
            Some(config.model.as_str()),
            config.endpoint.as_deref(),
        )
    }

    async fn request(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        });

        let response = client
            .post(&self.endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::AIUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(map_error_response(status, &body));
        }

        #[derive(serde::Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContent>,
        }
        #[derive(serde::Deserialize)]
        struct AnthropicContent {
            text: String,
        }

        let resp: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AppError::AIUnavailable(e.to_string()))?;

        resp.content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| AppError::AIUnavailable("Empty response".into()))
    }
}

impl AIProvider for AnthropicProvider {
    async fn explain_node(
        &self,
        node_id: &str,
        code_context: &str,
        dependencies: &[String],
    ) -> Result<NodeExplanation> {
        let prompt = format!(
            "Explicá este archivo de código de forma concisa:\n\n{}\n\n\
            Dependencias: {}\n\nRespondé en español con:\n\
            1. Resumen de 1-2 frases\n\
            2. Rol en la arquitectura\n\
            3. Detalles relevantes",
            code_context,
            dependencies.join(", ")
        );

        let response = self.request(&prompt).await?;

        Ok(NodeExplanation {
            node_id: node_id.to_string(),
            summary: response.lines().next().unwrap_or("").to_string(),
            details: response,
            dependencies_note: None,
            role: "Módulo del proyecto".to_string(),
        })
    }

    async fn chat(&self, messages: &[ChatMessage], context: &str) -> Result<ChatResponse> {
        let history = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Contexto del proyecto:\n{}\n\nHistorial:\n{}\n\nRespondé en español basándote únicamente en el código proporcionado.",
            context,
            history
        );

        let response = self.request(&prompt).await?;

        Ok(ChatResponse {
            message: ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: ChatRole::Assistant,
                content: response,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
            referenced_nodes: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppError;

    #[test]
    fn provider_creation() {
        let provider = AnthropicProvider::new("placeholder", Some("claude-3"));
        assert_eq!(provider.model, "claude-3");
        assert_eq!(provider.endpoint, DEFAULT_ANTHROPIC_ENDPOINT);
    }

    #[test]
    fn provider_creation_with_custom_endpoint() {
        let provider = AnthropicProvider::with_endpoint(
            "placeholder",
            Some("claude-3"),
            Some("https://gateway.example.test/v1/messages"),
        );

        assert_eq!(
            provider.endpoint,
            "https://gateway.example.test/v1/messages"
        );
    }

    #[test]
    fn error_mapping_invalid_api_key() {
        let err = map_error_response(401, "unauthorized");
        assert!(matches!(err, AppError::InvalidApiKey));
    }

    #[test]
    fn error_mapping_403_forbidden() {
        // 403 should also map to InvalidApiKey (forbidden != unauthorized)
        let err = map_error_response(403, "access forbidden");
        assert!(matches!(err, AppError::InvalidApiKey));
    }

    #[test]
    fn error_mapping_rate_limited() {
        let err = map_error_response(429, "rate limited");
        assert!(matches!(err, AppError::AIRateLimited));
    }

    #[test]
    fn error_mapping_token_limit() {
        let err = map_error_response(400, "maximum_token_limit_exceeded");
        assert!(matches!(err, AppError::AITokenLimit));
    }

    #[test]
    fn error_mapping_400_without_token_keyword() {
        // 400 without "token" in body should map to AIUnavailable, not AITokenLimit
        let err = map_error_response(400, "bad_request");
        assert!(matches!(err, AppError::AIUnavailable(_)));
    }

    #[test]
    fn error_mapping_server_error() {
        let err = map_error_response(500, "server error");
        assert!(matches!(err, AppError::AIUnavailable(_)));
    }
}
