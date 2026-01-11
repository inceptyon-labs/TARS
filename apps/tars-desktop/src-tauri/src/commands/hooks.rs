//! Hook management Tauri commands
//!
//! Commands for viewing and editing hooks in settings.json files.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// Hook event types
pub const HOOK_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "SubagentStop",
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PreCompact",
    "Notification",
];

/// A single hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    #[serde(rename = "type")]
    pub hook_type: String, // "command" or "prompt"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}

/// A hook matcher with its hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    pub matcher: String,
    pub hooks: Vec<HookDefinition>,
}

/// Hook configuration for a specific event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event: String,
    pub matchers: Vec<HookMatcher>,
}

/// All hooks from a settings file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    pub path: String,
    pub scope: String,
    pub events: Vec<HookEvent>,
}

/// Get the user settings.json path (cross-platform)
fn get_user_settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".claude").join("settings.json"))
}

/// Get the project settings.json path
fn get_project_settings_path(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(".claude").join("settings.json")
}

/// Read hooks from a settings.json file
fn read_hooks_from_settings(path: &PathBuf) -> Result<Vec<HookEvent>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read settings: {e}"))?;

    let settings: HashMap<String, Value> =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {e}"))?;

    // Hooks are nested under the "hooks" key
    let hooks_obj = match settings.get("hooks") {
        Some(Value::Object(obj)) => obj.clone(),
        _ => return Ok(vec![]),
    };

    let mut events = Vec::new();

    for event_name in HOOK_EVENTS {
        if let Some(value) = hooks_obj.get(*event_name) {
            if let Ok(matchers) = serde_json::from_value::<Vec<HookMatcher>>(value.clone()) {
                if !matchers.is_empty() {
                    events.push(HookEvent {
                        event: event_name.to_string(),
                        matchers,
                    });
                }
            }
        }
    }

    Ok(events)
}

/// Write hooks to a settings.json file
fn write_hooks_to_settings(path: &PathBuf, events: &[HookEvent]) -> Result<(), String> {
    // Read existing settings or create empty object
    let mut settings: serde_json::Map<String, Value> = if path.exists() {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read settings: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {e}"))?
    } else {
        serde_json::Map::new()
    };

    // Get or create the "hooks" object
    let mut hooks_obj = match settings.get("hooks") {
        Some(Value::Object(obj)) => obj.clone(),
        _ => serde_json::Map::new(),
    };

    // Remove all existing hook events from hooks object
    for event_name in HOOK_EVENTS {
        hooks_obj.remove(*event_name);
    }

    // Add the new hook events to hooks object
    for event in events {
        if !event.matchers.is_empty() {
            let value = serde_json::to_value(&event.matchers)
                .map_err(|e| format!("Failed to serialize hooks: {e}"))?;
            hooks_obj.insert(event.event.clone(), value);
        }
    }

    // Update the hooks key in settings
    settings.insert("hooks".to_string(), Value::Object(hooks_obj));

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
    }

    // Write back to file with pretty formatting
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;

    std::fs::write(path, content).map_err(|e| format!("Failed to write settings: {e}"))?;

    Ok(())
}

/// Get hooks from user scope
#[tauri::command]
pub async fn get_user_hooks() -> Result<HooksConfig, String> {
    let path = get_user_settings_path()?;
    let events = read_hooks_from_settings(&path)?;

    Ok(HooksConfig {
        path: path.display().to_string(),
        scope: "user".to_string(),
        events,
    })
}

/// Get hooks from a project scope
#[tauri::command]
pub async fn get_project_hooks(project_path: String) -> Result<HooksConfig, String> {
    let path = get_project_settings_path(&project_path);
    let events = read_hooks_from_settings(&path)?;

    Ok(HooksConfig {
        path: path.display().to_string(),
        scope: "project".to_string(),
        events,
    })
}

/// Save hooks to user scope
#[tauri::command]
pub async fn save_user_hooks(events: Vec<HookEvent>) -> Result<(), String> {
    let path = get_user_settings_path()?;
    write_hooks_to_settings(&path, &events)
}

/// Save hooks to a project scope
#[tauri::command]
pub async fn save_project_hooks(
    project_path: String,
    events: Vec<HookEvent>,
) -> Result<(), String> {
    let path = get_project_settings_path(&project_path);
    write_hooks_to_settings(&path, &events)
}

/// Get all available hook event types
#[tauri::command]
pub async fn get_hook_event_types() -> Vec<String> {
    HOOK_EVENTS
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
}
