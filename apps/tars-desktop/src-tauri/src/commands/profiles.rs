//! Profile management Tauri commands
//!
//! Commands for creating and managing profiles.

use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tars_core::export::export_as_plugin;
use tars_core::profile::snapshot::snapshot_from_project;
use tars_core::profile::sync::{convert_profile_to_local_overrides, sync_profile_to_projects};
use tars_core::profile::{ToolPermissions, ToolRef, ToolType};
use tars_core::storage::projects::ProjectStore;
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

/// Tool reference for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRefInfo {
    pub name: String,
    pub tool_type: String,
    pub source_scope: Option<String>,
    pub permissions: Option<ToolPermissionsInfo>,
}

impl From<&ToolRef> for ToolRefInfo {
    fn from(r: &ToolRef) -> Self {
        Self {
            name: r.name.clone(),
            tool_type: r.tool_type.to_string(),
            source_scope: r
                .source_scope
                .as_ref()
                .map(|s| format!("{s:?}").to_lowercase()),
            permissions: r.permissions.as_ref().map(ToolPermissionsInfo::from),
        }
    }
}

/// Tool permissions for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionsInfo {
    pub allowed_directories: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
}

impl From<&ToolPermissions> for ToolPermissionsInfo {
    fn from(p: &ToolPermissions) -> Self {
        Self {
            allowed_directories: p
                .allowed_directories
                .iter()
                .map(|d| d.display().to_string())
                .collect(),
            allowed_tools: p.allowed_tools.clone(),
            disallowed_tools: p.disallowed_tools.clone(),
        }
    }
}

/// Project reference for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRef {
    pub id: String,
    pub name: String,
    pub path: String,
}

/// Full profile details for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_refs: Vec<ToolRefInfo>,
    pub assigned_projects: Vec<ProjectRef>,
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
            tool_refs: p.tool_refs.iter().map(ToolRefInfo::from).collect(),
            assigned_projects: Vec::new(), // Will be populated separately
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

/// Create a new empty profile (without snapshotting from a project)
#[tauri::command]
pub async fn create_empty_profile(
    name: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProfileInfo, String> {
    let mut profile = Profile::new(name);
    profile.description = description;

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

/// Get profile details with assigned projects
#[tauri::command]
pub async fn get_profile(id: String, state: State<'_, AppState>) -> Result<ProfileDetails, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let profile_store = ProfileStore::new(db.connection());
        let project_store = ProjectStore::new(db.connection());

        let profile = profile_store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        // Get assigned projects
        let assigned = project_store
            .list_by_profile(uuid)
            .map_err(|e| format!("Failed to get assigned projects: {e}"))?;

        let mut details = ProfileDetails::from(&profile);
        details.assigned_projects = assigned
            .into_iter()
            .map(|p| ProjectRef {
                id: p.id.to_string(),
                name: p.name,
                path: p.path.display().to_string(),
            })
            .collect();

        Ok(details)
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

/// Tool reference input from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRefInput {
    pub name: String,
    pub tool_type: String,
    pub source_scope: Option<String>,
    pub permissions: Option<ToolPermissionsInput>,
}

/// Tool permissions input from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionsInput {
    pub allowed_directories: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
}

impl TryFrom<ToolRefInput> for ToolRef {
    type Error = String;

    fn try_from(input: ToolRefInput) -> Result<Self, Self::Error> {
        let tool_type = match input.tool_type.to_lowercase().as_str() {
            "mcp" => ToolType::Mcp,
            "skill" => ToolType::Skill,
            "agent" => ToolType::Agent,
            "hook" => ToolType::Hook,
            other => return Err(format!("Invalid tool type: {other}")),
        };

        let permissions = input.permissions.map(|p| ToolPermissions {
            allowed_directories: p
                .allowed_directories
                .into_iter()
                .map(PathBuf::from)
                .collect(),
            allowed_tools: p.allowed_tools,
            disallowed_tools: p.disallowed_tools,
        });

        Ok(ToolRef {
            name: input.name,
            tool_type,
            source_scope: None, // Source scope is determined by where the tool was discovered
            permissions,
        })
    }
}

/// Profile sync result for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResultInfo {
    pub affected_projects: usize,
    pub synced_at: String,
}

/// Update profile response including sync result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileResponse {
    pub profile: ProfileInfo,
    pub sync_result: SyncResultInfo,
}

/// Update a profile (name, description, and/or tool_refs)
#[tauri::command]
pub async fn update_profile(
    id: String,
    name: Option<String>,
    description: Option<String>,
    tool_refs: Option<Vec<ToolRefInput>>,
    state: State<'_, AppState>,
) -> Result<UpdateProfileResponse, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        let mut profile = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        // Update name if provided
        if let Some(new_name) = name {
            // Check if the new name is taken by another profile
            if new_name != profile.name {
                if let Some(existing) = store
                    .get_by_name(&new_name)
                    .map_err(|e| format!("Database error: {e}"))?
                {
                    if existing.id != uuid {
                        return Err(format!("Profile '{}' already exists", new_name));
                    }
                }
            }
            profile.name = new_name;
        }

        // Update description if provided (Some(None) clears it, None leaves it unchanged)
        if let Some(new_desc) = description {
            profile.description = if new_desc.is_empty() {
                None
            } else {
                Some(new_desc)
            };
        }

        // Update tool_refs if provided
        if let Some(refs) = tool_refs {
            profile.tool_refs = refs
                .into_iter()
                .map(ToolRef::try_from)
                .collect::<Result<Vec<_>, _>>()?;
        }

        profile.updated_at = Utc::now();

        store
            .update(&profile)
            .map_err(|e| format!("Failed to update profile: {e}"))?;

        // Sync to assigned projects
        let sync_result = sync_profile_to_projects(db.connection(), uuid)
            .map_err(|e| format!("Sync failed: {e}"))?;

        Ok(UpdateProfileResponse {
            profile: ProfileInfo {
                id: profile.id.to_string(),
                name: profile.name,
                description: profile.description,
                created_at: profile.created_at.to_rfc3339(),
                updated_at: profile.updated_at.to_rfc3339(),
            },
            sync_result: SyncResultInfo {
                affected_projects: sync_result.affected_projects,
                synced_at: sync_result.synced_at.to_rfc3339(),
            },
        })
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

    Err(
        "Export path must be within ~/Downloads, ~/Desktop, ~/Documents, or ~/.tars/exports"
            .to_string(),
    )
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
