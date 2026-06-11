//! AI application service.
//! Orchestrates use cases and depends on the provider resolver port.

use crate::ai::{AIProvider, AIProviderResolver};
use crate::models::{AIConfig, ChatMessage, ChatResponse, NodeExplanation};
use crate::ports::hexagonal::{Clock, IdGenerator, RandomIdGen, SystemClock};
use crate::Result;
use super::factory::ProviderFactory;

/// AI application service.
///
/// Generic over `R: AIProviderResolver`, `C: Clock`, and `I: IdGenerator` so tests
/// can inject mock doubles for deterministic time/ID generation. The production
/// composition root (`src-tauri/src/lib.rs`) injects real system adapters.
pub struct AIService<R, C, I> {
    resolver: R,
    clock: C,
    id_gen: I,
}

impl<R, C, I> AIService<R, C, I> {
    /// Construct a new `AIService`.
    ///
    /// `resolver`, `clock`, and `id_gen` are all injected from the composition root.
    pub fn new(resolver: R, clock: C, id_gen: I) -> Self {
        Self { resolver, clock, id_gen }
    }

    /// Exposes the resolver for use by the `AIServicePort` impl.
    pub(crate) fn resolver(&self) -> &R {
        &self.resolver
    }
}

impl Default for AIService<ProviderFactory, SystemClock, RandomIdGen> {
    fn default() -> Self {
        Self::new(ProviderFactory, SystemClock, RandomIdGen)
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
impl<R, C, I> AIServicePort for AIService<R, C, I>
where
    R: AIProviderResolver + Send + Sync,
    R::Provider: Send,
    C: Clock,
    I: IdGenerator,
{
    async fn explain_node_with_context(
        &self,
        config: &AIConfig,
        file_info: &crate::models::FileInfo,
        file_content: &str,
        graph: &crate::models::GraphData,
        outline: &[crate::models::OutlineItem],
    ) -> Result<NodeExplanation> {
        use crate::ai::ContextBuilder;

        // Build compact context — prefer semantic outline when available
        let code_context = if !outline.is_empty() {
            ContextBuilder::build_node_context_with_outline(
                file_content,
                &file_info.path,
                graph,
                &file_info.id,
                outline,
            )
        } else {
            ContextBuilder::build_node_context(file_content, &file_info.path, graph, &file_info.id)
        };

        // Build dependency labels list
        let deps: Vec<String> = graph
            .edges
            .iter()
            .filter(|e| e.source == file_info.id)
            .take(5)
            .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.target))
            .map(|n| n.label.clone())
            .collect();

        let provider = self.resolver().resolve(config)?;
        provider
            .explain_node(&file_info.id, &code_context, &deps)
            .await
    }

    async fn chat_with_context(
        &self,
        config: &AIConfig,
        _project_id: &str,
        _root_path: &str,
        file_contents: &[(String, String)],
        graph: &crate::models::GraphData,
        history: &[ChatMessage],
        new_user_message: &str,
    ) -> Result<ChatResponse> {
        use crate::ai::ContextBuilder;

        // Build chat context (use &str refs — ContextBuilder::build_chat_context
        // takes &[(&str, &str)], file_contents lifetime must outlive the call)
        let context = {
            let refs: Vec<(&str, &str)> = file_contents
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();
            ContextBuilder::build_chat_context(&refs, graph, new_user_message)
        };

        // Build full history including the new user message
        let mut full_history = history.to_vec();
        full_history.push(ChatMessage {
            id: self.id_gen.next_id().to_string(),
            role: crate::models::ChatRole::User,
            content: new_user_message.to_string(),
            timestamp: self.clock.now().to_rfc3339(),
        });

        let provider = self.resolver().resolve(config)?;
        provider.chat(&full_history, &context).await
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

    #[async_trait::async_trait]
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

    /// Helper: build an AIService backed by TestResolver with mock clock/id_gen.
    fn make_test_service(resolver: TestResolver) -> std::sync::Arc<dyn AIServicePort> {
        use crate::ports::hexagonal::{MockClock, MockIdGen};
        let clock = MockClock::new(chrono::Utc::now());
        let id_gen = MockIdGen::new();
        std::sync::Arc::new(AIService::new(resolver, clock, id_gen))
    }

    #[test]
    fn explain_node_with_context_uses_resolver_and_provider() {
        let seen_provider = Arc::new(Mutex::new(None));
        let resolver = TestResolver {
            seen_provider: seen_provider.clone(),
        };
        let service = make_test_service(resolver);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let file_info = crate::models::FileInfo {
            id: "file-1".to_string(),
            path: "src/main.rs".to_string(),
            name: "main.rs".to_string(),
            extension: "rs".to_string(),
            symbols: vec![],
            lines: 10,
        };
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };

        let explanation = rt
            .block_on(service.explain_node_with_context(
                &config("anthropic"),
                &file_info,
                "fn main() {}",
                &graph,
                &[],
            ))
            .unwrap();

        assert_eq!(seen_provider.lock().unwrap().as_deref(), Some("anthropic"));
        assert_eq!(explanation.node_id, "file-1");
    }

    #[test]
    fn chat_with_context_uses_resolver_and_provider() {
        let seen_provider = Arc::new(Mutex::new(None));
        let resolver = TestResolver {
            seen_provider: seen_provider.clone(),
        };
        let service = make_test_service(resolver);
        let history = vec![ChatMessage {
            id: "msg-0".to_string(),
            role: ChatRole::User,
            content: "Hola".to_string(),
            timestamp: "2026-06-07T00:00:00Z".to_string(),
        }];
        let rt = tokio::runtime::Runtime::new().unwrap();
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };

        let response = rt
            .block_on(service.chat_with_context(
                &config("custom"),
                "p1",
                "/repo",
                &[],
                &graph,
                &history,
                "hi",
            ))
            .unwrap();

        assert_eq!(seen_provider.lock().unwrap().as_deref(), Some("custom"));
        assert_eq!(response.message.role.to_string(), "assistant");
    }

    #[test]
    fn chat_with_context_resolver_errors_bubble_up() {
        struct FailingResolver;

        impl AIProviderResolver for FailingResolver {
            type Provider = TestProvider;

            fn resolve(&self, _config: &AIConfig) -> Result<Self::Provider> {
                Err(AppError::AIUnavailable("resolver failed".to_string()))
            }
        }

        let service: std::sync::Arc<dyn AIServicePort> = {
            use crate::ports::hexagonal::{MockClock, MockIdGen};
            std::sync::Arc::new(AIService::new(
                FailingResolver,
                MockClock::new(chrono::Utc::now()),
                MockIdGen::new(),
            ))
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };
        let err = rt
            .block_on(service.chat_with_context(
                &config("anthropic"),
                "p1",
                "/repo",
                &[],
                &graph,
                &[],
                "hi",
            ))
            .unwrap_err();

        assert!(matches!(err, AppError::AIUnavailable(_)));
    }

    // ─── AIServicePort trait tests ────────────────────────────────────

    #[test]
    fn aiservice_implements_aiserviceport_trait_object() {
        // Compile-time assertion: AIService<TestResolver, MockClock, MockIdGen>
        // can be coerced to Arc<dyn AIServicePort>. If the trait bound ever regresses
        // (e.g. someone drops the Send + Sync requirement on the impl),
        // this test fails to compile.
        use crate::ports::hexagonal::{MockClock, MockIdGen};
        fn assert_aiserviceport<R, C, I>(service: AIService<R, C, I>) -> std::sync::Arc<dyn AIServicePort>
        where
            R: AIProviderResolver + Send + Sync + 'static,
            C: Clock + 'static,
            I: IdGenerator + 'static,
        {
            std::sync::Arc::new(service)
        }

        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(chrono::Utc::now()),
            MockIdGen::new(),
        );
        let _boxed: std::sync::Arc<dyn AIServicePort> = assert_aiserviceport(service);
    }

    #[test]
    fn explain_node_with_context_builds_context_and_calls_provider() {
        // Verify that explain_node_with_context builds context using
        // ContextBuilder and calls the provider with the right arguments.
        let seen_provider = Arc::new(Mutex::new(None));
        let resolver = TestResolver {
            seen_provider: seen_provider.clone(),
        };
        let service = make_test_service(resolver);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let file_info = crate::models::FileInfo {
            id: "node-1".to_string(),
            path: "src/lib.rs".to_string(),
            name: "lib.rs".to_string(),
            extension: "rs".to_string(),
            symbols: vec![],
            lines: 50,
        };
        let graph = crate::models::GraphData {
            nodes: vec![crate::models::GraphNode {
                id: "node-1".to_string(),
                node_type: crate::models::NodeType::Service,
                label: "lib.rs".to_string(),
                path: "src/lib.rs".to_string(),
                symbol_count: 5,
                position: None,
            }],
            edges: vec![crate::models::GraphEdge {
                id: "edge-1".to_string(),
                source: "node-1".to_string(),
                target: "node-2".to_string(),
                imports: vec![],
            }],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };
        let outline = vec![crate::models::OutlineItem {
            id: "outline-1".to_string(),
            file_id: "node-1".to_string(),
            name: "fn main".to_string(),
            kind: crate::models::OutlineItemKind::Function,
            line_start: 1,
            line_end: 10,
            column_start: None,
            column_end: None,
            children: vec![],
        }];

        let explanation = rt
            .block_on(service.explain_node_with_context(
                &config("anthropic"),
                &file_info,
                "fn main() {}",
                &graph,
                &outline,
            ))
            .unwrap();

        // Verify provider was resolved with correct config
        assert_eq!(seen_provider.lock().unwrap().as_deref(), Some("anthropic"));
        // Verify provider was called with correct node_id
        assert_eq!(explanation.node_id, "node-1");
        // Verify the context was built with the outline (summary contains "summary:" prefix from TestProvider)
        assert!(explanation.summary.starts_with("summary:"));
    }

    #[test]
    fn chat_with_context_builds_context_and_calls_provider() {
        // Verify that chat_with_context builds context and calls provider.
        let seen_provider = Arc::new(Mutex::new(None));
        let resolver = TestResolver {
            seen_provider: seen_provider.clone(),
        };
        let service = make_test_service(resolver);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };
        let history = vec![ChatMessage {
            id: "msg-0".to_string(),
            role: ChatRole::User,
            content: "previous".to_string(),
            timestamp: "2026-06-09T00:00:00Z".to_string(),
        }];
        let file_contents = vec![("src/main.rs".to_string(), "fn main() {}".to_string())];

        let response = rt
            .block_on(service.chat_with_context(
                &config("custom"),
                "p1",
                "/repo",
                &file_contents,
                &graph,
                &history,
                "hello",
            ))
            .unwrap();

        assert_eq!(seen_provider.lock().unwrap().as_deref(), Some("custom"));
        assert_eq!(response.message.role.to_string(), "assistant");
        // TestProvider.format is "{context}:{messages.len()}", so with 2 messages and context="..."
        assert!(response.message.content.contains(':'));
    }

    #[test]
    fn chat_with_context_uses_deterministic_id_and_timestamp_from_ports() {
        struct CapturingProvider {
            seen_messages: Arc<Mutex<Vec<ChatMessage>>>,
        }

        #[async_trait::async_trait]
        impl AIProvider for CapturingProvider {
            async fn explain_node(
                &self,
                _node_id: &str,
                _code_context: &str,
                _dependencies: &[String],
            ) -> Result<NodeExplanation> {
                unreachable!()
            }

            async fn chat(&self, messages: &[ChatMessage], _context: &str) -> Result<ChatResponse> {
                *self.seen_messages.lock().unwrap() = messages.to_vec();
                Ok(ChatResponse {
                    message: ChatMessage {
                        id: "assistant-1".to_string(),
                        role: ChatRole::Assistant,
                        content: "ok".to_string(),
                        timestamp: "2026-06-07T00:00:00Z".to_string(),
                    },
                    referenced_nodes: None,
                })
            }
        }

        struct CapturingResolver {
            seen_messages: Arc<Mutex<Vec<ChatMessage>>>,
        }

        impl AIProviderResolver for CapturingResolver {
            type Provider = CapturingProvider;

            fn resolve(&self, _config: &AIConfig) -> Result<Self::Provider> {
                Ok(CapturingProvider {
                    seen_messages: self.seen_messages.clone(),
                })
            }
        }

        let seen_messages = Arc::new(Mutex::new(Vec::new()));
        let fixed_now = chrono::DateTime::parse_from_rfc3339("2026-06-10T12:34:56Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let service = AIService::new(
            CapturingResolver {
                seen_messages: seen_messages.clone(),
            },
            crate::ports::hexagonal::MockClock::new(fixed_now),
            crate::ports::hexagonal::MockIdGen::new(),
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };
        let history = vec![ChatMessage {
            id: "msg-0".to_string(),
            role: ChatRole::User,
            content: "previous".to_string(),
            timestamp: "2026-06-09T00:00:00Z".to_string(),
        }];

        rt.block_on(service.chat_with_context(
            &config("anthropic"),
            "p1",
            "/repo",
            &[],
            &graph,
            &history,
            "hello",
        ))
        .unwrap();

        let seen_messages = seen_messages.lock().unwrap();
        let appended = seen_messages.last().expect("new user message should be appended");
        assert_eq!(appended.id, uuid::Uuid::nil().to_string());
        assert_eq!(appended.timestamp, "2026-06-10T12:34:56+00:00");
        assert_eq!(appended.content, "hello");
        assert_eq!(appended.role.to_string(), "user");
    }
}
