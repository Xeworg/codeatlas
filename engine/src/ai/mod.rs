//! AI module — Application + Infrastructure layer
//! AI provider abstraction and context builder.

pub mod anthropic;
pub mod context;
pub mod provider;

pub use anthropic::AnthropicProvider;
pub use context::ContextBuilder;
pub use provider::AIProvider;
