//! Scanner Tauri commands
//!
//! Commands for scanning Claude Code configuration.

use crate::state::AppState;
use std::path::PathBuf;
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
