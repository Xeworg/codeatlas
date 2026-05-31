//! AI Provider trait — Domain abstraction for LLM backends.

use crate::models::{ChatMessage, ChatResponse, NodeExplanation};

/// Trait for AI providers (Anthropic, OpenAI, etc.)
pub trait AIProvider: Send + Sync {
    /// Generate an explanation for a code node.
    async fn explain_node(
        &self,
        node_id: &str,
        code_context: &str,
        dependencies: &[String],
    ) -> Result<NodeExplanation, crate::AppError>;

    /// Send a chat message with project context.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        context: &str,
    ) -> Result<ChatResponse, crate::AppError>;
}