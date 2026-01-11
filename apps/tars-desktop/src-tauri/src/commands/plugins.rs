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
        .map_err(|_| "Failed to run claude CLI".to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("Failed to add marketplace".to_string())
    }
}

/// Remove a plugin marketplace
#[tauri::command]
pub async fn plugin_marketplace_remove(name: String) -> Result<String, String> {
    validate_plugin_name(&name)?;

    let output = Command::new("claude")
        .args(["plugin", "marketplace", "remove", &name])
        .output()
        .map_err(|_| "Failed to run claude CLI".to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("Failed to remove marketplace".to_string())
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
        .map_err(|_| "Failed to run claude CLI".to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("Failed to update marketplace".to_string())
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
        .map_err(|_| "Failed to run claude CLI".to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("Failed to install plugin".to_string())
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
        Err(format!("Failed to uninstall plugin: {error_msg}"))
    }
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
        .map_err(|_| "Failed to run claude CLI".to_string())?;

    if !uninstall_output.status.success() {
        return Err(format!("Failed to uninstall from {from_scope} scope"));
    }

    // Then reinstall at new scope (uses full plugin@marketplace)
    let install_output = Command::new("claude")
        .args(["plugin", "install", &format!("--scope={to_scope}"), &plugin])
        .output()
        .map_err(|_| "Failed to run claude CLI".to_string())?;

    if install_output.status.success() {
        Ok(format!(
            "Moved {plugin_name} from {from_scope} to {to_scope} scope"
        ))
    } else {
        // Try to restore original installation if reinstall fails
        let _ = Command::new("claude")
            .args([
                "plugin",
                "install",
                &format!("--scope={from_scope}"),
                &plugin,
            ])
            .output();

        Err(format!("Failed to install at {to_scope} scope"))
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
