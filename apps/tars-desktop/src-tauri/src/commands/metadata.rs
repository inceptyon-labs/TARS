//! Project metadata and secrets Tauri commands

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tars_core::storage::metadata::ProjectMetadata;
use tars_core::storage::secrets::SecretInput;
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

/// Secret summary (no decrypted data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSummaryResponse {
    pub id: i64,
    pub project_id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Decrypted secret response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretResponse {
    pub id: i64,
    pub name: String,
    pub key: String,
    pub url: String,
    pub notes: String,
}

/// Input for saving/updating a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInputPayload {
    pub name: String,
    pub key: String,
    pub url: String,
    pub notes: String,
}

/// List all secrets for a project (values stay encrypted)
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
                name: s.name,
                created_at: s.created_at,
                updated_at: s.updated_at,
            })
            .collect())
    })
}

/// Decrypt and return a single secret
#[tauri::command]
pub async fn get_project_secret(
    project_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<SecretResponse, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        let secret = store
            .get(uuid, &name)
            .map_err(|e| format!("Failed to get secret: {e}"))?
            .ok_or_else(|| format!("Secret not found: {name}"))?;
        Ok(SecretResponse {
            id: secret.id,
            name: secret.name,
            key: secret.key,
            url: secret.url,
            notes: secret.notes,
        })
    })
}

/// Save a new secret (encrypts before storing)
#[tauri::command]
pub async fn save_project_secret(
    project_id: String,
    input: SecretInputPayload,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    let secret_input = SecretInput {
        name: input.name,
        key: input.key,
        url: input.url,
        notes: input.notes,
    };

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        store
            .save(uuid, &secret_input)
            .map_err(|e| format!("Failed to save secret: {e}"))
    })
}

/// Update an existing secret by id
#[tauri::command]
pub async fn update_project_secret(
    project_id: String,
    secret_id: i64,
    input: SecretInputPayload,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    let secret_input = SecretInput {
        name: input.name,
        key: input.key,
        url: input.url,
        notes: input.notes,
    };

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        store
            .update(uuid, secret_id, &secret_input)
            .map_err(|e| format!("Failed to update secret: {e}"))
    })
}

/// Delete a secret
#[tauri::command]
pub async fn delete_project_secret(
    project_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = SecretStore::new(db.connection());
        store
            .delete(uuid, &name)
            .map_err(|e| format!("Failed to delete secret: {e}"))
    })
}
