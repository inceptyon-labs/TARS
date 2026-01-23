//! Hook management Tauri commands
//!
//! Commands for viewing and editing hooks in settings.json files.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tars_scanner::plugins::{InstalledPlugin, PluginInventory};
use tars_scanner::types::Scope;
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use tars_core::storage::ProfileStore;

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
    /// Pattern to match against (defaults to "*" if not specified)
    #[serde(default = "default_matcher")]
    pub matcher: String,
    pub hooks: Vec<HookDefinition>,
}

fn default_matcher() -> String {
    "*".to_string()
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
    PathBuf::from(project_path)
        .join(".claude")
        .join("settings.json")
}

/// Get the profile hooks path (hooks.json)
fn get_profile_hooks_path(profile_id: &str) -> Result<PathBuf, String> {
    let profile_uuid = Uuid::parse_str(profile_id).map_err(|_| "Invalid profile ID".to_string())?;
    let profile_dir =
        tars_core::profile::storage::profile_dir(profile_uuid).map_err(|e| e.to_string())?;
    Ok(profile_dir.join("hooks.json"))
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

/// Read hooks from a plugin's hooks.json file
/// Plugin hooks.json uses the same format as settings.json hooks
fn read_hooks_from_plugin(plugin: &InstalledPlugin) -> Vec<HookEvent> {
    let hooks_path = resolve_plugin_hooks_path(plugin);
    let Some(path) = hooks_path else {
        return vec![];
    };

    // Plugin hooks.json uses the same format as settings.json
    read_hooks_from_settings(&path).unwrap_or_default()
}

/// Resolve the hooks.json path for a plugin
fn resolve_plugin_hooks_path(plugin: &InstalledPlugin) -> Option<PathBuf> {
    // Check manifest-specified hooks path first
    if let Some(path) = plugin.manifest.hooks.as_ref() {
        if path.is_absolute() && path.exists() {
            return Some(path.clone());
        }
        let relative_candidates = [
            plugin.path.join(path),
            plugin.path.join(".claude-plugin").join(path),
        ];
        if let Some(found) = relative_candidates
            .into_iter()
            .find(|candidate| candidate.exists())
        {
            return Some(found);
        }
    }

    // Try default locations
    let candidates = [
        plugin.path.join("hooks.json"),
        plugin.path.join(".claude-plugin").join("hooks.json"),
        plugin.path.join("hooks").join("hooks.json"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

/// Merge plugin hook events into existing events
fn merge_hook_events(
    mut base_events: Vec<HookEvent>,
    plugin_events: Vec<HookEvent>,
) -> Vec<HookEvent> {
    for plugin_event in plugin_events {
        // Find if this event type already exists
        if let Some(existing) = base_events
            .iter_mut()
            .find(|e| e.event == plugin_event.event)
        {
            // Merge matchers
            existing.matchers.extend(plugin_event.matchers);
        } else {
            base_events.push(plugin_event);
        }
    }
    base_events
}

/// Get hooks from user scope
#[tauri::command]
pub async fn get_user_hooks() -> Result<HooksConfig, String> {
    let path = get_user_settings_path()?;
    let mut events = read_hooks_from_settings(&path)?;

    // Also scan hooks from user-scoped plugins
    if let Ok(plugin_inventory) = PluginInventory::scan() {
        for plugin in &plugin_inventory.installed {
            // Only include user/managed scope plugins that are enabled
            if !matches!(plugin.scope, Scope::User | Scope::Managed) {
                continue;
            }
            if !plugin.enabled {
                continue;
            }

            let plugin_events = read_hooks_from_plugin(plugin);
            events = merge_hook_events(events, plugin_events);
        }
    }

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
    let mut events = read_hooks_from_settings(&path)?;

    // Also scan hooks from project-scoped plugins for this project
    if let Ok(plugin_inventory) = PluginInventory::scan() {
        let project_path_buf = PathBuf::from(&project_path);
        for plugin in &plugin_inventory.installed {
            // Only include project/local scope plugins for this specific project
            if !matches!(plugin.scope, Scope::Project | Scope::Local) {
                continue;
            }
            if !plugin.enabled {
                continue;
            }

            // Check if plugin is for this project
            let is_for_project = plugin.project_path.as_ref().is_some_and(|pp| {
                let plugin_project = PathBuf::from(pp);
                plugin_project == project_path_buf
                    || plugin_project
                        .canonicalize()
                        .ok()
                        .zip(project_path_buf.canonicalize().ok())
                        .is_some_and(|(a, b)| a == b)
            });

            if !is_for_project {
                continue;
            }

            let plugin_events = read_hooks_from_plugin(plugin);
            events = merge_hook_events(events, plugin_events);
        }
    }

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

/// Get hooks from a profile scope
#[tauri::command]
pub async fn get_profile_hooks(
    profile_id: String,
    _state: State<'_, AppState>,
) -> Result<HooksConfig, String> {
    let path = get_profile_hooks_path(&profile_id)?;
    let events = read_hooks_from_settings(&path)?;

    Ok(HooksConfig {
        path: path.display().to_string(),
        scope: "profile".to_string(),
        events,
    })
}

/// Save hooks to a profile scope
#[tauri::command]
pub async fn save_profile_hooks(
    state: State<'_, AppState>,
    profile_id: String,
    events: Vec<HookEvent>,
) -> Result<(), String> {
    let path = get_profile_hooks_path(&profile_id)?;
    write_hooks_to_settings(&path, &events)?;

    let profile_uuid =
        Uuid::parse_str(&profile_id).map_err(|_| "Invalid profile ID".to_string())?;

    state.with_db(|db| {
        let store = ProfileStore::new(db.connection());
        let mut profile = store
            .get(profile_uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        profile.updated_at = Utc::now();
        store
            .update(&profile)
            .map_err(|e| format!("Failed to update profile: {e}"))?;

        tars_core::profile::regenerate_profile_plugin(&profile)
            .map_err(|e| format!("Failed to regenerate plugin: {e}"))?;
        tars_core::profile::sync_profile_marketplace(&profile)
            .map_err(|e| format!("Failed to sync profile marketplace: {e}"))?;

        Ok(())
    })?;

    Ok(())
}

/// Get all available hook event types
#[tauri::command]
pub async fn get_hook_event_types() -> Vec<String> {
    HOOK_EVENTS
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
}

/// Info about a resolved hook script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookScriptInfo {
    /// Resolved path to the script
    pub path: String,
    /// Script content (if readable)
    pub content: Option<String>,
    /// Whether the script exists
    pub exists: bool,
    /// Whether the script is from a plugin (read-only)
    pub is_plugin_script: bool,
    /// Plugin name if from a plugin
    pub plugin_name: Option<String>,
}

/// Resolve and read a hook script
/// Handles `${CLAUDE_PLUGIN_ROOT}` variable resolution
#[tauri::command]
pub async fn get_hook_script(command: String) -> Result<HookScriptInfo, String> {
    // Check if this is a plugin script with ${CLAUDE_PLUGIN_ROOT}
    if command.contains("${CLAUDE_PLUGIN_ROOT}") {
        // Extract just the script path (remove arguments)
        let script_path = extract_script_path(&command);

        // Need to find which plugin this belongs to
        if let Ok(plugin_inventory) = PluginInventory::scan() {
            for plugin in &plugin_inventory.installed {
                if !plugin.enabled {
                    continue;
                }

                // Resolve the variable in the extracted script path
                let resolved =
                    script_path.replace("${CLAUDE_PLUGIN_ROOT}", &plugin.path.to_string_lossy());
                let resolved_path = PathBuf::from(&resolved);

                if resolved_path.exists() {
                    let content = std::fs::read_to_string(&resolved_path).ok();
                    return Ok(HookScriptInfo {
                        path: resolved,
                        content,
                        exists: true,
                        is_plugin_script: true,
                        plugin_name: Some(plugin.id.clone()),
                    });
                }
            }
        }

        // Variable not resolved, return as-is
        return Ok(HookScriptInfo {
            path: script_path.to_string(),
            content: None,
            exists: false,
            is_plugin_script: true,
            plugin_name: None,
        });
    }

    // Regular path - extract script path and expand ~ to home directory
    let script_path = extract_script_path(&command);
    let expanded_path = expand_tilde(script_path);
    let path = PathBuf::from(&expanded_path);
    let exists = path.exists();
    let content = if exists {
        std::fs::read_to_string(&path).ok()
    } else {
        None
    };

    Ok(HookScriptInfo {
        path: expanded_path,
        content,
        exists,
        is_plugin_script: false,
        plugin_name: None,
    })
}

/// Expand ~ to the user's home directory
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), &path[1..]);
        }
    }
    path.to_string()
}

/// Extract the script path from a command that may have arguments
/// e.g., "~/.claude/script.sh arg1 arg2" -> "~/.claude/script.sh"
/// Handles quoted paths and shell redirections
fn extract_script_path(command: &str) -> &str {
    let trimmed = command.trim();

    // Handle "sh " or "bash " prefix
    let without_shell = trimmed
        .strip_prefix("sh ")
        .or_else(|| trimmed.strip_prefix("bash "))
        .unwrap_or(trimmed)
        .trim_start();

    // Find where the path ends (first space, unless it's in quotes)
    if let Some(quoted) = without_shell.strip_prefix('"') {
        // Quoted path - find closing quote
        if let Some(end) = quoted.find('"') {
            return &quoted[..end];
        }
    }

    // Unquoted - find first space or shell operator
    without_shell
        .split(|c: char| c.is_whitespace() || c == '>' || c == '|' || c == '&' || c == ';')
        .next()
        .unwrap_or(without_shell)
}
