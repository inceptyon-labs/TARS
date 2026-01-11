//! Agent management Tauri commands
//!
//! Commands for viewing and editing agents.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tars_core::util::validate_name;

/// Agent information for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDetails {
    pub name: String,
    pub path: String,
    pub content: String,
    pub description: Option<String>,
    pub scope: String,
}

/// Read an agent file
#[tauri::command]
pub async fn read_agent(path: String) -> Result<AgentDetails, String> {
    let agent_path = PathBuf::from(&path);

    // Validate the path is within allowed agent directories
    let validated_path = validate_agent_path(&agent_path)?;

    if !validated_path.exists() {
        return Err("Agent file not found".to_string());
    }

    let content =
        std::fs::read_to_string(&validated_path).map_err(|_| "Failed to read agent".to_string())?;

    // Extract name from filename (without .md extension)
    let name = agent_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Determine scope from path
    let scope = determine_agent_scope(&validated_path);

    // Try to extract description from frontmatter
    let description = extract_description(&content);

    Ok(AgentDetails {
        name,
        path,
        content,
        description,
        scope,
    })
}

/// Save an agent file
#[tauri::command]
pub async fn save_agent(path: String, content: String) -> Result<(), String> {
    let agent_path = PathBuf::from(&path);

    // Validate the path is within allowed agent directories
    let validated_path = validate_agent_path(&agent_path)?;

    // Ensure parent directory exists
    if let Some(parent) = validated_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "Failed to create directory".to_string())?;
    }

    std::fs::write(&validated_path, content).map_err(|_| "Failed to save agent".to_string())?;

    Ok(())
}

/// Create a new agent
#[tauri::command]
pub async fn create_agent(
    name: String,
    scope: String,
    project_path: Option<String>,
) -> Result<AgentDetails, String> {
    // Validate the agent name to prevent path traversal
    validate_name(&name).map_err(|_| "Invalid agent name".to_string())?;

    let base_path = if scope == "user" {
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        home.join(".claude").join("agents")
    } else {
        let project = project_path.ok_or("Project path required for project-scoped agent")?;
        PathBuf::from(project).join(".claude").join("agents")
    };

    let agent_file = base_path.join(format!("{name}.md"));

    // Validate the final path is within allowed directories
    validate_agent_path(&agent_file)?;

    if agent_file.exists() {
        return Err(format!("Agent '{name}' already exists"));
    }

    // Create default agent content
    let content = format!(
        r"---
name: {name}
description: A new agent
tools:
  - Read
  - Glob
  - Grep
---

# {name}

Add your agent instructions here.

This agent has access to file reading tools by default.
",
    );

    std::fs::create_dir_all(&base_path)
        .map_err(|_| "Failed to create agents directory".to_string())?;
    std::fs::write(&agent_file, &content).map_err(|_| "Failed to create agent".to_string())?;

    Ok(AgentDetails {
        name,
        path: agent_file.display().to_string(),
        content,
        description: Some("A new agent".to_string()),
        scope,
    })
}

/// Move an agent to a different scope (supports multiple project destinations)
#[tauri::command]
#[allow(non_snake_case)]
pub async fn move_agent(
    path: String,
    targetScope: String,
    projectPaths: Option<Vec<String>>,
) -> Result<AgentDetails, String> {
    let source_path = PathBuf::from(&path);

    // Validate the source path
    let validated_source = validate_agent_path(&source_path)?;

    if !validated_source.exists() {
        return Err("Agent file not found".to_string());
    }

    // Extract name from filename
    let name = source_path
        .file_stem()
        .and_then(|n| n.to_str())
        .ok_or("Invalid agent filename")?
        .to_string();

    // Read the content first
    let content = std::fs::read_to_string(&validated_source)
        .map_err(|_| "Failed to read agent".to_string())?;

    let description = extract_description(&content);

    // Determine target(s) and copy
    let final_path: PathBuf;
    let final_scope: String;

    if targetScope == "user" {
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        let target_base = home.join(".claude").join("agents");
        let target_file = target_base.join(format!("{name}.md"));

        validate_agent_path(&target_file)?;

        if target_file.exists() {
            return Err(format!("Agent '{name}' already exists in user scope"));
        }

        std::fs::create_dir_all(&target_base)
            .map_err(|_| "Failed to create target directory".to_string())?;

        std::fs::write(&target_file, &content).map_err(|_| "Failed to write agent".to_string())?;

        final_path = target_file;
        final_scope = "user".to_string();
    } else {
        // Project scope - can have multiple destinations
        let projects = projectPaths.ok_or("Project paths required for project-scoped agent")?;

        if projects.is_empty() {
            return Err("At least one project must be selected".to_string());
        }

        // Validate all targets first before making any changes
        let mut targets: Vec<(PathBuf, PathBuf)> = Vec::new();
        for project in &projects {
            let target_base = PathBuf::from(project).join(".claude").join("agents");
            let target_file = target_base.join(format!("{name}.md"));

            validate_agent_path(&target_file)?;

            if target_file.exists() {
                return Err(format!(
                    "Agent '{name}' already exists in project '{project}'"
                ));
            }

            targets.push((target_base, target_file));
        }

        // Now copy to all destinations
        for (target_base, target_file) in &targets {
            std::fs::create_dir_all(target_base)
                .map_err(|_| "Failed to create target directory".to_string())?;

            std::fs::write(target_file, &content)
                .map_err(|_| "Failed to write agent".to_string())?;
        }

        // Return the first destination as the "primary" result
        final_path = targets[0].1.clone();
        final_scope = "project".to_string();
    }

    // Delete from old location
    std::fs::remove_file(&validated_source).map_err(|_| "Failed to remove agent".to_string())?;

    Ok(AgentDetails {
        name,
        path: final_path.display().to_string(),
        content,
        description,
        scope: final_scope,
    })
}

/// Disable an agent by moving it to .disabled subdirectory
#[tauri::command]
pub async fn disable_agent(path: String) -> Result<String, String> {
    let agent_path = PathBuf::from(&path);

    // Validate the path is within allowed agent directories
    let validated_path = validate_agent_path(&agent_path)?;

    if !validated_path.exists() {
        return Err("Agent not found".to_string());
    }

    // Verify this is actually a .md file
    let extension = validated_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if extension != "md" {
        return Err("Can only disable agent files (.md)".to_string());
    }

    // Get the parent (agents/) directory and filename
    let parent = validated_path.parent().ok_or("Invalid agent path")?;

    let filename = validated_path.file_name().ok_or("Invalid agent filename")?;

    // Check we're not already in .disabled
    if parent.file_name().and_then(|n| n.to_str()) == Some(".disabled") {
        return Err("Agent is already disabled".to_string());
    }

    // Create .disabled subdirectory
    let disabled_dir = parent.join(".disabled");
    std::fs::create_dir_all(&disabled_dir)
        .map_err(|_| "Failed to create disabled directory".to_string())?;

    // Move the file
    let disabled_path = disabled_dir.join(filename);
    std::fs::rename(&validated_path, &disabled_path)
        .map_err(|_| "Failed to disable agent".to_string())?;

    Ok(disabled_path.display().to_string())
}

/// Validate a disabled agent path (special case for .disabled directories)
fn validate_disabled_agent_path(path: &Path) -> Result<PathBuf, String> {
    let path_str = path.display().to_string();

    // Security: Reject any paths with parent directory references
    if path_str.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }

    // Security: Reject null bytes
    if path_str.contains('\0') {
        return Err("Invalid path".to_string());
    }

    // Must be a .md file
    if path.extension().and_then(|e| e.to_str()) != Some("md") {
        return Err("Can only enable agent files (.md)".to_string());
    }

    // Must be in a .disabled directory
    let parent = path.parent().ok_or("Invalid agent path")?;
    if parent.file_name().and_then(|n| n.to_str()) != Some(".disabled") {
        return Err("Agent is not disabled (not in .disabled directory)".to_string());
    }

    // Grandparent must be agents/
    let grandparent = parent.parent().ok_or("Invalid path structure")?;
    if grandparent.file_name().and_then(|n| n.to_str()) != Some("agents") {
        return Err("Invalid agent path structure".to_string());
    }

    // Verify path is within .claude directory structure
    if !path_str.contains("/.claude/agents/.disabled/")
        && !path_str.contains("\\.claude\\agents\\.disabled\\")
    {
        return Err("Path is not within an allowed agents directory".to_string());
    }

    if path.exists() {
        // Security: Reject symlinks
        if path.is_symlink() {
            return Err("Symlinks not allowed".to_string());
        }
        path.canonicalize().map_err(|_| "Invalid path".to_string())
    } else {
        Ok(path.to_path_buf())
    }
}

/// Enable a disabled agent by moving it back from .disabled subdirectory
#[tauri::command]
pub async fn enable_agent(path: String) -> Result<String, String> {
    let agent_path = PathBuf::from(&path);

    // Validate the disabled agent path (security fix)
    let validated_path = validate_disabled_agent_path(&agent_path)?;

    if !validated_path.exists() {
        return Err("Agent not found".to_string());
    }

    // Get the parent directory (.disabled/) and grandparent (agents/)
    let disabled_dir = validated_path.parent().ok_or("Invalid agent path")?;

    let filename = validated_path.file_name().ok_or("Invalid agent filename")?;

    // Get the agents/ parent directory
    let agents_dir = disabled_dir
        .parent()
        .ok_or("Invalid .disabled directory structure")?;

    // Move the file back
    let enabled_path = agents_dir.join(filename);

    if enabled_path.exists() {
        return Err("An agent with this name already exists in the active directory".to_string());
    }

    std::fs::rename(&validated_path, &enabled_path)
        .map_err(|_| "Failed to enable agent".to_string())?;

    // Clean up .disabled directory if empty
    if let Ok(entries) = std::fs::read_dir(disabled_dir) {
        if entries.count() == 0 {
            let _ = std::fs::remove_dir(disabled_dir);
        }
    }

    Ok(enabled_path.display().to_string())
}

/// Delete an agent
#[tauri::command]
pub async fn delete_agent(path: String) -> Result<(), String> {
    let agent_path = PathBuf::from(&path);

    // Validate the path is within allowed agent directories
    let validated_path = validate_agent_path(&agent_path)?;

    if !validated_path.exists() {
        return Err("Agent not found".to_string());
    }

    // Verify this is actually a .md file
    let extension = validated_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if extension != "md" {
        return Err("Can only delete agent files (.md)".to_string());
    }

    // Verify the file is directly under an agents/ directory
    let agents_parent = validated_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if agents_parent != "agents" {
        return Err("Agent must be directly under an agents directory".to_string());
    }

    // Now safe to remove the agent file
    std::fs::remove_file(&validated_path).map_err(|_| "Failed to delete agent".to_string())?;

    Ok(())
}

/// List disabled agents
/// When `project_path` is None, returns user-level disabled agents only
/// When `project_path` is Some, returns that project's disabled agents only
#[tauri::command]
pub async fn list_disabled_agents(
    project_path: Option<String>,
) -> Result<Vec<AgentDetails>, String> {
    let mut disabled_agents = Vec::new();

    match &project_path {
        None => {
            // Check user-level disabled agents only
            if let Some(home) = dirs::home_dir() {
                let user_disabled_dir = home.join(".claude").join("agents").join(".disabled");
                if user_disabled_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&user_disabled_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let name = path
                                        .file_stem()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    let description = extract_description(&content);
                                    disabled_agents.push(AgentDetails {
                                        name,
                                        path: path.display().to_string(),
                                        content,
                                        description,
                                        scope: "user".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        Some(project) => {
            // Check project-level disabled agents only
            let project_disabled_dir = PathBuf::from(project)
                .join(".claude")
                .join("agents")
                .join(".disabled");
            if project_disabled_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&project_disabled_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("md") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let name = path
                                    .file_stem()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let description = extract_description(&content);
                                disabled_agents.push(AgentDetails {
                                    name,
                                    path: path.display().to_string(),
                                    content,
                                    description,
                                    scope: "project".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(disabled_agents)
}

/// Determine if an agent path is user-scoped or project-scoped
fn determine_agent_scope(path: &Path) -> String {
    if let Some(home_path) = dirs::home_dir() {
        let user_agents_dir = home_path.join(".claude").join("agents");

        if let Ok(canonical_user_agents) = user_agents_dir.canonicalize() {
            if let Ok(canonical_path) = path.canonicalize() {
                if canonical_path.starts_with(&canonical_user_agents) {
                    return "user".to_string();
                }
            }
        }

        if path.starts_with(&user_agents_dir) {
            return "user".to_string();
        }
    }

    "project".to_string()
}

/// Get allowed agent root directories
fn get_agent_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let claude_dir = home.join(".claude");
        roots.push(claude_dir.join("agents"));
        roots.push(claude_dir.join("plugins").join("cache"));
        roots.push(claude_dir.join("plugins").join("marketplaces"));
    }

    roots
}

/// Validate that a path is within an allowed agent directory
fn validate_agent_path(path: &Path) -> Result<PathBuf, String> {
    let roots = get_agent_roots();
    let path_str = path.display().to_string();

    // Security: Reject any paths with parent directory references
    if path_str.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }

    // Security: Reject null bytes
    if path_str.contains('\0') {
        return Err("Invalid path".to_string());
    }

    if path.exists() {
        // Security: Reject symlinks to prevent TOCTOU attacks
        if path.is_symlink() {
            return Err("Symlinks not allowed".to_string());
        }

        let canonical = path
            .canonicalize()
            .map_err(|_| "Invalid path".to_string())?;

        for root in &roots {
            if root.exists() {
                if let Ok(canonical_root) = root.canonicalize() {
                    if canonical.starts_with(&canonical_root) {
                        return Ok(canonical);
                    }
                }
            }
        }

        let canonical_str = canonical.display().to_string();
        if canonical_str.contains("/.claude/agents/")
            || canonical_str.contains("\\.claude\\agents\\")
        {
            return Ok(canonical);
        }

        if canonical_str.contains("/.claude/plugins/")
            || canonical_str.contains("\\.claude\\plugins\\")
        {
            return Ok(canonical);
        }

        return Err("Path is not within an allowed agents directory".to_string());
    }

    // For non-existent paths, do a logical check
    for root in &roots {
        if path.starts_with(root) {
            return Ok(path.to_path_buf());
        }
    }

    if path_str.contains("/.claude/agents/") || path_str.contains("\\.claude\\agents\\") {
        return Ok(path.to_path_buf());
    }

    if path_str.contains("/.claude/plugins/") || path_str.contains("\\.claude\\plugins\\") {
        return Ok(path.to_path_buf());
    }

    Err("Path is not within an allowed agents directory".to_string())
}

/// Extract description from YAML frontmatter
fn extract_description(content: &str) -> Option<String> {
    if !content.starts_with("---") {
        return None;
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return None;
    }

    let frontmatter = parts[1];
    for line in frontmatter.lines() {
        let line = line.trim();
        if line.starts_with("description:") {
            return Some(line.trim_start_matches("description:").trim().to_string());
        }
    }

    None
}
