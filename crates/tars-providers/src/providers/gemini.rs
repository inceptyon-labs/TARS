//! Google Gemini provider implementation.
//!
//! Auth check and model discovery share `GET /v1beta/models`, with the API
//! key passed via the `x-goog-api-key` header (not the `?key=` query param)
//! so it never lands in the request URL or error messages.

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

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct GeminiProvider {
    client: Client,
    base_url: String,
}

impl GeminiProvider {
    /// Construct with default production base URL and HTTPS-only enforcement.
    ///
    /// # Panics
    /// Panics only if the underlying TLS stack fails to initialize.
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

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelDto>,
}

#[derive(Debug, Deserialize)]
struct ModelDto {
    name: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "inputTokenLimit")]
    input_token_limit: Option<u32>,
}

fn strip_models_prefix(name: &str) -> String {
    name.strip_prefix("models/").unwrap_or(name).to_string()
}

#[async_trait]
impl Provider for GeminiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Gemini
    }

    fn metadata(&self) -> &'static ProviderMetadata {
        metadata_for(ProviderId::Gemini)
    }

    async fn validate_key(&self, key: &str) -> Result<ValidationResult, ProviderError> {
        let url = format!("{}/v1beta/models", self.base_url);
        // Pass the key via `x-goog-api-key` header rather than the `?key=`
        // query parameter so the key never appears in the request URL. Any
        // reqwest error whose Display embeds the URL (parse failures, DNS
        // errors, etc.) would otherwise leak the key to log output and the
        // Tauri error surface.
        let resp = self
            .client
            .get(&url)
            .header("x-goog-api-key", key)
            .query(&[("pageSize", "1000")])
            .send()
            .await
            .map_err(ProviderError::from)?;

        let status = resp.status();
        match status {
            s if s.is_success() => Ok(ValidationResult {
                valid: true,
                message: None,
            }),
            // Only auth-specific statuses mark the key invalid. HTTP 400 in
            // particular is ambiguous (malformed request, quota config, etc.)
            // and must surface as a real error instead of silently tagging
            // a working key as rejected.
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Ok(ValidationResult {
                valid: false,
                message: Some(format!("Key rejected by Gemini (HTTP {})", status.as_u16())),
            }),
            other => Err(ProviderError::Http(format!("Gemini returned {other}"))),
        }
    }

    async fn list_models(&self, key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/v1beta/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("x-goog-api-key", key)
            .query(&[("pageSize", "1000")])
            .send()
            .await
            .map_err(ProviderError::from)?;

        match resp.status() {
            s if s.is_success() => {
                let parsed: ModelsResponse = resp.json().await.map_err(ProviderError::from)?;
                Ok(parsed
                    .models
                    .into_iter()
                    .map(|m| ModelInfo {
                        id: strip_models_prefix(&m.name),
                        display_name: m.display_name,
                        context_window: m.input_token_limit,
                        input_price_per_million: None,
                        output_price_per_million: None,
                    })
                    .collect())
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ProviderError::Unauthorized {
                status: resp.status().as_u16(),
            }),
            other => Err(ProviderError::Http(format!("Gemini returned {other}"))),
        }
    }

    async fn get_balance(&self, _key: &str) -> Result<Option<Balance>, ProviderError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer) -> GeminiProvider {
        GeminiProvider::with_base_url(server.uri())
    }

    #[tokio::test]
    async fn validate_key_valid() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1beta/models"))
            .and(header("x-goog-api-key", "AIza-good"))
            .and(query_param("pageSize", "1000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "models": []
            })))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("AIza-good").await.unwrap();
        assert!(r.valid);
    }

    #[tokio::test]
    async fn validate_key_surfaces_400_as_http_error() {
        // 400 is ambiguous on Gemini (malformed arg, quota config, etc.).
        // We surface it as a real error rather than tagging the key invalid.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1beta/models"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server)
            .await;

        let err = provider(&server).validate_key("bad").await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Http(_)),
            "expected Http error, got {err:?}"
        );
    }

    #[tokio::test]
    async fn validate_key_invalid_on_403() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1beta/models"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("bad").await.unwrap();
        assert!(!r.valid);
    }

    #[tokio::test]
    async fn list_models_strips_prefix_and_maps_fields() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1beta/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "models": [
                    {
                        "name": "models/gemini-1.5-pro",
                        "displayName": "Gemini 1.5 Pro",
                        "inputTokenLimit": 1_048_576
                    },
                    {
                        "name": "models/gemini-1.5-flash",
                        "displayName": "Gemini 1.5 Flash"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let models = provider(&server).list_models("AIza").await.unwrap();
        assert_eq!(models.len(), 2);

        let pro = models.iter().find(|m| m.id == "gemini-1.5-pro").unwrap();
        assert_eq!(pro.display_name.as_deref(), Some("Gemini 1.5 Pro"));
        assert_eq!(pro.context_window, Some(1_048_576));

        let flash = models.iter().find(|m| m.id == "gemini-1.5-flash").unwrap();
        assert_eq!(flash.context_window, None);
    }

    #[tokio::test]
    async fn list_models_unauthorized_on_403() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1beta/models"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let err = provider(&server).list_models("bad").await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Unauthorized { .. }),
            "expected Unauthorized, got {err:?}"
        );
    }

    #[test]
    fn metadata_matches_registry() {
        let p = GeminiProvider::new();
        assert_eq!(p.id(), ProviderId::Gemini);
        assert_eq!(p.metadata().display_name, "Google Gemini");
    }
}
