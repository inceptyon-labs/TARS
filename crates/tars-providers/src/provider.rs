//! `Provider` trait: async interface each provider impl satisfies

use async_trait::async_trait;

use crate::{
    error::ProviderError,
    types::{Balance, ModelInfo, ProviderId, ProviderMetadata, ValidationResult},
};

/// Common interface for AI providers.
///
/// Implementations live in their own modules (one per provider) and handle
/// auth-check, model discovery, and optional balance queries.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stable identifier
    fn id(&self) -> ProviderId;

    /// Static metadata (display name, docs URL, capabilities)
    fn metadata(&self) -> &'static ProviderMetadata;

    /// Check whether the given API key authenticates against the provider.
    ///
    /// # Errors
    /// Returns a `ProviderError` if the network call fails or the response
    /// is unparseable. An invalid key is reported via `ValidationResult.valid = false`,
    /// not an error.
    async fn validate_key(&self, key: &str) -> Result<ValidationResult, ProviderError>;

    /// Fetch the current list of models the given key can access.
    ///
    /// # Errors
    /// Returns `ProviderError::Unauthorized` if the key is rejected,
    /// `ProviderError::Unsupported` if the provider has no discovery endpoint,
    /// or other variants for network / parse failures.
    async fn list_models(&self, key: &str) -> Result<Vec<ModelInfo>, ProviderError>;

    /// Fetch the account balance if the provider exposes it.
    ///
    /// Returns `Ok(None)` if the provider does not support balance queries.
    ///
    /// # Errors
    /// Returns a `ProviderError` on network or auth failure.
    async fn get_balance(&self, key: &str) -> Result<Option<Balance>, ProviderError>;
}
