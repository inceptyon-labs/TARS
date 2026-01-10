//! Profile management Tauri commands
//!
//! Commands for creating and managing profiles.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tars_core::export::export_as_plugin;
use tars_core::profile::snapshot::snapshot_from_project;
use tars_core::storage::ProfileStore;
use tars_core::Profile;
use tauri::State;

/// Profile summary for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<tars_core::storage::profiles::ProfileSummary> for ProfileInfo {
    fn from(p: tars_core::storage::profiles::ProfileSummary) -> Self {
        Self {
            id: p.id.to_string(),
            name: p.name,
            description: p.description,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

/// Full profile details for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub skills_count: usize,
    pub commands_count: usize,
    pub agents_count: usize,
    pub has_claude_md: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Profile> for ProfileDetails {
    fn from(p: &Profile) -> Self {
        Self {
            id: p.id.to_string(),
            name: p.name.clone(),
            description: p.description.clone(),
            skills_count: p.repo_overlays.skills.len() + p.user_overlays.skills.len(),
            commands_count: p.repo_overlays.commands.len() + p.user_overlays.commands.len(),
            agents_count: p.repo_overlays.agents.len(),
            has_claude_md: p.repo_overlays.claude_md.is_some(),
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

/// List all profiles
#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileInfo>, String> {
    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let profiles = store
            .list()
            .map_err(|e| format!("Failed to list profiles: {e}"))?;
        Ok(profiles.into_iter().map(ProfileInfo::from).collect())
    })
}

/// Create a new profile from a project
#[tauri::command]
pub async fn create_profile(
    name: String,
    source_path: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProfileInfo, String> {
    let project_path = PathBuf::from(&source_path);

    if !project_path.exists() {
        return Err(format!("Path does not exist: {source_path}"));
    }

    let mut profile = snapshot_from_project(&project_path, name)
        .map_err(|e| format!("Failed to create profile snapshot: {e}"))?;

    if let Some(desc) = description {
        profile.description = Some(desc);
    }

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        // Check if profile name already exists
        if store
            .get_by_name(&profile.name)
            .map_err(|e| format!("Database error: {e}"))?
            .is_some()
        {
            return Err(format!("Profile '{}' already exists", profile.name));
        }

        store
            .create(&profile)
            .map_err(|e| format!("Failed to save profile: {e}"))?;

        Ok(ProfileInfo {
            id: profile.id.to_string(),
            name: profile.name,
            description: profile.description,
            created_at: profile.created_at.to_rfc3339(),
            updated_at: profile.updated_at.to_rfc3339(),
        })
    })
}

/// Get profile details
#[tauri::command]
pub async fn get_profile(id: String, state: State<'_, AppState>) -> Result<ProfileDetails, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let profile = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        Ok(ProfileDetails::from(&profile))
    })
}

/// Delete a profile
#[tauri::command]
pub async fn delete_profile(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        store
            .delete(uuid)
            .map_err(|e| format!("Failed to delete profile: {e}"))
    })
}

/// Plugin export options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExportOptions {
    pub name: String,
    pub version: String,
}

/// Validate export output path is within safe directories
fn validate_export_path(path: &Path) -> Result<PathBuf, String> {
    // Get safe export directories
    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
    let home_path = PathBuf::from(&home);

    let safe_dirs = vec![
        home_path.join("Downloads"),
        home_path.join("Desktop"),
        home_path.join("Documents"),
        home_path.join(".tars/exports"),
    ];

    // Canonicalize if the parent exists
    let canonical = if let Some(parent) = path.parent() {
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("Invalid export path: {e}"))?;
            canonical_parent.join(path.file_name().unwrap_or_default())
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    };

    // Check if the canonical path is under one of the safe directories
    for safe_dir in &safe_dirs {
        if safe_dir.exists() {
            if let Ok(canonical_safe) = safe_dir.canonicalize() {
                if canonical.starts_with(&canonical_safe) {
                    return Ok(canonical);
                }
            }
        } else if canonical.starts_with(safe_dir) {
            return Ok(canonical);
        }
    }

    Err(format!(
        "Export path must be within ~/Downloads, ~/Desktop, ~/Documents, or ~/.tars/exports"
    ))
}

/// Export a profile as a plugin
#[tauri::command]
pub async fn export_profile_as_plugin(
    profile_id: String,
    output_path: String,
    options: PluginExportOptions,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let uuid = uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid UUID: {e}"))?;
    let output = PathBuf::from(&output_path);

    // Validate the export path is within allowed directories
    let validated_output = validate_export_path(&output)?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let profile = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        export_as_plugin(&profile, &validated_output, &options.name, &options.version)
            .map_err(|e| format!("Export failed: {e}"))?;

        Ok(validated_output.display().to_string())
    })
}
