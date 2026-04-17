//! API key vault commands.
//!
//! Stores AI provider keys encrypted at rest. Validation and model discovery
//! will call into `tars_providers` once real provider impls land (issue #7);
//! for now the `validate_api_key` and `refresh_models` commands return a
//! `NotImplemented` error the UI can display gracefully.

use serde::{Deserialize, Serialize};
use tars_core::storage::api_keys::{ApiKeyInput, ApiKeyStore};
use tars_providers::{all_metadata, ProviderId};
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

/// Input payload for creating an API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInputPayload {
    pub provider_id: String,
    pub label: String,
    pub key: String,
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
/// Stub until issue #7 supplies real `Provider` implementations.
#[tauri::command]
pub async fn validate_api_key(
    id: i64,
    _state: State<'_, AppState>,
) -> Result<ValidationResponse, String> {
    let _ = id;
    Err("Validation not yet implemented — lands in issue #7".to_string())
}

/// Refresh the cached model list for the given provider.
///
/// Stub until issue #7 supplies real `Provider` implementations.
#[tauri::command]
pub async fn refresh_models(
    provider_id: String,
    _state: State<'_, AppState>,
) -> Result<usize, String> {
    if ProviderId::parse(&provider_id).is_none() {
        return Err(format!("Unknown provider: {provider_id}"));
    }
    Err("Model refresh not yet implemented — lands in issue #7".to_string())
}
