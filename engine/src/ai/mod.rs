//! AI module — ports, application services, and infrastructure adapters.

pub mod context;
pub mod provider;

mod anthropic;
mod factory;
mod resolved;
mod service;

// ProviderFactory is needed by the composition root in src-tauri.
pub use factory::{AIProviderResolver, ProviderFactory};
pub use provider::AIProvider;
// ContextBuilder is pub(crate) — internal to engine only.
pub use service::{AIService, AIServicePort, ChatContext, ExplainContext};
