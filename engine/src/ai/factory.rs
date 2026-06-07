//! Provider resolution for the AI application layer.
//! This is the first seam toward a hexagonal architecture: the application
//! depends on a resolver port, and infrastructure decides which adapter to use.

use crate::ai::{anthropic::AnthropicProvider, resolved::ResolvedProvider, AIProvider};
use crate::models::AIConfig;
use crate::{AppError, Result};

pub trait AIProviderResolver {
    type Provider: AIProvider;

    fn resolve(&self, config: &AIConfig) -> Result<Self::Provider>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProviderFactory;

impl AIProviderResolver for ProviderFactory {
    type Provider = ResolvedProvider;

    fn resolve(&self, config: &AIConfig) -> Result<Self::Provider> {
        match config.provider.as_str() {
            "anthropic" | "custom" => Ok(ResolvedProvider::Anthropic(
                AnthropicProvider::from_config(config),
            )),
            other => Err(AppError::AIUnavailable(format!(
                "Unsupported AI provider: {}",
                other
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(provider: &str) -> AIConfig {
        AIConfig {
            provider: provider.to_string(),
            api_key: "test-key".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            endpoint: None,
        }
    }

    #[test]
    fn resolves_anthropic_provider() {
        let factory = ProviderFactory;
        let provider = factory.resolve(&config("anthropic")).unwrap();

        assert!(matches!(provider, ResolvedProvider::Anthropic(_)));
    }

    #[test]
    fn keeps_custom_as_compatibility_alias_during_migration() {
        let factory = ProviderFactory;
        let provider = factory.resolve(&config("custom")).unwrap();

        assert!(matches!(provider, ResolvedProvider::Anthropic(_)));
    }

    #[test]
    fn rejects_unknown_provider() {
        let factory = ProviderFactory;
        let err = factory.resolve(&config("unknown")).unwrap_err();

        assert!(matches!(err, AppError::AIUnavailable(_)));
    }
}
