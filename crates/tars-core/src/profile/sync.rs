//! Profile sync operations
//!
//! This module handles syncing profile changes to all assigned projects.

use crate::profile::storage::copy_dir_recursive;
use crate::profile::types::Profile;
use crate::project::Project;
use crate::storage::db::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Result of syncing a profile to its assigned projects
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of projects affected by the sync
    pub affected_projects: usize,
    /// When the sync occurred
    pub synced_at: DateTime<Utc>,
}

impl SyncResult {
    /// Create a new sync result
    #[must_use]
    pub fn new(affected_projects: usize) -> Self {
        Self {
            affected_projects,
            synced_at: Utc::now(),
        }
    }
}

/// Sync a profile to all its assigned projects
///
/// This function finds all projects with the given profile assigned
/// and ensures their effective configuration is up-to-date.
///
/// # Errors
/// Returns an error if database operations fail
pub fn sync_profile_to_projects(
    conn: &Connection,
    profile_id: Uuid,
) -> Result<SyncResult, DatabaseError> {
    use crate::storage::projects::ProjectStore;

    let store = ProjectStore::new(conn);

    // Get all projects with this profile assigned
    let projects = store.list_by_profile(profile_id)?;
    let affected = projects.len();

    // For now, sync is implicit - projects always resolve their
    // effective configuration by looking up the profile.
    // This function exists to:
    // 1. Count affected projects for notification
    // 2. Future: trigger any cache invalidation needed

    Ok(SyncResult::new(affected))
}

/// Convert profile tools to local overrides when a profile is deleted
///
/// # Errors
/// Returns an error if database operations fail
pub fn convert_profile_to_local_overrides(
    conn: &Connection,
    profile_id: Uuid,
) -> Result<Vec<Project>, DatabaseError> {
    use crate::storage::profiles::ProfileStore;
    use crate::storage::projects::ProjectStore;

    let profile_store = ProfileStore::new(conn);
    let project_store = ProjectStore::new(conn);

    // Get the profile's tools before deletion
    let profile = profile_store
        .get(profile_id)?
        .ok_or_else(|| DatabaseError::Migration(format!("Profile not found: {profile_id}")))?;

    // Get all projects with this profile
    let projects = project_store.list_by_profile(profile_id)?;
    let mut updated_projects = Vec::new();

    for mut project in projects {
        // Move profile tools to local overrides
        for tool_ref in &profile.tool_refs {
            match tool_ref.tool_type {
                crate::profile::ToolType::Mcp => {
                    project.local_overrides.mcp_servers.push(tool_ref.clone());
                }
                crate::profile::ToolType::Skill => {
                    project.local_overrides.skills.push(tool_ref.clone());
                }
                crate::profile::ToolType::Agent => {
                    project.local_overrides.agents.push(tool_ref.clone());
                }
                crate::profile::ToolType::Hook => {
                    project.local_overrides.hooks.push(tool_ref.clone());
                }
            }
        }

        // Clear the profile assignment
        project.assigned_profile_id = None;
        project.updated_at = Utc::now();

        // Save the updated project
        project_store.update(&project)?;
        updated_projects.push(project);
    }

    Ok(updated_projects)
}

/// Apply a profile's tools to a project directory
///
/// This copies tool files from central profile storage (~/.tars/profiles/<id>/)
/// to the target project directory:
/// - MCP servers → .mcp.json (merged)
/// - Skills → .claude/skills/<name>/ (directory with SKILL.md)
/// - Agents → .claude/agents/<name>.md
/// - Commands → .claude/commands/<name>.md
///
/// # Errors
/// Returns an error if file operations fail
pub fn apply_profile_to_project(
    profile: &Profile,
    project_path: &Path,
) -> Result<ApplyResult, ApplyError> {
    use super::storage;

    let mut result = ApplyResult::default();

    // List tools stored in the profile's central storage
    let stored_tools =
        storage::list_profile_tools(profile.id).map_err(|e| ApplyError::Storage(e.to_string()))?;

    // Apply MCP servers to .mcp.json
    for server_name in &stored_tools.mcp_servers {
        let config = storage::get_mcp_server_config(profile.id, server_name)
            .map_err(|e| ApplyError::Storage(e.to_string()))?;

        apply_mcp_server_config(server_name, &config, project_path)?;
        result.mcp_servers_applied += 1;
    }

    // Apply skills from central storage
    for skill_name in &stored_tools.skills {
        storage::apply_skill_to_project(profile.id, skill_name, project_path)
            .map_err(|e| ApplyError::Storage(e.to_string()))?;
        result.skills_applied += 1;
    }

    // Apply agents from central storage
    for agent_name in &stored_tools.agents {
        storage::apply_agent_to_project(profile.id, agent_name, project_path)
            .map_err(|e| ApplyError::Storage(e.to_string()))?;
        result.agents_applied += 1;
    }

    // Apply commands from central storage
    for command_name in &stored_tools.commands {
        storage::apply_command_to_project(profile.id, command_name, project_path)
            .map_err(|e| ApplyError::Storage(e.to_string()))?;
        result.commands_applied += 1;
    }

    Ok(result)
}

/// Apply a single MCP server config to the project's .mcp.json
fn apply_mcp_server_config(
    server_name: &str,
    config: &Value,
    project_path: &Path,
) -> Result<(), ApplyError> {
    let mcp_path = project_path.join(".mcp.json");

    // Read existing config or create new
    let mut mcp_config: Value = if mcp_path.exists() {
        let content = fs::read_to_string(&mcp_path)
            .map_err(|e| ApplyError::Io(format!("Failed to read .mcp.json: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| ApplyError::Parse(format!("Failed to parse .mcp.json: {e}")))?
    } else {
        json!({})
    };

    // Ensure mcpServers object exists
    let root = mcp_config
        .as_object_mut()
        .ok_or_else(|| ApplyError::Parse("Expected JSON object".into()))?;

    if !root.contains_key("mcpServers") {
        root.insert("mcpServers".to_string(), json!({}));
    }

    let mcp_servers = root
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| ApplyError::Parse("mcpServers is not an object".into()))?;

    // Add/update the server config
    mcp_servers.insert(server_name.to_string(), config.clone());

    // Write back
    let content = serde_json::to_string_pretty(&mcp_config)
        .map_err(|e| ApplyError::Parse(format!("Failed to serialize .mcp.json: {e}")))?;
    fs::write(&mcp_path, content)
        .map_err(|e| ApplyError::Io(format!("Failed to write .mcp.json: {e}")))?;

    Ok(())
}

/// Result of applying a profile to a project
#[derive(Debug, Clone, Default)]
pub struct ApplyResult {
    /// Number of MCP servers applied
    pub mcp_servers_applied: usize,
    /// Number of skills applied
    pub skills_applied: usize,
    /// Number of agents applied
    pub agents_applied: usize,
    /// Number of commands applied
    pub commands_applied: usize,
}

impl ApplyResult {
    /// Total number of items applied
    pub fn total(&self) -> usize {
        self.mcp_servers_applied + self.skills_applied + self.agents_applied + self.commands_applied
    }
}

/// Error applying a profile
#[derive(Debug, Clone)]
pub enum ApplyError {
    /// IO error
    Io(String),
    /// Parse error
    Parse(String),
    /// Storage error
    Storage(String),
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::Io(msg) => write!(f, "IO error: {msg}"),
            ApplyError::Parse(msg) => write!(f, "Parse error: {msg}"),
            ApplyError::Storage(msg) => write!(f, "Storage error: {msg}"),
        }
    }
}

impl std::error::Error for ApplyError {}

// ============================================================================
// Plugin-based Assignment
// ============================================================================

pub const PROFILE_MARKETPLACE: &str = "tars-profiles";

/// Result of assigning a profile as a plugin
#[derive(Debug, Clone)]
pub struct PluginAssignResult {
    /// Path to the generated plugin
    pub plugin_path: PathBuf,
    /// Plugin ID for uninstall
    pub plugin_id: String,
    /// Whether the install was successful
    pub installed: bool,
    /// CLI output (for debugging)
    pub output: String,
}

/// Assign a profile to a project by generating and installing a plugin
///
/// This generates a Claude Code plugin from the profile's stored tools,
/// copies it to `~/.claude/plugins/`, and registers in `installed_plugins.json`
/// with project scope.
///
/// # Errors
/// Returns an error if plugin generation or installation fails
pub fn assign_profile_as_plugin(
    profile: &Profile,
    _project_path: &Path,
) -> Result<PluginAssignResult, ApplyError> {
    let sync_result = sync_profile_marketplace(profile)?;

    Ok(PluginAssignResult {
        plugin_path: sync_result.plugin_path.join(".claude-plugin"),
        plugin_id: sync_result.plugin_id,
        installed: false,
        output: format!(
            "Prepared marketplace at {} (install via Claude CLI)",
            sync_result.marketplace_path.display()
        ),
    })
}

/// Sanitize a plugin name for filesystem and CLI safety
fn sanitize_plugin_name(name: &str) -> String {
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

/// Unassign a profile plugin from a project
///
/// Removes plugin from `installed_plugins.json` for this project.
/// Plugin files are removed only if no other scopes use it.
///
/// # Errors
/// Returns an error if uninstall fails
pub fn unassign_profile_plugin(project_path: &Path, plugin_id: &str) -> Result<(), ApplyError> {
    // Validate plugin_id to prevent directory traversal
    if !is_valid_plugin_id(plugin_id) {
        return Err(ApplyError::Io("Invalid plugin ID".to_string()));
    }

    let home_dir = dirs::home_dir()
        .ok_or_else(|| ApplyError::Io("Could not determine home directory".to_string()))?;

    // Remove from installed_plugins.json for this project
    let project_path_str = project_path.to_string_lossy().to_string();
    unregister_installed_plugin(&home_dir, plugin_id, Some(&project_path_str))?;

    Ok(())
}

/// Validate a plugin ID for safety
fn is_valid_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && !id.contains("..")
        && !id.contains('\0')
        && !id.contains('/')
        && !id.contains('\\')
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '@')
}

/// Reinstall a profile plugin after updates
///
/// This uninstalls the old plugin and installs the updated version.
///
/// # Errors
/// Returns an error if reinstall fails
pub fn reinstall_profile_plugin(
    profile: &Profile,
    project_path: &Path,
) -> Result<PluginAssignResult, ApplyError> {
    // Use sanitize_plugin_name for consistent plugin ID derivation
    let plugin_id = format!("tars-profile-{}", sanitize_plugin_name(&profile.name));

    // Uninstall existing (ignore errors - may not exist)
    let _ = unassign_profile_plugin(project_path, &plugin_id);

    // Install fresh
    assign_profile_as_plugin(profile, project_path)
}

/// Generate the plugin for a profile into its storage directory
///
/// This creates a ready-to-install plugin at `~/.tars/profiles/<id>/plugin/`.
/// Called after profile creation or update.
///
/// Uses atomic directory replacement to avoid race conditions where the plugin
/// directory might be missing during regeneration.
///
/// # Errors
/// Returns an error if plugin generation fails
pub fn regenerate_profile_plugin(profile: &Profile) -> Result<PathBuf, ApplyError> {
    use crate::export::export_as_plugin_with_hash;
    use crate::profile::storage::{profile_dir, profile_plugin_dir};

    let profile_storage =
        profile_dir(profile.id).map_err(|e| ApplyError::Storage(e.to_string()))?;

    // Ensure the profile directory exists
    fs::create_dir_all(&profile_storage)
        .map_err(|e| ApplyError::Io(format!("Failed to create profile directory: {e}")))?;

    // Generate to a temporary directory within the profile storage
    // Using profile storage instead of system temp ensures same filesystem for atomic rename
    let temp_plugin_name = format!("plugin-{}", Uuid::new_v4());
    let temp_plugin_dir = profile_storage.join(&temp_plugin_name);

    // Clean up any previous temp directories (from interrupted operations)
    for entry in fs::read_dir(&profile_storage)
        .into_iter()
        .flatten()
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("plugin-") && name != "plugin" {
            let _ = fs::remove_dir_all(entry.path());
        }
    }

    // Create temp directory
    fs::create_dir_all(&temp_plugin_dir)
        .map_err(|e| ApplyError::Io(format!("Failed to create temp plugin directory: {e}")))?;

    // Generate plugin name from profile
    let plugin_name = format!("tars-profile-{}", sanitize_plugin_name(&profile.name));

    // Export profile as plugin with content-hash version to temp directory
    let generated_plugin_path = export_as_plugin_with_hash(profile, &temp_plugin_dir, &plugin_name)
        .map_err(|e| ApplyError::Storage(e.to_string()))?;

    // Get the final plugin directory path
    let final_plugin_dir =
        profile_plugin_dir(profile.id).map_err(|e| ApplyError::Storage(e.to_string()))?;

    // If old plugin exists, move it to a backup location first (for cleanup on success)
    let backup_dir = profile_storage.join("plugin-backup");
    if final_plugin_dir.exists() {
        // Remove any old backup
        if backup_dir.exists() {
            let _ = fs::remove_dir_all(&backup_dir);
        }
        // Move current to backup
        fs::rename(&final_plugin_dir, &backup_dir)
            .map_err(|e| ApplyError::Io(format!("Failed to backup existing plugin: {e}")))?;
    }

    // Atomically move temp to final location
    match fs::rename(&temp_plugin_dir, &final_plugin_dir) {
        Ok(()) => {
            // Success - clean up backup
            let _ = fs::remove_dir_all(&backup_dir);
            Ok(generated_plugin_path)
        }
        Err(e) => {
            // Failed - try to restore backup
            if backup_dir.exists() {
                let _ = fs::rename(&backup_dir, &final_plugin_dir);
            }
            // Clean up failed temp directory
            let _ = fs::remove_dir_all(&temp_plugin_dir);
            Err(ApplyError::Io(format!(
                "Failed to move plugin to final location: {e}"
            )))
        }
    }
}

/// Install a profile's pre-generated plugin to a project
///
/// Copies plugin to `~/.claude/plugins/` and registers in `installed_plugins.json`
/// with project scope.
///
/// # Errors
/// Returns an error if installation fails
pub fn install_profile_plugin_to_project(
    profile: &Profile,
    _project_path: &Path,
) -> Result<PluginAssignResult, ApplyError> {
    let sync_result = sync_profile_marketplace(profile)?;

    Ok(PluginAssignResult {
        plugin_path: sync_result.plugin_path.join(".claude-plugin"),
        plugin_id: sync_result.plugin_id,
        installed: false,
        output: format!(
            "Prepared marketplace at {} (install via Claude CLI)",
            sync_result.marketplace_path.display()
        ),
    })
}

/// Install a profile's pre-generated plugin globally (user scope)
///
/// Copies plugin to `~/.claude/plugins/` and registers in `installed_plugins.json`
/// with user scope.
///
/// # Errors
/// Returns an error if installation fails
pub fn install_profile_plugin_to_user(profile: &Profile) -> Result<PluginAssignResult, ApplyError> {
    let sync_result = sync_profile_marketplace(profile)?;

    Ok(PluginAssignResult {
        plugin_path: sync_result.plugin_path.join(".claude-plugin"),
        plugin_id: sync_result.plugin_id,
        installed: false,
        output: format!(
            "Prepared marketplace at {} (install via Claude CLI)",
            sync_result.marketplace_path.display()
        ),
    })
}

/// Copy plugin files to ~/.claude/plugins/ with atomic replacement
fn copy_plugin_to_claude_dir(
    source_dir: &Path,
    target_dir: &Path,
    plugin_name: &str,
) -> Result<(), ApplyError> {
    validate_plugin_root(source_dir)?;
    let parent_dir = target_dir
        .parent()
        .ok_or_else(|| ApplyError::Io("Invalid target directory".to_string()))?;

    // Create plugins directory if needed
    fs::create_dir_all(parent_dir)
        .map_err(|e| ApplyError::Io(format!("Failed to create plugins directory: {e}")))?;

    // Use atomic directory replacement for safety
    let backup_dir = parent_dir.join(format!("{plugin_name}.backup"));
    let temp_dir = parent_dir.join(format!("{plugin_name}.installing"));

    // Clean up any previous temp directory
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).ok();
    }

    // Copy plugin to temp location first
    copy_dir_recursive(source_dir, &temp_dir)
        .map_err(|e| ApplyError::Io(format!("Failed to copy plugin: {e}")))?;
    validate_plugin_root(&temp_dir)?;

    // Backup existing plugin if present
    if target_dir.exists() {
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir).ok();
        }
        fs::rename(target_dir, &backup_dir)
            .map_err(|e| ApplyError::Io(format!("Failed to backup existing plugin: {e}")))?;
    }

    // Atomic move from temp to final location
    match fs::rename(&temp_dir, target_dir) {
        Ok(()) => {
            // Success - remove backup
            if backup_dir.exists() {
                fs::remove_dir_all(&backup_dir).ok();
            }
            Ok(())
        }
        Err(e) => {
            // Failed - restore backup if exists
            if backup_dir.exists() {
                let _ = fs::rename(&backup_dir, target_dir);
            }
            let _ = fs::remove_dir_all(&temp_dir);
            Err(ApplyError::Io(format!("Failed to install plugin: {e}")))
        }
    }
}

fn profile_marketplace_dir(home_dir: &Path) -> PathBuf {
    home_dir
        .join(".claude")
        .join("plugins")
        .join("marketplaces")
        .join(PROFILE_MARKETPLACE)
}

fn profile_marketplace_plugins_dir(home_dir: &Path) -> PathBuf {
    profile_marketplace_dir(home_dir).join("plugins")
}

fn rebuild_profile_marketplace_manifest(marketplace_dir: &Path) -> Result<(), ApplyError> {
    let plugins_dir = marketplace_dir.join("plugins");
    let mut plugin_entries = Vec::new();

    if plugins_dir.exists() {
        for entry in fs::read_dir(&plugins_dir)
            .map_err(|e| ApplyError::Io(format!("Failed to read marketplace: {e}")))?
        {
            let entry = entry.map_err(|e| ApplyError::Io(format!("Failed to read plugin: {e}")))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let plugin_id = entry.file_name().to_string_lossy().to_string();
            if plugin_id.starts_with('.')
                || plugin_id.ends_with(".installing")
                || plugin_id.ends_with(".backup")
            {
                continue;
            }
            let manifest_path = path.join(".claude-plugin").join("plugin.json");
            let manifest_value = if manifest_path.exists() {
                fs::read_to_string(&manifest_path)
                    .ok()
                    .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            } else {
                None
            };

            let description = manifest_value
                .as_ref()
                .and_then(|json| json.get("description"))
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            let version = manifest_value
                .as_ref()
                .and_then(|json| json.get("version"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
                .to_string();

            plugin_entries.push(json!({
                "name": plugin_id,
                "description": description,
                "version": version,
                "source": format!("./plugins/{plugin_id}")
            }));
        }
    }

    let marketplace_manifest = json!({
        "$schema": "https://anthropic.com/claude-code/marketplace.schema.json",
        "name": PROFILE_MARKETPLACE,
        "description": "Profiles managed by TARS",
        "owner": {
            "name": "TARS"
        },
        "plugins": plugin_entries
    });

    let manifest_dir = marketplace_dir.join(".claude-plugin");
    fs::create_dir_all(&manifest_dir)
        .map_err(|e| ApplyError::Io(format!("Failed to create marketplace manifest dir: {e}")))?;
    let manifest_path = manifest_dir.join("marketplace.json");
    let content = serde_json::to_string_pretty(&marketplace_manifest)
        .map_err(|e| ApplyError::Io(format!("Failed to serialize marketplace.json: {e}")))?;
    fs::write(&manifest_path, content)
        .map_err(|e| ApplyError::Io(format!("Failed to write marketplace.json: {e}")))?;

    Ok(())
}

pub struct MarketplaceSyncResult {
    pub marketplace_path: PathBuf,
    pub plugin_id: String,
    pub plugin_path: PathBuf,
}

/// Sync a profile plugin into the local marketplace directory.
///
/// This prepares the marketplace on disk; installation is done via the Claude CLI.
pub fn sync_profile_marketplace(profile: &Profile) -> Result<MarketplaceSyncResult, ApplyError> {
    use crate::profile::storage::profile_plugin_dir;

    let plugin_dir =
        profile_plugin_dir(profile.id).map_err(|e| ApplyError::Storage(e.to_string()))?;

    if !plugin_dir.exists() {
        regenerate_profile_plugin(profile)?;
    }

    let plugin_name = format!("tars-profile-{}", sanitize_plugin_name(&profile.name));
    let home_dir = dirs::home_dir()
        .ok_or_else(|| ApplyError::Io("Could not determine home directory".to_string()))?;
    let marketplace_dir = profile_marketplace_dir(&home_dir);
    let plugins_dir = profile_marketplace_plugins_dir(&home_dir);

    fs::create_dir_all(&plugins_dir)
        .map_err(|e| ApplyError::Io(format!("Failed to create marketplace: {e}")))?;

    let target_plugin_dir = plugins_dir.join(&plugin_name);
    copy_plugin_to_claude_dir(&plugin_dir, &target_plugin_dir, &plugin_name)?;
    rebuild_profile_marketplace_manifest(&marketplace_dir)?;

    Ok(MarketplaceSyncResult {
        marketplace_path: marketplace_dir,
        plugin_id: plugin_name,
        plugin_path: target_plugin_dir,
    })
}

pub fn remove_profile_from_marketplace(profile_name: &str) -> Result<(), ApplyError> {
    let plugin_name = format!("tars-profile-{}", sanitize_plugin_name(profile_name));
    let home_dir = dirs::home_dir()
        .ok_or_else(|| ApplyError::Io("Could not determine home directory".to_string()))?;
    let marketplace_dir = profile_marketplace_dir(&home_dir);
    let plugins_dir = profile_marketplace_plugins_dir(&home_dir);
    let plugin_dir = plugins_dir.join(&plugin_name);

    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir)
            .map_err(|e| ApplyError::Io(format!("Failed to remove marketplace plugin: {e}")))?;
    }

    if marketplace_dir.exists() {
        rebuild_profile_marketplace_manifest(&marketplace_dir)?;
    }

    Ok(())
}

fn validate_plugin_root(path: &Path) -> Result<(), ApplyError> {
    let manifest_paths = [
        path.join(".claude-plugin").join("plugin.json"),
        path.join("plugin.json"),
    ];

    if manifest_paths.iter().any(|p| p.exists()) {
        Ok(())
    } else {
        Err(ApplyError::Io(format!(
            "Plugin manifest not found in {}",
            path.display()
        )))
    }
}

fn load_installed_plugins(
    home_dir: &Path,
    installed_plugins_path: &Path,
) -> Result<Value, ApplyError> {
    if !installed_plugins_path.exists() {
        return Ok(json!({
            "version": 2,
            "plugins": {}
        }));
    }

    let content = fs::read_to_string(installed_plugins_path)
        .map_err(|e| ApplyError::Io(format!("Failed to read installed_plugins.json: {e}")))?;
    let parsed: Value = serde_json::from_str(&content)
        .map_err(|e| ApplyError::Parse(format!("Failed to parse installed_plugins.json: {e}")))?;

    match parsed {
        Value::Array(entries) => migrate_legacy_installed_plugins(home_dir, entries),
        Value::Object(mut obj) => {
            if !obj.contains_key("version") {
                obj.insert("version".to_string(), json!(2));
            }
            match obj.get_mut("plugins") {
                Some(Value::Object(_)) => {}
                Some(_) => {
                    return Err(ApplyError::Parse(
                        "Invalid installed_plugins.json plugins format".to_string(),
                    ));
                }
                None => {
                    obj.insert("plugins".to_string(), Value::Object(serde_json::Map::new()));
                }
            }
            Ok(Value::Object(obj))
        }
        _ => Err(ApplyError::Parse(
            "Invalid installed_plugins.json format".to_string(),
        )),
    }
}

fn write_installed_plugins(path: &Path, content: &Value) -> Result<(), ApplyError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ApplyError::Io(format!("Failed to create plugins directory: {e}")))?;
    }
    let serialized = serde_json::to_string_pretty(content)
        .map_err(|e| ApplyError::Io(format!("Failed to serialize installed_plugins.json: {e}")))?;
    fs::write(path, serialized)
        .map_err(|e| ApplyError::Io(format!("Failed to write installed_plugins.json: {e}")))?;
    Ok(())
}

fn settings_path_for_scope(home_dir: &Path, project_path: Option<&str>) -> PathBuf {
    match project_path {
        Some(path) => PathBuf::from(path).join(".claude").join("settings.json"),
        None => home_dir.join(".claude").join("settings.json"),
    }
}

fn load_settings_json(path: &Path) -> Result<Value, ApplyError> {
    if !path.exists() {
        return Ok(json!({}));
    }

    let content = fs::read_to_string(path)
        .map_err(|e| ApplyError::Io(format!("Failed to read settings.json: {e}")))?;
    let parsed: Value = serde_json::from_str(&content)
        .map_err(|e| ApplyError::Parse(format!("Failed to parse settings.json: {e}")))?;

    match parsed {
        Value::Object(_) => Ok(parsed),
        _ => Err(ApplyError::Parse(
            "Invalid settings.json format".to_string(),
        )),
    }
}

fn write_settings_json(path: &Path, content: &Value) -> Result<(), ApplyError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ApplyError::Io(format!("Failed to create settings directory: {e}")))?;
    }
    let serialized = serde_json::to_string_pretty(content)
        .map_err(|e| ApplyError::Io(format!("Failed to serialize settings.json: {e}")))?;
    fs::write(path, serialized)
        .map_err(|e| ApplyError::Io(format!("Failed to write settings.json: {e}")))?;
    Ok(())
}

fn update_enabled_plugins(
    settings_path: &Path,
    plugin_key: &str,
    enabled: Option<bool>,
) -> Result<(), ApplyError> {
    if enabled.is_none() && !settings_path.exists() {
        return Ok(());
    }

    let mut settings = load_settings_json(settings_path)?;
    let settings_obj = settings
        .as_object_mut()
        .ok_or_else(|| ApplyError::Parse("Invalid settings.json format".to_string()))?;

    let enabled_plugins = settings_obj
        .entry("enabledPlugins".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let enabled_plugins = enabled_plugins.as_object_mut().ok_or_else(|| {
        ApplyError::Parse("Invalid settings.json enabledPlugins format".to_string())
    })?;

    match enabled {
        Some(value) => {
            enabled_plugins.insert(plugin_key.to_string(), json!(value));
        }
        None => {
            enabled_plugins.remove(plugin_key);
        }
    }

    write_settings_json(settings_path, &settings)?;
    Ok(())
}

fn plugin_key_matches_name(key: &str, plugin_name: &str) -> bool {
    key == plugin_name
        || (key.starts_with(plugin_name) && key.chars().nth(plugin_name.len()) == Some('@'))
}

#[allow(dead_code)]
fn resolve_plugin_key(plugins: &serde_json::Map<String, Value>, plugin_name: &str) -> String {
    plugins
        .keys()
        .find(|key| plugin_key_matches_name(key, plugin_name))
        .cloned()
        .unwrap_or_else(|| plugin_name.to_string())
}

fn install_matches_scope(install: &Value, project_path: Option<&str>) -> bool {
    let scope = install
        .get("scope")
        .and_then(|s| s.as_str())
        .unwrap_or("user");
    let install_project_path = install.get("projectPath").and_then(|p| p.as_str());

    match project_path {
        Some(path) => scope == "project" && install_project_path == Some(path),
        None => scope == "user" && install_project_path.is_none(),
    }
}

fn read_plugin_version(install_path: &Path) -> Option<String> {
    let candidate_paths = [
        install_path.join(".claude-plugin").join("plugin.json"),
        install_path.join("plugin.json"),
    ];

    for path in candidate_paths {
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path).ok()?;
        let json: Value = serde_json::from_str(&content).ok()?;
        if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
            return Some(version.to_string());
        }
    }

    None
}

fn migrate_legacy_installed_plugins(
    home_dir: &Path,
    entries: Vec<Value>,
) -> Result<Value, ApplyError> {
    let mut plugins = serde_json::Map::new();
    let default_base = home_dir.join(".claude").join("plugins");

    for entry in entries {
        let Some(name) = entry.get("name").and_then(|n| n.as_str()) else {
            continue;
        };

        let install_path = entry
            .get("installLocation")
            .and_then(|p| p.as_str())
            .map_or_else(|| default_base.join(name), PathBuf::from);
        let version = read_plugin_version(&install_path).unwrap_or_else(|| "unknown".to_string());

        let project_path = entry.get("projectPath").and_then(|p| p.as_str());
        let scope = if project_path.is_some() {
            "project"
        } else {
            "user"
        };

        let mut install = json!({
            "scope": scope,
            "installPath": install_path.to_string_lossy().to_string(),
            "version": version,
        });

        if let Some(path) = project_path {
            install["projectPath"] = json!(path);
        }

        let installs = plugins
            .entry(name.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(installs) = installs.as_array_mut() {
            installs.push(install);
        }
    }

    Ok(json!({
        "version": 2,
        "plugins": plugins
    }))
}

/// Register a plugin in `~/.claude/installed_plugins.json`
///
/// If `project_path` is Some, registers with project scope for that project.
/// If `project_path` is None, registers with user scope.
#[allow(dead_code)]
fn register_installed_plugin(
    home_dir: &Path,
    plugin_name: &str,
    project_path: Option<&str>,
    marketplace: Option<&str>,
    install_path: &Path,
) -> Result<(), ApplyError> {
    let installed_plugins_path = home_dir
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");

    let mut root = load_installed_plugins(home_dir, &installed_plugins_path)?;
    let plugins = root
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or_else(|| {
            ApplyError::Parse("Invalid installed_plugins.json plugins format".to_string())
        })?;

    let plugin_key = match marketplace {
        Some(marketplace) => format!("{plugin_name}@{marketplace}"),
        None => resolve_plugin_key(plugins, plugin_name),
    };
    let version = read_plugin_version(install_path).unwrap_or_else(|| "unknown".to_string());
    let scope = if project_path.is_some() {
        "project"
    } else {
        "user"
    };
    let now = Utc::now().to_rfc3339();

    let mut entry = json!({
        "scope": scope,
        "installPath": install_path.to_string_lossy().to_string(),
        "version": version,
        "installedAt": now,
        "lastUpdated": now,
    });

    if let Some(path) = project_path {
        entry["projectPath"] = json!(path);
    }

    let installs = plugins
        .entry(plugin_key.clone())
        .or_insert_with(|| Value::Array(Vec::new()));
    let installs = installs.as_array_mut().ok_or_else(|| {
        ApplyError::Parse("Invalid installed_plugins.json plugin entry format".to_string())
    })?;

    installs.retain(|install| !install_matches_scope(install, project_path));
    installs.push(entry);

    write_installed_plugins(&installed_plugins_path, &root)?;
    let settings_path = settings_path_for_scope(home_dir, project_path);
    update_enabled_plugins(&settings_path, &plugin_key, Some(true))?;
    Ok(())
}

/// Uninstall a profile plugin from user scope
///
/// Removes plugin directory from `~/.claude/plugins/` and unregisters from
/// `installed_plugins.json`.
///
/// # Errors
/// Returns an error if uninstall fails
pub fn uninstall_profile_plugin_from_user(plugin_id: &str) -> Result<(), ApplyError> {
    // Validate plugin_id to prevent directory traversal
    if !is_valid_plugin_id(plugin_id) {
        return Err(ApplyError::Io("Invalid plugin ID".to_string()));
    }

    let home_dir = dirs::home_dir()
        .ok_or_else(|| ApplyError::Io("Could not determine home directory".to_string()))?;

    // Remove from installed_plugins.json (user scope = no projectPath)
    unregister_installed_plugin(&home_dir, plugin_id, None)?;

    Ok(())
}

/// Uninstall a profile plugin from project scope
///
/// Removes plugin from `installed_plugins.json` for this project.
/// Note: Plugin files stay in `~/.claude/plugins/` as they may be used by other projects.
///
/// # Errors
/// Returns an error if uninstall fails
pub fn uninstall_profile_plugin_from_project(
    plugin_id: &str,
    project_path: &Path,
) -> Result<(), ApplyError> {
    // Validate plugin_id to prevent directory traversal
    if !is_valid_plugin_id(plugin_id) {
        return Err(ApplyError::Io("Invalid plugin ID".to_string()));
    }

    let home_dir = dirs::home_dir()
        .ok_or_else(|| ApplyError::Io("Could not determine home directory".to_string()))?;

    // Remove from installed_plugins.json for this project
    let project_path_str = project_path.to_string_lossy().to_string();
    unregister_installed_plugin(&home_dir, plugin_id, Some(&project_path_str))?;

    // Check if plugin is still used by any other scope
    // If not, remove the plugin directory
    if !is_plugin_still_installed(&home_dir, plugin_id)? {
        let plugin_dir = home_dir.join(".claude").join("plugins").join(plugin_id);
        if plugin_dir.exists() {
            fs::remove_dir_all(&plugin_dir)
                .map_err(|e| ApplyError::Io(format!("Failed to remove plugin: {e}")))?;
        }
    }

    Ok(())
}

/// Unregister a plugin from `~/.claude/installed_plugins.json`
fn unregister_installed_plugin(
    home_dir: &Path,
    plugin_name: &str,
    project_path: Option<&str>,
) -> Result<(), ApplyError> {
    let installed_plugins_path = home_dir
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");

    if !installed_plugins_path.exists() {
        return Ok(());
    }

    let mut root = load_installed_plugins(home_dir, &installed_plugins_path)?;
    let plugins = root
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or_else(|| {
            ApplyError::Parse("Invalid installed_plugins.json plugins format".to_string())
        })?;

    let plugin_keys: Vec<String> = plugins
        .keys()
        .filter(|key| plugin_key_matches_name(key, plugin_name))
        .cloned()
        .collect();

    if plugin_keys.is_empty() {
        return Ok(());
    }

    let mut removed_keys = Vec::new();
    for key in plugin_keys {
        let remove_key = if let Some(installs) = plugins.get_mut(&key) {
            let installs = installs.as_array_mut().ok_or_else(|| {
                ApplyError::Parse("Invalid installed_plugins.json plugin entry format".to_string())
            })?;
            let original_len = installs.len();
            installs.retain(|install| !install_matches_scope(install, project_path));
            if installs.len() < original_len {
                removed_keys.push(key.clone());
            }
            installs.is_empty()
        } else {
            false
        };

        if remove_key {
            plugins.remove(&key);
        }
    }

    write_installed_plugins(&installed_plugins_path, &root)?;
    let settings_path = settings_path_for_scope(home_dir, project_path);
    for key in removed_keys {
        update_enabled_plugins(&settings_path, &key, None)?;
    }
    Ok(())
}

/// Check if a plugin is still installed in any scope
fn is_plugin_still_installed(home_dir: &Path, plugin_name: &str) -> Result<bool, ApplyError> {
    let installed_plugins_path = home_dir
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");

    if !installed_plugins_path.exists() {
        return Ok(false);
    }

    let root = load_installed_plugins(home_dir, &installed_plugins_path)?;
    let plugins = root
        .get("plugins")
        .and_then(|p| p.as_object())
        .ok_or_else(|| {
            ApplyError::Parse("Invalid installed_plugins.json plugins format".to_string())
        })?;

    for (key, installs) in plugins {
        if !plugin_key_matches_name(key, plugin_name) {
            continue;
        }
        let installs = installs.as_array().ok_or_else(|| {
            ApplyError::Parse("Invalid installed_plugins.json plugin entry format".to_string())
        })?;
        if !installs.is_empty() {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::fs;
    use tempfile::TempDir;

    fn write_plugin_manifest(plugin_dir: &Path, version: &str) {
        let manifest_dir = plugin_dir.join(".claude-plugin");
        fs::create_dir_all(&manifest_dir).expect("create manifest dir");
        let manifest = json!({
            "name": "test-plugin",
            "version": version,
            "description": "test"
        });
        fs::write(
            manifest_dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
    }

    #[test]
    fn register_and_unregister_updates_installed_plugins_and_settings() {
        let temp_home = TempDir::new().expect("temp home");
        let home_dir = temp_home.path();
        let plugin_name = "tars-profile-test";
        let plugin_dir = home_dir.join(".claude").join("plugins").join(plugin_name);
        write_plugin_manifest(&plugin_dir, "1.0.0+test");

        register_installed_plugin(home_dir, plugin_name, None, None, &plugin_dir)
            .expect("register plugin");

        let installed_path = home_dir
            .join(".claude")
            .join("plugins")
            .join("installed_plugins.json");
        let installed_content =
            fs::read_to_string(&installed_path).expect("read installed_plugins");
        let installed_json: Value =
            serde_json::from_str(&installed_content).expect("parse installed_plugins");
        let plugins = installed_json
            .get("plugins")
            .and_then(|p| p.as_object())
            .expect("plugins map");
        let installs = plugins
            .get(plugin_name)
            .and_then(|p| p.as_array())
            .expect("plugin installs");
        assert_eq!(installs.len(), 1);
        assert_eq!(
            installs[0].get("scope").and_then(|v| v.as_str()),
            Some("user")
        );

        let settings_path = home_dir.join(".claude").join("settings.json");
        let settings_content = fs::read_to_string(&settings_path).expect("read settings.json");
        let settings_json: Value =
            serde_json::from_str(&settings_content).expect("parse settings.json");
        let enabled_plugins = settings_json
            .get("enabledPlugins")
            .and_then(|p| p.as_object())
            .expect("enabledPlugins map");
        assert_eq!(
            enabled_plugins.get(plugin_name).and_then(|v| v.as_bool()),
            Some(true)
        );

        unregister_installed_plugin(home_dir, plugin_name, None).expect("unregister plugin");

        let installed_content =
            fs::read_to_string(&installed_path).expect("read installed_plugins");
        let installed_json: Value =
            serde_json::from_str(&installed_content).expect("parse installed_plugins");
        let plugins = installed_json
            .get("plugins")
            .and_then(|p| p.as_object())
            .expect("plugins map");
        assert!(!plugins.contains_key(plugin_name));

        let settings_content = fs::read_to_string(&settings_path).expect("read settings.json");
        let settings_json: Value =
            serde_json::from_str(&settings_content).expect("parse settings.json");
        let enabled_plugins = settings_json
            .get("enabledPlugins")
            .and_then(|p| p.as_object());
        if let Some(enabled_plugins) = enabled_plugins {
            assert!(!enabled_plugins.contains_key(plugin_name));
        }
    }

    #[test]
    fn register_migrates_legacy_installed_plugins_format() {
        let temp_home = TempDir::new().expect("temp home");
        let home_dir = temp_home.path();
        let installed_path = home_dir
            .join(".claude")
            .join("plugins")
            .join("installed_plugins.json");
        fs::create_dir_all(installed_path.parent().expect("parent")).expect("create dir");
        let legacy = json!([{
            "name": "legacy-plugin",
            "installLocation": "/tmp/legacy-plugin"
        }]);
        fs::write(
            &installed_path,
            serde_json::to_string_pretty(&legacy).expect("serialize legacy"),
        )
        .expect("write legacy installed_plugins");

        let plugin_name = "tars-profile-test";
        let plugin_dir = home_dir.join(".claude").join("plugins").join(plugin_name);
        write_plugin_manifest(&plugin_dir, "1.0.0+test");

        register_installed_plugin(home_dir, plugin_name, None, None, &plugin_dir)
            .expect("register plugin");

        let installed_content =
            fs::read_to_string(&installed_path).expect("read installed_plugins");
        let installed_json: Value =
            serde_json::from_str(&installed_content).expect("parse installed_plugins");
        let plugins = installed_json
            .get("plugins")
            .and_then(|p| p.as_object())
            .expect("plugins map");
        assert!(plugins.contains_key("legacy-plugin"));
        assert!(plugins.contains_key(plugin_name));
    }
}
