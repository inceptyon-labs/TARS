//! Project metadata and secrets Tauri commands

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tars_core::storage::metadata::ProjectMetadata;
use tars_core::storage::{MetadataStore, SecretStore};
use tauri::State;

// ── Metadata commands ──────────────────────────────────────────────

/// Get project metadata
#[tauri::command]
pub async fn get_project_metadata(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ProjectMetadata>, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = MetadataStore::new(db.connection());
        store
            .get(uuid)
            .map_err(|e| format!("Failed to get metadata: {e}"))
    })
}

/// Save project metadata
#[tauri::command]
pub async fn save_project_metadata(
    project_id: String,
    metadata: ProjectMetadata,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = MetadataStore::new(db.connection());
        store
            .save(uuid, &metadata)
            .map_err(|e| format!("Failed to save metadata: {e}"))
    })
}

// ── Secrets commands ───────────────────────────────────────────────

/// Secret summary (no decrypted value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSummaryResponse {
    pub id: i64,
    pub project_id: String,
    pub key: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Decrypted secret response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretValueResponse {
    pub key: String,
    pub value: String,
}

/// List all secret keys for a project (values stay encrypted)
#[tauri::command]
pub async fn list_project_secrets(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<SecretSummaryResponse>, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        let secrets = store
            .list(uuid)
            .map_err(|e| format!("Failed to list secrets: {e}"))?;
        Ok(secrets
            .into_iter()
            .map(|s| SecretSummaryResponse {
                id: s.id,
                project_id: s.project_id,
                key: s.key,
                created_at: s.created_at,
                updated_at: s.updated_at,
            })
            .collect())
    })
}

/// Decrypt and return a single secret value
#[tauri::command]
pub async fn get_project_secret(
    project_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<SecretValueResponse, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        let secret = store
            .get(uuid, &key)
            .map_err(|e| format!("Failed to get secret: {e}"))?
            .ok_or_else(|| format!("Secret not found: {key}"))?;
        Ok(SecretValueResponse {
            key: secret.key,
            value: secret.value,
        })
    })
}

/// Save a secret (encrypts before storing)
#[tauri::command]
pub async fn save_project_secret(
    project_id: String,
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        store
            .save(uuid, &key, &value)
            .map_err(|e| format!("Failed to save secret: {e}"))
    })
}

/// Delete a secret
#[tauri::command]
pub async fn delete_project_secret(
    project_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        store
            .delete(uuid, &key)
            .map_err(|e| format!("Failed to delete secret: {e}"))
    })
}
