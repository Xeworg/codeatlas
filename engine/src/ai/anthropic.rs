//! Anthropic AI provider implementation.

use crate::ai::AIProvider;
use crate::models::{
    ChatMessage, ChatResponse, ChatRole, NodeExplanation,
};
use crate::{AppError, Result};

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    endpoint: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.unwrap_or("minimax").to_string(),
            endpoint: "https://api.anthropic.com/v1/messages".to_string(),
        }
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
            let status = response.status();
            return Err(match status.as_u16() {
                401 | 403 => AppError::InvalidApiKey,
                429 => AppError::AIRateLimited,
                400 if response.text().await.unwrap_or_default().contains("token") => {
                    AppError::AITokenLimit
                }
                _ => AppError::AIUnavailable(format!("HTTP {}", status)),
            });
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

    async fn chat(
        &self,
        messages: &[ChatMessage],
        context: &str,
    ) -> Result<ChatResponse> {
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

    #[test]
    fn provider_creation() {
        let provider = AnthropicProvider::new("test-key", Some("claude-3"));
        assert_eq!(provider.model, "claude-3");
    }
}