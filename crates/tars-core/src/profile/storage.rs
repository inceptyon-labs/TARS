//! Profile file storage
//!
//! Handles storing and retrieving profile tool files in a central location.
//! Profile files are stored at `~/.tars/profiles/<profile-id>/`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Get the base directory for all profile storage
pub fn profiles_base_dir() -> Result<PathBuf, StorageError> {
    let home = dirs::home_dir().ok_or(StorageError::NoHomeDir)?;
    Ok(home.join(".tars").join("profiles"))
}

/// Get the directory for a specific profile's files
pub fn profile_dir(profile_id: Uuid) -> Result<PathBuf, StorageError> {
    Ok(profiles_base_dir()?.join(profile_id.to_string()))
}

/// Ensure a profile's storage directory exists
pub fn ensure_profile_dir(profile_id: Uuid) -> Result<PathBuf, StorageError> {
    let dir = profile_dir(profile_id)?;
    fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;
    Ok(dir)
}

/// Copy a skill from a source project to the profile's central storage
pub fn copy_skill_to_profile(
    profile_id: Uuid,
    skill_name: &str,
    source_skill_dir: &Path,
) -> Result<PathBuf, StorageError> {
    let profile_dir = ensure_profile_dir(profile_id)?;
    let skills_dir = profile_dir.join("skills");
    fs::create_dir_all(&skills_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_skill_dir = skills_dir.join(skill_name);

    // Remove existing if present
    if dest_skill_dir.exists() {
        fs::remove_dir_all(&dest_skill_dir).map_err(|e| StorageError::Io(e.to_string()))?;
    }

    // Copy the entire skill directory
    copy_dir_recursive(source_skill_dir, &dest_skill_dir)?;

    Ok(dest_skill_dir)
}

/// Copy an agent from a source project to the profile's central storage
pub fn copy_agent_to_profile(
    profile_id: Uuid,
    agent_name: &str,
    source_agent_file: &Path,
) -> Result<PathBuf, StorageError> {
    let profile_dir = ensure_profile_dir(profile_id)?;
    let agents_dir = profile_dir.join("agents");
    fs::create_dir_all(&agents_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = agents_dir.join(format!("{agent_name}.md"));
    fs::copy(source_agent_file, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(dest_file)
}

/// Copy a command from a source project to the profile's central storage
pub fn copy_command_to_profile(
    profile_id: Uuid,
    command_name: &str,
    source_command_file: &Path,
) -> Result<PathBuf, StorageError> {
    let profile_dir = ensure_profile_dir(profile_id)?;
    let commands_dir = profile_dir.join("commands");
    fs::create_dir_all(&commands_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = commands_dir.join(format!("{command_name}.md"));
    fs::copy(source_command_file, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(dest_file)
}

/// Store an MCP server config in the profile's central storage
pub fn store_mcp_server(
    profile_id: Uuid,
    server_name: &str,
    config: &serde_json::Value,
) -> Result<PathBuf, StorageError> {
    let profile_dir = ensure_profile_dir(profile_id)?;
    let mcp_dir = profile_dir.join("mcp-servers");
    fs::create_dir_all(&mcp_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = mcp_dir.join(format!("{server_name}.json"));
    let content =
        serde_json::to_string_pretty(config).map_err(|e| StorageError::Io(e.to_string()))?;
    fs::write(&dest_file, content).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(dest_file)
}

/// Copy a skill from profile storage to a target project
pub fn apply_skill_to_project(
    profile_id: Uuid,
    skill_name: &str,
    target_project: &Path,
) -> Result<(), StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let source_skill_dir = profile_dir.join("skills").join(skill_name);

    if !source_skill_dir.exists() {
        return Err(StorageError::NotFound(format!(
            "Skill '{skill_name}' not found in profile"
        )));
    }

    let target_skills_dir = target_project.join(".claude").join("skills");
    fs::create_dir_all(&target_skills_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_skill_dir = target_skills_dir.join(skill_name);

    // Remove existing if present
    if dest_skill_dir.exists() {
        fs::remove_dir_all(&dest_skill_dir).map_err(|e| StorageError::Io(e.to_string()))?;
    }

    copy_dir_recursive(&source_skill_dir, &dest_skill_dir)?;

    Ok(())
}

/// Copy an agent from profile storage to a target project
pub fn apply_agent_to_project(
    profile_id: Uuid,
    agent_name: &str,
    target_project: &Path,
) -> Result<(), StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let source_file = profile_dir.join("agents").join(format!("{agent_name}.md"));

    if !source_file.exists() {
        return Err(StorageError::NotFound(format!(
            "Agent '{agent_name}' not found in profile"
        )));
    }

    let target_agents_dir = target_project.join(".claude").join("agents");
    fs::create_dir_all(&target_agents_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = target_agents_dir.join(format!("{agent_name}.md"));
    fs::copy(&source_file, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(())
}

/// Copy a command from profile storage to a target project
pub fn apply_command_to_project(
    profile_id: Uuid,
    command_name: &str,
    target_project: &Path,
) -> Result<(), StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let source_file = profile_dir
        .join("commands")
        .join(format!("{command_name}.md"));

    if !source_file.exists() {
        return Err(StorageError::NotFound(format!(
            "Command '{command_name}' not found in profile"
        )));
    }

    let target_commands_dir = target_project.join(".claude").join("commands");
    fs::create_dir_all(&target_commands_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = target_commands_dir.join(format!("{command_name}.md"));
    fs::copy(&source_file, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(())
}

/// Get the stored MCP server config from profile storage
pub fn get_mcp_server_config(
    profile_id: Uuid,
    server_name: &str,
) -> Result<serde_json::Value, StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let source_file = profile_dir
        .join("mcp-servers")
        .join(format!("{server_name}.json"));

    if !source_file.exists() {
        return Err(StorageError::NotFound(format!(
            "MCP server '{server_name}' not found in profile"
        )));
    }

    let content = fs::read_to_string(&source_file).map_err(|e| StorageError::Io(e.to_string()))?;
    serde_json::from_str(&content).map_err(|e| StorageError::Io(e.to_string()))
}

// ============================================================================
// Plugin Manifest Storage
// ============================================================================

/// Store a plugin manifest in the profile's central storage
///
/// Plugin manifests are stored at `~/.tars/profiles/<profile-id>/plugins/<plugin-id>.json`
pub fn store_plugin_manifest(
    profile_id: Uuid,
    manifest: &PluginManifest,
) -> Result<PathBuf, StorageError> {
    let profile_dir = ensure_profile_dir(profile_id)?;
    let plugins_dir = profile_dir.join("plugins");
    fs::create_dir_all(&plugins_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    // Sanitize plugin ID for filename (replace @ and / with safe chars)
    let safe_name = manifest.id.replace('@', "").replace('/', "_");
    let dest_file = plugins_dir.join(format!("{safe_name}.json"));

    let content =
        serde_json::to_string_pretty(manifest).map_err(|e| StorageError::Io(e.to_string()))?;
    fs::write(&dest_file, content).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(dest_file)
}

/// Get a plugin manifest from profile storage
pub fn get_plugin_manifest(
    profile_id: Uuid,
    plugin_id: &str,
) -> Result<PluginManifest, StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let safe_name = plugin_id.replace('@', "").replace('/', "_");
    let source_file = profile_dir
        .join("plugins")
        .join(format!("{safe_name}.json"));

    if !source_file.exists() {
        return Err(StorageError::NotFound(format!(
            "Plugin '{plugin_id}' not found in profile"
        )));
    }

    let content = fs::read_to_string(&source_file).map_err(|e| StorageError::Io(e.to_string()))?;
    serde_json::from_str(&content).map_err(|e| StorageError::Io(e.to_string()))
}

/// List all plugin manifests stored in a profile
pub fn list_plugin_manifests(profile_id: Uuid) -> Result<Vec<PluginManifest>, StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let plugins_dir = profile_dir.join("plugins");

    let mut manifests = Vec::new();

    if plugins_dir.exists() {
        for entry in fs::read_dir(&plugins_dir).map_err(|e| StorageError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content =
                    fs::read_to_string(&path).map_err(|e| StorageError::Io(e.to_string()))?;
                if let Ok(manifest) = serde_json::from_str(&content) {
                    manifests.push(manifest);
                }
            }
        }
    }

    Ok(manifests)
}

/// Delete a plugin manifest from profile storage
pub fn delete_plugin_manifest(profile_id: Uuid, plugin_id: &str) -> Result<bool, StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let safe_name = plugin_id.replace('@', "").replace('/', "_");
    let file = profile_dir
        .join("plugins")
        .join(format!("{safe_name}.json"));

    if file.exists() {
        fs::remove_file(&file).map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// ============================================================================
// Project State Tracking
// ============================================================================

/// Get the base directory for project state tracking
pub fn projects_state_dir() -> Result<PathBuf, StorageError> {
    let home = dirs::home_dir().ok_or(StorageError::NoHomeDir)?;
    Ok(home.join(".tars").join("projects"))
}

/// Get the state file path for a specific project
pub fn project_state_path(project_id: Uuid) -> Result<PathBuf, StorageError> {
    Ok(projects_state_dir()?
        .join(project_id.to_string())
        .join("profile_state.json"))
}

/// State tracking for a project's profile assignment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectProfileState {
    /// Currently assigned profile ID
    pub profile_id: Option<Uuid>,
    /// Plugins that were installed by TARS (for cleanup on unassign)
    #[serde(default)]
    pub plugins_installed_by_tars: Vec<String>,
    /// When the profile was assigned
    pub assigned_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last time state was updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ProjectProfileState {
    /// Create a new project state with a profile assignment
    pub fn new(profile_id: Uuid) -> Self {
        let now = chrono::Utc::now();
        Self {
            profile_id: Some(profile_id),
            plugins_installed_by_tars: Vec::new(),
            assigned_at: Some(now),
            updated_at: now,
        }
    }

    /// Add a plugin that was installed by TARS
    pub fn add_installed_plugin(&mut self, plugin_id: String) {
        if !self.plugins_installed_by_tars.contains(&plugin_id) {
            self.plugins_installed_by_tars.push(plugin_id);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Clear the profile assignment
    pub fn clear_profile(&mut self) {
        self.profile_id = None;
        self.assigned_at = None;
        self.updated_at = chrono::Utc::now();
    }
}

/// Load project profile state from disk
pub fn load_project_state(project_id: Uuid) -> Result<Option<ProjectProfileState>, StorageError> {
    let path = project_state_path(project_id)?;

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path).map_err(|e| StorageError::Io(e.to_string()))?;
    let state = serde_json::from_str(&content).map_err(|e| StorageError::Io(e.to_string()))?;
    Ok(Some(state))
}

/// Save project profile state to disk
pub fn save_project_state(
    project_id: Uuid,
    state: &ProjectProfileState,
) -> Result<(), StorageError> {
    let path = project_state_path(project_id)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| StorageError::Io(e.to_string()))?;
    }

    let content =
        serde_json::to_string_pretty(state).map_err(|e| StorageError::Io(e.to_string()))?;
    fs::write(&path, content).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(())
}

/// Delete project profile state from disk
pub fn delete_project_state(project_id: Uuid) -> Result<(), StorageError> {
    let path = project_state_path(project_id)?;

    if path.exists() {
        fs::remove_file(&path).map_err(|e| StorageError::Io(e.to_string()))?;
    }

    // Also try to remove the project directory if empty
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir(parent); // Ignore error if not empty
    }

    Ok(())
}

/// List all tools stored in a profile
pub fn list_profile_tools(profile_id: Uuid) -> Result<ProfileTools, StorageError> {
    let profile_dir = profile_dir(profile_id)?;

    let mut tools = ProfileTools::default();

    // List MCP servers
    let mcp_dir = profile_dir.join("mcp-servers");
    if mcp_dir.exists() {
        for entry in fs::read_dir(&mcp_dir).map_err(|e| StorageError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            if let Some(name) = entry.path().file_stem() {
                tools.mcp_servers.push(name.to_string_lossy().to_string());
            }
        }
    }

    // List skills
    let skills_dir = profile_dir.join("skills");
    if skills_dir.exists() {
        for entry in fs::read_dir(&skills_dir).map_err(|e| StorageError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            if entry.path().is_dir() {
                tools
                    .skills
                    .push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }

    // List agents
    let agents_dir = profile_dir.join("agents");
    if agents_dir.exists() {
        for entry in fs::read_dir(&agents_dir).map_err(|e| StorageError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            if let Some(name) = entry.path().file_stem() {
                tools.agents.push(name.to_string_lossy().to_string());
            }
        }
    }

    // List commands
    let commands_dir = profile_dir.join("commands");
    if commands_dir.exists() {
        for entry in fs::read_dir(&commands_dir).map_err(|e| StorageError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            if let Some(name) = entry.path().file_stem() {
                tools.commands.push(name.to_string_lossy().to_string());
            }
        }
    }

    // List plugins (read manifest files to get original plugin IDs)
    let plugins_dir = profile_dir.join("plugins");
    if plugins_dir.exists() {
        for entry in fs::read_dir(&plugins_dir).map_err(|e| StorageError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                // Try to read the manifest to get the original plugin ID
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                        tools.plugins.push(manifest.id);
                    }
                }
            }
        }
    }

    Ok(tools)
}

/// Delete a profile's storage directory
pub fn delete_profile_storage(profile_id: Uuid) -> Result<(), StorageError> {
    let dir = profile_dir(profile_id)?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;
    }
    Ok(())
}

/// Tools stored in a profile
#[derive(Debug, Clone, Default)]
pub struct ProfileTools {
    pub mcp_servers: Vec<String>,
    pub skills: Vec<String>,
    pub agents: Vec<String>,
    pub commands: Vec<String>,
    pub plugins: Vec<String>,
}

/// Plugin manifest stored in a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin identifier (e.g., "@anthropic/claude-code-plugin")
    pub id: String,
    /// Marketplace it comes from (e.g., "official" or custom marketplace name)
    pub marketplace: Option<String>,
    /// Version at time of addition
    pub version: Option<String>,
    /// Whether the plugin should be enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// When this plugin was added to the profile
    pub added_at: chrono::DateTime<chrono::Utc>,
}

fn default_true() -> bool {
    true
}

impl PluginManifest {
    /// Create a new plugin manifest
    pub fn new(id: String, marketplace: Option<String>, version: Option<String>) -> Self {
        Self {
            id,
            marketplace,
            version,
            enabled: true,
            added_at: chrono::Utc::now(),
        }
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), StorageError> {
    fs::create_dir_all(dst).map_err(|e| StorageError::Io(e.to_string()))?;

    for entry in fs::read_dir(src).map_err(|e| StorageError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| StorageError::Io(e.to_string()))?;
        }
    }

    Ok(())
}

/// Storage error types
#[derive(Debug, Clone)]
pub enum StorageError {
    /// Home directory not found
    NoHomeDir,
    /// IO error
    Io(String),
    /// Resource not found
    NotFound(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::NoHomeDir => write!(f, "Home directory not found"),
            StorageError::Io(msg) => write!(f, "IO error: {msg}"),
            StorageError::NotFound(msg) => write!(f, "Not found: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}
