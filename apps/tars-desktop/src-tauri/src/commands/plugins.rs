//! Plugin management Tauri commands
//!
//! Commands for managing Claude Code plugins via the CLI.

use serde::{Deserialize, Serialize};
use std::process::Command;
use tars_scanner::plugins::PluginInventory;
use tars_scanner::CacheCleanupReport;

/// Validate a plugin source string (marketplace URL or plugin@marketplace format)
/// Prevents command injection by restricting to safe characters
fn validate_plugin_source(source: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("Source cannot be empty".to_string());
    }
    if source.len() > 500 {
        return Err("Source string too long".to_string());
    }
    // Allow alphanumeric, hyphens, underscores, dots, @, /, :, and common URL chars
    // Reject shell metacharacters and control characters
    let forbidden_chars = [
        '`', '$', '(', ')', '{', '}', '[', ']', '|', ';', '&', '<', '>', '\\', '\n', '\r', '\0',
        '\'', '"', '!', '*', '?',
    ];
    for ch in forbidden_chars {
        if source.contains(ch) {
            return Err(format!("Source contains forbidden character: {ch}"));
        }
    }
    Ok(())
}

/// Validate a plugin name (alphanumeric, hyphens, underscores, dots)
fn validate_plugin_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Plugin name cannot be empty".to_string());
    }
    if name.len() > 200 {
        return Err("Plugin name too long".to_string());
    }
    // Plugin names should be simple identifiers
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("Plugin name contains invalid characters".to_string());
    }
    if name.starts_with('.') || name.starts_with('-') {
        return Err("Plugin name cannot start with dot or hyphen".to_string());
    }
    Ok(())
}

/// Validate a scope string
fn validate_scope(scope: &str) -> Result<(), String> {
    match scope {
        "user" | "project" | "local" => Ok(()),
        _ => Err(format!(
            "Invalid scope: {scope}. Must be user, project, or local"
        )),
    }
}

// ============================================================================
// Plugin Inventory Commands
// ============================================================================

/// Plugin manifest for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestInfo {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

/// Plugin scope for frontend display (matches frontend `PluginScope` type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginScopeInfo {
    #[serde(rename = "type")]
    pub scope_type: String,
}

/// Installed plugin for frontend display (matches frontend `InstalledPlugin` interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPluginInfo {
    pub id: String,
    pub marketplace: Option<String>,
    pub version: String,
    pub scope: PluginScopeInfo,
    pub enabled: bool,
    pub path: String,
    pub manifest: PluginManifestInfo,
    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
    pub project_path: Option<String>,
}

/// List all installed plugins
#[tauri::command]
pub async fn plugin_list() -> Result<Vec<InstalledPluginInfo>, String> {
    let inventory = PluginInventory::scan().map_err(|e| format!("Failed to scan plugins: {e}"))?;

    let plugins: Vec<InstalledPluginInfo> = inventory
        .installed
        .into_iter()
        .map(|p| InstalledPluginInfo {
            id: p.id,
            marketplace: p.marketplace,
            version: p.version.clone(),
            scope: PluginScopeInfo {
                scope_type: format!("{:?}", p.scope),
            },
            enabled: p.enabled,
            path: p.path.display().to_string(),
            manifest: PluginManifestInfo {
                name: p.manifest.name,
                description: Some(p.manifest.description),
                version: Some(p.version),
            },
            installed_at: p.installed_at,
            last_updated: p.last_updated,
            project_path: p.project_path,
        })
        .collect();

    Ok(plugins)
}

// ============================================================================
// Plugin Marketplace Commands
// ============================================================================

/// Add a plugin marketplace
#[tauri::command]
pub async fn plugin_marketplace_add(source: String) -> Result<String, String> {
    validate_plugin_source(&source)?;

    let output = Command::new("claude")
        .args(["plugin", "marketplace", "add", &source])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to add marketplace: {error_msg}"))
    }
}

/// Remove a plugin marketplace
#[tauri::command]
pub async fn plugin_marketplace_remove(name: String) -> Result<String, String> {
    validate_plugin_name(&name)?;

    let output = Command::new("claude")
        .args(["plugin", "marketplace", "remove", &name])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to remove marketplace: {error_msg}"))
    }
}

/// Update plugin marketplaces
#[tauri::command]
pub async fn plugin_marketplace_update(name: Option<String>) -> Result<String, String> {
    if let Some(ref n) = name {
        validate_plugin_name(n)?;
    }

    let mut args = vec!["plugin", "marketplace", "update"];
    if let Some(ref n) = name {
        args.push(n);
    }

    let output = Command::new("claude")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to update marketplace: {error_msg}"))
    }
}

/// Install a plugin
/// For project/local scope, `project_path` must be provided to run CLI from that directory
#[tauri::command]
pub async fn plugin_install(
    plugin: String,
    scope: Option<String>,
    project_path: Option<String>,
) -> Result<String, String> {
    // Validate plugin source (can be name@marketplace format)
    validate_plugin_source(&plugin)?;

    // Validate scope if provided
    if let Some(ref s) = scope {
        validate_scope(s)?;
    }

    let mut args = vec!["plugin", "install"];

    // Add scope if specified (user, project, or local)
    let scope_flag;
    if let Some(ref s) = scope {
        scope_flag = format!("--scope={s}");
        args.push(&scope_flag);
    }

    args.push(&plugin);

    let mut cmd = Command::new("claude");
    cmd.args(&args);

    // For project/local scope, run from the project directory
    if let Some(ref path) = project_path {
        cmd.current_dir(path);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to install plugin: {error_msg}"))
    }
}

/// Uninstall a plugin
/// Note: CLI only accepts plugin name without marketplace
/// For project scope, `project_path` must be provided to run CLI from that directory
#[tauri::command]
pub async fn plugin_uninstall(
    plugin: String,
    scope: Option<String>,
    project_path: Option<String>,
) -> Result<String, String> {
    // Validate plugin source first
    validate_plugin_source(&plugin)?;

    // Validate scope if provided
    if let Some(ref s) = scope {
        validate_scope(s)?;
    }

    // Extract plugin name (without marketplace) for uninstall
    // Format may be "pluginName@marketplace" - uninstall only wants pluginName
    let plugin_name = plugin.split('@').next().unwrap_or(&plugin);

    let mut args = vec!["plugin", "uninstall"];

    // Add scope if specified
    let scope_flag;
    if let Some(ref s) = scope {
        scope_flag = format!("--scope={s}");
        args.push(&scope_flag);
    }

    args.push(plugin_name);

    let mut cmd = Command::new("claude");
    cmd.args(&args);

    // For project scope, run from the project directory
    if let Some(ref path) = project_path {
        cmd.current_dir(path);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };

        // Workaround for Claude CLI bug #14202: CLI doesn't properly handle
        // project-scoped plugin uninstall. Fall back to direct JSON editing.
        if error_msg.contains("not found in installed plugins") {
            return uninstall_plugin_directly(&plugin, scope.as_deref(), project_path.as_deref());
        }

        Err(format!("Failed to uninstall plugin: {error_msg}"))
    }
}

/// Direct uninstall by editing JSON files (workaround for Claude CLI bug #14202)
fn uninstall_plugin_directly(
    plugin: &str,
    scope: Option<&str>,
    project_path: Option<&str>,
) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let installed_file = home
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");

    if !installed_file.exists() {
        return Err("No installed plugins file found".to_string());
    }

    // Read installed_plugins.json
    let content = std::fs::read_to_string(&installed_file)
        .map_err(|e| format!("Failed to read installed plugins: {e}"))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed plugins: {e}"))?;

    let plugins = json
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or("Invalid installed plugins format")?;

    // Find the plugin key (could be "plugin" or "plugin@marketplace")
    let plugin_key = if plugin.contains('@') {
        plugin.to_string()
    } else {
        // Find key that starts with the plugin name
        plugins
            .keys()
            .find(|k| {
                k.starts_with(plugin)
                    && (k.len() == plugin.len() || k.chars().nth(plugin.len()) == Some('@'))
            })
            .cloned()
            .unwrap_or_else(|| plugin.to_string())
    };

    let scope_str = scope.unwrap_or("user");
    let mut removed = false;

    if let Some(installations) = plugins.get_mut(&plugin_key).and_then(|v| v.as_array_mut()) {
        let original_len = installations.len();

        // Filter out the installation matching scope and project_path
        installations.retain(|install| {
            let install_scope = install
                .get("scope")
                .and_then(|s| s.as_str())
                .unwrap_or("user");
            let install_project = install.get("projectPath").and_then(|p| p.as_str());

            // Keep if scope doesn't match
            if install_scope != scope_str {
                return true;
            }

            // For project scope, also check project path
            if scope_str == "project" {
                match (install_project, project_path) {
                    (Some(install_proj), Some(target_proj)) => {
                        // Normalize paths for comparison
                        let install_normalized = install_proj.replace('\\', "/").to_lowercase();
                        let target_normalized = target_proj.replace('\\', "/").to_lowercase();
                        if install_normalized != target_normalized {
                            return true; // Keep - different project
                        }
                        // Paths match - remove this one
                    }
                    (Some(_), None) => {
                        // Project path not provided but entry has one - keep it
                        // (we don't know which project to uninstall from)
                        return true;
                    }
                    (None, _) => {
                        // Entry has no project path - shouldn't happen for project scope, but remove it
                    }
                }
            }

            false // Remove this installation
        });

        removed = installations.len() < original_len;

        // If no installations left, remove the entire plugin entry
        if installations.is_empty() {
            plugins.remove(&plugin_key);
        }
    }

    if !removed {
        return Err(format!("Plugin {plugin} not found for scope {scope_str}"));
    }

    // Write back installed_plugins.json
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize installed plugins: {e}"))?;
    std::fs::write(&installed_file, updated)
        .map_err(|e| format!("Failed to write installed plugins: {e}"))?;

    // Also remove from project settings.json if project scope
    if scope_str == "project" {
        if let Some(proj_path) = project_path {
            let settings_file = std::path::PathBuf::from(proj_path)
                .join(".claude")
                .join("settings.json");
            if settings_file.exists() {
                if let Ok(settings_content) = std::fs::read_to_string(&settings_file) {
                    if let Ok(mut settings_json) =
                        serde_json::from_str::<serde_json::Value>(&settings_content)
                    {
                        if let Some(enabled) = settings_json
                            .get_mut("enabledPlugins")
                            .and_then(|e| e.as_object_mut())
                        {
                            enabled.remove(&plugin_key);
                            // Also try without marketplace suffix
                            let plugin_name = plugin.split('@').next().unwrap_or(plugin);
                            for key in enabled.keys().cloned().collect::<Vec<_>>() {
                                if key.starts_with(plugin_name) {
                                    enabled.remove(&key);
                                }
                            }
                            if let Ok(updated_settings) =
                                serde_json::to_string_pretty(&settings_json)
                            {
                                let _ = std::fs::write(&settings_file, updated_settings);
                            }
                        }
                    }
                }
            }
        }
    }

    // Also remove from user settings.json if user scope
    if scope_str == "user" {
        let user_settings = home.join(".claude").join("settings.json");
        if user_settings.exists() {
            if let Ok(settings_content) = std::fs::read_to_string(&user_settings) {
                if let Ok(mut settings_json) =
                    serde_json::from_str::<serde_json::Value>(&settings_content)
                {
                    if let Some(enabled) = settings_json
                        .get_mut("enabledPlugins")
                        .and_then(|e| e.as_object_mut())
                    {
                        enabled.remove(&plugin_key);
                        if let Ok(updated_settings) = serde_json::to_string_pretty(&settings_json) {
                            let _ = std::fs::write(&user_settings, updated_settings);
                        }
                    }
                }
            }
        }
    }

    Ok(format!("Uninstalled {plugin} (via direct edit workaround)"))
}

/// Move a plugin to a different scope (uninstall + reinstall)
/// Note: uninstall takes just the plugin name, install takes plugin@marketplace
#[tauri::command]
pub async fn plugin_move_scope(
    plugin: String,
    from_scope: String,
    to_scope: String,
) -> Result<String, String> {
    // Validate all inputs
    validate_plugin_source(&plugin)?;
    validate_scope(&from_scope)?;
    validate_scope(&to_scope)?;

    // Extract plugin name (without marketplace) for uninstall
    // Format is "pluginName@marketplace" - uninstall only wants pluginName
    let plugin_name = plugin.split('@').next().unwrap_or(&plugin);

    // First uninstall from current scope (uses just plugin name)
    let uninstall_output = Command::new("claude")
        .args([
            "plugin",
            "uninstall",
            &format!("--scope={from_scope}"),
            plugin_name,
        ])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if !uninstall_output.status.success() {
        let stderr = String::from_utf8_lossy(&uninstall_output.stderr);
        let stdout = String::from_utf8_lossy(&uninstall_output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        return Err(format!(
            "Failed to uninstall from {from_scope} scope: {error_msg}"
        ));
    }

    // Then reinstall at new scope (uses full plugin@marketplace)
    let install_output = Command::new("claude")
        .args(["plugin", "install", &format!("--scope={to_scope}"), &plugin])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if install_output.status.success() {
        Ok(format!(
            "Moved {plugin_name} from {from_scope} to {to_scope} scope"
        ))
    } else {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };

        // Try to restore original installation if reinstall fails
        let _ = Command::new("claude")
            .args([
                "plugin",
                "install",
                &format!("--scope={from_scope}"),
                &plugin,
            ])
            .output();

        Err(format!(
            "Failed to install at {to_scope} scope: {error_msg}"
        ))
    }
}

/// Enable a plugin by setting it to true in enabledPlugins
#[tauri::command]
pub async fn plugin_enable(plugin: String) -> Result<String, String> {
    validate_plugin_source(&plugin)?;
    set_plugin_enabled(&plugin, true)
}

/// Disable a plugin by setting it to false in enabledPlugins
#[tauri::command]
pub async fn plugin_disable(plugin: String) -> Result<String, String> {
    validate_plugin_source(&plugin)?;
    set_plugin_enabled(&plugin, false)
}

/// Set a plugin's enabled state in ~/.claude/settings.json (cross-platform)
fn set_plugin_enabled(plugin: &str, enabled: bool) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let settings_file = home.join(".claude").join("settings.json");

    // Read existing settings or create empty object
    let mut settings: serde_json::Value = if settings_file.exists() {
        let content = std::fs::read_to_string(&settings_file)
            .map_err(|_| "Failed to read settings file".to_string())?;
        serde_json::from_str(&content).map_err(|_| "Failed to parse settings file".to_string())?
    } else {
        serde_json::json!({})
    };

    // Ensure enabledPlugins object exists
    if settings.get("enabledPlugins").is_none() {
        settings["enabledPlugins"] = serde_json::json!({});
    }

    // Set the plugin's enabled state
    if let Some(enabled_plugins) = settings
        .get_mut("enabledPlugins")
        .and_then(|p| p.as_object_mut())
    {
        enabled_plugins.insert(plugin.to_string(), serde_json::Value::Bool(enabled));
    }

    // Write back to file
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|_| "Failed to serialize settings".to_string())?;
    std::fs::write(&settings_file, content)
        .map_err(|_| "Failed to write settings file".to_string())?;

    Ok(format!(
        "Plugin {} {}",
        plugin,
        if enabled { "enabled" } else { "disabled" }
    ))
}

/// Toggle auto-update for a marketplace
#[tauri::command]
pub async fn plugin_marketplace_set_auto_update(
    name: String,
    auto_update: bool,
) -> Result<String, String> {
    validate_plugin_name(&name)?;

    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let marketplaces_file = home
        .join(".claude")
        .join("plugins")
        .join("known_marketplaces.json");

    if !marketplaces_file.exists() {
        return Err("Marketplaces file not found".to_string());
    }

    // Read the file
    let content = std::fs::read_to_string(&marketplaces_file)
        .map_err(|_| "Failed to read marketplaces file".to_string())?;

    // Parse as JSON
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|_| "Failed to parse marketplaces file".to_string())?;

    // Update the autoUpdate field for the marketplace
    if let Some(marketplace) = json.get_mut(&name) {
        if let Some(obj) = marketplace.as_object_mut() {
            obj.insert(
                "autoUpdate".to_string(),
                serde_json::Value::Bool(auto_update),
            );
        } else {
            return Err("Invalid marketplace configuration".to_string());
        }
    } else {
        return Err("Marketplace not found".to_string());
    }

    // Write back
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|_| "Failed to serialize marketplaces".to_string())?;

    std::fs::write(&marketplaces_file, updated)
        .map_err(|_| "Failed to write marketplaces file".to_string())?;

    Ok(format!(
        "Auto-update {} for {}",
        if auto_update { "enabled" } else { "disabled" },
        name
    ))
}

/// Stale cache entry for UI display
#[derive(Debug, Clone, Serialize)]
pub struct CacheEntry {
    pub path: String,
    pub plugin_name: String,
    pub marketplace: String,
    pub version: String,
    pub size_bytes: u64,
}

/// Cache status response for UI
#[derive(Debug, Clone, Serialize)]
pub struct CacheStatusResponse {
    pub stale_entries: Vec<CacheEntry>,
    pub total_size_bytes: u64,
    pub total_size_formatted: String,
    pub installed_count: usize,
}

/// Get cache cleanup status
#[tauri::command]
pub async fn cache_status() -> Result<CacheStatusResponse, String> {
    let report = CacheCleanupReport::scan().map_err(|e| format!("Failed to scan cache: {e}"))?;

    Ok(CacheStatusResponse {
        stale_entries: report
            .stale_entries
            .iter()
            .map(|e| CacheEntry {
                path: e.path.display().to_string(),
                plugin_name: e.plugin_name.clone(),
                marketplace: e.marketplace.clone(),
                version: e.version.clone(),
                size_bytes: e.size_bytes,
            })
            .collect(),
        total_size_bytes: report.total_size_bytes,
        total_size_formatted: report.format_size(),
        installed_count: report.installed_count,
    })
}

/// Clean result response for UI
#[derive(Debug, Clone, Serialize)]
pub struct CacheCleanResult {
    pub deleted_count: usize,
    pub deleted_bytes: u64,
    pub deleted_size_formatted: String,
    pub errors: Vec<String>,
}

/// Clean stale cache entries
#[tauri::command]
pub async fn cache_clean() -> Result<CacheCleanResult, String> {
    let report = CacheCleanupReport::scan().map_err(|e| format!("Failed to scan cache: {e}"))?;

    if report.stale_entries.is_empty() {
        return Ok(CacheCleanResult {
            deleted_count: 0,
            deleted_bytes: 0,
            deleted_size_formatted: "0 bytes".to_string(),
            errors: vec![],
        });
    }

    let result = report
        .clean()
        .map_err(|e| format!("Failed to clean cache: {e}"))?;

    Ok(CacheCleanResult {
        deleted_count: result.deleted_count,
        deleted_bytes: result.deleted_bytes,
        deleted_size_formatted: result.format_size(),
        errors: result.errors,
    })
}

/// Open Terminal with Claude Code and run a specific skill/command
/// The skill should be in format "/plugin-name:skill-name"
#[tauri::command]
pub async fn open_claude_with_skill(skill_invocation: String) -> Result<(), String> {
    // Validate the skill invocation format
    if !skill_invocation.starts_with('/') {
        return Err("Skill invocation must start with /".to_string());
    }

    // Validate for shell safety - only allow safe characters
    let forbidden_chars = [
        '`', '$', '(', ')', '{', '}', '[', ']', '|', ';', '&', '<', '>', '\\', '\n', '\r', '\0',
        '\'', '"', '!', '*', '?',
    ];
    for ch in forbidden_chars {
        if skill_invocation.contains(ch) {
            return Err(format!(
                "Skill invocation contains forbidden character: {ch}"
            ));
        }
    }

    // Copy the skill command to clipboard, then open Terminal with Claude
    // Skills are processed by Claude Code's runtime when typed interactively,
    // not via -p flag, so user needs to paste the command

    // First, copy to clipboard
    let copy_script = format!(r#"set the clipboard to "{skill_invocation}""#);
    Command::new("osascript")
        .args(["-e", &copy_script])
        .output()
        .map_err(|_| "Failed to copy to clipboard".to_string())?;

    // Then open Terminal with Claude and instructions
    let terminal_script = r#"tell application "Terminal"
    activate
    do script "echo '📋 Skill command copied to clipboard - paste it after Claude starts\n' && claude"
end tell"#;

    Command::new("osascript")
        .args(["-e", terminal_script])
        .spawn()
        .map_err(|_| "Failed to open Terminal".to_string())?;

    Ok(())
}
