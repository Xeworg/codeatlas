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

// ─── Port trait ─────────────────────────────────────────────────────────────

/// Port trait through which the presentation layer consumes AI
/// functionality.
///
/// The presentation layer (Tauri commands) must NOT depend on the
/// concrete `AIService` struct; it consumes the AI service exclusively
/// through `Arc<dyn AIServicePort>`. This keeps the boundary firm
/// even after the `AppState` refactor moves to trait objects (PR-B
/// steps B.4 onwards).
///
/// `Send + Sync` is required so the trait object can live inside
/// `tauri::State<AppState>`, which is itself `Send + Sync`.
///
/// The `_with_context` naming communicates that the methods take the
/// full context (file content, graph, outline) and orchestrate
/// `ContextBuilder` internally — the caller does not pre-build the
/// prompt. The previous methods (`explain_node`, `chat`) remain
/// available for direct use by other engine callers; the presentation
/// layer migrates to this trait in PR-B Tasks B.10 and B.11.
#[allow(dead_code)] // consumed by presentation in PR-B Tasks B.4-B.11
#[async_trait::async_trait]
pub trait AIServicePort: Send + Sync {
    /// Explain a code node given its file content, graph, and outline.
    /// The service composes the code context from the inputs.
    async fn explain_node_with_context(
        &self,
        config: &AIConfig,
        file_info: &crate::models::FileInfo,
        file_content: &str,
        graph: &crate::models::GraphData,
        outline: &[crate::models::OutlineItem],
    ) -> Result<NodeExplanation>;

    /// Continue a chat conversation given the project context.
    async fn chat_with_context(
        &self,
        config: &AIConfig,
        project_id: &str,
        root_path: &str,
        file_contents: &[(String, String)],
        graph: &crate::models::GraphData,
        history: &[ChatMessage],
        new_user_message: &str,
    ) -> Result<ChatResponse>;
}

#[async_trait::async_trait]
impl<R> AIServicePort for AIService<R>
where
    R: AIProviderResolver + Send + Sync,
{
    async fn explain_node_with_context(
        &self,
        _config: &AIConfig,
        _file_info: &crate::models::FileInfo,
        _file_content: &str,
        _graph: &crate::models::GraphData,
        _outline: &[crate::models::OutlineItem],
    ) -> Result<NodeExplanation> {
        // Body lands in PR-B Task B.10. For now, return an explicit
        // AppError so any caller that reaches this method before the
        // body is implemented gets a clear diagnostic instead of a
        // panic.
        Err(crate::AppError::Internal(
            "AIServicePort::explain_node_with_context not yet implemented; \
             see PR-B Task B.10"
                .to_string(),
        ))
    }

    async fn chat_with_context(
        &self,
        _config: &AIConfig,
        _project_id: &str,
        _root_path: &str,
        _file_contents: &[(String, String)],
        _graph: &crate::models::GraphData,
        _history: &[ChatMessage],
        _new_user_message: &str,
    ) -> Result<ChatResponse> {
        // Body lands in PR-B Task B.11. See note above.
        Err(crate::AppError::Internal(
            "AIServicePort::chat_with_context not yet implemented; \
             see PR-B Task B.11"
                .to_string(),
        ))
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

    // ─── AIServicePort trait tests ────────────────────────────────────

    #[test]
    fn aiservice_implements_aiserviceport_trait_object() {
        // Compile-time assertion: AIService<TestResolver> can be coerced
        // to Arc<dyn AIServicePort>. If the trait bound ever regresses
        // (e.g. someone drops the Send + Sync requirement on the impl),
        // this test fails to compile.
        fn assert_aiserviceport<R>(service: AIService<R>) -> std::sync::Arc<dyn AIServicePort>
        where
            R: AIProviderResolver + Send + Sync + 'static,
        {
            std::sync::Arc::new(service)
        }

        let service = AIService::new(TestResolver {
            seen_provider: Arc::new(Mutex::new(None)),
        });
        let _boxed: std::sync::Arc<dyn AIServicePort> = assert_aiserviceport(service);
    }

    #[test]
    fn explain_node_with_context_returns_implementation_pending_error() {
        // Until PR-B Task B.10 fills the body, the trait method must
        // surface a clear AppError::Internal so any caller that reaches
        // it before the implementation lands gets a diagnostic instead
        // of a panic.
        let service: std::sync::Arc<dyn AIServicePort> =
            std::sync::Arc::new(AIService::new(TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            }));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let file_info = crate::models::FileInfo {
            id: "f1".to_string(),
            path: "src/main.rs".to_string(),
            name: "main.rs".to_string(),
            extension: "rs".to_string(),
            symbols: vec![],
            lines: 1,
        };
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-08T00:00:00Z".to_string(),
        };

        let err = rt
            .block_on(service.explain_node_with_context(
                &config("anthropic"),
                &file_info,
                "fn main() {}",
                &graph,
                &[],
            ))
            .unwrap_err();

        assert!(
            matches!(err, AppError::Internal(_)),
            "expected AppError::Internal, got {:?}",
            err
        );
    }

    #[test]
    fn chat_with_context_returns_implementation_pending_error() {
        let service: std::sync::Arc<dyn AIServicePort> =
            std::sync::Arc::new(AIService::new(TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            }));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-08T00:00:00Z".to_string(),
        };

        let err = rt
            .block_on(service.chat_with_context(
                &config("anthropic"),
                "p1",
                "/repo",
                &[],
                &graph,
                &[],
                "hola",
            ))
            .unwrap_err();

        assert!(matches!(err, AppError::Internal(_)));
    }
}
