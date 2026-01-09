//! Utility Tauri commands
//!
//! Commands for file dialogs, path operations, etc.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Directory info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub path: String,
    pub name: String,
    pub has_claude_config: bool,
    pub is_git_repo: bool,
}

/// Check if a directory exists
#[tauri::command]
pub async fn directory_exists(path: String) -> bool {
    let p = PathBuf::from(&path);
    p.exists() && p.is_dir()
}

/// Get directory info
#[tauri::command]
pub async fn get_directory_info(path: String) -> Result<DirectoryInfo, String> {
    let p = PathBuf::from(&path);

    if !p.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    if !p.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }

    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let has_claude_config = p.join(".claude").exists() || p.join("CLAUDE.md").exists();
    let is_git_repo = p.join(".git").exists();

    Ok(DirectoryInfo {
        path,
        name,
        has_claude_config,
        is_git_repo,
    })
}

/// Get home directory path
#[tauri::command]
pub async fn get_home_dir() -> Result<String, String> {
    std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())
}

/// List subdirectories in a path
#[tauri::command]
pub async fn list_subdirectories(path: String) -> Result<Vec<DirectoryInfo>, String> {
    let p = PathBuf::from(&path);

    if !p.exists() || !p.is_dir() {
        return Err(format!("Invalid directory: {path}"));
    }

    let mut dirs = Vec::new();

    let entries = std::fs::read_dir(&p).map_err(|e| format!("Failed to read directory: {e}"))?;

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            // Skip hidden directories (except .claude)
            let name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if name.starts_with('.') && name != ".claude" {
                continue;
            }

            let has_claude_config =
                entry_path.join(".claude").exists() || entry_path.join("CLAUDE.md").exists();
            let is_git_repo = entry_path.join(".git").exists();

            dirs.push(DirectoryInfo {
                path: entry_path.display().to_string(),
                name,
                has_claude_config,
                is_git_repo,
            });
        }
    }

    // Sort by name
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(dirs)
}

/// Get app version
#[tauri::command]
pub async fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
