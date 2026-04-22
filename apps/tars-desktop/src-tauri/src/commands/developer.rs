//! Developer account, credential, app target, and command preset commands.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use tars_core::storage::developer::{
    AppTarget, AppTargetCredential, AppTargetInput, DeveloperCommandInput, DeveloperCommandPreset,
    DeveloperCredentialInput, DeveloperCredentialSummary,
};
use tars_core::storage::DeveloperStore;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperCredentialFile {
    pub path: String,
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedCredentialFile {
    pub path: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperCredentialSummaryResponse {
    pub id: i64,
    pub provider: String,
    pub credential_type: String,
    pub label: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

impl From<DeveloperCredentialSummary> for DeveloperCredentialSummaryResponse {
    fn from(value: DeveloperCredentialSummary) -> Self {
        Self {
            id: value.id,
            provider: value.provider,
            credential_type: value.credential_type,
            label: value.label,
            tags: value.tags,
            metadata: value.metadata,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct DeveloperCredentialInputPayload {
    pub provider: String,
    pub credential_type: String,
    pub label: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub secret: String,
}

impl fmt::Debug for DeveloperCredentialInputPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeveloperCredentialInputPayload")
            .field("provider", &self.provider)
            .field("credential_type", &self.credential_type)
            .field("label", &self.label)
            .field("tags", &self.tags)
            .field("metadata", &self.metadata)
            .field("secret", &"<redacted>")
            .finish()
    }
}

impl From<DeveloperCredentialInputPayload> for DeveloperCredentialInput {
    fn from(value: DeveloperCredentialInputPayload) -> Self {
        Self {
            provider: value.provider,
            credential_type: value.credential_type,
            label: value.label,
            tags: value.tags,
            metadata: value.metadata,
            secret: value.secret,
        }
    }
}

#[tauri::command]
pub async fn read_developer_credential_file(
    path: String,
) -> Result<DeveloperCredentialFile, String> {
    let path_buf = PathBuf::from(&path);
    let content = std::fs::read_to_string(&path_buf)
        .map_err(|e| format!("Failed to read credential file: {e}"))?;
    let file_name = path_buf
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("credential")
        .to_string();

    Ok(DeveloperCredentialFile {
        path,
        file_name,
        content,
    })
}

#[tauri::command]
pub async fn list_developer_credentials(
    state: State<'_, AppState>,
) -> Result<Vec<DeveloperCredentialSummaryResponse>, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        let credentials = store
            .list_credentials()
            .map_err(|e| format!("Failed to list developer credentials: {e}"))?;
        Ok(credentials.into_iter().map(Into::into).collect())
    })
}

#[tauri::command]
pub async fn add_developer_credential(
    input: DeveloperCredentialInputPayload,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .save_credential(&input.into())
            .map_err(|e| format!("Failed to save developer credential: {e}"))
    })
}

#[tauri::command]
pub async fn update_developer_credential(
    id: i64,
    input: DeveloperCredentialInputPayload,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .update_credential(id, &input.into())
            .map_err(|e| format!("Failed to update developer credential: {e}"))
    })
}

#[tauri::command]
pub async fn delete_developer_credential(
    id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .delete_credential(id)
            .map_err(|e| format!("Failed to delete developer credential: {e}"))
    })
}

#[tauri::command]
pub async fn reveal_developer_credential(
    id: i64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        let credential = store
            .get_credential(id)
            .map_err(|e| format!("Failed to load developer credential: {e}"))?
            .ok_or_else(|| format!("Developer credential {id} not found"))?;
        Ok(credential.secret)
    })
}

#[tauri::command]
pub async fn materialize_developer_credential_file(
    id: i64,
    state: State<'_, AppState>,
) -> Result<MaterializedCredentialFile, String> {
    let credential = state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .get_credential(id)
            .map_err(|e| format!("Failed to load developer credential: {e}"))?
            .ok_or_else(|| format!("Developer credential {id} not found"))
    })?;

    let file_name = credential_file_name(&credential.label, &credential.metadata);
    let output_dir = state.data_dir().join("tmp-credentials");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create credential export directory: {e}"))?;

    let path = output_dir.join(&file_name);
    std::fs::write(&path, credential.secret)
        .map_err(|e| format!("Failed to write credential file: {e}"))?;
    set_owner_read_write(&path)?;

    Ok(MaterializedCredentialFile {
        path: path.display().to_string(),
        file_name,
    })
}

#[tauri::command]
pub async fn delete_materialized_developer_credential_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let requested = PathBuf::from(&path);
    let export_dir = state.data_dir().join("tmp-credentials");
    let requested_parent = requested
        .parent()
        .ok_or_else(|| "Invalid credential file path".to_string())?;
    let canonical_parent = requested_parent
        .canonicalize()
        .map_err(|e| format!("Failed to inspect credential file path: {e}"))?;
    let canonical_export_dir = export_dir
        .canonicalize()
        .map_err(|e| format!("Failed to inspect credential export directory: {e}"))?;

    if canonical_parent != canonical_export_dir {
        return Err(
            "Refusing to delete a file outside the credential export directory".to_string(),
        );
    }

    if !requested.exists() {
        return Ok(false);
    }

    std::fs::remove_file(&requested)
        .map_err(|e| format!("Failed to delete credential file: {e}"))?;
    Ok(true)
}

#[tauri::command]
pub async fn list_app_targets(state: State<'_, AppState>) -> Result<Vec<AppTarget>, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .list_app_targets()
            .map_err(|e| format!("Failed to list app targets: {e}"))
    })
}

fn credential_file_name(label: &str, metadata: &serde_json::Value) -> String {
    if let Some(file_name) = metadata
        .get("file_name")
        .and_then(serde_json::Value::as_str)
    {
        return sanitize_file_name(file_name, "credential");
    }

    let extension = match metadata
        .get("credential_extension")
        .and_then(serde_json::Value::as_str)
    {
        Some("json") => "json",
        Some("p12") => "p12",
        Some("jks") => "jks",
        Some("keystore") => "keystore",
        _ => "p8",
    };
    let key_id = metadata
        .get("key_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let base = key_id.map_or_else(
        || label.to_string(),
        |value| format!("AuthKey_{}", value.trim()),
    );

    sanitize_file_name(&format!("{base}.{extension}"), "credential.p8")
}

fn sanitize_file_name(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string();

    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

#[cfg(unix)]
fn set_owner_read_write(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, permissions)
        .map_err(|e| format!("Failed to restrict credential file permissions: {e}"))
}

#[cfg(not(unix))]
fn set_owner_read_write(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn add_app_target(
    input: AppTargetInput,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .save_app_target(&input)
            .map_err(|e| format!("Failed to save app target: {e}"))
    })
}

#[tauri::command]
pub async fn update_app_target(
    id: i64,
    input: AppTargetInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .update_app_target(id, &input)
            .map_err(|e| format!("Failed to update app target: {e}"))
    })
}

#[tauri::command]
pub async fn delete_app_target(id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .delete_app_target(id)
            .map_err(|e| format!("Failed to delete app target: {e}"))
    })
}

#[tauri::command]
pub async fn list_app_target_credentials(
    app_target_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<AppTargetCredential>, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .list_app_target_credentials(app_target_id)
            .map_err(|e| format!("Failed to list app target credentials: {e}"))
    })
}

#[tauri::command]
pub async fn link_app_target_credential(
    app_target_id: i64,
    credential_id: i64,
    role: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .link_credential(app_target_id, credential_id, &role)
            .map_err(|e| format!("Failed to link credential: {e}"))
    })
}

#[tauri::command]
pub async fn unlink_app_target_credential(
    app_target_id: i64,
    credential_id: i64,
    role: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .unlink_credential(app_target_id, credential_id, &role)
            .map_err(|e| format!("Failed to unlink credential: {e}"))
    })
}

#[tauri::command]
pub async fn list_developer_commands(
    state: State<'_, AppState>,
) -> Result<Vec<DeveloperCommandPreset>, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .list_command_presets()
            .map_err(|e| format!("Failed to list developer commands: {e}"))
    })
}

#[tauri::command]
pub async fn add_developer_command(
    input: DeveloperCommandInput,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .save_command_preset(&input)
            .map_err(|e| format!("Failed to save developer command: {e}"))
    })
}

#[tauri::command]
pub async fn update_developer_command(
    id: i64,
    input: DeveloperCommandInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .update_command_preset(id, &input)
            .map_err(|e| format!("Failed to update developer command: {e}"))
    })
}

#[tauri::command]
pub async fn delete_developer_command(id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    state.with_db(|db| {
        let store = DeveloperStore::new(db.connection());
        store
            .delete_command_preset(id)
            .map_err(|e| format!("Failed to delete developer command: {e}"))
    })
}
