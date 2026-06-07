//! AI application service.
//! Orchestrates use cases and depends on the provider resolver port.

use crate::ai::{factory::ProviderFactory, AIProvider, AIProviderResolver};
use crate::models::{AIConfig, ChatMessage, ChatResponse, NodeExplanation};
use crate::Result;

pub struct AIService<R = ProviderFactory> {
    resolver: R,
}

impl Default for AIService<ProviderFactory> {
    fn default() -> Self {
        Self {
            resolver: ProviderFactory,
        }
    }
}

impl<R> AIService<R> {
    pub fn new(resolver: R) -> Self {
        Self { resolver }
    }
}

impl<R> AIService<R>
where
    R: AIProviderResolver,
{
    pub async fn explain_node(
        &self,
        config: &AIConfig,
        node_id: &str,
        code_context: &str,
        dependencies: &[String],
    ) -> Result<NodeExplanation> {
        let provider = self.resolver.resolve(config)?;
        provider
            .explain_node(node_id, code_context, dependencies)
            .await
    }

    pub async fn chat(
        &self,
        config: &AIConfig,
        messages: &[ChatMessage],
        context: &str,
    ) -> Result<ChatResponse> {
        let provider = self.resolver.resolve(config)?;
        provider.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::AIProviderResolver;
    use crate::models::{ChatMessage, ChatRole};
    use crate::AppError;
    use std::sync::{Arc, Mutex};

    struct TestProvider;

    impl AIProvider for TestProvider {
        async fn explain_node(
            &self,
            node_id: &str,
            code_context: &str,
            dependencies: &[String],
        ) -> Result<NodeExplanation> {
            Ok(NodeExplanation {
                node_id: node_id.to_string(),
                summary: format!("summary:{}", code_context),
                details: dependencies.join(","),
                dependencies_note: None,
                role: "test".to_string(),
            })
        }

        async fn chat(&self, messages: &[ChatMessage], context: &str) -> Result<ChatResponse> {
            Ok(ChatResponse {
                message: ChatMessage {
                    id: "msg-1".to_string(),
                    role: ChatRole::Assistant,
                    content: format!("{}:{}", context, messages.len()),
                    timestamp: "2026-06-07T00:00:00Z".to_string(),
                },
                referenced_nodes: None,
            })
        }
    }

    #[derive(Clone)]
    struct TestResolver {
        seen_provider: Arc<Mutex<Option<String>>>,
    }

    impl AIProviderResolver for TestResolver {
        type Provider = TestProvider;

        fn resolve(&self, config: &AIConfig) -> Result<Self::Provider> {
            *self.seen_provider.lock().unwrap() = Some(config.provider.clone());
            Ok(TestProvider)
        }
    }

    fn config(provider: &str) -> AIConfig {
        AIConfig {
            provider: provider.to_string(),
            api_key: "placeholder".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            endpoint: None,
        }
    }

    #[test]
    fn explain_node_uses_resolver_and_provider() {
        let seen_provider = Arc::new(Mutex::new(None));
        let service = AIService::new(TestResolver {
            seen_provider: seen_provider.clone(),
        });
        let dependencies = vec!["dep-a".to_string(), "dep-b".to_string()];
        let rt = tokio::runtime::Runtime::new().unwrap();

        let explanation = rt
            .block_on(service.explain_node(
                &config("anthropic"),
                "file-1",
                "fn main() {}",
                &dependencies,
            ))
            .unwrap();

        assert_eq!(seen_provider.lock().unwrap().as_deref(), Some("anthropic"));
        assert_eq!(explanation.node_id, "file-1");
        assert_eq!(explanation.details, "dep-a,dep-b");
    }

    #[test]
    fn chat_uses_resolver_and_provider() {
        let seen_provider = Arc::new(Mutex::new(None));
        let service = AIService::new(TestResolver {
            seen_provider: seen_provider.clone(),
        });
        let history = vec![ChatMessage {
            id: "msg-0".to_string(),
            role: ChatRole::User,
            content: "Hola".to_string(),
            timestamp: "2026-06-07T00:00:00Z".to_string(),
        }];
        let rt = tokio::runtime::Runtime::new().unwrap();

        let response = rt
            .block_on(service.chat(&config("custom"), &history, "ctx"))
            .unwrap();

        assert_eq!(seen_provider.lock().unwrap().as_deref(), Some("custom"));
        assert_eq!(response.message.role.to_string(), "assistant");
        assert_eq!(response.message.content, "ctx:1");
    }

    #[test]
    fn resolver_errors_bubble_up() {
        struct FailingResolver;

        impl AIProviderResolver for FailingResolver {
            type Provider = TestProvider;

            fn resolve(&self, _config: &AIConfig) -> Result<Self::Provider> {
                Err(AppError::AIUnavailable("resolver failed".to_string()))
            }
        }

        let service = AIService::new(FailingResolver);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(service.chat(&config("anthropic"), &[], "ctx"))
            .unwrap_err();

        assert!(matches!(err, AppError::AIUnavailable(_)));
    }
}
