//! API key vault commands.
//!
//! Stores AI provider keys encrypted at rest. Validation and model discovery
//! delegate to `tars_providers` for HTTP calls and to `tars_core::storage`
//! for the cached model catalog.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use tars_core::storage::api_keys::{ApiKeyInput, ApiKeyRecord, ApiKeyStore};
use tars_core::storage::model_cache::{ModelCache, ModelRow};
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

/// Result of a validation attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub message: Option<String>,
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

    let result = provider
        .validate_key(&record.key)
        .await
        .map_err(|e| format!("Validation failed: {e}"))?;

    // Only fetch balance on successful validation, and only for providers
    // that expose it. A balance-query failure must not clobber the valid=true
    // outcome — we still record the key as valid, just without balance.
    let balance_value = if result.valid && provider.metadata().supports_balance {
        match provider.get_balance(&record.key).await {
            Ok(Some(b)) => Some(b.raw),
            Ok(None) | Err(_) => None,
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
