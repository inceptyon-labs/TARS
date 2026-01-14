//! Settings file commands
//!
//! Read/write Claude Code settings files for user/project/local scopes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct SettingsFile {
    pub path: String,
    pub content: Option<String>,
    pub exists: bool,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
pub struct SettingsFileParams {
    pub scope: String,
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
    pub content: Option<String>,
}

fn ensure_project_dir(project_path: &str) -> Result<PathBuf, String> {
    let project = PathBuf::from(project_path);
    if !project.exists() {
        return Err(format!("Project path does not exist: {project_path}"));
    }
    if !project.is_dir() {
        return Err(format!("Project path is not a directory: {project_path}"));
    }
    Ok(project)
}

fn settings_path(scope: &str, project_path: Option<&str>) -> Result<PathBuf, String> {
    match scope {
        "user" => {
            let home = dirs::home_dir().ok_or("Cannot find home directory")?;
            Ok(home.join(".claude").join("settings.json"))
        }
        "project" => {
            let project_path = project_path.ok_or("Project path is required for project scope")?;
            let project = ensure_project_dir(project_path)?;
            Ok(project.join(".claude").join("settings.json"))
        }
        "local" => {
            let project_path = project_path.ok_or("Project path is required for local scope")?;
            let project = ensure_project_dir(project_path)?;
            Ok(project.join(".claude").join("settings.local.json"))
        }
        other => Err(format!("Unsupported settings scope: {other}")),
    }
}

/// Read settings file by scope (if it exists)
#[tauri::command]
pub async fn read_settings_file(params: SettingsFileParams) -> Result<SettingsFile, String> {
    let path = settings_path(&params.scope, params.project_path.as_deref())?;
    if !path.exists() {
        return Ok(SettingsFile {
            path: path.display().to_string(),
            content: None,
            exists: false,
            scope: params.scope,
        });
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read settings.json: {e}"))?;

    Ok(SettingsFile {
        path: path.display().to_string(),
        content: Some(content),
        exists: true,
        scope: params.scope,
    })
}

/// Write settings file by scope (valid JSON required)
#[tauri::command]
pub async fn save_settings_file(params: SettingsFileParams) -> Result<(), String> {
    let content = params.content.ok_or("Missing settings content")?;
    let path = settings_path(&params.scope, params.project_path.as_deref())?;

    serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|e| format!("Invalid JSON: {e}"))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {e}"))?;
    }

    std::fs::write(&path, content).map_err(|e| format!("Failed to write settings.json: {e}"))?;

    Ok(())
}
