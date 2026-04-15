//! Project metadata and secrets Tauri commands

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tars_core::storage::metadata::ProjectMetadata;
use tars_core::storage::secrets::SecretInput;
use tars_core::storage::{MetadataStore, SecretStore};
use tauri::State;

use std::collections::HashMap;

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

/// Get categories for all projects: path → "Apps" | "Websites" | "Tools"
#[tauri::command]
pub async fn get_project_categories(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    state.with_db(|db| {
        let project_store = tars_core::storage::projects::ProjectStore::new(db.connection());
        let meta_store = MetadataStore::new(db.connection());
        let projects = project_store
            .list()
            .map_err(|e| format!("Failed to list projects: {e}"))?;

        let mut result = HashMap::new();
        for proj in projects {
            let category = match meta_store.get(proj.id) {
                Ok(Some(meta)) => {
                    let has_app = meta
                        .platforms
                        .iter()
                        .any(|p| p == "iOS" || p == "Android" || p == "macOS");
                    let has_web = meta.platforms.iter().any(|p| p == "Web");
                    if has_app {
                        "Apps"
                    } else if has_web {
                        "Websites"
                    } else {
                        "Tools"
                    }
                }
                _ => "Tools",
            };
            result.insert(proj.path.display().to_string(), category.to_string());
        }
        Ok(result)
    })
}

/// Fetch description from a GitHub repo URL using the `gh` CLI (supports private repos)
#[tauri::command]
pub async fn fetch_github_description(github_url: String) -> Result<Option<String>, String> {
    // Parse owner/repo from URL like https://github.com/owner/repo
    let path = github_url
        .trim_end_matches('/')
        .strip_prefix("https://github.com/")
        .or_else(|| {
            github_url
                .trim_end_matches('/')
                .strip_prefix("http://github.com/")
        })
        .ok_or_else(|| "Not a GitHub URL".to_string())?;

    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() < 2 {
        return Err("Could not parse owner/repo from URL".to_string());
    }
    let (owner, repo) = (parts[0], parts[1]);
    let nwo = format!("{owner}/{repo}");

    let nwo_clone = nwo.clone();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("gh")
            .args([
                "repo",
                "view",
                &nwo_clone,
                "--json",
                "description",
                "-q",
                ".description",
            ])
            .output()
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
    .map_err(|e| format!("Failed to run gh CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh repo view failed: {stderr}"));
    }

    let desc = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if desc.is_empty() {
        Ok(None)
    } else {
        Ok(Some(desc))
    }
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
