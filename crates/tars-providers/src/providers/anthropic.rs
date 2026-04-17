//! Anthropic provider implementation.
//!
//! Uses `GET /v1/models` (requires `anthropic-version` header) for both key
//! validation and model discovery.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::time::Duration;

use crate::{
    error::ProviderError,
    provider::Provider,
    registry::metadata_for,
    types::{Balance, ModelInfo, ProviderId, ProviderMetadata, ValidationResult},
};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub struct AnthropicProvider {
    client: Client,
    base_url: String,
}

impl AnthropicProvider {
    /// Construct with default production base URL.
    #[must_use]
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    /// Construct with a custom base URL (used by tests pointing at a mock).
    ///
    /// # Panics
    /// Panics only if the underlying TLS stack fails to initialize.
    #[must_use]
    pub fn with_base_url(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent("tars/0.4")
            .build()
            .expect("reqwest client builds");
        Self { client, base_url }
    }
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelDto>,
}

#[derive(Debug, Deserialize)]
struct ModelDto {
    id: String,
    display_name: Option<String>,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Anthropic
    }

    fn metadata(&self) -> &'static ProviderMetadata {
        metadata_for(ProviderId::Anthropic)
    }

    async fn validate_key(&self, key: &str) -> Result<ValidationResult, ProviderError> {
        let url = format!("{}/v1/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", key)
            .header("anthropic-version", API_VERSION)
            .send()
            .await
            .map_err(ProviderError::from)?;

        let status = resp.status();
        match status {
            s if s.is_success() => Ok(ValidationResult {
                valid: true,
                message: None,
            }),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Ok(ValidationResult {
                valid: false,
                message: Some(format!(
                    "Key rejected by Anthropic (HTTP {})",
                    status.as_u16()
                )),
            }),
            other => Err(ProviderError::Http(format!("Anthropic returned {other}"))),
        }
    }

    async fn list_models(&self, key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/v1/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", key)
            .header("anthropic-version", API_VERSION)
            .send()
            .await
            .map_err(ProviderError::from)?;

        match resp.status() {
            s if s.is_success() => {
                let parsed: ModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| ProviderError::Parse(e.to_string()))?;
                Ok(parsed
                    .data
                    .into_iter()
                    .map(|m| ModelInfo {
                        id: m.id,
                        display_name: m.display_name,
                        context_window: None,
                        input_price_per_million: None,
                        output_price_per_million: None,
                    })
                    .collect())
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ProviderError::Unauthorized {
                status: resp.status().as_u16(),
            }),
            other => Err(ProviderError::Http(format!("Anthropic returned {other}"))),
        }
    }

    async fn get_balance(&self, _key: &str) -> Result<Option<Balance>, ProviderError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer) -> AnthropicProvider {
        AnthropicProvider::with_base_url(server.uri())
    }

    #[tokio::test]
    async fn validate_key_valid() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("x-api-key", "sk-ant-good"))
            .and(header("anthropic-version", API_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"id": "claude-sonnet-4"}]
            })))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("sk-ant-good").await.unwrap();
        assert!(r.valid);
    }

    #[tokio::test]
    async fn validate_key_invalid_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("sk-ant-bad").await.unwrap();
        assert!(!r.valid);
        assert!(r.message.is_some());
    }

    #[tokio::test]
    async fn list_models_parses_display_name() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": "claude-sonnet-4", "display_name": "Claude Sonnet 4"},
                    {"id": "claude-haiku-4", "display_name": null}
                ]
            })))
            .mount(&server)
            .await;

        let models = provider(&server).list_models("sk-ant").await.unwrap();
        assert_eq!(models.len(), 2);
        let sonnet = models.iter().find(|m| m.id == "claude-sonnet-4").unwrap();
        assert_eq!(sonnet.display_name.as_deref(), Some("Claude Sonnet 4"));
        let haiku = models.iter().find(|m| m.id == "claude-haiku-4").unwrap();
        assert_eq!(haiku.display_name, None);
    }

    #[tokio::test]
    async fn list_models_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let err = provider(&server)
            .list_models("sk-ant-bad")
            .await
            .unwrap_err();
        matches!(err, ProviderError::Unauthorized { .. });
    }

    #[tokio::test]
    async fn get_balance_returns_none() {
        let server = MockServer::start().await;
        assert!(provider(&server).get_balance("x").await.unwrap().is_none());
    }

    #[test]
    fn metadata_matches_registry() {
        let p = AnthropicProvider::new();
        assert_eq!(p.id(), ProviderId::Anthropic);
        assert_eq!(p.metadata().display_name, "Anthropic");
    }
}
