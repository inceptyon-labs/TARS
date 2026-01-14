//! Profile to plugin conversion

use crate::profile::storage::{
    compute_profile_content_hash, copy_dir_recursive, get_mcp_server_config, profile_dir,
    sanitize_tool_name, StorageError,
};
use crate::profile::{Profile, ToolType};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use thiserror::Error;

use super::archive::create_archive;
use super::manifest::ExportManifest;
use super::structure::{create_plugin_structure, StructureError};

/// Errors during export
#[derive(Error, Debug)]
pub enum ExportError {
    #[error("Structure error: {0}")]
    Structure(#[from] StructureError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Archive error: {0}")]
    Archive(#[from] super::archive::ArchiveError),
}

impl From<StorageError> for ExportError {
    fn from(e: StorageError) -> Self {
        ExportError::Storage(e.to_string())
    }
}

/// Export a profile as a Claude Code plugin
///
/// Creates a complete plugin directory structure with all tools from the profile's
/// central storage. The plugin can be installed via `claude plugin install`.
///
/// # Errors
/// Returns an error if export fails
pub fn export_as_plugin(
    profile: &Profile,
    output_dir: &Path,
    plugin_name: &str,
    version: &str,
) -> Result<PathBuf, ExportError> {
    // Create directory structure
    create_plugin_structure(output_dir)?;

    let plugin_dir = output_dir.join(".claude-plugin");
    let profile_storage = profile_dir(profile.id)?;

    // Copy skills from profile storage
    for tool in profile
        .tool_refs
        .iter()
        .filter(|t| t.tool_type == ToolType::Skill)
    {
        let safe_name = sanitize_tool_name(&tool.name)?;
        let src = profile_storage.join("skills").join(&safe_name);
        if src.exists() {
            let dst = output_dir.join("skills").join(&safe_name);
            copy_dir_recursive(&src, &dst)?;
        }
    }

    // Copy agents from profile storage
    for tool in profile
        .tool_refs
        .iter()
        .filter(|t| t.tool_type == ToolType::Agent)
    {
        let safe_name = sanitize_tool_name(&tool.name)?;
        let src = profile_storage
            .join("agents")
            .join(format!("{safe_name}.md"));
        if src.exists() {
            let dst = output_dir.join("agents").join(format!("{safe_name}.md"));
            fs::copy(&src, &dst)?;
        }
    }

    // Copy commands from profile storage
    // Commands go in the commands/ directory at the plugin root
    for tool in profile
        .tool_refs
        .iter()
        .filter(|t| t.tool_type == ToolType::Hook)
    {
        // In Claude Code, "commands" and "hooks" are related
        // Commands are the MD files, hooks are in settings
        let safe_name = sanitize_tool_name(&tool.name)?;
        let src = profile_storage
            .join("commands")
            .join(format!("{safe_name}.md"));
        if src.exists() {
            let dst = output_dir.join("commands").join(format!("{safe_name}.md"));
            fs::copy(&src, &dst)?;
        }
    }

    // Copy hooks configuration if present
    let hooks_path = profile_storage.join("hooks.json");
    if hooks_path.exists() {
        let dest_path = output_dir.join("hooks.json");
        fs::copy(&hooks_path, &dest_path)?;
    }

    // Generate MCP config in FLAT format (no mcpServers wrapper)
    // Plugin .mcp.json format: { "server-name": { "command": "...", "args": [...] } }
    let mcp_tools: Vec<_> = profile
        .tool_refs
        .iter()
        .filter(|t| t.tool_type == ToolType::Mcp)
        .collect();

    if !mcp_tools.is_empty() {
        let mut mcp_config = serde_json::Map::new();

        for tool in mcp_tools {
            if let Ok(config) = get_mcp_server_config(profile.id, &tool.name) {
                mcp_config.insert(tool.name.clone(), config);
            }
        }

        if !mcp_config.is_empty() {
            let mcp_path = output_dir.join(".mcp.json");
            let mcp_json = serde_json::to_string_pretty(&serde_json::Value::Object(mcp_config))?;
            fs::write(mcp_path, mcp_json)?;
        }
    }

    // Create manifest with correct paths
    let manifest = ExportManifest::new(
        plugin_name.to_string(),
        version.to_string(),
        profile.description.clone().unwrap_or_default(),
    );

    // Write manifest
    let manifest_path = plugin_dir.join("plugin.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(manifest_path, manifest_json)?;

    Ok(plugin_dir)
}

/// Generate a plugin from a profile with content-hash versioning
///
/// The version will be in format: `1.0.0+<hash>` where hash is the first 16 chars
/// of the SHA256 hash of all profile content (16 chars for better uniqueness).
///
/// # Errors
/// Returns an error if export fails
pub fn export_as_plugin_with_hash(
    profile: &Profile,
    output_dir: &Path,
    plugin_name: &str,
) -> Result<PathBuf, ExportError> {
    let content_hash = compute_profile_content_hash(profile.id)?;
    // Use 16 chars of hash for better uniqueness (reduces collision probability)
    let version = format!("1.0.0+{}", &content_hash[..16.min(content_hash.len())]);
    export_as_plugin(profile, output_dir, plugin_name, &version)
}

/// Export a profile as a ZIP archive containing a Claude Code plugin
///
/// Creates the plugin structure in a temporary directory, then zips it to `output_path`.
///
/// # Errors
/// Returns an error if export or archive creation fails
pub fn export_as_plugin_zip(
    profile: &Profile,
    output_path: &Path,
    plugin_name: &str,
    version: &str,
) -> Result<PathBuf, ExportError> {
    let temp_dir = TempDir::new()?;
    let plugin_root = temp_dir.path().join(plugin_name);
    fs::create_dir_all(&plugin_root)?;

    export_as_plugin(profile, &plugin_root, plugin_name, version)?;
    create_archive(&plugin_root, output_path)?;

    Ok(output_path.to_path_buf())
}
