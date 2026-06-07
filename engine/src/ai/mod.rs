//! AI module — ports, application services, and infrastructure adapters.

pub mod anthropic;
pub mod context;
pub mod factory;
pub mod provider;
pub mod resolved;
pub mod service;

// AnthropicProvider is internal; do not expose as public API.
pub use context::ContextBuilder;
pub use factory::AIProviderResolver;
pub use provider::AIProvider;
// ResolvedProvider is internal; do not expose as public API.
pub use service::AIService;
