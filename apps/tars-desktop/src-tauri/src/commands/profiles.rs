//! Profile management Tauri commands
//!
//! Commands for creating and managing profiles.

use super::plugins::{
    plugin_install, plugin_marketplace_add, plugin_marketplace_update, plugin_uninstall,
};
use super::utils::find_claude_binary;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tars_core::config::McpServerConfig;
use tars_core::export::export_as_plugin_zip;
use tars_core::profile::snapshot::snapshot_from_project;
use tars_core::profile::storage::{list_plugin_manifests, save_project_state, ProjectProfileState};
use tars_core::profile::sync::{
    apply_profile_to_project, convert_profile_to_local_overrides, sync_profile_marketplace,
    sync_profile_to_projects,
};
use tars_core::profile::updates::create_source_ref;
use tars_core::profile::{
    PluginRef, SourceMode, SourceRef, ToolPermissions, ToolRef, ToolType, PROFILE_MARKETPLACE,
};
use tars_core::storage::projects::ProjectStore;
use tars_core::storage::ProfileStore;
use tars_core::Profile;
use tars_scanner::types::Scope;
use tauri::State;

/// Profile summary for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<tars_core::storage::profiles::ProfileSummary> for ProfileInfo {
    fn from(p: tars_core::storage::profiles::ProfileSummary) -> Self {
        Self {
            id: p.id.to_string(),
            name: p.name,
            description: p.description,
            tool_count: p.tool_count,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

/// Source reference for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRefInfo {
    pub source_path: String,
    pub source_hash: String,
    pub mode: String, // "pin" or "track"
    pub copied_at: String,
}

impl From<&tars_core::profile::SourceRef> for SourceRefInfo {
    fn from(s: &tars_core::profile::SourceRef) -> Self {
        Self {
            source_path: s.source_path.display().to_string(),
            source_hash: s.source_hash.clone(),
            mode: match s.mode {
                tars_core::profile::SourceMode::Pin => "pin".to_string(),
                tars_core::profile::SourceMode::Track => "track".to_string(),
            },
            copied_at: s.copied_at.clone(),
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
    pub source_ref: Option<SourceRefInfo>,
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
            source_ref: r.source_ref.as_ref().map(SourceRefInfo::from),
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

/// Plugin reference for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRefInfo {
    pub id: String,
    pub marketplace: Option<String>,
    pub scope: String,
    pub enabled: bool,
}

impl From<&tars_core::profile::PluginRef> for PluginRefInfo {
    fn from(r: &tars_core::profile::PluginRef) -> Self {
        Self {
            id: r.id.clone(),
            marketplace: r.marketplace.clone(),
            scope: format!("{:?}", r.scope).to_lowercase(),
            enabled: r.enabled,
        }
    }
}

/// Full profile details for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_refs: Vec<ToolRefInfo>,
    pub plugin_refs: Vec<PluginRefInfo>,
    pub assigned_projects: Vec<ProjectRef>,
    pub mcp_count: usize,
    pub skills_count: usize,
    pub commands_count: usize,
    pub agents_count: usize,
    pub plugins_count: usize,
    pub has_claude_md: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Profile> for ProfileDetails {
    fn from(p: &Profile) -> Self {
        // Count tool_refs by type
        let mcp_refs = p
            .tool_refs
            .iter()
            .filter(|t| t.tool_type == ToolType::Mcp)
            .count();
        let skill_refs = p
            .tool_refs
            .iter()
            .filter(|t| t.tool_type == ToolType::Skill)
            .count();
        let command_refs = p
            .tool_refs
            .iter()
            .filter(|t| t.tool_type == ToolType::Hook)
            .count();
        let agent_refs = p
            .tool_refs
            .iter()
            .filter(|t| t.tool_type == ToolType::Agent)
            .count();

        // Combine overlay counts with tool_ref counts
        let mcp_count = mcp_refs;
        let skills_count = p.repo_overlays.skills.len() + p.user_overlays.skills.len() + skill_refs;
        let commands_count =
            p.repo_overlays.commands.len() + p.user_overlays.commands.len() + command_refs;
        let agents_count = p.repo_overlays.agents.len() + agent_refs;
        let plugins_count = p.plugin_set.plugins.len();

        Self {
            id: p.id.to_string(),
            name: p.name.clone(),
            description: p.description.clone(),
            tool_refs: p.tool_refs.iter().map(ToolRefInfo::from).collect(),
            plugin_refs: p
                .plugin_set
                .plugins
                .iter()
                .map(PluginRefInfo::from)
                .collect(),
            assigned_projects: Vec::new(), // Will be populated separately
            mcp_count,
            skills_count,
            commands_count,
            agents_count,
            plugins_count,
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

        ensure_unique_plugin_slug(&store, &profile.name, None)?;

        let tool_refs = collect_tools_from_project(profile.id, &project_path)
            .map_err(|e| format!("Failed to collect tools from project: {e}"))?;
        profile.tool_refs = tool_refs;

        store
            .create(&profile)
            .map_err(|e| format!("Failed to save profile: {e}"))?;

        // Regenerate the plugin after profile creation
        tars_core::profile::regenerate_profile_plugin(&profile)
            .map_err(|e| format!("Failed to generate plugin: {e}"))?;
        tars_core::profile::sync_profile_marketplace(&profile)
            .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;
        tars_core::profile::sync_profile_marketplace(&profile)
            .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;

        Ok(ProfileInfo {
            id: profile.id.to_string(),
            name: profile.name,
            description: profile.description,
            tool_count: profile.tool_refs.len(),
            created_at: profile.created_at.to_rfc3339(),
            updated_at: profile.updated_at.to_rfc3339(),
        })
    })
}

/// Validate a profile name
fn validate_profile_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }
    if trimmed.len() > 100 {
        return Err("Profile name cannot exceed 100 characters".to_string());
    }
    Ok(trimmed.to_string())
}

fn ensure_unique_plugin_slug(
    store: &ProfileStore,
    name: &str,
    current_id: Option<uuid::Uuid>,
) -> Result<(), String> {
    let slug = sanitize_plugin_name_for_unassign(name);
    let profiles = store.list().map_err(|e| format!("Database error: {e}"))?;

    if let Some(conflict) = profiles.into_iter().find(|profile| {
        sanitize_plugin_name_for_unassign(&profile.name) == slug && Some(profile.id) != current_id
    }) {
        return Err(format!(
            "Profile name '{}' conflicts with '{}' after plugin ID sanitization",
            name, conflict.name
        ));
    }

    Ok(())
}

/// Create a new empty profile (without snapshotting from a project)
#[tauri::command]
pub async fn create_empty_profile(
    name: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProfileInfo, String> {
    let validated_name = validate_profile_name(&name)?;
    let mut profile = Profile::new(validated_name);
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

        ensure_unique_plugin_slug(&store, &profile.name, None)?;

        store
            .create(&profile)
            .map_err(|e| format!("Failed to save profile: {e}"))?;

        // Regenerate the plugin after profile creation
        tars_core::profile::regenerate_profile_plugin(&profile)
            .map_err(|e| format!("Failed to generate plugin: {e}"))?;

        Ok(ProfileInfo {
            id: profile.id.to_string(),
            name: profile.name,
            description: profile.description,
            tool_count: profile.tool_refs.len(),
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

/// Response for profile deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProfileResponse {
    pub deleted: bool,
    pub converted_projects: usize,
}

/// Response for profile deletion with cleanup (no conversion)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProfileCleanupResponse {
    pub deleted: bool,
    pub projects_unassigned: usize,
    pub local_overrides_removed: usize,
}

/// Delete a profile, converting assigned tools to local overrides
#[tauri::command]
pub async fn delete_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<DeleteProfileResponse, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    let (profile_name, project_paths, deleted, converted_count) = state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let profile = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        // First, convert profile tools to local overrides for all assigned projects
        let converted = convert_profile_to_local_overrides(db.connection(), uuid)
            .map_err(|e| format!("Failed to convert profile tools: {e}"))?;

        let project_paths = converted
            .iter()
            .map(|project| project.path.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let converted_count = converted.len();

        // Now delete the profile
        let deleted = store
            .delete(uuid)
            .map_err(|e| format!("Failed to delete profile: {e}"))?;

        let _ = tars_core::profile::storage::delete_profile_storage(profile.id);

        Ok((profile.name, project_paths, deleted, converted_count))
    })?;

    let plugin_id = format!(
        "tars-profile-{}",
        sanitize_plugin_name_for_unassign(&profile_name)
    );
    let plugin_key = format!("{plugin_id}@{PROFILE_MARKETPLACE}");

    for project_path in project_paths {
        let _ = plugin_uninstall(
            plugin_key.clone(),
            Some("project".to_string()),
            Some(project_path),
        )
        .await;
    }

    let _ = plugin_uninstall(plugin_key, Some("user".to_string()), None).await;
    let _ = tars_core::profile::remove_profile_from_marketplace(&profile_name);

    Ok(DeleteProfileResponse {
        deleted,
        converted_projects: converted_count,
    })
}

/// Delete a profile and remove its tools from assigned projects without conversion
#[tauri::command]
pub async fn delete_profile_cleanup(
    id: String,
    state: State<'_, AppState>,
) -> Result<DeleteProfileCleanupResponse, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    let (profile_name, project_paths, deleted, projects_unassigned, overrides_removed) = state
        .with_db(|db| {
            let store = ProfileStore::new(db.connection());
            let project_store = ProjectStore::new(db.connection());
            let profile = store
                .get(uuid)
                .map_err(|e| format!("Database error: {e}"))?
                .ok_or_else(|| "Profile not found".to_string())?;

            let projects = project_store
                .list_by_profile(uuid)
                .map_err(|e| format!("Failed to list projects: {e}"))?;

            let mut removed_total = 0usize;
            let mut project_paths = Vec::new();

            let mcp_names: Vec<_> = profile
                .tool_refs
                .iter()
                .filter(|t| t.tool_type == ToolType::Mcp)
                .map(|t| t.name.as_str())
                .collect();
            let skill_names: Vec<_> = profile
                .tool_refs
                .iter()
                .filter(|t| t.tool_type == ToolType::Skill)
                .map(|t| t.name.as_str())
                .collect();
            let agent_names: Vec<_> = profile
                .tool_refs
                .iter()
                .filter(|t| t.tool_type == ToolType::Agent)
                .map(|t| t.name.as_str())
                .collect();
            let hook_names: Vec<_> = profile
                .tool_refs
                .iter()
                .filter(|t| t.tool_type == ToolType::Hook)
                .map(|t| t.name.as_str())
                .collect();

            for mut project in projects {
                let mut removed = 0usize;

                let len_before = project.local_overrides.mcp_servers.len();
                project
                    .local_overrides
                    .mcp_servers
                    .retain(|t| !mcp_names.contains(&t.name.as_str()));
                removed += len_before.saturating_sub(project.local_overrides.mcp_servers.len());

                let len_before = project.local_overrides.skills.len();
                project
                    .local_overrides
                    .skills
                    .retain(|t| !skill_names.contains(&t.name.as_str()));
                removed += len_before.saturating_sub(project.local_overrides.skills.len());

                let len_before = project.local_overrides.agents.len();
                project
                    .local_overrides
                    .agents
                    .retain(|t| !agent_names.contains(&t.name.as_str()));
                removed += len_before.saturating_sub(project.local_overrides.agents.len());

                let len_before = project.local_overrides.hooks.len();
                project
                    .local_overrides
                    .hooks
                    .retain(|t| !hook_names.contains(&t.name.as_str()));
                removed += len_before.saturating_sub(project.local_overrides.hooks.len());

                if removed > 0 || project.assigned_profile_id.is_some() {
                    project.assigned_profile_id = None;
                    project.updated_at = Utc::now();
                    project_store
                        .update(&project)
                        .map_err(|e| format!("Failed to update project: {e}"))?;
                }

                removed_total += removed;
                project_paths.push(project.path.to_string_lossy().to_string());
            }

            let deleted = store
                .delete(uuid)
                .map_err(|e| format!("Failed to delete profile: {e}"))?;
            let _ = tars_core::profile::storage::delete_profile_storage(profile.id);

            Ok((
                profile.name,
                project_paths,
                deleted,
                projects.len(),
                removed_total,
            ))
        })?;

    let plugin_id = format!(
        "tars-profile-{}",
        sanitize_plugin_name_for_unassign(&profile_name)
    );
    let plugin_key = format!("{plugin_id}@{PROFILE_MARKETPLACE}");

    for project_path in project_paths {
        let _ = plugin_uninstall(
            plugin_key.clone(),
            Some("project".to_string()),
            Some(project_path),
        )
        .await;
    }

    let _ = plugin_uninstall(plugin_key, Some("user".to_string()), None).await;
    let _ = tars_core::profile::remove_profile_from_marketplace(&profile_name);

    Ok(DeleteProfileCleanupResponse {
        deleted,
        projects_unassigned,
        local_overrides_removed: overrides_removed,
    })
}

/// Tool reference input from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRefInput {
    pub name: String,
    pub tool_type: String,
    #[serde(default)]
    pub source_scope: Option<String>,
    #[serde(default)]
    pub permissions: Option<ToolPermissionsInput>,
}

/// Tool permissions input from frontend
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolPermissionsInput {
    #[serde(default)]
    pub allowed_directories: Vec<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub disallowed_tools: Vec<String>,
}

/// Plugin reference input from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRefInput {
    pub id: String,
    pub marketplace: Option<String>,
    pub scope: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl TryFrom<PluginRefInput> for PluginRef {
    type Error = String;

    fn try_from(input: PluginRefInput) -> Result<Self, Self::Error> {
        let id = input.id.trim().to_string();
        if id.is_empty() {
            return Err("Plugin ID cannot be empty".to_string());
        }

        let scope: Scope = input
            .scope
            .parse()
            .map_err(|_| format!("Invalid scope: {}", input.scope))?;

        Ok(PluginRef {
            id,
            marketplace: input.marketplace,
            scope,
            enabled: input.enabled,
        })
    }
}

/// Validate that a directory path is relative and doesn't contain path traversal
fn validate_allowed_directory(path: &str) -> Result<PathBuf, String> {
    let path_buf = PathBuf::from(path);

    // Reject absolute paths
    if path_buf.is_absolute() {
        return Err(format!(
            "Allowed directory must be relative, got absolute path: {path}"
        ));
    }

    // Reject path traversal
    if path.contains("..") {
        return Err(format!("Allowed directory cannot contain '..': {path}"));
    }

    // Normalize the path
    let normalized: PathBuf = path_buf.components().collect();
    Ok(normalized)
}

impl TryFrom<ToolRefInput> for ToolRef {
    type Error = String;

    fn try_from(input: ToolRefInput) -> Result<Self, Self::Error> {
        // Validate tool name is not empty
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err("Tool name cannot be empty".to_string());
        }

        let tool_type = match input.tool_type.to_lowercase().as_str() {
            "mcp" => ToolType::Mcp,
            "skill" => ToolType::Skill,
            "agent" => ToolType::Agent,
            "hook" => ToolType::Hook,
            other => return Err(format!("Invalid tool type: {other}")),
        };

        let permissions = match input.permissions {
            Some(p) => {
                // Validate allowed_directories
                let allowed_directories: Result<Vec<PathBuf>, String> = p
                    .allowed_directories
                    .iter()
                    .map(|d| validate_allowed_directory(d))
                    .collect();

                Some(ToolPermissions {
                    allowed_directories: allowed_directories?,
                    allowed_tools: p.allowed_tools,
                    disallowed_tools: p.disallowed_tools,
                })
            }
            None => None,
        };

        Ok(ToolRef {
            name,
            tool_type,
            source_scope: None, // Source scope is determined by where the tool was discovered
            permissions,
            source_ref: None,
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

/// Input for updating a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileInput {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tool_refs: Option<Vec<ToolRefInput>>,
    #[serde(default)]
    pub plugin_refs: Option<Vec<PluginRefInput>>,
}

/// Update a profile (name, description, `tool_refs`, and/or `plugin_refs`)
#[tauri::command]
pub async fn update_profile(
    input: UpdateProfileInput,
    state: State<'_, AppState>,
) -> Result<UpdateProfileResponse, String> {
    let id = input.id;
    let name = input.name;
    let description = input.description;
    let tool_refs = input.tool_refs;
    let plugin_refs = input.plugin_refs;

    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        let mut profile = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;
        let previous_name = profile.name.clone();

        // Update name if provided
        if let Some(new_name) = name {
            // Check if the new name is taken by another profile
            if new_name != profile.name {
                if let Some(existing) = store
                    .get_by_name(&new_name)
                    .map_err(|e| format!("Database error: {e}"))?
                {
                    if existing.id != uuid {
                        return Err(format!("Profile '{new_name}' already exists"));
                    }
                }
                ensure_unique_plugin_slug(&store, &new_name, Some(uuid))?;
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

        // Update plugin_refs if provided
        if let Some(refs) = plugin_refs {
            profile.plugin_set.plugins = refs
                .into_iter()
                .map(PluginRef::try_from)
                .collect::<Result<Vec<_>, _>>()?;
        }

        profile.updated_at = Utc::now();

        store
            .update(&profile)
            .map_err(|e| format!("Failed to update profile: {e}"))?;

        // Regenerate the plugin after profile update
        tars_core::profile::regenerate_profile_plugin(&profile)
            .map_err(|e| format!("Failed to regenerate plugin: {e}"))?;
        tars_core::profile::sync_profile_marketplace(&profile)
            .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;
        if previous_name != profile.name {
            tars_core::profile::remove_profile_from_marketplace(&previous_name)
                .map_err(|e| format!("Failed to remove old marketplace entry: {e}"))?;
        }

        // Sync to assigned projects
        let sync_result = sync_profile_to_projects(db.connection(), uuid)
            .map_err(|e| format!("Failed to sync profile: {e}"))?;

        Ok(UpdateProfileResponse {
            profile: ProfileInfo {
                id: profile.id.to_string(),
                name: profile.name,
                description: profile.description,
                tool_count: profile.tool_refs.len(),
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
    // Reject paths containing path traversal components
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return Err("Export path cannot contain '..' path traversal".to_string());
    }

    // Require absolute path
    if !path.is_absolute() {
        return Err("Export path must be absolute".to_string());
    }

    // Get safe export directories
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;

    let safe_dirs = vec![
        home.join("Downloads"),
        home.join("Desktop"),
        home.join("Documents"),
        home.join(".tars").join("exports"),
    ];

    // Normalize the path by iterating components (handles redundant separators)
    let normalized: PathBuf = path.components().collect();

    // For existing parent directories, use canonicalize to resolve symlinks
    let resolved = if let Some(parent) = normalized.parent() {
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("Invalid export path: {e}"))?;

            // Check that canonical parent doesn't escape via symlink
            let canonical_str = canonical_parent.to_string_lossy();
            if canonical_str.contains("..") {
                return Err("Export path resolves outside allowed directories".to_string());
            }

            canonical_parent.join(normalized.file_name().unwrap_or_default())
        } else {
            // Parent doesn't exist - verify the path prefix matches a safe dir
            normalized.clone()
        }
    } else {
        normalized.clone()
    };

    // Check if the resolved path is under one of the safe directories
    for safe_dir in &safe_dirs {
        // Canonicalize safe dir if it exists to handle symlinks consistently
        let safe_canonical = if safe_dir.exists() {
            safe_dir.canonicalize().unwrap_or_else(|_| safe_dir.clone())
        } else {
            safe_dir.clone()
        };

        if resolved.starts_with(&safe_canonical) {
            return Ok(resolved);
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
    let mut output = PathBuf::from(&output_path);
    let ext = output
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    if ext.as_deref() != Some("zip") {
        output.set_extension("zip");
    }

    // Validate the export path is within allowed directories
    let validated_output = validate_export_path(&output)?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let profile = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        export_as_plugin_zip(&profile, &validated_output, &options.name, &options.version)
            .map_err(|e| format!("Export failed: {e}"))?;

        Ok(validated_output.display().to_string())
    })
}

// ============================================================================
// Profile Assignment Commands (US2)
// ============================================================================

/// Response for profile assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignProfileResponse {
    pub project_id: String,
    pub profile_id: String,
    pub assigned_at: String,
    /// Number of plugins that were installed by this assignment
    pub plugins_installed: usize,
    /// Plugins that failed to install (name, error message)
    pub plugin_errors: Vec<(String, String)>,
}

/// Assign a profile to a project
#[tauri::command]
pub async fn assign_profile(
    project_id: String,
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<AssignProfileResponse, String> {
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    // Get profile and project info from database
    let (_profile, project_path) = state.with_db(|db| {
        let profile_store = ProfileStore::new(db.connection());
        let project_store = ProjectStore::new(db.connection());

        // Get the profile (we need its content for applying)
        let profile = profile_store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        // Get the project
        let mut project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Project not found".to_string())?;

        // Apply the profile overlays to the project directory
        let _apply_result = apply_profile_to_project(&profile, &project.path)
            .map_err(|e| format!("Failed to apply profile: {e}"))?;

        // Assign the profile in the database
        project.assigned_profile_id = Some(profile_uuid);
        project.updated_at = Utc::now();

        project_store
            .update(&project)
            .map_err(|e| format!("Failed to update project: {e}"))?;

        Ok((profile, project.path.clone()))
    })?;

    // Install plugins from the profile's central storage
    let mut plugins_installed = 0usize;
    let mut plugin_errors: Vec<(String, String)> = Vec::new();
    let mut project_state = ProjectProfileState::new(profile_uuid);

    // Get plugins from profile storage
    let plugin_manifests = list_plugin_manifests(profile_uuid).unwrap_or_default();

    if !plugin_manifests.is_empty() {
        // Find Claude binary for plugin installation
        let claude_binary = find_claude_binary();

        for manifest in &plugin_manifests {
            if !manifest.enabled {
                continue; // Skip disabled plugins
            }

            // Build plugin identifier (plugin@marketplace format if marketplace is known)
            let plugin_identifier = match &manifest.marketplace {
                Some(marketplace) => format!("{}@{}", manifest.id, marketplace),
                None => manifest.id.clone(),
            };

            match &claude_binary {
                Ok(claude) => {
                    // Install plugin with project scope
                    let output = Command::new(claude)
                        .args(["plugin", "install", "--scope=project", &plugin_identifier])
                        .current_dir(&project_path)
                        .output();

                    match output {
                        Ok(result) if result.status.success() => {
                            plugins_installed += 1;
                            project_state.add_installed_plugin(manifest.id.clone());
                        }
                        Ok(result) => {
                            let stderr = String::from_utf8_lossy(&result.stderr);
                            let stdout = String::from_utf8_lossy(&result.stdout);
                            let error_msg = if !stderr.is_empty() {
                                stderr.to_string()
                            } else if !stdout.is_empty() {
                                stdout.to_string()
                            } else {
                                "Unknown error".to_string()
                            };

                            // Check if already installed (not an error)
                            if error_msg.contains("already installed") {
                                // Plugin already exists, not an error
                                project_state.add_installed_plugin(manifest.id.clone());
                            } else {
                                plugin_errors.push((manifest.id.clone(), error_msg));
                            }
                        }
                        Err(e) => {
                            plugin_errors
                                .push((manifest.id.clone(), format!("Failed to run CLI: {e}")));
                        }
                    }
                }
                Err(e) => {
                    plugin_errors.push((manifest.id.clone(), format!("Claude CLI not found: {e}")));
                }
            }
        }
    }

    // Save project state for cleanup tracking
    if let Err(e) = save_project_state(project_uuid, &project_state) {
        // Log warning but don't fail the assignment
        eprintln!("Warning: Failed to save project state: {e}");
    }

    let assigned_at = Utc::now();

    Ok(AssignProfileResponse {
        project_id: project_uuid.to_string(),
        profile_id: profile_uuid.to_string(),
        assigned_at: assigned_at.to_rfc3339(),
        plugins_installed,
        plugin_errors,
    })
}

/// Response for profile unassignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnassignProfileResponse {
    pub project_id: String,
    pub unassigned_at: String,
}

/// Unassign a profile from a project
#[tauri::command]
pub async fn unassign_profile(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<UnassignProfileResponse, String> {
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;

    state.with_db(|db| {
        let project_store = ProjectStore::new(db.connection());

        // Get the project
        let mut project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Project not found".to_string())?;

        // Unassign the profile
        project.assigned_profile_id = None;
        project.updated_at = Utc::now();

        project_store
            .update(&project)
            .map_err(|e| format!("Failed to update project: {e}"))?;

        Ok(UnassignProfileResponse {
            project_id: project_uuid.to_string(),
            unassigned_at: project.updated_at.to_rfc3339(),
        })
    })
}

/// Tool reference with source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRefWithSource {
    pub name: String,
    pub tool_type: String,
    pub source_scope: Option<String>,
    pub permissions: Option<ToolPermissionsInfo>,
    pub source: String, // "profile" or "local"
}

/// Response for getting project tools (combined profile + local)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectToolsResponse {
    pub project_id: String,
    pub profile: Option<ProfileRefInfo>,
    pub profile_tools: Vec<ToolRefWithSource>,
    pub local_tools: Vec<ToolRefWithSource>,
}

/// Profile reference for tools response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRefInfo {
    pub id: String,
    pub name: String,
}

/// Get all tools for a project (combined profile + local overrides)
#[tauri::command]
pub async fn get_project_tools(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<ProjectToolsResponse, String> {
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;

    state.with_db(|db| {
        let profile_store = ProfileStore::new(db.connection());
        let project_store = ProjectStore::new(db.connection());

        // Get the project
        let project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Project not found".to_string())?;

        // Get profile tools if assigned
        let (profile_info, profile_tools) = if let Some(profile_id) = project.assigned_profile_id {
            let profile = profile_store
                .get(profile_id)
                .map_err(|e| format!("Database error: {e}"))?;

            if let Some(p) = profile {
                let tools: Vec<ToolRefWithSource> = p
                    .tool_refs
                    .iter()
                    .map(|r| ToolRefWithSource {
                        name: r.name.clone(),
                        tool_type: r.tool_type.to_string(),
                        source_scope: r
                            .source_scope
                            .as_ref()
                            .map(|s| format!("{s:?}").to_lowercase()),
                        permissions: r.permissions.as_ref().map(ToolPermissionsInfo::from),
                        source: "profile".to_string(),
                    })
                    .collect();

                (
                    Some(ProfileRefInfo {
                        id: p.id.to_string(),
                        name: p.name.clone(),
                    }),
                    tools,
                )
            } else {
                (None, Vec::new())
            }
        } else {
            (None, Vec::new())
        };

        // Get local tools
        let local_tools: Vec<ToolRefWithSource> = project
            .local_overrides
            .mcp_servers
            .iter()
            .chain(project.local_overrides.skills.iter())
            .chain(project.local_overrides.agents.iter())
            .chain(project.local_overrides.hooks.iter())
            .map(|r| ToolRefWithSource {
                name: r.name.clone(),
                tool_type: r.tool_type.to_string(),
                source_scope: r
                    .source_scope
                    .as_ref()
                    .map(|s| format!("{s:?}").to_lowercase()),
                permissions: r.permissions.as_ref().map(ToolPermissionsInfo::from),
                source: "local".to_string(),
            })
            .collect();

        Ok(ProjectToolsResponse {
            project_id: project_uuid.to_string(),
            profile: profile_info,
            profile_tools,
            local_tools,
        })
    })
}

// ============================================================================
// Local Override Commands (US4)
// ============================================================================

/// Response for adding a local tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLocalToolResponse {
    pub project_id: String,
    pub tool_name: String,
    pub added_at: String,
}

/// Add a local tool override to a project
#[tauri::command]
pub async fn add_local_tool(
    project_id: String,
    tool: ToolRefInput,
    state: State<'_, AppState>,
) -> Result<AddLocalToolResponse, String> {
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;

    let tool_ref = ToolRef::try_from(tool)?;
    let tool_name = tool_ref.name.clone();
    let tool_type = tool_ref.tool_type;

    state.with_db(|db| {
        let project_store = ProjectStore::new(db.connection());

        // Get the project
        let mut project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Project not found".to_string())?;

        // Add to the appropriate list based on tool type
        match tool_type {
            ToolType::Mcp => {
                // Check for duplicates
                if project
                    .local_overrides
                    .mcp_servers
                    .iter()
                    .any(|t| t.name == tool_name)
                {
                    return Err(format!(
                        "MCP server '{tool_name}' already exists as local override"
                    ));
                }
                project.local_overrides.mcp_servers.push(tool_ref);
            }
            ToolType::Skill => {
                if project
                    .local_overrides
                    .skills
                    .iter()
                    .any(|t| t.name == tool_name)
                {
                    return Err(format!(
                        "Skill '{tool_name}' already exists as local override"
                    ));
                }
                project.local_overrides.skills.push(tool_ref);
            }
            ToolType::Agent => {
                if project
                    .local_overrides
                    .agents
                    .iter()
                    .any(|t| t.name == tool_name)
                {
                    return Err(format!(
                        "Agent '{tool_name}' already exists as local override"
                    ));
                }
                project.local_overrides.agents.push(tool_ref);
            }
            ToolType::Hook => {
                if project
                    .local_overrides
                    .hooks
                    .iter()
                    .any(|t| t.name == tool_name)
                {
                    return Err(format!(
                        "Hook '{tool_name}' already exists as local override"
                    ));
                }
                project.local_overrides.hooks.push(tool_ref);
            }
        }

        project.updated_at = Utc::now();

        project_store
            .update(&project)
            .map_err(|e| format!("Failed to update project: {e}"))?;

        Ok(AddLocalToolResponse {
            project_id: project_uuid.to_string(),
            tool_name,
            added_at: project.updated_at.to_rfc3339(),
        })
    })
}

/// Response for removing a local tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveLocalToolResponse {
    pub project_id: String,
    pub removed: bool,
}

/// Remove a local tool override from a project
#[tauri::command]
pub async fn remove_local_tool(
    project_id: String,
    tool_name: String,
    tool_type: String,
    state: State<'_, AppState>,
) -> Result<RemoveLocalToolResponse, String> {
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;

    let tool_type_enum = match tool_type.to_lowercase().as_str() {
        "mcp" => ToolType::Mcp,
        "skill" => ToolType::Skill,
        "agent" => ToolType::Agent,
        "hook" => ToolType::Hook,
        other => return Err(format!("Invalid tool type: {other}")),
    };

    state.with_db(|db| {
        let project_store = ProjectStore::new(db.connection());

        // Get the project
        let mut project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Project not found".to_string())?;

        // Remove from the appropriate list based on tool type
        let removed = match tool_type_enum {
            ToolType::Mcp => {
                let len_before = project.local_overrides.mcp_servers.len();
                project
                    .local_overrides
                    .mcp_servers
                    .retain(|t| t.name != tool_name);
                project.local_overrides.mcp_servers.len() < len_before
            }
            ToolType::Skill => {
                let len_before = project.local_overrides.skills.len();
                project
                    .local_overrides
                    .skills
                    .retain(|t| t.name != tool_name);
                project.local_overrides.skills.len() < len_before
            }
            ToolType::Agent => {
                let len_before = project.local_overrides.agents.len();
                project
                    .local_overrides
                    .agents
                    .retain(|t| t.name != tool_name);
                project.local_overrides.agents.len() < len_before
            }
            ToolType::Hook => {
                let len_before = project.local_overrides.hooks.len();
                project
                    .local_overrides
                    .hooks
                    .retain(|t| t.name != tool_name);
                project.local_overrides.hooks.len() < len_before
            }
        };

        if removed {
            project.updated_at = Utc::now();
            project_store
                .update(&project)
                .map_err(|e| format!("Failed to update project: {e}"))?;
        }

        Ok(RemoveLocalToolResponse {
            project_id: project_uuid.to_string(),
            removed,
        })
    })
}

// ============================================================================
// Add Tools from Source Commands
// ============================================================================

use tars_core::profile::storage;

/// Input for adding tools from a source project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddToolsFromSourceInput {
    pub profile_id: String,
    #[serde(default = "default_source_scope")]
    pub source_scope: String,
    #[serde(default)]
    pub source_project_path: Option<String>,
    pub tools: Vec<ToolFromSource>,
}

/// Tool to add from source project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFromSource {
    pub name: String,
    pub tool_type: String,
    /// Source tracking mode: "pin" or "track" (defaults to "track" if not provided)
    #[serde(default = "default_source_mode")]
    pub source_mode: String,
}

fn default_source_mode() -> String {
    "track".to_string()
}

fn default_source_scope() -> String {
    "project".to_string()
}

fn skill_source_dir(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        Some(path.to_path_buf())
    } else {
        path.parent().map(|p| p.to_path_buf())
    }
}

fn collect_tools_from_project(
    profile_id: uuid::Uuid,
    project_path: &Path,
) -> Result<Vec<ToolRef>, String> {
    let inventory = tars_scanner::scope::scan_project(project_path)
        .map_err(|e| format!("Failed to scan project: {e}"))?;

    let mut tool_refs = Vec::new();
    let source_mode = SourceMode::Track;

    if let Some(mcp_config) = &inventory.mcp {
        for server in &mcp_config.servers {
            let transport = match server.transport {
                tars_scanner::settings::McpTransport::Stdio => "stdio",
                tars_scanner::settings::McpTransport::Http => "http",
                tars_scanner::settings::McpTransport::Sse => "sse",
            };

            let mut config = serde_json::Map::new();
            config.insert("type".to_string(), serde_json::json!(transport));
            if let Some(ref cmd) = server.command {
                config.insert("command".to_string(), serde_json::json!(cmd));
            }
            if !server.args.is_empty() {
                config.insert("args".to_string(), serde_json::json!(server.args));
            }
            if !server.env.is_empty() {
                config.insert("env".to_string(), serde_json::json!(server.env));
            }
            if let Some(ref url) = server.url {
                config.insert("url".to_string(), serde_json::json!(url));
            }

            storage::store_mcp_server(profile_id, &server.name, &serde_json::Value::Object(config))
                .map_err(|e| format!("Failed to store MCP server: {e}"))?;

            let source_ref = if mcp_config.path.exists() {
                create_source_ref(mcp_config.path.clone(), ToolType::Mcp, source_mode)
                    .map_err(|e| format!("Failed to create source ref: {e}"))?
            } else {
                SourceRef {
                    source_path: project_path.to_path_buf(),
                    source_hash: "mcp-config".to_string(),
                    mode: source_mode,
                    copied_at: chrono::Utc::now().to_rfc3339(),
                }
            };

            tool_refs.push(ToolRef {
                name: server.name.clone(),
                tool_type: ToolType::Mcp,
                source_scope: Some(Scope::Project),
                permissions: None,
                source_ref: Some(source_ref),
            });
        }
    }

    for skill in &inventory.skills {
        let Some(skill_dir) = skill_source_dir(&skill.path) else {
            continue;
        };
        if skill_dir.exists() && skill_dir.is_dir() {
            storage::copy_skill_to_profile(profile_id, &skill.name, &skill_dir)
                .map_err(|e| format!("Failed to copy skill: {e}"))?;
            let skill_source_ref =
                create_source_ref(skill_dir.clone(), ToolType::Skill, source_mode)
                    .map_err(|e| format!("Failed to create source ref: {e}"))?;

            tool_refs.push(ToolRef {
                name: skill.name.clone(),
                tool_type: ToolType::Skill,
                source_scope: Some(skill.scope.clone()),
                permissions: None,
                source_ref: Some(skill_source_ref),
            });
        }
    }

    for agent in &inventory.agents {
        if agent.path.exists() {
            storage::copy_agent_to_profile(profile_id, &agent.name, &agent.path)
                .map_err(|e| format!("Failed to copy agent: {e}"))?;
            let agent_source_ref =
                create_source_ref(agent.path.clone(), ToolType::Agent, source_mode)
                    .map_err(|e| format!("Failed to create source ref: {e}"))?;

            tool_refs.push(ToolRef {
                name: agent.name.clone(),
                tool_type: ToolType::Agent,
                source_scope: Some(agent.scope.clone()),
                permissions: None,
                source_ref: Some(agent_source_ref),
            });
        }
    }

    for command in &inventory.commands {
        if command.path.exists() {
            storage::copy_command_to_profile(profile_id, &command.name, &command.path)
                .map_err(|e| format!("Failed to copy command: {e}"))?;
            let command_source_ref =
                create_source_ref(command.path.clone(), ToolType::Hook, source_mode)
                    .map_err(|e| format!("Failed to create source ref: {e}"))?;

            tool_refs.push(ToolRef {
                name: command.name.clone(),
                tool_type: ToolType::Hook,
                source_scope: Some(command.scope.clone()),
                permissions: None,
                source_ref: Some(command_source_ref),
            });
        }
    }

    Ok(tool_refs)
}

/// Response for adding tools from source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddToolsFromSourceResponse {
    pub added_count: usize,
    pub mcp_servers_added: usize,
    pub skills_added: usize,
    pub agents_added: usize,
    pub commands_added: usize,
}

/// Add tools to a profile by copying them to central profile storage
///
/// Tools are copied from the source project to ~/.tars/profiles/<profile-id>/
/// This ensures profiles remain valid even if the source project is deleted.
#[tauri::command]
pub async fn add_tools_from_source(
    input: AddToolsFromSourceInput,
    state: State<'_, AppState>,
) -> Result<AddToolsFromSourceResponse, String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&input.profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;
    let source_scope = input.source_scope.to_lowercase();
    let (source_path, scope_for_refs) = match source_scope.as_str() {
        "project" => {
            let path = input
                .source_project_path
                .as_ref()
                .ok_or_else(|| "Source project path is required for project scope".to_string())?;
            let path = PathBuf::from(path);
            if !path.exists() {
                return Err(format!(
                    "Source project path does not exist: {}",
                    path.display()
                ));
            }
            (Some(path), Scope::Project)
        }
        "user" => (None, Scope::User),
        other => return Err(format!("Invalid source scope: {other}")),
    };

    // Verify profile exists
    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())
    })?;

    let (mcp_config, skills, agents, commands) = if scope_for_refs == Scope::Project {
        let source_path = source_path
            .as_ref()
            .ok_or_else(|| "Source project path is required for project scope".to_string())?;
        let inventory = tars_scanner::scope::scan_project(source_path)
            .map_err(|e| format!("Failed to scan source project: {e}"))?;
        (
            inventory.mcp,
            inventory.skills,
            inventory.agents,
            inventory.commands,
        )
    } else {
        let inventory = tars_scanner::scope::scan_user_scope()
            .map_err(|e| format!("Failed to scan user scope: {e}"))?;
        (
            inventory.mcp,
            inventory.skills,
            inventory.agents,
            inventory.commands,
        )
    };

    let mut mcp_servers_added = 0usize;
    let mut skills_added = 0usize;
    let mut agents_added = 0usize;
    let mut commands_added = 0usize;
    let mut tool_refs: Vec<ToolRef> = Vec::new();
    let mcp_source_path = if scope_for_refs == Scope::User {
        let home =
            dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
        home.join(".claude.json")
    } else {
        source_path
            .as_ref()
            .ok_or_else(|| "Source project path is required for project scope".to_string())?
            .clone()
    };

    for tool in &input.tools {
        let tool_type = match tool.tool_type.to_lowercase().as_str() {
            "mcp" => ToolType::Mcp,
            "skill" => ToolType::Skill,
            "agent" => ToolType::Agent,
            "hook" => ToolType::Hook,
            other => return Err(format!("Invalid tool type: {other}")),
        };

        let source_mode = match tool.source_mode.to_lowercase().as_str() {
            "pin" => SourceMode::Pin,
            _ => SourceMode::Track, // Default to track
        };

        match tool_type {
            ToolType::Mcp => {
                // Find MCP server in inventory and store config as JSON
                if let Some(mcp_config) = &mcp_config {
                    if let Some(server) = mcp_config.servers.iter().find(|s| s.name == tool.name) {
                        let transport = match server.transport {
                            tars_scanner::settings::McpTransport::Stdio => "stdio",
                            tars_scanner::settings::McpTransport::Http => "http",
                            tars_scanner::settings::McpTransport::Sse => "sse",
                        };

                        // Build the server config JSON
                        let mut config = serde_json::Map::new();
                        config.insert("type".to_string(), serde_json::json!(transport));
                        if let Some(ref cmd) = server.command {
                            config.insert("command".to_string(), serde_json::json!(cmd));
                        }
                        if !server.args.is_empty() {
                            config.insert("args".to_string(), serde_json::json!(server.args));
                        }
                        if !server.env.is_empty() {
                            config.insert("env".to_string(), serde_json::json!(server.env));
                        }
                        if let Some(ref url) = server.url {
                            config.insert("url".to_string(), serde_json::json!(url));
                        }

                        storage::store_mcp_server(
                            profile_uuid,
                            &server.name,
                            &serde_json::Value::Object(config),
                        )
                        .map_err(|e| format!("Failed to store MCP server: {e}"))?;

                        // MCP servers come from config files; use scope-appropriate source path
                        let mcp_source_ref = SourceRef {
                            source_path: mcp_source_path.clone(),
                            source_hash: "mcp-config".to_string(), // MCP configs are stored inline
                            mode: source_mode,
                            copied_at: chrono::Utc::now().to_rfc3339(),
                        };

                        tool_refs.push(ToolRef {
                            name: tool.name.clone(),
                            tool_type: ToolType::Mcp,
                            source_scope: Some(scope_for_refs.clone()),
                            permissions: None,
                            source_ref: Some(mcp_source_ref),
                        });
                        mcp_servers_added += 1;
                    }
                }
            }
            ToolType::Skill => {
                // Find skill in inventory and copy the entire skill directory
                if let Some(skill) = skills.iter().find(|s| s.name == tool.name) {
                    let Some(skill_dir) = skill_source_dir(&skill.path) else {
                        continue;
                    };
                    if skill_dir.exists() && skill_dir.is_dir() {
                        storage::copy_skill_to_profile(profile_uuid, &skill.name, &skill_dir)
                            .map_err(|e| format!("Failed to copy skill: {e}"))?;

                        // Create source_ref for tracking
                        let skill_source_ref =
                            create_source_ref(skill_dir.clone(), ToolType::Skill, source_mode)
                                .map_err(|e| format!("Failed to create source ref: {e}"))?;

                        tool_refs.push(ToolRef {
                            name: tool.name.clone(),
                            tool_type: ToolType::Skill,
                            source_scope: Some(scope_for_refs.clone()),
                            permissions: None,
                            source_ref: Some(skill_source_ref),
                        });
                        skills_added += 1;
                    }
                }
            }
            ToolType::Agent => {
                // Find agent in inventory and copy the file
                if let Some(agent) = agents.iter().find(|a| a.name == tool.name) {
                    if agent.path.exists() {
                        storage::copy_agent_to_profile(profile_uuid, &agent.name, &agent.path)
                            .map_err(|e| format!("Failed to copy agent: {e}"))?;

                        // Create source_ref for tracking
                        let agent_source_ref =
                            create_source_ref(agent.path.clone(), ToolType::Agent, source_mode)
                                .map_err(|e| format!("Failed to create source ref: {e}"))?;

                        tool_refs.push(ToolRef {
                            name: tool.name.clone(),
                            tool_type: ToolType::Agent,
                            source_scope: Some(scope_for_refs.clone()),
                            permissions: None,
                            source_ref: Some(agent_source_ref),
                        });
                        agents_added += 1;
                    }
                }
            }
            ToolType::Hook => {
                // Commands are stored as hooks in profiles
                if let Some(command) = commands.iter().find(|c| c.name == tool.name) {
                    if command.path.exists() {
                        storage::copy_command_to_profile(
                            profile_uuid,
                            &command.name,
                            &command.path,
                        )
                        .map_err(|e| format!("Failed to copy command: {e}"))?;

                        let command_source_ref =
                            create_source_ref(command.path.clone(), ToolType::Hook, source_mode)
                                .map_err(|e| format!("Failed to create source ref: {e}"))?;

                        tool_refs.push(ToolRef {
                            name: tool.name.clone(),
                            tool_type: ToolType::Hook,
                            source_scope: Some(command.scope.clone()),
                            permissions: None,
                            source_ref: Some(command_source_ref),
                        });
                        commands_added += 1;
                    }
                }
            }
        }
    }

    // Update the profile's tool_refs in the database
    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        let mut profile = store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        // Add tool refs (avoiding duplicates)
        for tool_ref in tool_refs {
            if !profile
                .tool_refs
                .iter()
                .any(|t| t.name == tool_ref.name && t.tool_type == tool_ref.tool_type)
            {
                profile.tool_refs.push(tool_ref);
            }
        }

        profile.updated_at = Utc::now();

        store
            .update(&profile)
            .map_err(|e| format!("Failed to update profile: {e}"))?;

        // Regenerate the plugin after adding tools
        tars_core::profile::regenerate_profile_plugin(&profile)
            .map_err(|e| format!("Failed to regenerate plugin: {e}"))?;
        tars_core::profile::sync_profile_marketplace(&profile)
            .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;

        Ok(AddToolsFromSourceResponse {
            added_count: mcp_servers_added + skills_added + agents_added + commands_added,
            mcp_servers_added,
            skills_added,
            agents_added,
            commands_added,
        })
    })
}

// ========================================================================
// Profile MCP Creation
// ========================================================================

/// Input for creating an MCP server directly in profile storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileMcpInput {
    pub profile_id: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    #[serde(rename = "docsUrl")]
    pub docs_url: Option<String>,
}

/// Create an MCP server directly in a profile
#[tauri::command]
pub async fn create_profile_mcp_server(
    input: CreateProfileMcpInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&input.profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    tars_core::util::validate_name(&input.name)
        .map_err(|_| "Invalid MCP server name".to_string())?;

    let transport = input.transport.to_lowercase();
    let transport_value = match transport.as_str() {
        "stdio" | "http" | "sse" => transport,
        other => return Err(format!("Invalid transport: {other}")),
    };

    let mut config = serde_json::Map::new();
    config.insert("type".to_string(), serde_json::json!(transport_value));

    if let Some(command) = input.command {
        if !command.is_empty() {
            config.insert("command".to_string(), serde_json::json!(command));
        }
    }
    if let Some(args) = input.args {
        if !args.is_empty() {
            config.insert("args".to_string(), serde_json::json!(args));
        }
    }
    if let Some(env) = input.env {
        if !env.is_empty() {
            config.insert("env".to_string(), serde_json::json!(env));
        }
    }
    if let Some(url) = input.url {
        if !url.is_empty() {
            config.insert("url".to_string(), serde_json::json!(url));
        }
    }
    if let Some(docs_url) = input.docs_url {
        if !docs_url.is_empty() {
            config.insert("docsUrl".to_string(), serde_json::json!(docs_url));
        }
    }

    storage::store_mcp_server(
        profile_uuid,
        &input.name,
        &serde_json::Value::Object(config),
    )
    .map_err(|e| format!("Failed to store MCP server: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let mut profile = store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        if !profile
            .tool_refs
            .iter()
            .any(|tool| tool.name == input.name && tool.tool_type == ToolType::Mcp)
        {
            profile.tool_refs.push(ToolRef {
                name: input.name,
                tool_type: ToolType::Mcp,
                source_scope: None,
                permissions: None,
                source_ref: None,
            });
        }

        profile.updated_at = Utc::now();
        store
            .update(&profile)
            .map_err(|e| format!("Failed to update profile: {e}"))?;

        tars_core::profile::regenerate_profile_plugin(&profile)
            .map_err(|e| format!("Failed to regenerate plugin: {e}"))?;
        sync_profile_marketplace(&profile)
            .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;

        Ok(())
    })?;

    Ok(())
}

/// MCP server item from profile storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMcpServerItem {
    pub name: String,
    pub scope: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "docsUrl", skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "profileName")]
    pub profile_name: String,
}

/// List MCP servers stored in profiles
#[tauri::command]
pub async fn list_profile_mcp_servers(
    state: State<'_, AppState>,
) -> Result<Vec<ProfileMcpServerItem>, String> {
    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let profiles = store
            .list()
            .map_err(|e| format!("Failed to list profiles: {e}"))?;

        let mut servers = Vec::new();

        for profile in profiles {
            let tools = storage::list_profile_tools(profile.id)
                .map_err(|e| format!("Failed to list profile tools: {e}"))?;

            for name in tools.mcp_servers {
                let config_value = match storage::get_mcp_server_config(profile.id, &name) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                let Ok(config) = serde_json::from_value::<McpServerConfig>(config_value) else {
                    continue;
                };

                if config.validate().is_err() {
                    continue;
                }

                let safe_name = match storage::sanitize_tool_name(&name) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let file_path = storage::profile_dir(profile.id)
                    .map_err(|e| format!("Failed to resolve profile directory: {e}"))?
                    .join("mcp-servers")
                    .join(format!("{safe_name}.json"));

                servers.push(ProfileMcpServerItem {
                    name: name.clone(),
                    scope: "profile".to_string(),
                    transport: format!("{:?}", config.transport).to_lowercase(),
                    command: config.command,
                    args: config.args,
                    env: config.env,
                    url: config.url,
                    file_path: file_path.display().to_string(),
                    docs_url: config.docs_url,
                    profile_id: profile.id.to_string(),
                    profile_name: profile.name.clone(),
                });
            }
        }

        Ok(servers)
    })
}

/// Input for removing a profile-scoped MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveProfileMcpInput {
    pub profile_id: String,
    pub name: String,
}

/// Remove an MCP server from profile storage
#[tauri::command]
pub async fn remove_profile_mcp_server(
    input: RemoveProfileMcpInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&input.profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    let removed_file = storage::delete_mcp_server(profile_uuid, &input.name)
        .map_err(|e| format!("Failed to delete MCP server: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let mut profile = store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        let before_len = profile.tool_refs.len();
        profile
            .tool_refs
            .retain(|tool| !(tool.tool_type == ToolType::Mcp && tool.name == input.name));
        let removed_ref = profile.tool_refs.len() != before_len;

        if removed_file || removed_ref {
            profile.updated_at = Utc::now();
            store
                .update(&profile)
                .map_err(|e| format!("Failed to update profile: {e}"))?;

            tars_core::profile::regenerate_profile_plugin(&profile)
                .map_err(|e| format!("Failed to regenerate plugin: {e}"))?;
            sync_profile_marketplace(&profile)
                .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;
        }

        Ok(())
    })
}

// ============================================================================
// Add Plugin to Profile Commands
// ============================================================================

use tars_core::profile::PluginManifest;

/// Input for adding a plugin to a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPluginToProfileInput {
    pub profile_id: String,
    pub plugin_id: String,
    pub marketplace: Option<String>,
    pub version: Option<String>,
}

/// Response for adding a plugin to a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPluginToProfileResponse {
    pub profile_id: String,
    pub plugin_id: String,
    pub added: bool,
}

/// Add a plugin to a profile's central storage
///
/// Plugins are stored as manifests in ~/.tars/profiles/<profile-id>/plugins/
/// When the profile is assigned to a project, these plugins will be installed.
#[tauri::command]
pub async fn add_plugin_to_profile(
    input: AddPluginToProfileInput,
    state: State<'_, AppState>,
) -> Result<AddPluginToProfileResponse, String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&input.profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    // Verify profile exists
    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())
    })?;

    // Create and store the plugin manifest
    let manifest = PluginManifest::new(input.plugin_id.clone(), input.marketplace, input.version);

    storage::store_plugin_manifest(profile_uuid, &manifest)
        .map_err(|e| format!("Failed to store plugin manifest: {e}"))?;

    Ok(AddPluginToProfileResponse {
        profile_id: profile_uuid.to_string(),
        plugin_id: input.plugin_id,
        added: true,
    })
}

/// Response for removing a plugin from a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePluginFromProfileResponse {
    pub profile_id: String,
    pub plugin_id: String,
    pub removed: bool,
}

/// Remove a plugin from a profile's central storage
#[tauri::command]
pub async fn remove_plugin_from_profile(
    profile_id: String,
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<RemovePluginFromProfileResponse, String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    // Verify profile exists
    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())
    })?;

    let removed = storage::delete_plugin_manifest(profile_uuid, &plugin_id)
        .map_err(|e| format!("Failed to remove plugin: {e}"))?;

    Ok(RemovePluginFromProfileResponse {
        profile_id: profile_uuid.to_string(),
        plugin_id,
        removed,
    })
}

/// List plugins stored in a profile
#[tauri::command]
pub async fn list_profile_plugins(
    profile_id: String,
    _state: State<'_, AppState>,
) -> Result<Vec<PluginManifestInfo>, String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    let manifests = storage::list_plugin_manifests(profile_uuid)
        .map_err(|e| format!("Failed to list plugins: {e}"))?;

    Ok(manifests
        .into_iter()
        .map(|m| PluginManifestInfo {
            id: m.id,
            marketplace: m.marketplace,
            version: m.version,
            enabled: m.enabled,
            added_at: m.added_at.to_rfc3339(),
        })
        .collect())
}

/// Plugin manifest info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestInfo {
    pub id: String,
    pub marketplace: Option<String>,
    pub version: Option<String>,
    pub enabled: bool,
    pub added_at: String,
}

// ============================================================================
// Profile Export/Import Commands (US5)
// ============================================================================

use tars_core::profile::export::{
    export_profile as core_export, import_profile as core_import, preview_import as core_preview,
};

/// Response for exporting a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProfileResponse {
    pub path: String,
    pub size_bytes: u64,
    pub exported_at: String,
}

/// Export a profile to .tars-profile.json
#[tauri::command]
pub async fn export_profile_json(
    profile_id: String,
    output_path: String,
    state: State<'_, AppState>,
) -> Result<ExportProfileResponse, String> {
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

        let export =
            core_export(&profile, &validated_output).map_err(|e| format!("Export failed: {e}"))?;

        // Get file size
        let metadata = std::fs::metadata(&validated_output)
            .map_err(|e| format!("Failed to read exported file: {e}"))?;

        Ok(ExportProfileResponse {
            path: validated_output.display().to_string(),
            size_bytes: metadata.len(),
            exported_at: export.exported_at.to_rfc3339(),
        })
    })
}

/// Preview for importing a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewResponse {
    pub name: String,
    pub description: Option<String>,
    pub tool_count: usize,
    pub has_name_collision: bool,
    pub existing_profile_id: Option<String>,
    pub version: u32,
}

/// Validate an import file path for safety
fn validate_import_path(path: &Path) -> Result<PathBuf, String> {
    // Reject paths containing path traversal components
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return Err("Import path cannot contain '..' path traversal".to_string());
    }

    // Check for null bytes
    if path_str.contains('\0') {
        return Err("Import path contains invalid characters".to_string());
    }

    // Require absolute path
    if !path.is_absolute() {
        return Err("Import path must be absolute".to_string());
    }

    // Verify the file exists and is a regular file
    if !path.exists() {
        return Err("Import file does not exist".to_string());
    }

    if !path.is_file() {
        return Err("Import path must be a file, not a directory".to_string());
    }

    // Verify it has the expected extension
    // Case-insensitive extension check
    let is_json = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
    if !is_json {
        return Err("Import file must be a .json file".to_string());
    }

    // Canonicalize to resolve any symlinks and get absolute path
    path.canonicalize()
        .map_err(|e| format!("Failed to resolve import path: {e}"))
}

/// Preview what would be imported from a file
#[tauri::command]
pub async fn preview_profile_import(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<ImportPreviewResponse, String> {
    let path = PathBuf::from(&file_path);
    let validated_path = validate_import_path(&path)?;

    let preview =
        core_preview(&validated_path).map_err(|e| format!("Failed to preview import: {e}"))?;

    // Check for name collision
    let (has_collision, existing_id) = state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        match store.get_by_name(&preview.name) {
            Ok(Some(existing)) => Ok((true, Some(existing.id.to_string()))),
            Ok(None) => Ok((false, None)),
            Err(e) => Err(format!("Database error: {e}")),
        }
    })?;

    Ok(ImportPreviewResponse {
        name: preview.name,
        description: preview.description,
        tool_count: preview.tool_count,
        has_name_collision: has_collision,
        existing_profile_id: existing_id,
        version: preview.version,
    })
}

/// Response for importing a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProfileResponse {
    pub profile: ProfileInfo,
    pub imported_from: String,
    pub collision_resolved: bool,
}

/// Import a profile from .tars-profile.json
#[tauri::command]
pub async fn import_profile_json(
    file_path: String,
    rename_to: Option<String>,
    state: State<'_, AppState>,
) -> Result<ImportProfileResponse, String> {
    let path = PathBuf::from(&file_path);
    let validated_path = validate_import_path(&path)?;

    // Import the profile
    let mut profile = core_import(&validated_path).map_err(|e| format!("Failed to import: {e}"))?;

    // Validate the imported profile name
    validate_profile_name(&profile.name)?;

    // Handle rename if provided
    let collision_resolved = if let Some(new_name) = rename_to {
        let validated = validate_profile_name(&new_name)?;
        profile.name = validated;
        true
    } else {
        false
    };

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        // Check for name collision
        if store
            .get_by_name(&profile.name)
            .map_err(|e| format!("Database error: {e}"))?
            .is_some()
        {
            return Err(format!(
                "Profile '{}' already exists. Provide rename_to to resolve.",
                profile.name
            ));
        }

        store
            .create(&profile)
            .map_err(|e| format!("Failed to save profile: {e}"))?;

        Ok(ImportProfileResponse {
            profile: ProfileInfo {
                id: profile.id.to_string(),
                name: profile.name,
                description: profile.description,
                tool_count: profile.tool_refs.len(),
                created_at: profile.created_at.to_rfc3339(),
                updated_at: profile.updated_at.to_rfc3339(),
            },
            imported_from: file_path,
            collision_resolved,
        })
    })
}

// ============================================================================
// Profile Update Detection Commands
// ============================================================================

/// Tool update info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUpdateInfoResponse {
    pub name: String,
    pub tool_type: String,
    pub source_path: String,
    pub old_hash: String,
    pub new_hash: String,
    pub mode: String,
}

/// Profile update check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUpdateCheckResponse {
    pub updates: Vec<ToolUpdateInfoResponse>,
    pub missing_sources: Vec<String>,
    pub total_checked: usize,
}

/// Check a profile for available updates to tracked tools
#[tauri::command]
pub async fn check_profile_updates(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<ProfileUpdateCheckResponse, String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        let profile = store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Profile not found")?;

        let check = tars_core::profile::check_profile_updates(&profile)
            .map_err(|e| format!("Failed to check updates: {e}"))?;

        Ok(ProfileUpdateCheckResponse {
            updates: check
                .updates
                .iter()
                .map(|u| ToolUpdateInfoResponse {
                    name: u.name.clone(),
                    tool_type: u.tool_type.to_string(),
                    source_path: u.source_path.display().to_string(),
                    old_hash: u.old_hash.clone(),
                    new_hash: u.new_hash.clone(),
                    mode: match u.mode {
                        tars_core::profile::SourceMode::Pin => "pin".to_string(),
                        tars_core::profile::SourceMode::Track => "track".to_string(),
                    },
                })
                .collect(),
            missing_sources: check.missing_sources,
            total_checked: check.total_checked,
        })
    })
}

/// Pull an update for a specific tool in a profile
#[tauri::command]
pub async fn pull_tool_update(
    state: State<'_, AppState>,
    profile_id: String,
    tool_name: String,
) -> Result<(), String> {
    use tars_core::profile::storage;

    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        let mut profile = store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Profile not found")?;

        // Find the tool
        let tool = profile
            .tool_refs
            .iter_mut()
            .find(|t| t.name == tool_name)
            .ok_or("Tool not found in profile")?;

        let source_ref = tool
            .source_ref
            .as_ref()
            .ok_or("Tool has no source reference")?;

        if !source_ref.source_path.exists() {
            return Err("Source file no longer exists".to_string());
        }

        // Compute new hash and copy content
        let new_hash = match tool.tool_type {
            ToolType::Skill => {
                storage::copy_skill_to_profile(profile_uuid, &tool.name, &source_ref.source_path)
                    .map_err(|e| format!("Failed to copy skill: {e}"))?;
                storage::compute_dir_hash(&source_ref.source_path)
                    .map_err(|e| format!("Failed to compute hash: {e}"))?
            }
            ToolType::Agent => {
                storage::copy_agent_to_profile(profile_uuid, &tool.name, &source_ref.source_path)
                    .map_err(|e| format!("Failed to copy agent: {e}"))?;
                storage::compute_file_hash(&source_ref.source_path)
                    .map_err(|e| format!("Failed to compute hash: {e}"))?
            }
            _ => return Err("Unsupported tool type for update".to_string()),
        };

        // Update the source_ref hash
        if let Some(ref mut sr) = tool.source_ref {
            sr.source_hash = new_hash;
            sr.copied_at = chrono::Utc::now().to_rfc3339();
        }

        // Save updated profile
        store
            .update(&profile)
            .map_err(|e| format!("Failed to update profile: {e}"))?;

        Ok(())
    })
}

/// Set the source mode (pin/track) for a tool in a profile
#[tauri::command]
pub async fn set_tool_source_mode(
    state: State<'_, AppState>,
    profile_id: String,
    tool_name: String,
    mode: String,
) -> Result<(), String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    let source_mode = match mode.as_str() {
        "pin" => tars_core::profile::SourceMode::Pin,
        "track" => tars_core::profile::SourceMode::Track,
        _ => return Err("Invalid mode. Use 'pin' or 'track'.".to_string()),
    };

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());

        let mut profile = store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Profile not found")?;

        // Find the tool and update its mode
        let tool = profile
            .tool_refs
            .iter_mut()
            .find(|t| t.name == tool_name)
            .ok_or("Tool not found in profile")?;

        if let Some(ref mut sr) = tool.source_ref {
            sr.mode = source_mode;
        } else {
            return Err("Tool has no source reference".to_string());
        }

        // Save updated profile
        store
            .update(&profile)
            .map_err(|e| format!("Failed to update profile: {e}"))?;

        Ok(())
    })
}

/// Assign a profile to a project as a plugin
#[tauri::command]
pub async fn assign_profile_as_plugin(
    state: State<'_, AppState>,
    project_id: String,
    profile_id: String,
) -> Result<PluginAssignResponse, String> {
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    let (project, profile, old_profile) = state.with_db(|db| {
        let project_store = tars_core::storage::projects::ProjectStore::new(db.connection());
        let profile_store = ProfileStore::new(db.connection());

        let project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Project not found")?;

        let profile = profile_store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Profile not found")?;

        let old_profile = if let Some(old_profile_id) = project.assigned_profile_id {
            if old_profile_id != profile_uuid {
                profile_store
                    .get(old_profile_id)
                    .map_err(|e| format!("Database error loading previous profile: {e}"))?
            } else {
                None
            }
        } else {
            None
        };

        Ok((project, profile, old_profile))
    })?;

    let project_path_str = project.path.to_string_lossy().to_string();

    if let Some(old_profile) = old_profile {
        let old_plugin_id = format!(
            "tars-profile-{}",
            sanitize_plugin_name_for_unassign(&old_profile.name)
        );
        let old_plugin_key = format!("{old_plugin_id}@{PROFILE_MARKETPLACE}");
        let _ = plugin_uninstall(
            old_plugin_key,
            Some("project".to_string()),
            Some(project_path_str.clone()),
        )
        .await;
    }

    let marketplace_sync = sync_profile_marketplace(&profile)
        .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;

    ensure_profile_marketplace(&marketplace_sync.marketplace_path)
        .await
        .map_err(|e| format!("Failed to register marketplace: {e}"))?;

    let plugin_key = format!("{}@{PROFILE_MARKETPLACE}", marketplace_sync.plugin_id);
    let output = plugin_install(
        plugin_key,
        Some("project".to_string()),
        Some(project_path_str.clone()),
    )
    .await
    .map_err(|e| format!("Failed to install profile plugin: {e}"))?;

    state.with_db(|db| {
        let project_store = tars_core::storage::projects::ProjectStore::new(db.connection());
        let mut project = project;
        project.assigned_profile_id = Some(profile_uuid);
        project.updated_at = chrono::Utc::now();
        project_store
            .update(&project)
            .map_err(|e| format!("Failed to update project: {e}"))?;
        Ok(())
    })?;

    Ok(PluginAssignResponse {
        plugin_id: marketplace_sync.plugin_id,
        installed: true,
        output,
    })
}

/// Sanitize a plugin name for unassign (matches sync.rs logic)
fn sanitize_plugin_name_for_unassign(name: &str) -> String {
    name.chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '-' || c == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

/// Plugin assignment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAssignResponse {
    pub plugin_id: String,
    pub installed: bool,
    pub output: String,
}

async fn ensure_profile_marketplace(path: &Path) -> Result<(), String> {
    let source = path.to_string_lossy().to_string();
    match plugin_marketplace_add(source).await {
        Ok(_) => {}
        Err(err) => {
            let lower = err.to_lowercase();
            if !(lower.contains("already") || lower.contains("exists")) {
                return Err(err);
            }
        }
    }

    plugin_marketplace_update(Some(PROFILE_MARKETPLACE.to_string())).await?;
    Ok(())
}

/// Unassign a profile plugin from a project
///
/// Looks up the assigned profile and computes the `plugin_id` automatically.
#[tauri::command]
pub async fn unassign_profile_plugin(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<(), String> {
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;

    let (project, profile) = state.with_db(|db| {
        let project_store = tars_core::storage::projects::ProjectStore::new(db.connection());
        let profile_store = ProfileStore::new(db.connection());

        let project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Project not found")?;

        let profile_id = project
            .assigned_profile_id
            .ok_or("No profile assigned to this project")?;

        let profile = profile_store
            .get(profile_id)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Assigned profile not found")?;

        Ok((project, profile))
    })?;

    let plugin_id = format!(
        "tars-profile-{}",
        sanitize_plugin_name_for_unassign(&profile.name)
    );
    let plugin_key = format!("{plugin_id}@{PROFILE_MARKETPLACE}");
    let project_path = project.path.to_string_lossy().to_string();

    plugin_uninstall(plugin_key, Some("project".to_string()), Some(project_path))
        .await
        .map_err(|e| format!("Failed to uninstall profile plugin: {e}"))?;

    state.with_db(|db| {
        let project_store = tars_core::storage::projects::ProjectStore::new(db.connection());
        let mut project = project;
        project.assigned_profile_id = None;
        project.updated_at = chrono::Utc::now();
        project_store
            .update(&project)
            .map_err(|e| format!("Failed to update project: {e}"))?;
        Ok(())
    })
}

// ============================================================================
// Install Profile Plugin Commands
// ============================================================================

/// Install a profile plugin to a specific project
#[tauri::command]
pub async fn install_profile_to_project(
    state: State<'_, AppState>,
    profile_id: String,
    project_id: String,
) -> Result<PluginAssignResponse, String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;
    let project_uuid =
        uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid project ID: {e}"))?;

    let (profile, project) = state.with_db(|db| {
        let profile_store = ProfileStore::new(db.connection());
        let project_store = tars_core::storage::projects::ProjectStore::new(db.connection());

        let profile = profile_store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Profile not found")?;

        let project = project_store
            .get(project_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Project not found")?;

        Ok((profile, project))
    })?;

    let marketplace_sync = sync_profile_marketplace(&profile)
        .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;

    ensure_profile_marketplace(&marketplace_sync.marketplace_path)
        .await
        .map_err(|e| format!("Failed to register marketplace: {e}"))?;

    let plugin_key = format!("{}@{PROFILE_MARKETPLACE}", marketplace_sync.plugin_id);
    let project_path = project.path.to_string_lossy().to_string();
    let output = plugin_install(plugin_key, Some("project".to_string()), Some(project_path))
        .await
        .map_err(|e| format!("Failed to install profile to project: {e}"))?;

    Ok(PluginAssignResponse {
        plugin_id: marketplace_sync.plugin_id,
        installed: true,
        output,
    })
}

/// Install a profile plugin globally (user scope)
#[tauri::command]
pub async fn install_profile_to_user(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<PluginAssignResponse, String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    let profile = state.with_db(|db| {
        let profile_store = ProfileStore::new(db.connection());

        profile_store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Profile not found".to_string())
    })?;

    let marketplace_sync = sync_profile_marketplace(&profile)
        .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;

    ensure_profile_marketplace(&marketplace_sync.marketplace_path)
        .await
        .map_err(|e| format!("Failed to register marketplace: {e}"))?;

    let plugin_key = format!("{}@{PROFILE_MARKETPLACE}", marketplace_sync.plugin_id);
    let output = plugin_install(plugin_key, Some("user".to_string()), None)
        .await
        .map_err(|e| format!("Failed to install profile to user: {e}"))?;

    Ok(PluginAssignResponse {
        plugin_id: marketplace_sync.plugin_id,
        installed: true,
        output,
    })
}

/// Uninstall a profile plugin from user scope
#[tauri::command]
pub async fn uninstall_profile_from_user(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<(), String> {
    let profile_uuid =
        uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    state.with_db(|db| {
        let profile_store = ProfileStore::new(db.connection());

        let profile = profile_store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or("Profile not found")?;

        // Compute plugin_id from profile name
        let plugin_id = format!(
            "tars-profile-{}",
            sanitize_plugin_name_for_unassign(&profile.name)
        );

        // Uninstall the plugin from user scope
        tars_core::profile::uninstall_profile_plugin_from_user(&plugin_id)
            .map_err(|e| format!("Failed to uninstall profile from user: {e}"))?;

        Ok(())
    })
}
