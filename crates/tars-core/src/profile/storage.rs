//! Profile file storage
//!
//! Handles storing and retrieving profile tool files in a central location.
//! Profile files are stored at `~/.tars/profiles/<profile-id>/`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

// ============================================================================
// Constants for Safety Limits
// ============================================================================

/// Maximum directory depth for recursive operations
const MAX_DEPTH: usize = 50;

/// Maximum number of files to process in a directory
const MAX_FILES: usize = 10_000;

/// Maximum file size to read entirely into memory (10 MB)
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

// ============================================================================
// Hash Utilities for Source Tracking
// ============================================================================

/// Compute SHA256 hash of a single file's contents
pub fn compute_file_hash(path: &Path) -> Result<String, StorageError> {
    let metadata = fs::metadata(path).map_err(|e| {
        StorageError::Io(format!(
            "Failed to read metadata for {}: {e}",
            path.display()
        ))
    })?;

    // For large files, stream the content
    if metadata.len() > MAX_FILE_SIZE {
        return compute_file_hash_streaming(path);
    }

    let content = fs::read(path)
        .map_err(|e| StorageError::Io(format!("Failed to read {}: {e}", path.display())))?;
    let hash = Sha256::digest(&content);
    Ok(format!("{hash:x}"))
}

/// Compute hash by streaming file contents (for large files)
fn compute_file_hash_streaming(path: &Path) -> Result<String, StorageError> {
    let mut file = fs::File::open(path)
        .map_err(|e| StorageError::Io(format!("Failed to open {}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| StorageError::Io(format!("Failed to read {}: {e}", path.display())))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute SHA256 hash of a directory's contents
///
/// Hashes are computed from all files in sorted order for consistency.
pub fn compute_dir_hash(dir: &Path) -> Result<String, StorageError> {
    let mut hasher = Sha256::new();
    let mut files: Vec<PathBuf> = Vec::new();

    // Collect all files recursively with depth and file limits
    collect_files_safe(dir, &mut files, 0)?;

    // Sort for consistent ordering
    files.sort();

    // Hash each file's path (relative) and contents
    for file in files {
        if let Ok(relative) = file.strip_prefix(dir) {
            hasher.update(relative.to_string_lossy().as_bytes());
        }

        // Stream large files
        let metadata = fs::metadata(&file).map_err(|e| {
            StorageError::Io(format!(
                "Failed to read metadata for {}: {e}",
                file.display()
            ))
        })?;

        if metadata.len() > MAX_FILE_SIZE {
            let file_hash = compute_file_hash_streaming(&file)?;
            hasher.update(file_hash.as_bytes());
        } else {
            let content = fs::read(&file)
                .map_err(|e| StorageError::Io(format!("Failed to read {}: {e}", file.display())))?;
            hasher.update(&content);
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Collect all files in a directory recursively with safety limits
fn collect_files_safe(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), StorageError> {
    // Check depth limit to prevent stack overflow
    if depth > MAX_DEPTH {
        return Err(StorageError::Io(format!(
            "Directory depth exceeds maximum of {MAX_DEPTH}: {}",
            dir.display()
        )));
    }

    // Check file count limit to prevent memory exhaustion
    if files.len() > MAX_FILES {
        return Err(StorageError::Io(format!(
            "File count exceeds maximum of {MAX_FILES}"
        )));
    }

    if !dir.exists() {
        return Ok(());
    }

    // Skip symlinks to prevent loops
    let metadata = fs::symlink_metadata(dir).map_err(|e| {
        StorageError::Io(format!(
            "Failed to read metadata for {}: {e}",
            dir.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)
        .map_err(|e| StorageError::Io(format!("Failed to read directory {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| {
            StorageError::Io(format!("Failed to read entry in {}: {e}", dir.display()))
        })?;
        let path = entry.path();

        // Skip symlinks
        let entry_metadata = fs::symlink_metadata(&path).map_err(|e| {
            StorageError::Io(format!(
                "Failed to read metadata for {}: {e}",
                path.display()
            ))
        })?;
        if entry_metadata.file_type().is_symlink() {
            continue;
        }

        if path.is_dir() {
            collect_files_safe(&path, files, depth + 1)?;
        } else {
            files.push(path);
        }
    }

    Ok(())
}

/// Compute a hash of all content in a profile's storage
///
/// This is used to generate unique version strings for plugins.
pub fn compute_profile_content_hash(profile_id: Uuid) -> Result<String, StorageError> {
    let dir = profile_dir(profile_id)?;
    if !dir.exists() {
        return Ok("empty".to_string());
    }
    compute_dir_hash(&dir)
}

// ============================================================================
// Profile Directory Management
// ============================================================================

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
    // Sanitize the skill name to prevent path traversal
    let safe_name = sanitize_tool_name(skill_name)?;

    let profile_dir = ensure_profile_dir(profile_id)?;
    let skills_dir = profile_dir.join("skills");
    fs::create_dir_all(&skills_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_skill_dir = skills_dir.join(&safe_name);

    // Remove existing if present
    if dest_skill_dir.exists() {
        if dest_skill_dir.is_dir() {
            fs::remove_dir_all(&dest_skill_dir).map_err(|e| StorageError::Io(e.to_string()))?;
        } else {
            fs::remove_file(&dest_skill_dir).map_err(|e| StorageError::Io(e.to_string()))?;
        }
    }

    let mut source_dir = source_skill_dir.to_path_buf();
    if source_dir.is_file()
        && source_dir
            .file_name()
            .is_some_and(|name| name == "SKILL.md")
    {
        if let Some(parent) = source_dir.parent() {
            source_dir = parent.to_path_buf();
        }
    }

    if source_dir.is_file() {
        fs::create_dir_all(&dest_skill_dir).map_err(|e| StorageError::Io(e.to_string()))?;
        let dest_file = dest_skill_dir.join("SKILL.md");
        fs::copy(&source_dir, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;
    } else {
        // Copy the entire skill directory
        copy_dir_recursive(&source_dir, &dest_skill_dir)?;
    }

    Ok(dest_skill_dir)
}

/// Copy an agent from a source project to the profile's central storage
pub fn copy_agent_to_profile(
    profile_id: Uuid,
    agent_name: &str,
    source_agent_file: &Path,
) -> Result<PathBuf, StorageError> {
    // Sanitize the agent name to prevent path traversal
    let safe_name = sanitize_tool_name(agent_name)?;

    let profile_dir = ensure_profile_dir(profile_id)?;
    let agents_dir = profile_dir.join("agents");
    fs::create_dir_all(&agents_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = agents_dir.join(format!("{safe_name}.md"));
    fs::copy(source_agent_file, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(dest_file)
}

/// Copy a command from a source project to the profile's central storage
pub fn copy_command_to_profile(
    profile_id: Uuid,
    command_name: &str,
    source_command_file: &Path,
) -> Result<PathBuf, StorageError> {
    // Sanitize the command name to prevent path traversal
    let safe_name = sanitize_tool_name(command_name)?;

    let profile_dir = ensure_profile_dir(profile_id)?;
    let commands_dir = profile_dir.join("commands");
    fs::create_dir_all(&commands_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = commands_dir.join(format!("{safe_name}.md"));
    fs::copy(source_command_file, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(dest_file)
}

/// Store an MCP server config in the profile's central storage
pub fn store_mcp_server(
    profile_id: Uuid,
    server_name: &str,
    config: &serde_json::Value,
) -> Result<PathBuf, StorageError> {
    // Sanitize the server name to prevent path traversal
    let safe_name = sanitize_tool_name(server_name)?;

    let profile_dir = ensure_profile_dir(profile_id)?;
    let mcp_dir = profile_dir.join("mcp-servers");
    fs::create_dir_all(&mcp_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = mcp_dir.join(format!("{safe_name}.json"));
    let content =
        serde_json::to_string_pretty(config).map_err(|e| StorageError::Io(e.to_string()))?;
    fs::write(&dest_file, content).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(dest_file)
}

/// Delete an MCP server config from a profile's storage
pub fn delete_mcp_server(profile_id: Uuid, server_name: &str) -> Result<bool, StorageError> {
    let safe_name = sanitize_tool_name(server_name)?;
    let profile_dir = profile_dir(profile_id)?;
    let file_path = profile_dir
        .join("mcp-servers")
        .join(format!("{safe_name}.json"));

    if !file_path.exists() {
        return Ok(false);
    }

    if file_path.is_dir() {
        fs::remove_dir_all(&file_path).map_err(|e| StorageError::Io(e.to_string()))?;
    } else {
        fs::remove_file(&file_path).map_err(|e| StorageError::Io(e.to_string()))?;
    }

    Ok(true)
}

/// Copy a skill from profile storage to a target project
pub fn apply_skill_to_project(
    profile_id: Uuid,
    skill_name: &str,
    target_project: &Path,
) -> Result<(), StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let safe_name = sanitize_tool_name(skill_name)?;
    let source_skill_dir = profile_dir.join("skills").join(&safe_name);

    if !source_skill_dir.exists() {
        return Err(StorageError::NotFound(format!(
            "Skill '{skill_name}' not found in profile"
        )));
    }

    let target_skills_dir = target_project.join(".claude").join("skills");
    fs::create_dir_all(&target_skills_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_skill_dir = target_skills_dir.join(&safe_name);

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
    let safe_name = sanitize_tool_name(agent_name)?;
    let source_file = profile_dir.join("agents").join(format!("{safe_name}.md"));

    if !source_file.exists() {
        return Err(StorageError::NotFound(format!(
            "Agent '{agent_name}' not found in profile"
        )));
    }

    let target_agents_dir = target_project.join(".claude").join("agents");
    fs::create_dir_all(&target_agents_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = target_agents_dir.join(format!("{safe_name}.md"));
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
    let safe_name = sanitize_tool_name(command_name)?;
    let source_file = profile_dir.join("commands").join(format!("{safe_name}.md"));

    if !source_file.exists() {
        return Err(StorageError::NotFound(format!(
            "Command '{command_name}' not found in profile"
        )));
    }

    let target_commands_dir = target_project.join(".claude").join("commands");
    fs::create_dir_all(&target_commands_dir).map_err(|e| StorageError::Io(e.to_string()))?;

    let dest_file = target_commands_dir.join(format!("{safe_name}.md"));
    fs::copy(&source_file, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;

    Ok(())
}

/// Get the stored MCP server config from profile storage
pub fn get_mcp_server_config(
    profile_id: Uuid,
    server_name: &str,
) -> Result<serde_json::Value, StorageError> {
    let safe_name = sanitize_tool_name(server_name)?;
    let profile_dir = profile_dir(profile_id)?;
    let source_file = profile_dir
        .join("mcp-servers")
        .join(format!("{safe_name}.json"));

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

/// Sanitize a plugin ID for safe filesystem usage
///
/// This prevents path traversal attacks and ensures the ID can be used as a filename.
fn sanitize_plugin_id(id: &str) -> Result<String, StorageError> {
    // Check for dangerous patterns
    if id.is_empty() {
        return Err(StorageError::Io("Plugin ID cannot be empty".to_string()));
    }

    if id.len() > 256 {
        return Err(StorageError::Io(
            "Plugin ID too long (max 256 chars)".to_string(),
        ));
    }

    if id.contains("..") {
        return Err(StorageError::Io(
            "Plugin ID contains path traversal sequence".to_string(),
        ));
    }

    if id.contains('\0') {
        return Err(StorageError::Io("Plugin ID contains null byte".to_string()));
    }

    // Build safe filename: keep alphanumeric and hyphens, replace others
    let safe: String = id
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' {
                Some(c.to_ascii_lowercase())
            } else if c == '@' || c == '/' || c == '_' || c == ' ' {
                Some('_')
            } else {
                None
            }
        })
        .collect();

    if safe.is_empty() {
        return Err(StorageError::Io(
            "Plugin ID contains no valid characters".to_string(),
        ));
    }

    Ok(safe)
}

/// Store a plugin manifest in the profile's central storage
///
/// Plugin manifests are stored at `~/.tars/profiles/<profile-id>/plugins/<plugin-id>.json`
pub fn store_plugin_manifest(
    profile_id: Uuid,
    manifest: &PluginManifest,
) -> Result<PathBuf, StorageError> {
    let profile_dir = ensure_profile_dir(profile_id)?;
    let plugins_dir = profile_dir.join("plugins");
    fs::create_dir_all(&plugins_dir)
        .map_err(|e| StorageError::Io(format!("Failed to create plugins directory: {e}")))?;

    // Sanitize plugin ID for filename
    let safe_name = sanitize_plugin_id(&manifest.id)?;
    let dest_file = plugins_dir.join(format!("{safe_name}.json"));

    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| StorageError::Io(format!("Failed to serialize manifest: {e}")))?;
    fs::write(&dest_file, &content)
        .map_err(|e| StorageError::Io(format!("Failed to write plugin manifest: {e}")))?;

    Ok(dest_file)
}

/// Get a plugin manifest from profile storage
pub fn get_plugin_manifest(
    profile_id: Uuid,
    plugin_id: &str,
) -> Result<PluginManifest, StorageError> {
    let profile_dir = profile_dir(profile_id)?;
    let safe_name = sanitize_plugin_id(plugin_id)?;
    let source_file = profile_dir
        .join("plugins")
        .join(format!("{safe_name}.json"));

    if !source_file.exists() {
        return Err(StorageError::NotFound(format!(
            "Plugin '{plugin_id}' not found in profile"
        )));
    }

    let content = fs::read_to_string(&source_file)
        .map_err(|e| StorageError::Io(format!("Failed to read plugin manifest: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| StorageError::Io(format!("Failed to parse plugin manifest: {e}")))
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
    let safe_name = sanitize_plugin_id(plugin_id)?;
    let file = profile_dir
        .join("plugins")
        .join(format!("{safe_name}.json"));

    if file.exists() {
        fs::remove_file(&file)
            .map_err(|e| StorageError::Io(format!("Failed to delete plugin manifest: {e}")))?;
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

/// Get the plugin output directory for a profile
///
/// This is where the pre-generated plugin is stored: `~/.tars/profiles/<id>/plugin/`
pub fn profile_plugin_dir(profile_id: Uuid) -> Result<PathBuf, StorageError> {
    Ok(profile_dir(profile_id)?.join("plugin"))
}

/// Ensure the plugin directory exists for a profile
pub fn ensure_plugin_dir(profile_id: Uuid) -> Result<PathBuf, StorageError> {
    let dir = profile_plugin_dir(profile_id)?;
    fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;
    Ok(dir)
}

/// Clear the plugin directory for a profile (before regenerating)
pub fn clear_plugin_dir(profile_id: Uuid) -> Result<(), StorageError> {
    let dir = profile_plugin_dir(profile_id)?;
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

/// Recursively copy a directory with security protections
///
/// This function includes:
/// - Symlink skipping to prevent loops and directory escape
/// - Depth limiting to prevent stack overflow
/// - File count limiting to prevent resource exhaustion
/// - Path validation to prevent path traversal
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), StorageError> {
    copy_dir_recursive_impl(src, dst, dst, 0, &mut 0)
}

/// Internal implementation with tracking parameters
fn copy_dir_recursive_impl(
    src: &Path,
    dst: &Path,
    dst_root: &Path,
    depth: usize,
    file_count: &mut usize,
) -> Result<(), StorageError> {
    // Check depth limit
    if depth > MAX_DEPTH {
        return Err(StorageError::Io(format!(
            "Directory depth exceeds maximum of {MAX_DEPTH}"
        )));
    }

    // Check file count limit
    if *file_count > MAX_FILES {
        return Err(StorageError::Io(format!(
            "File count exceeds maximum of {MAX_FILES}"
        )));
    }

    // Skip if source doesn't exist
    if !src.exists() {
        return Ok(());
    }

    // Skip symlinks to prevent loops and directory escape
    let src_metadata = fs::symlink_metadata(src).map_err(|e| {
        StorageError::Io(format!(
            "Failed to read metadata for {}: {e}",
            src.display()
        ))
    })?;
    if src_metadata.file_type().is_symlink() {
        return Ok(());
    }

    fs::create_dir_all(dst).map_err(|e| StorageError::Io(e.to_string()))?;

    for entry in fs::read_dir(src).map_err(|e| StorageError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // Skip symlinks
        let entry_metadata = fs::symlink_metadata(&src_path).map_err(|e| {
            StorageError::Io(format!(
                "Failed to read metadata for {}: {e}",
                src_path.display()
            ))
        })?;
        if entry_metadata.file_type().is_symlink() {
            continue;
        }

        // Validate destination path stays within dst_root (prevent path traversal)
        let canonical_dst = if dst_path.exists() {
            dst_path.canonicalize().map_err(|e| {
                StorageError::Io(format!(
                    "Failed to canonicalize {}: {e}",
                    dst_path.display()
                ))
            })?
        } else {
            // For new files, check the parent exists and is valid
            if let Some(parent) = dst_path.parent() {
                if parent.exists() {
                    let canonical_parent = parent.canonicalize().map_err(|e| {
                        StorageError::Io(format!(
                            "Failed to canonicalize parent {}: {e}",
                            parent.display()
                        ))
                    })?;
                    canonical_parent.join(dst_path.file_name().unwrap_or_default())
                } else {
                    dst_path.clone()
                }
            } else {
                dst_path.clone()
            }
        };

        // Ensure the destination is under the root
        let canonical_root = if dst_root.exists() {
            dst_root
                .canonicalize()
                .unwrap_or_else(|_| dst_root.to_path_buf())
        } else {
            dst_root.to_path_buf()
        };

        if !canonical_dst.starts_with(&canonical_root) {
            return Err(StorageError::Io(format!(
                "Path traversal detected: {} escapes {}",
                canonical_dst.display(),
                canonical_root.display()
            )));
        }

        *file_count += 1;

        if src_path.is_dir() {
            copy_dir_recursive_impl(&src_path, &dst_path, dst_root, depth + 1, file_count)?;
        } else {
            // Check file size before copying
            if entry_metadata.len() > MAX_FILE_SIZE {
                return Err(StorageError::Io(format!(
                    "File {} exceeds maximum size of {} bytes",
                    src_path.display(),
                    MAX_FILE_SIZE
                )));
            }
            fs::copy(&src_path, &dst_path).map_err(|e| StorageError::Io(e.to_string()))?;
        }
    }

    Ok(())
}

/// Sanitize a tool name (skill, agent, command) for safe filesystem usage
///
/// Prevents path traversal and ensures the name can be used as a filename.
pub fn sanitize_tool_name(name: &str) -> Result<String, StorageError> {
    // Check for empty names
    if name.is_empty() {
        return Err(StorageError::Io("Tool name cannot be empty".to_string()));
    }

    // Check length limit
    if name.len() > 256 {
        return Err(StorageError::Io(
            "Tool name too long (max 256 chars)".to_string(),
        ));
    }

    // Check for path traversal patterns
    if name.contains("..") {
        return Err(StorageError::Io(
            "Tool name contains path traversal sequence".to_string(),
        ));
    }

    // Check for null bytes
    if name.contains('\0') {
        return Err(StorageError::Io("Tool name contains null byte".to_string()));
    }

    // Check for path separators
    if name.contains('/') || name.contains('\\') {
        return Err(StorageError::Io(
            "Tool name contains path separators".to_string(),
        ));
    }

    // Build safe name: keep alphanumeric, hyphens, underscores
    let safe: String = name
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                Some(c)
            } else if c == ' ' {
                Some('-')
            } else {
                None
            }
        })
        .collect();

    if safe.is_empty() {
        return Err(StorageError::Io(
            "Tool name contains no valid characters".to_string(),
        ));
    }

    Ok(safe)
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
