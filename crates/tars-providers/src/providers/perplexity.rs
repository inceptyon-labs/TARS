//! Perplexity provider (unverifiable).
//!
//! Perplexity's API does not expose a GET-able auth-check endpoint — the
//! only public surface is `POST /chat/completions`, which costs tokens. We
//! therefore store the key but return `ProviderError::Unsupported` from
//! `validate_key` to signal "unverifiable". Callers should surface this to
//! the user as a badge rather than a hard error.

use async_trait::async_trait;

use crate::{
    error::ProviderError,
    provider::Provider,
    registry::metadata_for,
    types::{Balance, ModelInfo, ProviderId, ProviderMetadata, ValidationResult},
};

#[derive(Default)]
pub struct PerplexityProvider;

impl PerplexityProvider {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Provider for PerplexityProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Perplexity
    }

    fn metadata(&self) -> &'static ProviderMetadata {
        metadata_for(ProviderId::Perplexity)
    }

    async fn validate_key(&self, _key: &str) -> Result<ValidationResult, ProviderError> {
        Err(ProviderError::Unsupported)
    }

    async fn list_models(&self, _key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        Err(ProviderError::Unsupported)
    }

    async fn get_balance(&self, _key: &str) -> Result<Option<Balance>, ProviderError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn validate_key_returns_unsupported() {
        let p = PerplexityProvider::new();
        let err = p.validate_key("anything").await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Unsupported),
            "expected Unsupported, got {err:?}"
        );
    }

    #[tokio::test]
    async fn list_models_returns_unsupported() {
        let p = PerplexityProvider::new();
        let err = p.list_models("x").await.unwrap_err();
        assert!(matches!(err, ProviderError::Unsupported));
    }

    #[tokio::test]
    async fn get_balance_returns_none() {
        let p = PerplexityProvider::new();
        assert!(p.get_balance("x").await.unwrap().is_none());
    }

    #[test]
    fn metadata_matches_registry() {
        let p = PerplexityProvider::new();
        assert_eq!(p.id(), ProviderId::Perplexity);
        assert_eq!(p.metadata().display_name, "Perplexity");
        assert!(!p.metadata().supports_models);
        assert!(!p.metadata().supports_balance);
    }
}
