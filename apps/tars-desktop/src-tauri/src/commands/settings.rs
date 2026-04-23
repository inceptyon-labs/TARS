//! Settings file commands
//!
//! Read/write Claude Code settings files for user/project/local scopes.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

fn codex_system_config_path() -> PathBuf {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        PathBuf::from("/etc/codex/config.toml")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        PathBuf::new()
    }
}

fn codex_managed_config_path(_home: &Path) -> PathBuf {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        PathBuf::from("/etc/codex/managed_config.toml")
    }
    #[cfg(target_os = "windows")]
    {
        _home.join(".codex").join("managed_config.toml")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::new()
    }
}

fn runtime_config_path(
    runtime: &str,
    scope: &str,
    project_path: Option<&str>,
) -> Result<(PathBuf, &'static str), String> {
    match runtime {
        "claude-code" => Ok((settings_path(scope, project_path)?, "json")),
        "codex" => {
            let home = dirs::home_dir().ok_or("Cannot find home directory")?;
            match scope {
                "user" => Ok((home.join(".codex").join("config.toml"), "toml")),
                "project" => {
                    let project_path =
                        project_path.ok_or("Project path is required for project scope")?;
                    let project = ensure_project_dir(project_path)?;
                    Ok((project.join(".codex").join("config.toml"), "toml"))
                }
                "system" => {
                    let path = codex_system_config_path();
                    if path.as_os_str().is_empty() {
                        return Err(
                            "Codex system config is not supported on this platform".to_string()
                        );
                    }
                    Ok((path, "toml"))
                }
                "managed" => {
                    let path = codex_managed_config_path(&home);
                    if path.as_os_str().is_empty() {
                        return Err(
                            "Codex managed config is not supported on this platform".to_string()
                        );
                    }
                    Ok((path, "toml"))
                }
                other => Err(format!("Unsupported Codex config scope: {other}")),
            }
        }
        other => Err(format!("Unsupported runtime: {other}")),
    }
}

fn validate_config_content(content: &str, format: &str) -> Result<(), String> {
    match format {
        "json" => serde_json::from_str::<serde_json::Value>(content)
            .map(|_| ())
            .map_err(|e| format!("Invalid JSON: {e}")),
        "toml" => content
            .parse::<toml::Value>()
            .map(|_| ())
            .map_err(|e| format!("Invalid TOML: {e}")),
        other => Err(format!("Unsupported config format: {other}")),
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

#[derive(Debug, Serialize)]
pub struct RuntimeConfigFile {
    pub path: String,
    pub content: Option<String>,
    pub exists: bool,
    pub runtime: String,
    pub scope: String,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeConfigFileParams {
    pub runtime: String,
    pub scope: String,
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
    pub content: Option<String>,
}

#[tauri::command]
pub async fn read_runtime_config_file(
    params: RuntimeConfigFileParams,
) -> Result<RuntimeConfigFile, String> {
    let (path, format) = runtime_config_path(
        &params.runtime,
        &params.scope,
        params.project_path.as_deref(),
    )?;

    if !path.exists() {
        return Ok(RuntimeConfigFile {
            path: path.display().to_string(),
            content: None,
            exists: false,
            runtime: params.runtime,
            scope: params.scope,
            format: format.to_string(),
        });
    }

    let content = std::fs::read_to_string(&path).map_err(|e| {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config file");
        format!("Failed to read {file_name}: {e}")
    })?;

    Ok(RuntimeConfigFile {
        path: path.display().to_string(),
        content: Some(content),
        exists: true,
        runtime: params.runtime,
        scope: params.scope,
        format: format.to_string(),
    })
}

#[tauri::command]
pub async fn save_runtime_config_file(params: RuntimeConfigFileParams) -> Result<(), String> {
    let content = params.content.ok_or("Missing config content")?;
    let (path, format) = runtime_config_path(
        &params.runtime,
        &params.scope,
        params.project_path.as_deref(),
    )?;

    validate_config_content(&content, format)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }

    std::fs::write(&path, content).map_err(|e| {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config file");
        format!("Failed to write {file_name}: {e}")
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_config_content;

    #[test]
    fn validate_config_content_accepts_json() {
        assert!(validate_config_content("{\"theme\":\"dark\"}", "json").is_ok());
    }

    #[test]
    fn validate_config_content_rejects_invalid_json() {
        assert!(validate_config_content("{theme:", "json").is_err());
    }

    #[test]
    fn validate_config_content_accepts_toml() {
        assert!(validate_config_content("model = \"gpt-5.4\"\n", "toml").is_ok());
    }

    #[test]
    fn validate_config_content_rejects_invalid_toml() {
        assert!(validate_config_content("model = ", "toml").is_err());
    }
}
