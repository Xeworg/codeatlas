//! AI module — ports, application services, and infrastructure adapters.

pub mod context;
pub mod provider;

mod anthropic;
mod factory;
mod resolved;
mod service;

// AnthropicProvider is internal; do not expose as public API.
pub use context::ContextBuilder;
pub use factory::{AIProviderResolver, ProviderFactory};
pub use provider::AIProvider;
// ResolvedProvider is internal; do not expose as public API.
pub use service::{AIService, AIServicePort};
