//! AI Provider trait — Domain abstraction for LLM backends.

use crate::models::{ChatMessage, ChatResponse, NodeExplanation};

/// Trait for AI providers (Anthropic, OpenAI, etc.).
///
/// Note: `async fn` in traits is unstable in Rust stable without the
/// `async_fn_in_trait` nightly feature or explicit `Send` bounds.
/// This code targets Tauri + Tokio which guarantee `Send` futures;
/// we allow the lint here and document the rationale to keep the API
/// clean while development stays on stable Rust.
#[allow(async_fn_in_trait)]
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
