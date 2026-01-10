//! Command management Tauri commands
//!
//! Commands for viewing and editing Claude Code slash commands.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tars_core::util::validate_name;

/// Command information for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDetails {
    pub name: String,
    pub path: String,
    pub content: String,
    pub description: Option<String>,
    pub scope: String,
}

/// Read a command file
#[tauri::command]
pub async fn read_command(path: String) -> Result<CommandDetails, String> {
    let command_path = PathBuf::from(&path);

    // Validate the path is within allowed command directories
    let validated_path = validate_command_path(&command_path)?;

    if !validated_path.exists() {
        return Err("Command file not found".to_string());
    }

    let content = std::fs::read_to_string(&validated_path)
        .map_err(|_| "Failed to read command".to_string())?;

    // Extract name from filename (without .md extension)
    let name = command_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Determine scope from path
    let scope = determine_command_scope(&validated_path);

    // Try to extract description from frontmatter
    let description = extract_description(&content);

    Ok(CommandDetails {
        name,
        path,
        content,
        description,
        scope,
    })
}

/// Save a command file
#[tauri::command]
pub async fn save_command(path: String, content: String) -> Result<(), String> {
    let command_path = PathBuf::from(&path);

    // Validate the path is within allowed command directories
    let validated_path = validate_command_path(&command_path)?;

    // Ensure parent directory exists
    if let Some(parent) = validated_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "Failed to create directory".to_string())?;
    }

    std::fs::write(&validated_path, content).map_err(|_| "Failed to save command".to_string())?;

    Ok(())
}

/// Create a new command
#[tauri::command]
pub async fn create_command(
    name: String,
    scope: String,
    project_path: Option<String>,
) -> Result<CommandDetails, String> {
    // Validate the command name to prevent path traversal
    validate_name(&name).map_err(|_| "Invalid command name".to_string())?;

    let base_path = if scope == "user" {
        let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
        PathBuf::from(home).join(".claude/commands")
    } else {
        let project = project_path.ok_or("Project path required for project-scoped command")?;
        PathBuf::from(project).join(".claude/commands")
    };

    let command_file = base_path.join(format!("{name}.md"));

    // Validate the final path is within allowed directories
    validate_command_path(&command_file)?;

    if command_file.exists() {
        return Err(format!("Command '{name}' already exists"));
    }

    // Create default command content
    let content = format!(
        r"---
description: A new command
---

# /{name}

This command does something useful.

Arguments provided by the user: $ARGUMENTS

Add your command instructions here.
",
    );

    std::fs::create_dir_all(&base_path)
        .map_err(|_| "Failed to create commands directory".to_string())?;
    std::fs::write(&command_file, &content).map_err(|_| "Failed to create command".to_string())?;

    Ok(CommandDetails {
        name,
        path: command_file.display().to_string(),
        content,
        description: Some("A new command".to_string()),
        scope,
    })
}

/// Move a command to a different scope (supports multiple project destinations)
#[tauri::command]
#[allow(non_snake_case)]
pub async fn move_command(
    path: String,
    targetScope: String,
    projectPaths: Option<Vec<String>>,
) -> Result<CommandDetails, String> {
    let source_path = PathBuf::from(&path);

    // Validate the source path
    let validated_source = validate_command_path(&source_path)?;

    if !validated_source.exists() {
        return Err("Command file not found".to_string());
    }

    // Extract name from filename
    let name = source_path
        .file_stem()
        .and_then(|n| n.to_str())
        .ok_or("Invalid command filename")?
        .to_string();

    // Read the content first
    let content = std::fs::read_to_string(&validated_source)
        .map_err(|_| "Failed to read command".to_string())?;

    let description = extract_description(&content);

    // Determine target(s) and copy
    let final_path: PathBuf;
    let final_scope: String;

    if targetScope == "user" {
        let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
        let target_base = PathBuf::from(home).join(".claude/commands");
        let target_file = target_base.join(format!("{name}.md"));

        validate_command_path(&target_file)?;

        if target_file.exists() {
            return Err(format!("Command '{name}' already exists in user scope"));
        }

        std::fs::create_dir_all(&target_base)
            .map_err(|_| "Failed to create target directory".to_string())?;

        std::fs::write(&target_file, &content)
            .map_err(|_| "Failed to write command".to_string())?;

        final_path = target_file;
        final_scope = "user".to_string();
    } else {
        // Project scope - can have multiple destinations
        let projects = projectPaths.ok_or("Project paths required for project-scoped command")?;

        if projects.is_empty() {
            return Err("At least one project must be selected".to_string());
        }

        // Validate all targets first before making any changes
        let mut targets: Vec<(PathBuf, PathBuf)> = Vec::new();
        for project in &projects {
            let target_base = PathBuf::from(project).join(".claude/commands");
            let target_file = target_base.join(format!("{name}.md"));

            validate_command_path(&target_file)?;

            if target_file.exists() {
                return Err(format!(
                    "Command '{name}' already exists in project '{project}'"
                ));
            }

            targets.push((target_base, target_file));
        }

        // Now copy to all destinations
        for (target_base, target_file) in &targets {
            std::fs::create_dir_all(target_base)
                .map_err(|_| "Failed to create target directory".to_string())?;

            std::fs::write(target_file, &content)
                .map_err(|_| "Failed to write command".to_string())?;
        }

        // Return the first destination as the "primary" result
        final_path = targets[0].1.clone();
        final_scope = "project".to_string();
    }

    // Delete from old location
    std::fs::remove_file(&validated_source).map_err(|_| "Failed to remove command".to_string())?;

    Ok(CommandDetails {
        name,
        path: final_path.display().to_string(),
        content,
        description,
        scope: final_scope,
    })
}

/// Delete a command
#[tauri::command]
pub async fn delete_command(path: String) -> Result<(), String> {
    let command_path = PathBuf::from(&path);

    // Validate the path is within allowed command directories
    let validated_path = validate_command_path(&command_path)?;

    if !validated_path.exists() {
        return Err("Command not found".to_string());
    }

    // Verify this is actually a .md file
    let extension = validated_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if extension != "md" {
        return Err("Can only delete command files (.md)".to_string());
    }

    // Verify the file is directly under a commands/ directory
    let commands_parent = validated_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if commands_parent != "commands" {
        return Err("Command must be directly under a commands directory".to_string());
    }

    // Now safe to remove the command file
    std::fs::remove_file(&validated_path).map_err(|_| "Failed to delete command".to_string())?;

    Ok(())
}

/// Determine if a command path is user-scoped or project-scoped
fn determine_command_scope(path: &Path) -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();

    if let Some(home_str) = home {
        let home_path = PathBuf::from(&home_str);
        let user_commands_dir = home_path.join(".claude").join("commands");

        if let Ok(canonical_user_commands) = user_commands_dir.canonicalize() {
            if let Ok(canonical_path) = path.canonicalize() {
                if canonical_path.starts_with(&canonical_user_commands) {
                    return "user".to_string();
                }
            }
        }

        if path.starts_with(&user_commands_dir) {
            return "user".to_string();
        }
    }

    "project".to_string()
}

/// Get allowed command root directories
fn get_command_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let claude_dir = PathBuf::from(&home).join(".claude");
        roots.push(claude_dir.join("commands"));
        roots.push(claude_dir.join("plugins/cache"));
        roots.push(claude_dir.join("plugins/marketplaces"));
    }

    roots
}

/// Validate that a path is within an allowed command directory
fn validate_command_path(path: &Path) -> Result<PathBuf, String> {
    let roots = get_command_roots();
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
        if canonical_str.contains("/.claude/commands/")
            || canonical_str.contains("\\.claude\\commands\\")
        {
            return Ok(canonical);
        }

        if canonical_str.contains("/.claude/plugins/")
            || canonical_str.contains("\\.claude\\plugins\\")
        {
            return Ok(canonical);
        }

        return Err("Path is not within an allowed commands directory".to_string());
    }

    // For non-existent paths, do a logical check
    for root in &roots {
        if path.starts_with(root) {
            return Ok(path.to_path_buf());
        }
    }

    if path_str.contains("/.claude/commands/") || path_str.contains("\\.claude\\commands\\") {
        return Ok(path.to_path_buf());
    }

    if path_str.contains("/.claude/plugins/") || path_str.contains("\\.claude\\plugins\\") {
        return Ok(path.to_path_buf());
    }

    Err("Path is not within an allowed commands directory".to_string())
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
