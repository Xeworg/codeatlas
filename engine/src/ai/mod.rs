//! AI module — Application + Infrastructure layer
//! AI provider abstraction and context builder.

pub mod provider;
pub mod anthropic;
pub mod context;

pub use provider::AIProvider;
pub use anthropic::AnthropicProvider;
pub use context::ContextBuilder;