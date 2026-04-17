//! `OpenAI` provider implementation.
//!
//! Auth check and model discovery share the same endpoint: `GET /v1/models`.
//! A 200 response means the key is valid; a 401/403 means invalid (surfaced as
//! `ValidationResult { valid: false }` from `validate_key` and as
//! `ProviderError::Unauthorized` from `list_models`).

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

const DEFAULT_BASE_URL: &str = "https://api.openai.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct OpenAiProvider {
    client: Client,
    base_url: String,
}

impl OpenAiProvider {
    /// Construct with default production base URL and HTTPS-only enforcement.
    ///
    /// # Panics
    /// Panics only if the underlying TLS stack fails to initialize, which is
    /// treated as a non-recoverable environment error.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: build_client(true),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Construct with a custom base URL (used by tests pointing at a mock).
    /// HTTPS-only is relaxed here so `wiremock`'s plain HTTP server works.
    ///
    /// # Panics
    /// Panics only if the underlying TLS stack fails to initialize.
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

impl Default for OpenAiProvider {
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
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::OpenAi
    }

    fn metadata(&self) -> &'static ProviderMetadata {
        metadata_for(ProviderId::OpenAi)
    }

    async fn validate_key(&self, key: &str) -> Result<ValidationResult, ProviderError> {
        let url = format!("{}/v1/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(key)
            .send()
            .await
            .map_err(ProviderError::from)?;

        match resp.status() {
            s if s.is_success() => Ok(ValidationResult {
                valid: true,
                message: None,
            }),
            status @ (StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) => Ok(ValidationResult {
                valid: false,
                message: Some(format!("Key rejected by OpenAI (HTTP {})", status.as_u16())),
            }),
            other => Err(ProviderError::Http(format!("OpenAI returned {other}"))),
        }
    }

    async fn list_models(&self, key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/v1/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(key)
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
                        display_name: None,
                        context_window: None,
                        input_price_per_million: None,
                        output_price_per_million: None,
                    })
                    .collect())
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ProviderError::Unauthorized {
                status: resp.status().as_u16(),
            }),
            other => Err(ProviderError::Http(format!("OpenAI returned {other}"))),
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

    fn provider(server: &MockServer) -> OpenAiProvider {
        OpenAiProvider::with_base_url(server.uri())
    }

    #[tokio::test]
    async fn validate_key_returns_valid_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("authorization", "Bearer sk-good"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"id": "gpt-4o"}]
            })))
            .mount(&server)
            .await;

        let result = provider(&server).validate_key("sk-good").await.unwrap();
        assert!(result.valid);
        assert!(result.message.is_none());
    }

    #[tokio::test]
    async fn validate_key_returns_invalid_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let result = provider(&server).validate_key("sk-bad").await.unwrap();
        assert!(!result.valid);
        assert!(result.message.is_some(), "user-facing message expected");
    }

    #[tokio::test]
    async fn validate_key_propagates_server_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = provider(&server).validate_key("sk-x").await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Http(_)),
            "expected Http error, got {err:?}"
        );
    }

    #[tokio::test]
    async fn list_models_parses_data_array() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"id": "gpt-4o", "object": "model"},
                    {"id": "gpt-3.5-turbo", "object": "model"}
                ]
            })))
            .mount(&server)
            .await;

        let models = provider(&server).list_models("sk-good").await.unwrap();
        assert_eq!(models.len(), 2);
        let ids: Vec<_> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"gpt-4o"));
        assert!(ids.contains(&"gpt-3.5-turbo"));
    }

    #[tokio::test]
    async fn list_models_returns_unauthorized_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let err = provider(&server).list_models("sk-bad").await.unwrap_err();
        match err {
            ProviderError::Unauthorized { status } => assert_eq!(status, 401),
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_balance_returns_none() {
        let server = MockServer::start().await;
        let p = provider(&server);
        assert!(p.get_balance("sk-x").await.unwrap().is_none());
    }

    #[test]
    fn metadata_matches_registry() {
        let p = OpenAiProvider::new();
        assert_eq!(p.id(), ProviderId::OpenAi);
        assert_eq!(p.metadata().display_name, "OpenAI");
    }
}
