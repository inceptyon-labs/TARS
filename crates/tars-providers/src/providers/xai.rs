//! xAI (Grok) provider.
//!
//! Simple-storage provider: auth-check only via `GET /v1/api-key` with
//! `Authorization: Bearer`. This xAI-specific endpoint returns metadata
//! about the key itself (name, owner, redacted id). No model list surfaced,
//! no balance.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::time::Duration;

use crate::{
    error::ProviderError,
    provider::Provider,
    registry::metadata_for,
    types::{Balance, ModelInfo, ProviderId, ProviderMetadata, ValidationResult},
};

const DEFAULT_BASE_URL: &str = "https://api.x.ai";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct XAiProvider {
    client: Client,
    base_url: String,
}

impl XAiProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: build_client(true),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    #[must_use]
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: build_client(false),
            base_url,
        }
    }
}

fn build_client(https_only: bool) -> Client {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .user_agent("tars/0.4")
        .https_only(https_only)
        .build()
        .expect("reqwest client builds")
}

impl Default for XAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for XAiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::XAi
    }

    fn metadata(&self) -> &'static ProviderMetadata {
        metadata_for(ProviderId::XAi)
    }

    async fn validate_key(&self, key: &str) -> Result<ValidationResult, ProviderError> {
        let url = format!("{}/v1/api-key", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(key)
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
                message: Some(format!("Key rejected by xAI (HTTP {})", status.as_u16())),
            }),
            other => Err(ProviderError::Http(format!("xAI returned {other}"))),
        }
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
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer) -> XAiProvider {
        XAiProvider::with_base_url(server.uri())
    }

    #[tokio::test]
    async fn validate_key_valid_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/api-key"))
            .and(header("authorization", "Bearer xai-good"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "api_key_id": "...",
                "name": "test"
            })))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("xai-good").await.unwrap();
        assert!(r.valid);
    }

    #[tokio::test]
    async fn validate_key_invalid_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/api-key"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("xai-bad").await.unwrap();
        assert!(!r.valid);
        assert!(r.message.is_some());
    }

    #[tokio::test]
    async fn validate_key_propagates_server_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/api-key"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = provider(&server).validate_key("x").await.unwrap_err();
        assert!(matches!(err, ProviderError::Http(_)));
    }

    #[tokio::test]
    async fn list_models_returns_unsupported() {
        let server = MockServer::start().await;
        let err = provider(&server).list_models("x").await.unwrap_err();
        assert!(matches!(err, ProviderError::Unsupported));
    }

    #[tokio::test]
    async fn get_balance_returns_none() {
        let server = MockServer::start().await;
        assert!(provider(&server).get_balance("x").await.unwrap().is_none());
    }

    #[test]
    fn metadata_matches_registry() {
        let p = XAiProvider::new();
        assert_eq!(p.id(), ProviderId::XAi);
        assert_eq!(p.metadata().display_name, "xAI");
        assert!(!p.metadata().supports_models);
        assert!(!p.metadata().supports_balance);
    }
}
