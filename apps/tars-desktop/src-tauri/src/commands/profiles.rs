//! Profile management Tauri commands
//!
//! Commands for creating and managing profiles.

use super::utils::find_claude_binary;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tars_core::export::export_as_plugin;
use tars_core::profile::snapshot::snapshot_from_project;
use tars_core::profile::storage::{list_plugin_manifests, save_project_state, ProjectProfileState};
use tars_core::profile::sync::{
    apply_profile_to_project, convert_profile_to_local_overrides, sync_profile_to_projects,
};
use tars_core::profile::{PluginRef, ToolPermissions, ToolRef, ToolType};
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
        let agent_refs = p
            .tool_refs
            .iter()
            .filter(|t| t.tool_type == ToolType::Agent)
            .count();

        // Combine overlay counts with tool_ref counts
        let mcp_count = mcp_refs;
        let skills_count = p.repo_overlays.skills.len() + p.user_overlays.skills.len() + skill_refs;
        let commands_count = p.repo_overlays.commands.len() + p.user_overlays.commands.len();
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

        store
            .create(&profile)
            .map_err(|e| format!("Failed to save profile: {e}"))?;

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

        store
            .create(&profile)
            .map_err(|e| format!("Failed to save profile: {e}"))?;

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

/// Delete a profile, converting assigned tools to local overrides
#[tauri::command]
pub async fn delete_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<DeleteProfileResponse, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        // First, convert profile tools to local overrides for all assigned projects
        let converted = convert_profile_to_local_overrides(db.connection(), uuid)
            .map_err(|e| format!("Failed to convert profile tools: {e}"))?;

        let converted_count = converted.len();

        // Now delete the profile
        let store = ProfileStore::new(db.connection());
        let deleted = store
            .delete(uuid)
            .map_err(|e| format!("Failed to delete profile: {e}"))?;

        Ok(DeleteProfileResponse {
            deleted,
            converted_projects: converted_count,
        })
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

        // Sync to assigned projects
        let sync_result = sync_profile_to_projects(db.connection(), uuid)
            .map_err(|e| format!("Sync failed: {e}"))?;

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
    pub source_project_path: String,
    pub tools: Vec<ToolFromSource>,
}

/// Tool to add from source project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFromSource {
    pub name: String,
    pub tool_type: String,
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
    let source_path = PathBuf::from(&input.source_project_path);

    if !source_path.exists() {
        return Err(format!(
            "Source project path does not exist: {}",
            input.source_project_path
        ));
    }

    // Verify profile exists
    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())
    })?;

    // Scan the source project
    let inventory = tars_scanner::scope::scan_project(&source_path)
        .map_err(|e| format!("Failed to scan source project: {e}"))?;

    let mut mcp_servers_added = 0usize;
    let mut skills_added = 0usize;
    let mut agents_added = 0usize;
    let commands_added = 0usize;
    let mut tool_refs: Vec<ToolRef> = Vec::new();

    for tool in &input.tools {
        let tool_type = match tool.tool_type.to_lowercase().as_str() {
            "mcp" => ToolType::Mcp,
            "skill" => ToolType::Skill,
            "agent" => ToolType::Agent,
            "hook" => ToolType::Hook,
            other => return Err(format!("Invalid tool type: {other}")),
        };

        match tool_type {
            ToolType::Mcp => {
                // Find MCP server in inventory and store config as JSON
                if let Some(mcp_config) = &inventory.mcp {
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

                        tool_refs.push(ToolRef {
                            name: tool.name.clone(),
                            tool_type: ToolType::Mcp,
                            source_scope: Some(Scope::Project),
                            permissions: None,
                        });
                        mcp_servers_added += 1;
                    }
                }
            }
            ToolType::Skill => {
                // Find skill in inventory and copy the entire skill directory
                if let Some(skill) = inventory.skills.iter().find(|s| s.name == tool.name) {
                    // skill.path is the skill directory (contains SKILL.md)
                    if skill.path.exists() {
                        storage::copy_skill_to_profile(profile_uuid, &skill.name, &skill.path)
                            .map_err(|e| format!("Failed to copy skill: {e}"))?;

                        tool_refs.push(ToolRef {
                            name: tool.name.clone(),
                            tool_type: ToolType::Skill,
                            source_scope: Some(Scope::Project),
                            permissions: None,
                        });
                        skills_added += 1;
                    }
                }
            }
            ToolType::Agent => {
                // Find agent in inventory and copy the file
                if let Some(agent) = inventory.agents.iter().find(|a| a.name == tool.name) {
                    if agent.path.exists() {
                        storage::copy_agent_to_profile(profile_uuid, &agent.name, &agent.path)
                            .map_err(|e| format!("Failed to copy agent: {e}"))?;

                        tool_refs.push(ToolRef {
                            name: tool.name.clone(),
                            tool_type: ToolType::Agent,
                            source_scope: Some(Scope::Project),
                            permissions: None,
                        });
                        agents_added += 1;
                    }
                }
            }
            ToolType::Hook => {
                // Hooks are handled via settings, not as file-based copies
                // Just add the reference for now
                tool_refs.push(ToolRef {
                    name: tool.name.clone(),
                    tool_type: ToolType::Hook,
                    source_scope: Some(Scope::Project),
                    permissions: None,
                });
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

        Ok(AddToolsFromSourceResponse {
            added_count: mcp_servers_added + skills_added + agents_added + commands_added,
            mcp_servers_added,
            skills_added,
            agents_added,
            commands_added,
        })
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

/// Preview what would be imported from a file
#[tauri::command]
pub async fn preview_profile_import(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<ImportPreviewResponse, String> {
    let path = PathBuf::from(&file_path);

    let preview = core_preview(&path).map_err(|e| format!("Failed to preview import: {e}"))?;

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

    // Import the profile
    let mut profile = core_import(&path).map_err(|e| format!("Failed to import: {e}"))?;

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
