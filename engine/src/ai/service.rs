//! AI application service.
//! Orchestrates use cases and depends on the provider resolver port.

use crate::ai::{AIProvider, AIProviderResolver};
use crate::models::{AIConfig, ChatMessage, ChatResponse, NodeExplanation};
use crate::ports::hexagonal::{Clock, IdGenerator};
use crate::ports::{GraphRepository, ScanRepository};
use crate::Result;
use serde::{Deserialize, Serialize};

// ─── DTOs ────────────────────────────────────────────────────────────────────

/// Context DTO for node explanation use case.
/// Prepared by `AIService::prepare_explain_context` and consumed by the
/// presentation layer shim in `commands.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainContext {
    /// Node identifier.
    pub node_id: String,
    /// File information for the node.
    pub file_info: crate::models::FileInfo,
    /// Compact context string built from file content, graph, and outline.
    pub code_context: String,
    /// Dependency labels (up to 5).
    pub dependencies: Vec<String>,
    /// Dependent node labels (up to 3).
    pub dependents: Vec<String>,
}

/// Context DTO for chat continuation use case.
/// Prepared by `AIService::prepare_chat_context` and consumed by the
/// presentation layer shim in `commands.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatContext {
    /// Compact context string built from file contents, graph, and question.
    pub context: String,
    /// Full chat history INCLUDING the new user message (exactly one push).
    pub full_history: Vec<ChatMessage>,
}

#[cfg(test)]
mod chat_context_tests {
    use super::*;
    use crate::models::ChatRole;

    #[test]
    fn chat_context_serializes_to_camel_case_json() {
        let ctx = ChatContext {
            context: "question context".to_string(),
            full_history: vec![ChatMessage {
                id: "msg-1".to_string(),
                role: ChatRole::User,
                content: "hello".to_string(),
                timestamp: "2026-06-10T00:00:00Z".to_string(),
            }],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("context"));
        assert!(json.contains("fullHistory"));
    }

    #[test]
    fn chat_context_deserializes_from_camel_case_json() {
        let json = r#"{"context":"ctx","fullHistory":[{"id":"m1","role":"user","content":"hi","timestamp":"2026-06-10T00:00:00Z"}]}"#;
        let ctx: ChatContext = serde_json::from_str(json).unwrap();
        assert_eq!(ctx.context, "ctx");
        assert_eq!(ctx.full_history.len(), 1);
        assert_eq!(ctx.full_history[0].content, "hi");
    }
}

#[cfg(test)]
mod explain_context_tests {
    use super::*;

    #[test]
    fn explain_context_serializes_to_camel_case_json() {
        let ctx = ExplainContext {
            node_id: "node-1".to_string(),
            file_info: crate::models::FileInfo {
                id: "node-1".to_string(),
                path: "src/main.rs".to_string(),
                name: "main.rs".to_string(),
                extension: "rs".to_string(),
                symbols: vec![],
                lines: 10,
            },
            code_context: "fn main() {}".to_string(),
            dependencies: vec!["dep1.rs".to_string()],
            dependents: vec![],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        // camelCase keys — field names in JSON must be camelCase
        assert!(json.contains("nodeId"));
        assert!(json.contains("fileInfo"));
        assert!(json.contains("codeContext"));
        assert!(json.contains("dependencies"));
        assert!(json.contains("dependents"));
    }

    #[test]
    fn explain_context_deserializes_from_camel_case_json() {
        let json = r#"{"nodeId":"n1","fileInfo":{"id":"n1","path":"x.rs","name":"x.rs","extension":"rs","symbols":[],"lines":5},"codeContext":"code","dependencies":["a.rs"],"dependents":[]}"#;
        let ctx: ExplainContext = serde_json::from_str(json).unwrap();
        assert_eq!(ctx.node_id, "n1");
        assert_eq!(ctx.dependencies.len(), 1);
    }
}

/// AI application service.
///
/// Generic over `R: AIProviderResolver`, `C: Clock`, and `I: IdGenerator` so tests
/// can inject mock doubles for deterministic time/ID generation. The production
/// composition root (`src-tauri/src/lib.rs`) injects real system adapters.
/// `S` and `G` are the repository ports used by the thin-shim commands.
pub struct AIService<R, C, I, S, G> {
    resolver: R,
    clock: C,
    id_gen: I,
    scan_repo: S,
    graph_repo: G,
}

impl<R, C: Clock, I: IdGenerator, S: ScanRepository, G: GraphRepository> AIService<R, C, I, S, G> {
    /// Construct a new `AIService`.
    ///
    /// `resolver`, `clock`, and `id_gen` are all injected from the composition root.
    /// `scan_repo` and `graph_repo` are injected to enable the thin-shim commands
    /// (`explain_node` / `chat`) that delegate without orchestration.
    pub fn new(resolver: R, clock: C, id_gen: I, scan_repo: S, graph_repo: G) -> Self {
        Self {
            resolver,
            clock,
            id_gen,
            scan_repo,
            graph_repo,
        }
    }

    /// Prepare a compact context DTO for node explanation.
    ///
    /// Builds the context from `file_content`, `graph`, and `outline` using
    /// `ContextBuilder` — same logic as `explain_node_with_context` — but
    /// returns the raw `ExplainContext` DTO without calling the AI provider.
    /// The presentation layer shim (`commands.rs::explain_node`) uses this to
    /// obtain the context and then calls the provider separately.
    pub fn prepare_explain_context(
        &self,
        file_info: &crate::models::FileInfo,
        file_content: &str,
        graph: &crate::models::GraphData,
        outline: &[crate::models::OutlineItem],
    ) -> ExplainContext {
        use crate::ai::context::ContextBuilder;

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

        let dependencies: Vec<String> = graph
            .edges
            .iter()
            .filter(|e| e.source == file_info.id)
            .take(5)
            .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.target))
            .map(|n| n.label.clone())
            .collect();

        let dependents: Vec<String> = graph
            .edges
            .iter()
            .filter(|e| e.target == file_info.id)
            .take(3)
            .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.source))
            .map(|n| n.label.clone())
            .collect();

        ExplainContext {
            node_id: file_info.id.clone(),
            file_info: file_info.clone(),
            code_context,
            dependencies,
            dependents,
        }
    }

    /// Prepare a context DTO for chat continuation.
    ///
    /// Builds the chat context from `file_contents`, `graph`, and `new_user_message`
    /// using `ContextBuilder::build_chat_context`. Also builds `full_history` by
    /// appending the new user message with a deterministic ID and timestamp from
    /// the injected `id_gen` and `clock` ports. The presentation layer shim
    /// (`commands.rs::chat`) receives this DTO and calls the AI provider with it.
    ///
    /// NOTE: the user message is pushed exactly ONCE here — callers must NOT push
    /// it again before passing `full_history` to the provider.
    pub fn prepare_chat_context(
        &self,
        file_contents: &[(String, String)],
        graph: &crate::models::GraphData,
        history: &[ChatMessage],
        new_user_message: &str,
    ) -> ChatContext {
        use crate::ai::context::ContextBuilder;

        // Build compact chat context
        let refs: Vec<(&str, &str)> = file_contents
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let context = ContextBuilder::build_chat_context(&refs, graph, new_user_message);

        // Build full history including the new user message (exactly one push)
        let mut full_history = history.to_vec();
        full_history.push(ChatMessage {
            id: self.id_gen.next_id().to_string(),
            role: crate::models::ChatRole::User,
            content: new_user_message.to_string(),
            timestamp: self.clock.now().to_rfc3339(),
        });

        ChatContext {
            context,
            full_history,
        }
    }

    /// Exposes the resolver for use by the `AIServicePort` impl.
    pub(crate) fn resolver(&self) -> &R {
        &self.resolver
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

    /// Prepare a compact context DTO for node explanation.
    /// Returns `ExplainContext` without calling the AI provider.
    /// The caller (presentation layer shim) uses this to obtain context
    /// and then calls the AI provider separately.
    fn prepare_explain_context(
        &self,
        file_info: &crate::models::FileInfo,
        file_content: &str,
        graph: &crate::models::GraphData,
        outline: &[crate::models::OutlineItem],
    ) -> ExplainContext;

    /// Prepare a compact context DTO for chat continuation.
    /// Returns `ChatContext` with `full_history` containing exactly ONE
    /// push of the new user message (uses the injected clock/id_gen for
    /// deterministic timestamp/ID). The caller must NOT push the message
    /// again before passing `full_history` to the provider.
    fn prepare_chat_context(
        &self,
        file_contents: &[(String, String)],
        graph: &crate::models::GraphData,
        history: &[ChatMessage],
        new_user_message: &str,
    ) -> ChatContext;

    /// Explain a node using a pre-built `ExplainContext` DTO.
    /// The shim calls `prepare_explain_context` first, then this method
    /// to skip the redundant context-rebuilding that `explain_node_with_context` does.
    async fn explain_node_from_context(
        &self,
        config: &AIConfig,
        ctx: ExplainContext,
    ) -> Result<NodeExplanation>;

    /// Continue chat using a pre-built `ChatContext` DTO.
    /// The shim calls `prepare_chat_context` first (which builds `full_history`
    /// with exactly one user-message push), then this method to skip the
    /// redundant history-building that `chat_with_context` does.
    async fn chat_from_context(&self, config: &AIConfig, ctx: ChatContext) -> Result<ChatResponse>;

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

    /// Thin-shim command handler: fetches file info, content, graph, and outline
    /// internally, then delegates to `prepare_explain_context` + `explain_node_from_context`.
    /// This is the sole entry point for `commands.rs::explain_node` — the Tauri
    /// layer must NOT orchestrate repo/filesystem work.
    async fn explain_node(
        &self,
        config: &AIConfig,
        node_id: &str,
        project_id: &str,
        root_path: &str,
    ) -> Result<NodeExplanation>;

    /// Thin-shim command handler: fetches project files, graph, and history
    /// internally, then delegates to `prepare_chat_context` + `chat_from_context`.
    /// This is the sole entry point for `commands.rs::chat` — the Tauri layer
    /// must NOT orchestrate repo/filesystem work.
    async fn chat(
        &self,
        config: &AIConfig,
        project_id: &str,
        root_path: &str,
        message: &str,
        history: &[ChatMessage],
    ) -> Result<ChatResponse>;
}

#[async_trait::async_trait]
impl<R, C, I, S, G> AIServicePort for AIService<R, C, I, S, G>
where
    R: AIProviderResolver + Send + Sync,
    R::Provider: Send,
    C: Clock,
    I: IdGenerator,
    S: ScanRepository,
    G: GraphRepository,
{
    async fn explain_node_with_context(
        &self,
        config: &AIConfig,
        file_info: &crate::models::FileInfo,
        file_content: &str,
        graph: &crate::models::GraphData,
        outline: &[crate::models::OutlineItem],
    ) -> Result<NodeExplanation> {
        use crate::ai::context::ContextBuilder;

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
        use crate::ai::context::ContextBuilder;

        // Build chat context (use &str refs — ContextBuilder::build_chat_context
        // takes &[(&str, &str)], file_contents lifetime must outlive the call)
        let context = {
            let refs: Vec<(&str, &str)> = file_contents
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();
            ContextBuilder::build_chat_context(&refs, graph, new_user_message)
        };

        // NOTE: full_history is already built by the caller (commands.rs::chat).
        // chat_with_context does NOT push the user message again — that was the
        // double-push bug fixed in C3a.4.

        let provider = self.resolver().resolve(config)?;
        provider.chat(history, &context).await
    }

    fn prepare_explain_context(
        &self,
        file_info: &crate::models::FileInfo,
        file_content: &str,
        graph: &crate::models::GraphData,
        outline: &[crate::models::OutlineItem],
    ) -> ExplainContext {
        AIService::prepare_explain_context(self, file_info, file_content, graph, outline)
    }

    fn prepare_chat_context(
        &self,
        file_contents: &[(String, String)],
        graph: &crate::models::GraphData,
        history: &[ChatMessage],
        new_user_message: &str,
    ) -> ChatContext {
        AIService::prepare_chat_context(self, file_contents, graph, history, new_user_message)
    }

    async fn explain_node_from_context(
        &self,
        config: &AIConfig,
        ctx: ExplainContext,
    ) -> Result<NodeExplanation> {
        let provider = self.resolver().resolve(config)?;
        provider
            .explain_node(&ctx.node_id, &ctx.code_context, &ctx.dependencies)
            .await
    }

    async fn chat_from_context(&self, config: &AIConfig, ctx: ChatContext) -> Result<ChatResponse> {
        let provider = self.resolver().resolve(config)?;
        provider.chat(&ctx.full_history, &ctx.context).await
    }

    async fn explain_node(
        &self,
        config: &AIConfig,
        node_id: &str,
        project_id: &str,
        root_path: &str,
    ) -> Result<NodeExplanation> {
        use std::path::Path;

        // Fetch file info from DB
        let file_info = self
            .scan_repo
            .get_file_by_id(node_id)?
            .ok_or_else(|| crate::AppError::FileNotFound(node_id.to_string()))?;

        // Resolve project root (DB root if available, otherwise state fallback)
        let root = self
            .scan_repo
            .get_project(project_id)?
            .and_then(|(_, r, _)| if r.is_empty() { None } else { Some(r) })
            .unwrap_or_else(|| root_path.to_string());

        // Read actual file content from disk (join root with relative path)
        let file_content =
            std::fs::read_to_string(Path::new(&root).join(&file_info.path)).unwrap_or_default();

        // Get cached graph (fallback to empty)
        let graph_json = self.graph_repo.get_graph_cache(project_id)?;
        let graph = graph_json
            .and_then(|json| serde_json::from_str::<crate::models::GraphData>(&json).ok())
            .unwrap_or_else(|| crate::models::GraphData {
                nodes: vec![],
                edges: vec![],
                project_id: project_id.to_string(),
                generated_at: self.clock.now().to_rfc3339(),
            });

        // Load outline
        let outline = self
            .scan_repo
            .get_outline_items(node_id)
            .unwrap_or_default();

        // Delegate to prepare + from_context
        let ctx = self.prepare_explain_context(&file_info, &file_content, &graph, &outline);
        self.explain_node_from_context(config, ctx).await
    }

    async fn chat(
        &self,
        config: &AIConfig,
        project_id: &str,
        root_path: &str,
        message: &str,
        history: &[ChatMessage],
    ) -> Result<ChatResponse> {
        use std::path::Path;

        // Resolve project root (DB or state fallback)
        let root = self
            .scan_repo
            .get_project(project_id)?
            .and_then(|(_, r, _)| if r.is_empty() { None } else { Some(r) })
            .unwrap_or_else(|| root_path.to_string());

        // Fetch project files and read their contents (limit to 10)
        let files = self.scan_repo.get_files(project_id)?;
        let file_contents: Vec<(String, String)> = files
            .iter()
            .take(10)
            .filter_map(|f| {
                let path = Path::new(&root).join(&f.path);
                std::fs::read_to_string(&path)
                    .ok()
                    .map(|content| (f.path.clone(), content))
            })
            .collect();

        // Get cached graph (fallback to empty)
        let graph_json = self.graph_repo.get_graph_cache(project_id)?;
        let graph = graph_json
            .and_then(|json| serde_json::from_str::<crate::models::GraphData>(&json).ok())
            .unwrap_or_else(|| crate::models::GraphData {
                nodes: vec![],
                edges: vec![],
                project_id: project_id.to_string(),
                generated_at: self.clock.now().to_rfc3339(),
            });

        // Delegate to prepare + from_context (single push in prepare_chat_context)
        let ctx = self.prepare_chat_context(&file_contents, &graph, history, message);
        self.chat_from_context(config, ctx).await
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

    /// Minimal test doubles for ScanRepository and GraphRepository.
    /// All methods return empty/None to satisfy the trait bounds without
    /// affecting tests that don't exercise the new explain_node/chat methods.
    #[derive(Clone, Default)]
    struct TestScanRepo;
    impl ScanRepository for TestScanRepo {
        fn save_scan_result(&self, _: &crate::models::ScanResult) -> Result<()> {
            Ok(())
        }
        fn get_project_by_path(&self, _: &str) -> Result<Option<crate::models::ProjectMeta>> {
            Ok(None)
        }
        fn get_project(&self, _: &str) -> Result<Option<(String, String, i64)>> {
            Ok(None)
        }
        fn get_files(&self, _: &str) -> Result<Vec<crate::models::FileInfo>> {
            Ok(vec![])
        }
        fn get_imports(&self, _: &str) -> Result<Vec<crate::models::ImportInfo>> {
            Ok(vec![])
        }
        fn save_import(&self, _: &crate::models::ImportInfo) -> Result<()> {
            Ok(())
        }
        fn get_file_by_id(&self, _: &str) -> Result<Option<crate::models::FileInfo>> {
            Ok(None)
        }
        fn save_outline_items(&self, _: &str, _: &[crate::models::OutlineItem]) -> Result<()> {
            Ok(())
        }
        fn get_outline_items(&self, _: &str) -> Result<Vec<crate::models::OutlineItem>> {
            Ok(vec![])
        }
        fn get_scan_status(&self, _: &str) -> Result<Option<crate::models::ScanStatus>> {
            Ok(None)
        }
        fn cancel(&self, _: &str) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct TestGraphRepo;
    impl GraphRepository for TestGraphRepo {
        fn save_graph_cache(&self, _: &str, _: &str) -> Result<()> {
            Ok(())
        }
        fn get_graph_cache(&self, _: &str) -> Result<Option<String>> {
            Ok(None)
        }
        fn search_files(&self, _: &str, _: &str, _: usize) -> Result<Vec<crate::models::FileInfo>> {
            Ok(vec![])
        }
        fn get_project_root_for_file(&self, _: &str) -> Result<Option<String>> {
            Ok(None)
        }
        fn save_outline_items(&self, _: &str, _: &[crate::models::OutlineItem]) -> Result<()> {
            Ok(())
        }
        fn get_outline_items(&self, _: &str) -> Result<Vec<crate::models::OutlineItem>> {
            Ok(vec![])
        }
        fn get_dependencies(&self, _: &str) -> Result<Vec<crate::models::NodeRef>> {
            Ok(vec![])
        }
        fn get_dependents(&self, _: &str) -> Result<Vec<crate::models::NodeRef>> {
            Ok(vec![])
        }
    }

    /// Helper: build an AIService backed by TestResolver with mock clock/id_gen.
    fn make_test_service(resolver: TestResolver) -> std::sync::Arc<dyn AIServicePort> {
        use crate::ports::hexagonal::{MockClock, MockIdGen};
        let clock = MockClock::new(chrono::Utc::now());
        let id_gen = MockIdGen::new();
        std::sync::Arc::new(AIService::new(
            resolver,
            clock,
            id_gen,
            TestScanRepo,
            TestGraphRepo,
        ))
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
                TestScanRepo,
                TestGraphRepo,
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
        fn assert_aiserviceport<R, C, I, S, G>(
            service: AIService<R, C, I, S, G>,
        ) -> std::sync::Arc<dyn AIServicePort>
        where
            R: AIProviderResolver + Send + Sync + 'static,
            C: Clock + 'static,
            I: IdGenerator + 'static,
            S: ScanRepository + 'static,
            G: GraphRepository + 'static,
        {
            std::sync::Arc::new(service)
        }

        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(chrono::Utc::now()),
            MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
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
    fn chat_with_context_passes_history_as_is_to_provider() {
        // After C3a.4 fix: chat_with_context does NOT append the user message.
        // The caller (commands.rs::chat) is responsible for building full_history
        // with the message appended. chat_with_context passes history unchanged.
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
        let service = AIService::new(
            CapturingResolver {
                seen_messages: seen_messages.clone(),
            },
            crate::ports::hexagonal::MockClock::new(chrono::Utc::now()),
            crate::ports::hexagonal::MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };
        // Pre-built history with the user message already appended (as commands.rs does)
        let history = vec![
            ChatMessage {
                id: "msg-0".to_string(),
                role: ChatRole::User,
                content: "previous".to_string(),
                timestamp: "2026-06-09T00:00:00Z".to_string(),
            },
            ChatMessage {
                id: uuid::Uuid::nil().to_string(),
                role: ChatRole::User,
                content: "hello".to_string(),
                timestamp: "2026-06-10T12:34:56+00:00".to_string(),
            },
        ];

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

        // Verify the provider received exactly the history we passed (no extra pushes)
        let seen_messages = seen_messages.lock().unwrap();
        assert_eq!(
            seen_messages.len(),
            2,
            "provider should receive exactly 2 messages"
        );
        assert_eq!(seen_messages[0].content, "previous");
        assert_eq!(seen_messages[1].content, "hello");
    }

    // ─── prepare_explain_context tests ─────────────────────────────────────────

    #[test]
    fn prepare_explain_context_returns_explain_context_with_node_id() {
        use crate::ports::hexagonal::{MockClock, MockIdGen};

        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(chrono::Utc::now()),
            MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
        );

        let file_info = crate::models::FileInfo {
            id: "node-1".to_string(),
            path: "src/lib.rs".to_string(),
            name: "lib.rs".to_string(),
            extension: "rs".to_string(),
            symbols: vec![],
            lines: 50,
        };
        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };

        let ctx = service.prepare_explain_context(&file_info, "fn main() {}", &graph, &[]);

        assert_eq!(ctx.node_id, "node-1");
        assert_eq!(ctx.file_info.id, "node-1");
    }

    #[test]
    fn prepare_explain_context_builds_code_context_with_outline() {
        use crate::ports::hexagonal::{MockClock, MockIdGen};

        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(chrono::Utc::now()),
            MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
        );

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
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };
        let outline = vec![crate::models::OutlineItem {
            id: "outline-1".to_string(),
            file_id: "node-1".to_string(),
            name: "my_fn".to_string(),
            kind: crate::models::OutlineItemKind::Function,
            line_start: 1,
            line_end: 10,
            column_start: None,
            column_end: None,
            children: vec![],
        }];

        let ctx = service.prepare_explain_context(&file_info, "fn my_fn() {}", &graph, &outline);

        // Outline semantic context should appear in code_context
        assert!(ctx.code_context.contains("my_fn"));
        assert!(ctx.code_context.contains("Outline"));
    }

    #[test]
    fn prepare_explain_context_collects_dependencies_and_dependents() {
        use crate::ports::hexagonal::{MockClock, MockIdGen};

        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(chrono::Utc::now()),
            MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
        );

        let file_info = crate::models::FileInfo {
            id: "main".to_string(),
            path: "src/main.rs".to_string(),
            name: "main.rs".to_string(),
            extension: "rs".to_string(),
            symbols: vec![],
            lines: 10,
        };
        let graph = crate::models::GraphData {
            nodes: vec![
                crate::models::GraphNode {
                    id: "main".to_string(),
                    node_type: crate::models::NodeType::Unknown,
                    label: "main.rs".to_string(),
                    path: "src/main.rs".to_string(),
                    symbol_count: 1,
                    position: None,
                },
                crate::models::GraphNode {
                    id: "dep1".to_string(),
                    node_type: crate::models::NodeType::Unknown,
                    label: "dep1.rs".to_string(),
                    path: "src/dep1.rs".to_string(),
                    symbol_count: 1,
                    position: None,
                },
                crate::models::GraphNode {
                    id: "dependent1".to_string(),
                    node_type: crate::models::NodeType::Unknown,
                    label: "use1.rs".to_string(),
                    path: "src/use1.rs".to_string(),
                    symbol_count: 1,
                    position: None,
                },
            ],
            edges: vec![
                crate::models::GraphEdge {
                    id: "e1".to_string(),
                    source: "main".to_string(),
                    target: "dep1".to_string(),
                    imports: vec![],
                },
                crate::models::GraphEdge {
                    id: "e2".to_string(),
                    source: "dependent1".to_string(),
                    target: "main".to_string(),
                    imports: vec![],
                },
            ],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };

        let ctx = service.prepare_explain_context(&file_info, "fn main() {}", &graph, &[]);

        // main depends on dep1
        assert!(ctx.dependencies.contains(&"dep1.rs".to_string()));
        // dependent1 depends on main
        assert!(ctx.dependents.contains(&"use1.rs".to_string()));
    }

    // ─── prepare_chat_context tests ──────────────────────────────────────────

    #[test]
    fn prepare_chat_context_returns_chat_context_with_single_user_message() {
        use crate::ports::hexagonal::{MockClock, MockIdGen};

        let fixed_now = chrono::DateTime::parse_from_rfc3339("2026-06-10T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(fixed_now),
            MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
        );

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

        let ctx = service.prepare_chat_context(&file_contents, &graph, &history, "hello");

        // Context must be built
        assert!(!ctx.context.is_empty());
        // full_history: 1 previous + 1 new = 2 messages
        assert_eq!(ctx.full_history.len(), 2, "should have exactly 2 messages");
        // New user message is the last one
        let new_msg = ctx.full_history.last().unwrap();
        assert_eq!(new_msg.content, "hello");
        assert_eq!(new_msg.role.to_string(), "user");
        assert_eq!(new_msg.timestamp, "2026-06-10T12:00:00+00:00");
        assert_eq!(new_msg.id, uuid::Uuid::nil().to_string());
    }

    #[test]
    fn prepare_chat_context_user_message_appears_exactly_once() {
        use crate::ports::hexagonal::{MockClock, MockIdGen};

        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(chrono::Utc::now()),
            MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
        );

        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };

        let ctx = service.prepare_chat_context(&[], &graph, &[], "only-once");

        // Count how many times "only-once" appears in full_history
        let count = ctx
            .full_history
            .iter()
            .filter(|m| m.content == "only-once")
            .count();
        assert_eq!(
            count, 1,
            "user message must appear exactly once in full_history"
        );
    }

    #[test]
    fn prepare_chat_context_includes_prior_history() {
        use crate::ports::hexagonal::{MockClock, MockIdGen};

        let service = AIService::new(
            TestResolver {
                seen_provider: Arc::new(Mutex::new(None)),
            },
            MockClock::new(chrono::Utc::now()),
            MockIdGen::new(),
            TestScanRepo,
            TestGraphRepo,
        );

        let graph = crate::models::GraphData {
            nodes: vec![],
            edges: vec![],
            project_id: "p1".to_string(),
            generated_at: "2026-06-09T00:00:00Z".to_string(),
        };
        let prior = vec![
            ChatMessage {
                id: "msg-0".to_string(),
                role: ChatRole::User,
                content: "first".to_string(),
                timestamp: "2026-06-09T00:00:00Z".to_string(),
            },
            ChatMessage {
                id: "msg-1".to_string(),
                role: ChatRole::Assistant,
                content: "response".to_string(),
                timestamp: "2026-06-09T00:01:00Z".to_string(),
            },
        ];

        let ctx = service.prepare_chat_context(&[], &graph, &prior, "third");

        // prior (2) + new (1) = 3
        assert_eq!(ctx.full_history.len(), 3);
        assert_eq!(ctx.full_history[0].content, "first");
        assert_eq!(ctx.full_history[1].content, "response");
        assert_eq!(ctx.full_history[2].content, "third");
    }
}
