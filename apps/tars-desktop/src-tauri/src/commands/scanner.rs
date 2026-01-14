//! Scanner Tauri commands
//!
//! Commands for scanning Claude Code configuration.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tars_core::storage::ProfileStore;
use tars_scanner::artifacts::{AgentInfo, CommandInfo, SkillInfo};
use tars_scanner::scope::user::{
    scan_agents_directory, scan_commands_directory, scan_skills_directory,
};
use tars_scanner::types::Scope;
use tars_scanner::{Inventory, Scanner};
use tauri::State;

/// Scan a project directory for Claude Code configuration
#[tauri::command]
pub async fn scan_project(path: String, _state: State<'_, AppState>) -> Result<Inventory, String> {
    let project_path = PathBuf::from(&path);

    if !project_path.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    let scanner = Scanner::new();
    let project_paths = vec![project_path.as_path()];

    scanner
        .scan_all(&project_paths)
        .map_err(|e| format!("Scan failed: {e}"))
}

/// Scan only user-level configuration
#[tauri::command]
pub async fn scan_user_scope(_state: State<'_, AppState>) -> Result<Inventory, String> {
    let scanner = Scanner::new();

    scanner
        .scan_all(&[])
        .map_err(|e| format!("Scan failed: {e}"))
}

/// Scan multiple projects at once
#[tauri::command]
pub async fn scan_projects(
    paths: Vec<String>,
    _state: State<'_, AppState>,
) -> Result<Inventory, String> {
    let project_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    // Validate all paths exist
    for path in &project_paths {
        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }
    }

    let scanner = Scanner::new();
    let path_refs: Vec<&std::path::Path> = project_paths
        .iter()
        .map(std::path::PathBuf::as_path)
        .collect();

    scanner
        .scan_all(&path_refs)
        .map_err(|e| format!("Scan failed: {e}"))
}

/// Profile tool inventory for UI listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileToolInventory {
    pub skills: Vec<SkillInfo>,
    pub commands: Vec<CommandInfo>,
    pub agents: Vec<AgentInfo>,
}

/// Scan profile storage for tools (skills, commands, agents)
#[tauri::command]
pub async fn scan_profiles(state: State<'_, AppState>) -> Result<ProfileToolInventory, String> {
    let profiles = state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        store.list().map_err(|e| format!("Database error: {e}"))
    })?;

    let mut inventory = ProfileToolInventory {
        skills: Vec::new(),
        commands: Vec::new(),
        agents: Vec::new(),
    };

    for profile in profiles {
        let profile_dir = tars_core::profile::storage::profile_dir(profile.id)
            .map_err(|e| format!("Failed to read profile storage: {e}"))?;
        let scope = Scope::Plugin(format!("tars-profile-{}", profile.id));

        let skills = scan_skills_directory(&profile_dir.join("skills"), scope.clone())
            .map_err(|e| format!("Failed to scan profile skills: {e}"))?;
        inventory.skills.extend(skills);

        let commands = scan_commands_directory(&profile_dir.join("commands"), scope.clone())
            .map_err(|e| format!("Failed to scan profile commands: {e}"))?;
        inventory.commands.extend(commands);

        let agents = scan_agents_directory(&profile_dir.join("agents"), scope.clone())
            .map_err(|e| format!("Failed to scan profile agents: {e}"))?;
        inventory.agents.extend(agents);
    }

    Ok(inventory)
}

/// Info about a discovered Claude project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredProject {
    /// Project directory path
    pub path: String,
    /// Project name (directory name)
    pub name: String,
    /// Whether it has a .claude/ directory
    pub has_claude_dir: bool,
    /// Whether it has a CLAUDE.md file
    pub has_claude_md: bool,
    /// Whether it has an .mcp.json file
    pub has_mcp_json: bool,
}

/// Discover all Claude projects in a directory (non-recursive first level only)
#[tauri::command]
pub async fn discover_claude_projects(
    folder: String,
    _state: State<'_, AppState>,
) -> Result<Vec<DiscoveredProject>, String> {
    let folder_path = PathBuf::from(&folder);

    if !folder_path.exists() {
        return Err(format!("Folder does not exist: {folder}"));
    }

    if !folder_path.is_dir() {
        return Err(format!("Path is not a directory: {folder}"));
    }

    let mut projects = Vec::new();

    // Read the directory entries
    let entries =
        std::fs::read_dir(&folder_path).map_err(|e| format!("Failed to read directory: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();

        // Only look at directories
        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        // Check for Claude configuration indicators
        let claude_dir = path.join(".claude");
        let claude_md = path.join("CLAUDE.md");
        let mcp_json = path.join(".mcp.json");

        let has_claude_dir = claude_dir.is_dir();
        let has_claude_md = claude_md.is_file();
        let has_mcp_json = mcp_json.is_file();

        // Only include if it has some Claude configuration
        if has_claude_dir || has_claude_md || has_mcp_json {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            projects.push(DiscoveredProject {
                path: path.to_string_lossy().to_string(),
                name,
                has_claude_dir,
                has_claude_md,
                has_mcp_json,
            });
        }
    }

    // Sort by name
    projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(projects)
}
