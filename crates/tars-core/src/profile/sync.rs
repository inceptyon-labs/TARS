//! Profile sync operations
//!
//! This module handles syncing profile changes to all assigned projects.

use crate::profile::types::Profile;
use crate::project::Project;
use crate::storage::db::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
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
