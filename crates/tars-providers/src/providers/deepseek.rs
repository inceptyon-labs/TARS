//! `DeepSeek` provider implementation.
//!
//! Uses `GET /user/balance` for both key validation and balance (the balance
//! endpoint doubles as an auth check). Model discovery is via the `OpenAI`-
//! compatible `GET /v1/models`.

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

const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub struct DeepseekProvider {
    client: Client,
    base_url: String,
}

impl DeepseekProvider {
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

impl Default for DeepseekProvider {
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

#[derive(Debug, Deserialize)]
struct BalanceResponse {
    #[serde(default)]
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
}

#[async_trait]
impl Provider for DeepseekProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Deepseek
    }

    fn metadata(&self) -> &'static ProviderMetadata {
        metadata_for(ProviderId::Deepseek)
    }

    async fn validate_key(&self, key: &str) -> Result<ValidationResult, ProviderError> {
        let url = format!("{}/user/balance", self.base_url);
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
                    "Key rejected by DeepSeek (HTTP {})",
                    status.as_u16()
                )),
            }),
            other => Err(ProviderError::Http(format!("DeepSeek returned {other}"))),
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
            other => Err(ProviderError::Http(format!("DeepSeek returned {other}"))),
        }
    }

    async fn get_balance(&self, key: &str) -> Result<Option<Balance>, ProviderError> {
        let url = format!("{}/user/balance", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(key)
            .send()
            .await
            .map_err(ProviderError::from)?;

        match resp.status() {
            s if s.is_success() => {
                // Capture the raw JSON for display so the UI can show all fields
                // DeepSeek returns (e.g. granted/topped-up breakdown).
                let raw: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| ProviderError::Parse(e.to_string()))?;
                let parsed: BalanceResponse = serde_json::from_value(raw.clone())
                    .map_err(|e| ProviderError::Parse(e.to_string()))?;
                let first = parsed.balance_infos.into_iter().next();
                match first {
                    Some(info) => {
                        let amount = info
                            .total_balance
                            .parse::<f64>()
                            .map_err(|e| ProviderError::Parse(format!("Bad total_balance: {e}")))?;
                        Ok(Some(Balance {
                            currency: info.currency,
                            amount,
                            raw,
                        }))
                    }
                    None => Ok(Some(Balance {
                        currency: String::new(),
                        amount: 0.0,
                        raw,
                    })),
                }
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ProviderError::Unauthorized {
                status: resp.status().as_u16(),
            }),
            other => Err(ProviderError::Http(format!("DeepSeek returned {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer) -> DeepseekProvider {
        DeepseekProvider::with_base_url(server.uri())
    }

    #[tokio::test]
    async fn validate_key_valid() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/balance"))
            .and(header("authorization", "Bearer sk-good"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "is_available": true,
                "balance_infos": [
                    {"currency": "USD", "total_balance": "12.34",
                     "granted_balance": "0", "topped_up_balance": "12.34"}
                ]
            })))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("sk-good").await.unwrap();
        assert!(r.valid);
    }

    #[tokio::test]
    async fn validate_key_invalid_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/balance"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let r = provider(&server).validate_key("bad").await.unwrap();
        assert!(!r.valid);
        assert!(r.message.is_some());
    }

    #[tokio::test]
    async fn get_balance_parses_first_info() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/balance"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "is_available": true,
                "balance_infos": [
                    {"currency": "USD", "total_balance": "42.50",
                     "granted_balance": "10.00", "topped_up_balance": "32.50"}
                ]
            })))
            .mount(&server)
            .await;

        let bal = provider(&server)
            .get_balance("sk-good")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bal.currency, "USD");
        assert!((bal.amount - 42.50).abs() < 0.001);
        assert_eq!(bal.raw["is_available"], serde_json::Value::Bool(true));
    }

    #[tokio::test]
    async fn get_balance_handles_empty_infos() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/balance"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "is_available": false,
                "balance_infos": []
            })))
            .mount(&server)
            .await;

        let bal = provider(&server)
            .get_balance("sk-x")
            .await
            .unwrap()
            .unwrap();
        assert!(bal.amount.abs() < f64::EPSILON);
        assert!(bal.currency.is_empty());
    }

    #[tokio::test]
    async fn get_balance_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/balance"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let err = provider(&server).get_balance("bad").await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Unauthorized { .. }),
            "expected Unauthorized, got {err:?}"
        );
    }

    #[tokio::test]
    async fn list_models_parses_data() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": "deepseek-chat", "object": "model"},
                    {"id": "deepseek-reasoner", "object": "model"}
                ]
            })))
            .mount(&server)
            .await;

        let models = provider(&server).list_models("sk-good").await.unwrap();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn metadata_matches_registry() {
        let p = DeepseekProvider::new();
        assert_eq!(p.id(), ProviderId::Deepseek);
        assert_eq!(p.metadata().display_name, "DeepSeek");
        assert!(p.metadata().supports_balance);
    }
}
