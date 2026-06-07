//! Provider adapter enum used by the application layer.
//! Keeps provider dispatch centralized while preserving static dispatch.

use crate::ai::{anthropic::AnthropicProvider, AIProvider};
use crate::models::{ChatMessage, ChatResponse, NodeExplanation};
use crate::Result;

#[derive(Debug)]
pub enum ResolvedProvider {
    Anthropic(AnthropicProvider),
}

impl AIProvider for ResolvedProvider {
    async fn explain_node(
        &self,
        node_id: &str,
        code_context: &str,
        dependencies: &[String],
    ) -> Result<NodeExplanation> {
        match self {
            Self::Anthropic(provider) => {
                provider
                    .explain_node(node_id, code_context, dependencies)
                    .await
            }
        }
    }

    async fn chat(&self, messages: &[ChatMessage], context: &str) -> Result<ChatResponse> {
        match self {
            Self::Anthropic(provider) => provider.chat(messages, context).await,
        }
    }
}
