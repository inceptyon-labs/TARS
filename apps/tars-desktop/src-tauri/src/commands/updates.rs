//! Update checking Tauri commands
//!
//! Commands for checking Claude Code version and fetching changelog.

use serde::{Deserialize, Serialize};
use std::process::Command;
use tars_scanner::plugins::PluginInventory;

/// Version info for Claude Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeVersionInfo {
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

/// A single changelog entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub version: String,
    pub content: String,
}

/// Full changelog response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogResponse {
    pub entries: Vec<ChangelogEntry>,
    pub raw_content: String,
    pub fetched_at: String,
}

/// Get the installed Claude Code version
#[tauri::command]
pub async fn get_installed_claude_version() -> Result<Option<String>, String> {
    // Try to run `claude --version`
    let output = Command::new("claude")
        .arg("--version")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout);
                // Parse "2.1.3 (Claude Code)" -> "2.1.3"
                let version = version_str
                    .trim()
                    .split_whitespace()
                    .next()
                    .map(|s| s.to_string());
                Ok(version)
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None), // Claude not installed or not in PATH
    }
}

/// Fetch and parse the Claude Code changelog from GitHub
#[tauri::command]
pub async fn fetch_claude_changelog() -> Result<ChangelogResponse, String> {
    let url = "https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md";

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch changelog: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch changelog: HTTP {}", response.status()));
    }

    let content = response
        .text()
        .await
        .map_err(|e| format!("Failed to read changelog: {e}"))?;

    let entries = parse_changelog(&content);
    let fetched_at = chrono::Utc::now().to_rfc3339();

    Ok(ChangelogResponse {
        entries,
        raw_content: content,
        fetched_at,
    })
}

/// Parse changelog markdown into version entries
fn parse_changelog(content: &str) -> Vec<ChangelogEntry> {
    let mut entries = Vec::new();
    let mut current_version: Option<String> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        // Check for version header (## 2.1.3 or ## [2.1.3])
        if line.starts_with("## ") {
            // Save previous entry if exists
            if let Some(version) = current_version.take() {
                entries.push(ChangelogEntry {
                    version,
                    content: current_content.trim().to_string(),
                });
                current_content.clear();
            }

            // Extract version number
            let version_part = line.trim_start_matches("## ").trim();
            // Handle both "2.1.3" and "[2.1.3]" formats
            let version = version_part
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split_whitespace()
                .next()
                .unwrap_or(version_part)
                .to_string();

            current_version = Some(version);
        } else if current_version.is_some() {
            // Accumulate content for current version
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Don't forget the last entry
    if let Some(version) = current_version {
        entries.push(ChangelogEntry {
            version,
            content: current_content.trim().to_string(),
        });
    }

    entries
}

/// Get version info comparing installed vs latest
#[tauri::command]
pub async fn get_claude_version_info() -> Result<ClaudeVersionInfo, String> {
    let installed = get_installed_claude_version().await?;

    // Fetch changelog to get latest version
    let changelog = fetch_claude_changelog().await.ok();
    let latest = changelog
        .as_ref()
        .and_then(|c| c.entries.first())
        .map(|e| e.version.clone());

    let update_available = match (&installed, &latest) {
        (Some(inst), Some(lat)) => {
            // Simple version comparison - could be improved with semver
            inst != lat && version_compare(lat, inst) == std::cmp::Ordering::Greater
        }
        _ => false,
    };

    Ok(ClaudeVersionInfo {
        installed_version: installed,
        latest_version: latest,
        update_available,
    })
}

/// Compare semantic versions
fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let parse_version = |s: &str| -> Vec<u32> {
        s.split('.')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect()
    };

    let va = parse_version(a);
    let vb = parse_version(b);

    for (pa, pb) in va.iter().zip(vb.iter()) {
        match pa.cmp(pb) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    va.len().cmp(&vb.len())
}

// ============================================================================
// Plugin Updates
// ============================================================================

/// Information about a plugin update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUpdateInfo {
    pub plugin_id: String,
    pub plugin_name: String,
    pub marketplace: String,
    pub installed_version: String,
    pub available_version: String,
    pub update_available: bool,
}

/// Response for plugin updates check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUpdatesResponse {
    pub updates: Vec<PluginUpdateInfo>,
    pub total_plugins: usize,
    pub plugins_with_updates: usize,
}

/// Check for plugin updates by comparing installed versions with marketplace versions
#[tauri::command]
pub async fn check_plugin_updates() -> Result<PluginUpdatesResponse, String> {
    let inventory = PluginInventory::scan().map_err(|e: tars_scanner::ScanError| e.to_string())?;

    let mut updates = Vec::new();

    for installed in &inventory.installed {
        // Skip plugins without a marketplace (local installs)
        let Some(ref marketplace_name) = installed.marketplace else {
            continue;
        };
        let marketplace_name: &String = marketplace_name;

        // Find the marketplace
        let Some(marketplace) = inventory
            .marketplaces
            .iter()
            .find(|m| &m.name == marketplace_name)
        else {
            continue;
        };

        // Find the available plugin in the marketplace
        let Some(available) = marketplace
            .available_plugins
            .iter()
            .find(|p| p.id == installed.id)
        else {
            continue;
        };

        // Compare versions
        let installed_version = installed.manifest.version.clone();
        let available_version = available.version.clone().unwrap_or_else(|| "unknown".to_string());

        let update_available = if available_version != "unknown" && installed_version != "unknown" {
            version_compare(&available_version, &installed_version) == std::cmp::Ordering::Greater
        } else {
            false
        };

        updates.push(PluginUpdateInfo {
            plugin_id: installed.id.clone(),
            plugin_name: installed.manifest.name.clone(),
            marketplace: marketplace_name.to_string(),
            installed_version,
            available_version,
            update_available,
        });
    }

    let plugins_with_updates = updates.iter().filter(|u| u.update_available).count();
    let total_plugins = updates.len();

    // Sort: updates first, then alphabetically
    updates.sort_by(|a, b| {
        match (a.update_available, b.update_available) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.plugin_name.cmp(&b.plugin_name),
        }
    });

    Ok(PluginUpdatesResponse {
        updates,
        total_plugins,
        plugins_with_updates,
    })
}
