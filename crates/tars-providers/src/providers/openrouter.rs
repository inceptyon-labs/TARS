//! OpenRouter provider.
//!
//! Simple-storage provider: auth-check only via `GET /api/v1/auth/key` with
//! `Authorization: Bearer`. This endpoint returns rate-limit/credit info
//! about the key itself, but we only use it as an auth check here.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::time::Duration;

use crate::{
    error::ProviderError,
    provider::Provider,
    registry::metadata_for,
    types::{Balance, ModelInfo, ProviderId, ProviderMetadata, ValidationResult},
};

const DEFAULT_BASE_URL: &str = "https://openrouter.ai";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct OpenRouterProvider {
    client: Client,
    base_url: String,
}

impl OpenRouterProvider {
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

impl Default for OpenRouterProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn id(&self) -> ProviderId {
        ProviderId::OpenRouter
    }

    fn metadata(&self) -> &'static ProviderMetadata {
        metadata_for(ProviderId::OpenRouter)
    }

    async fn validate_key(&self, key: &str) -> Result<ValidationResult, ProviderError> {
        let url = format!("{}/api/v1/auth/key", self.base_url);
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
                message: Some(format!(
                    "Key rejected by OpenRouter (HTTP {})",
                    status.as_u16()
                )),
            }),
            other => Err(ProviderError::Http(format!("OpenRouter returned {other}"))),
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

    fn provider(server: &MockServer) -> OpenRouterProvider {
        OpenRouterProvider::with_base_url(server.uri())
    }

    #[tokio::test]
    async fn validate_key_valid_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/auth/key"))
            .and(header("authorization", "Bearer sk-or-good"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "usage": 0 }
            })))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("sk-or-good").await.unwrap();
        assert!(r.valid);
    }

    #[tokio::test]
    async fn validate_key_invalid_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/auth/key"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("sk-or-bad").await.unwrap();
        assert!(!r.valid);
        assert!(r.message.is_some());
    }

    #[tokio::test]
    async fn validate_key_propagates_server_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/auth/key"))
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
        let p = OpenRouterProvider::new();
        assert_eq!(p.id(), ProviderId::OpenRouter);
        assert_eq!(p.metadata().display_name, "OpenRouter");
        assert!(!p.metadata().supports_models);
        assert!(!p.metadata().supports_balance);
    }
}
