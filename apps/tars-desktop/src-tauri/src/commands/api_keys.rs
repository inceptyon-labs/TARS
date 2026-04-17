//! API key vault commands.
//!
//! Stores AI provider keys encrypted at rest. Validation and model discovery
//! delegate to `tars_providers` for HTTP calls and to `tars_core::storage`
//! for the cached model catalog.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use tars_core::storage::api_keys::{ApiKeyInput, ApiKeyRecord, ApiKeyStore};
use tars_core::storage::model_cache::{CachedModel, ModelCache, ModelRow};
use tars_providers::{all_metadata, provider_for, ProviderId};
use tauri::State;

use crate::state::AppState;

/// Summary row returned to the frontend (never includes the decrypted key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeySummaryResponse {
    pub id: i64,
    pub provider_id: String,
    pub label: String,
    pub last_validated_at: Option<String>,
    pub last_valid: Option<bool>,
    pub balance: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input payload for creating an API key.
///
/// `Deserialize` only — the `key` field is plaintext and must not be
/// serialized outbound. `Debug` is hand-rolled to redact the key.
#[derive(Clone, Deserialize)]
pub struct ApiKeyInputPayload {
    pub provider_id: String,
    pub label: String,
    pub key: String,
}

impl fmt::Debug for ApiKeyInputPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyInputPayload")
            .field("provider_id", &self.provider_id)
            .field("label", &self.label)
            .field("key", &"<redacted>")
            .finish()
    }
}

/// Static provider metadata surfaced to the UI
#[derive(Debug, Clone, Serialize)]
pub struct ProviderMetadataResponse {
    pub id: String,
    pub display_name: String,
    pub docs_url: String,
    pub key_format_hint: String,
    pub supports_models: bool,
    pub supports_balance: bool,
}

/// Result of a validation attempt.
///
/// `unverifiable: true` means the provider does not expose a usable
/// auth-check endpoint (e.g. Perplexity), so the key was stored but no
/// remote validation was performed and the persisted `last_valid` state was
/// left untouched. The UI should surface this as a neutral message rather
/// than a success or failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unverifiable: bool,
}

/// Static metadata for every supported provider
#[tauri::command]
pub fn list_providers() -> Vec<ProviderMetadataResponse> {
    all_metadata()
        .into_iter()
        .map(|m| ProviderMetadataResponse {
            id: m.id.as_str().to_string(),
            display_name: m.display_name.to_string(),
            docs_url: m.docs_url.to_string(),
            key_format_hint: m.key_format_hint.to_string(),
            supports_models: m.supports_models,
            supports_balance: m.supports_balance,
        })
        .collect()
}

/// Add a new API key (encrypted before storage). Returns the new row id.
#[tauri::command]
pub async fn add_api_key(
    input: ApiKeyInputPayload,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    if ProviderId::parse(&input.provider_id).is_none() {
        return Err(format!("Unknown provider: {}", input.provider_id));
    }

    let store_input = ApiKeyInput {
        provider_id: input.provider_id,
        label: input.label,
        key: input.key,
    };

    state.with_db(|db| {
        let store = ApiKeyStore::new(db.connection());
        store
            .save(&store_input)
            .map_err(|e| format!("Failed to save api key: {e}"))
    })
}

/// List all stored keys (masked — never returns decrypted material).
#[tauri::command]
pub async fn list_api_keys(
    state: State<'_, AppState>,
) -> Result<Vec<ApiKeySummaryResponse>, String> {
    state.with_db(|db| {
        let store = ApiKeyStore::new(db.connection());
        let keys = store
            .list()
            .map_err(|e| format!("Failed to list api keys: {e}"))?;
        Ok(keys
            .into_iter()
            .map(|k| ApiKeySummaryResponse {
                id: k.id,
                provider_id: k.provider_id,
                label: k.label,
                last_validated_at: k.last_validated_at,
                last_valid: k.last_valid,
                balance: k.balance,
                created_at: k.created_at,
                updated_at: k.updated_at,
            })
            .collect())
    })
}

/// Delete a key by id.
#[tauri::command]
pub async fn delete_api_key(id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    state.with_db(|db| {
        let store = ApiKeyStore::new(db.connection());
        store
            .delete(id)
            .map_err(|e| format!("Failed to delete api key: {e}"))
    })
}

/// Re-validate the stored key against the provider.
///
/// Decrypts the stored key, calls the provider's `validate_key` endpoint,
/// and — for providers that support it (currently `DeepSeek`) — also fetches
/// the account balance on success. The validation outcome and balance are
/// persisted via `ApiKeyStore::update_validation` so the UI sees a fresh
/// `last_valid` and `balance`.
#[tauri::command]
pub async fn validate_api_key(
    id: i64,
    state: State<'_, AppState>,
) -> Result<ValidationResponse, String> {
    let record: ApiKeyRecord = state.with_db(|db| {
        let store = ApiKeyStore::new(db.connection());
        store
            .get(id)
            .map_err(|e| format!("Failed to load api key: {e}"))?
            .ok_or_else(|| format!("API key {id} not found"))
    })?;

    let provider_id = ProviderId::parse(&record.provider_id).ok_or_else(|| {
        format!(
            "Unknown provider stored for key {id}: {}",
            record.provider_id
        )
    })?;
    let provider = provider_for(provider_id);

    let result = match provider.validate_key(&record.key).await {
        Ok(r) => r,
        // Providers that expose no auth-check endpoint (e.g. Perplexity) short
        // circuit here: we preserve the existing `last_valid` state (no
        // `update_validation` call) and flag the response as unverifiable so
        // the UI can render a neutral badge instead of an error toast.
        Err(tars_providers::ProviderError::Unsupported) => {
            return Ok(ValidationResponse {
                valid: false,
                message: Some(
                    "This provider does not expose an auth-check endpoint — key stored but not verified."
                        .to_string(),
                ),
                unverifiable: true,
            });
        }
        Err(e) => return Err(format!("Validation failed: {e}")),
    };

    // Only fetch balance on successful validation and only for providers that
    // expose it. Error policy:
    //   - Transient errors (network, 5xx): preserve `record.balance` so the
    //     UI keeps showing the last known value until the next refresh.
    //   - Unauthorized: clear — the balance call sees the key as rejected,
    //     so the prior value is no longer trustworthy even though `validate`
    //     just succeeded (likely a race with revocation).
    //   - `Ok(None)`: provider explicitly reported no balance; clear.
    let balance_value = if result.valid {
        if provider.metadata().supports_balance {
            match provider.get_balance(&record.key).await {
                Ok(Some(b)) => Some(b.raw),
                Ok(None) | Err(tars_providers::ProviderError::Unauthorized { .. }) => None,
                Err(_) => record.balance.clone(),
            }
        } else {
            None
        }
    } else {
        None
    };

    state.with_db(|db| {
        let store = ApiKeyStore::new(db.connection());
        store
            .update_validation(id, result.valid, balance_value.as_ref())
            .map(|_| ())
            .map_err(|e| format!("Failed to persist validation: {e}"))
    })?;

    Ok(ValidationResponse {
        valid: result.valid,
        message: result.message,
        unverifiable: false,
    })
}

/// Cached model row exposed to the frontend.
///
/// Mirrors `tars_core::storage::model_cache::CachedModel` but uses an RFC 3339
/// string for `fetched_at` so the value round-trips cleanly through JSON.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CachedModelResponse {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: Option<String>,
    pub context_window: Option<u32>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub fetched_at: String,
}

impl From<CachedModel> for CachedModelResponse {
    fn from(m: CachedModel) -> Self {
        Self {
            provider_id: m.provider_id,
            model_id: m.model_id,
            display_name: m.display_name,
            context_window: m.context_window,
            input_price: m.input_price,
            output_price: m.output_price,
            fetched_at: m.fetched_at.to_rfc3339(),
        }
    }
}

/// Decrypt and return the plaintext value of a stored API key.
///
/// Used by the UI for click-to-reveal and copy-to-clipboard. The plaintext is
/// only ever held in memory long enough to round-trip through the IPC layer; we
/// do not log, persist, or otherwise echo it. Callers must treat the result as
/// short-lived secret material.
#[tauri::command]
pub async fn reveal_api_key(id: i64, state: State<'_, AppState>) -> Result<String, String> {
    let record: ApiKeyRecord = state.with_db(|db| {
        let store = ApiKeyStore::new(db.connection());
        store
            .get(id)
            .map_err(|e| format!("Failed to load api key: {e}"))?
            .ok_or_else(|| format!("API key {id} not found"))
    })?;
    Ok(record.key)
}

/// List the cached model catalog for a provider.
///
/// Returns the rows previously written by `refresh_models`. Empty result is
/// valid (no refresh has run yet, or the provider has no models). The UI is
/// responsible for prompting the user to refresh when this is empty.
#[tauri::command]
pub async fn list_provider_models(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<CachedModelResponse>, String> {
    if ProviderId::parse(&provider_id).is_none() {
        return Err(format!("Unknown provider: {provider_id}"));
    }
    state.with_db(|db| {
        let cache = ModelCache::new(db.connection());
        let rows = cache
            .list_for_provider(&provider_id)
            .map_err(|e| format!("Failed to list models: {e}"))?;
        Ok(rows.into_iter().map(CachedModelResponse::from).collect())
    })
}

/// Refresh the cached model list for the given provider.
///
/// Picks a stored key for the provider (preferring keys previously marked
/// valid, falling back to any stored key), calls `list_models`, and replaces
/// the cached rows atomically. Returns the number of models written.
///
/// Returns an error if no key is stored for the provider, if the key is
/// rejected by the upstream API, or if the network call fails.
#[tauri::command]
pub async fn refresh_models(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let pid = ProviderId::parse(&provider_id)
        .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;

    // Pull all keys for this provider, then pick one: prefer last_valid=true,
    // otherwise fall back to the first key stored.
    let record: ApiKeyRecord = state.with_db(|db| {
        let store = ApiKeyStore::new(db.connection());
        let summaries = store
            .list_by_provider(&provider_id)
            .map_err(|e| format!("Failed to list api keys: {e}"))?;
        if summaries.is_empty() {
            return Err(format!(
                "No API key stored for provider '{provider_id}'. Add a key before refreshing."
            ));
        }
        let chosen = summaries
            .iter()
            .find(|s| s.last_valid == Some(true))
            .unwrap_or(&summaries[0]);
        store
            .get(chosen.id)
            .map_err(|e| format!("Failed to load api key: {e}"))?
            .ok_or_else(|| format!("API key {} vanished between list and get", chosen.id))
    })?;

    let provider = provider_for(pid);
    let models = provider
        .list_models(&record.key)
        .await
        .map_err(|e| format!("Model list fetch failed: {e}"))?;

    let rows: Vec<ModelRow> = models
        .into_iter()
        .map(|m| ModelRow {
            model_id: m.id,
            display_name: m.display_name,
            context_window: m.context_window,
            input_price: m.input_price_per_million,
            output_price: m.output_price_per_million,
        })
        .collect();

    let now = Utc::now();
    let provider_key = pid.as_str().to_string();
    state.with_db(|db| {
        let cache = ModelCache::new(db.connection());
        cache
            .upsert_all(&provider_key, &rows, now)
            .map_err(|e| format!("Failed to cache models: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn cached_model_response_from_cached_model_preserves_fields() {
        let fetched = Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap();
        let m = CachedModel {
            provider_id: "openai".into(),
            model_id: "gpt-4o".into(),
            display_name: Some("GPT-4o".into()),
            context_window: Some(128_000),
            input_price: Some(2.5),
            output_price: Some(10.0),
            fetched_at: fetched,
        };
        let resp: CachedModelResponse = m.into();
        assert_eq!(resp.provider_id, "openai");
        assert_eq!(resp.model_id, "gpt-4o");
        assert_eq!(resp.display_name.as_deref(), Some("GPT-4o"));
        assert_eq!(resp.context_window, Some(128_000));
        assert_eq!(resp.input_price, Some(2.5));
        assert_eq!(resp.output_price, Some(10.0));
        // RFC 3339 round-trip — must be parseable back to the same instant.
        let parsed = chrono::DateTime::parse_from_rfc3339(&resp.fetched_at)
            .expect("fetched_at must serialize as RFC 3339");
        assert_eq!(parsed.with_timezone(&Utc), fetched);
    }

    #[test]
    fn cached_model_response_handles_optional_fields() {
        let m = CachedModel {
            provider_id: "anthropic".into(),
            model_id: "claude-3-haiku".into(),
            display_name: None,
            context_window: None,
            input_price: None,
            output_price: None,
            fetched_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        };
        let resp: CachedModelResponse = m.into();
        assert!(resp.display_name.is_none());
        assert!(resp.context_window.is_none());
        assert!(resp.input_price.is_none());
        assert!(resp.output_price.is_none());
    }

    #[test]
    fn validation_response_omits_unverifiable_when_false() {
        let resp = ValidationResponse {
            valid: true,
            message: None,
            unverifiable: false,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(
            json.get("unverifiable").is_none(),
            "unverifiable=false must be omitted so old clients ignore it"
        );
        assert_eq!(json["valid"], true);
    }

    #[test]
    fn validation_response_serializes_unverifiable_when_true() {
        let resp = ValidationResponse {
            valid: false,
            message: Some("not checked".into()),
            unverifiable: true,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["unverifiable"], true);
        assert_eq!(json["valid"], false);
        assert_eq!(json["message"], "not checked");
    }

    #[test]
    fn cached_model_response_serializes_to_camel_case_compatible_json() {
        // The frontend expects snake_case on the wire (matches the existing
        // ApiKeySummaryResponse convention).
        let resp = CachedModelResponse {
            provider_id: "openai".into(),
            model_id: "gpt-4o".into(),
            display_name: None,
            context_window: Some(8000),
            input_price: None,
            output_price: None,
            fetched_at: "2026-04-17T12:00:00+00:00".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["provider_id"], "openai");
        assert_eq!(json["model_id"], "gpt-4o");
        assert_eq!(json["context_window"], 8000);
        assert!(json["display_name"].is_null());
    }
}
