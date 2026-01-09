//! Plugin management Tauri commands
//!
//! Commands for managing Claude Code plugins via the CLI.

use serde::Serialize;
use std::process::Command;
use tars_scanner::CacheCleanupReport;

/// Add a plugin marketplace
#[tauri::command]
pub async fn plugin_marketplace_add(source: String) -> Result<String, String> {
    let output = Command::new("claude")
        .args(["plugin", "marketplace", "add", &source])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Remove a plugin marketplace
#[tauri::command]
pub async fn plugin_marketplace_remove(name: String) -> Result<String, String> {
    let output = Command::new("claude")
        .args(["plugin", "marketplace", "remove", &name])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Update plugin marketplaces
#[tauri::command]
pub async fn plugin_marketplace_update(name: Option<String>) -> Result<String, String> {
    let mut args = vec!["plugin", "marketplace", "update"];
    if let Some(ref n) = name {
        args.push(n);
    }

    let output = Command::new("claude")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Install a plugin
/// For project/local scope, project_path must be provided to run CLI from that directory
#[tauri::command]
pub async fn plugin_install(
    plugin: String,
    scope: Option<String>,
    project_path: Option<String>,
) -> Result<String, String> {
    let mut args = vec!["plugin", "install"];

    // Add scope if specified (user, project, or local)
    let scope_flag;
    if let Some(ref s) = scope {
        scope_flag = format!("--scope={}", s);
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
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Uninstall a plugin
/// Note: CLI only accepts plugin name without marketplace
#[tauri::command]
pub async fn plugin_uninstall(plugin: String, scope: Option<String>) -> Result<String, String> {
    // Extract plugin name (without marketplace) for uninstall
    // Format may be "pluginName@marketplace" - uninstall only wants pluginName
    let plugin_name = plugin.split('@').next().unwrap_or(&plugin);

    let mut args = vec!["plugin", "uninstall"];

    // Add scope if specified
    let scope_flag;
    if let Some(ref s) = scope {
        scope_flag = format!("--scope={}", s);
        args.push(&scope_flag);
    }

    args.push(plugin_name);

    let output = Command::new("claude")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
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
    // Extract plugin name (without marketplace) for uninstall
    // Format is "pluginName@marketplace" - uninstall only wants pluginName
    let plugin_name = plugin.split('@').next().unwrap_or(&plugin);

    // First uninstall from current scope (uses just plugin name)
    let uninstall_output = Command::new("claude")
        .args(["plugin", "uninstall", &format!("--scope={}", from_scope), plugin_name])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if !uninstall_output.status.success() {
        return Err(format!(
            "Failed to uninstall from {} scope: {}",
            from_scope,
            String::from_utf8_lossy(&uninstall_output.stderr)
        ));
    }

    // Then reinstall at new scope (uses full plugin@marketplace)
    let install_output = Command::new("claude")
        .args(["plugin", "install", &format!("--scope={}", to_scope), &plugin])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if install_output.status.success() {
        Ok(format!("Moved {} from {} to {} scope", plugin_name, from_scope, to_scope))
    } else {
        // Try to restore original installation if reinstall fails
        let _ = Command::new("claude")
            .args(["plugin", "install", &format!("--scope={}", from_scope), &plugin])
            .output();

        Err(format!(
            "Failed to install at {} scope: {}",
            to_scope,
            String::from_utf8_lossy(&install_output.stderr)
        ))
    }
}

/// Enable a plugin
#[tauri::command]
pub async fn plugin_enable(plugin: String) -> Result<String, String> {
    let output = Command::new("claude")
        .args(["plugin", "enable", &plugin])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Disable a plugin
#[tauri::command]
pub async fn plugin_disable(plugin: String) -> Result<String, String> {
    let output = Command::new("claude")
        .args(["plugin", "disable", &plugin])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Toggle auto-update for a marketplace
#[tauri::command]
pub async fn plugin_marketplace_set_auto_update(
    name: String,
    auto_update: bool,
) -> Result<String, String> {
    let home = std::env::var("HOME")
        .map_err(|_| "Could not find HOME environment variable")?;
    let marketplaces_file = std::path::PathBuf::from(home)
        .join(".claude")
        .join("plugins")
        .join("known_marketplaces.json");

    if !marketplaces_file.exists() {
        return Err("Marketplaces file not found".to_string());
    }

    // Read the file
    let content = std::fs::read_to_string(&marketplaces_file)
        .map_err(|e| format!("Failed to read marketplaces file: {}", e))?;

    // Parse as JSON
    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Update the autoUpdate field for the marketplace
    if let Some(marketplace) = json.get_mut(&name) {
        if let Some(obj) = marketplace.as_object_mut() {
            obj.insert("autoUpdate".to_string(), serde_json::Value::Bool(auto_update));
        } else {
            return Err(format!("Marketplace '{}' is not an object", name));
        }
    } else {
        return Err(format!("Marketplace '{}' not found", name));
    }

    // Write back
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    std::fs::write(&marketplaces_file, updated)
        .map_err(|e| format!("Failed to write marketplaces file: {}", e))?;

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
    let report = CacheCleanupReport::scan()
        .map_err(|e| format!("Failed to scan cache: {}", e))?;

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
    let report = CacheCleanupReport::scan()
        .map_err(|e| format!("Failed to scan cache: {}", e))?;

    if report.stale_entries.is_empty() {
        return Ok(CacheCleanResult {
            deleted_count: 0,
            deleted_bytes: 0,
            deleted_size_formatted: "0 bytes".to_string(),
            errors: vec![],
        });
    }

    let result = report.clean()
        .map_err(|e| format!("Failed to clean cache: {}", e))?;

    Ok(CacheCleanResult {
        deleted_count: result.deleted_count,
        deleted_bytes: result.deleted_bytes,
        deleted_size_formatted: result.format_size(),
        errors: result.errors,
    })
}
