//! Update checking Tauri commands
//!
//! Commands for checking Claude Code version, fetching changelog,
//! and TARS app updates.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tars_scanner::plugins::PluginInventory;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

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
    // GUI apps don't inherit shell PATH, so we check common installation locations
    let claude_paths = get_claude_binary_paths();

    for path in claude_paths {
        if let Ok(output) = Command::new(&path).arg("--version").output() {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout);
                // Parse "2.1.3 (Claude Code)" -> "2.1.3"
                let version = version_str
                    .split_whitespace()
                    .next()
                    .map(std::string::ToString::to_string);
                return Ok(version);
            }
        }
    }

    Ok(None) // Claude not installed or not found
}

/// Get possible paths where the claude binary might be installed
fn get_claude_binary_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Platform-specific paths
    #[cfg(target_os = "windows")]
    {
        add_windows_paths(&mut paths);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        add_unix_paths(&mut paths);
    }

    #[cfg(target_os = "macos")]
    {
        add_macos_paths(&mut paths);
    }

    #[cfg(target_os = "linux")]
    {
        add_linux_paths(&mut paths);
    }

    // Always try bare "claude" in case PATH is available
    #[cfg(target_os = "windows")]
    {
        paths.push(PathBuf::from("claude.cmd"));
        paths.push(PathBuf::from("claude.exe"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        paths.push(PathBuf::from("claude"));
    }

    paths
}

/// Add Windows-specific paths
#[cfg(target_os = "windows")]
fn add_windows_paths(paths: &mut Vec<PathBuf>) {
    // USERPROFILE (Windows home directory)
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let home = PathBuf::from(&userprofile);
        // Scoop
        paths.push(home.join("scoop").join("shims").join("claude.exe"));
    }

    // APPDATA (npm global)
    if let Ok(appdata) = std::env::var("APPDATA") {
        let appdata = PathBuf::from(&appdata);
        paths.push(appdata.join("npm").join("claude.cmd"));
    }

    // LOCALAPPDATA (Volta, per-user installs)
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let local = PathBuf::from(&localappdata);
        // Volta
        paths.push(local.join("Volta").join("bin").join("claude.exe"));
        // Per-user installer locations
        paths.push(local.join("Programs").join("Claude").join("claude.exe"));
        paths.push(
            local
                .join("Programs")
                .join("Claude Code")
                .join("claude.exe"),
        );
    }

    // Chocolatey
    if let Ok(programdata) = std::env::var("ProgramData") {
        paths.push(
            PathBuf::from(programdata)
                .join("chocolatey")
                .join("bin")
                .join("claude.exe"),
        );
    }

    // Program Files (machine-wide installs)
    if let Ok(programfiles) = std::env::var("ProgramFiles") {
        paths.push(
            PathBuf::from(&programfiles)
                .join("Claude")
                .join("claude.exe"),
        );
    }
    if let Ok(programfiles86) = std::env::var("ProgramFiles(x86)") {
        paths.push(
            PathBuf::from(&programfiles86)
                .join("Claude")
                .join("claude.exe"),
        );
    }
}

/// Add Unix-specific paths (macOS and Linux)
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn add_unix_paths(paths: &mut Vec<PathBuf>) {
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(&home);

        // npm global install locations
        paths.push(home.join(".local").join("bin").join("claude"));
        paths.push(home.join(".npm-global").join("bin").join("claude"));

        // Volta
        paths.push(home.join(".volta").join("bin").join("claude"));

        // pnpm
        paths.push(
            home.join(".local")
                .join("share")
                .join("pnpm")
                .join("claude"),
        );

        // Yarn
        paths.push(home.join(".yarn").join("bin").join("claude"));
        paths.push(
            home.join(".config")
                .join("yarn")
                .join("global")
                .join("node_modules")
                .join(".bin")
                .join("claude"),
        );

        // asdf
        paths.push(home.join(".asdf").join("shims").join("claude"));

        // mise
        paths.push(
            home.join(".local")
                .join("share")
                .join("mise")
                .join("shims")
                .join("claude"),
        );
    }

    // System paths
    paths.push(PathBuf::from("/usr/local/bin/claude"));
    paths.push(PathBuf::from("/usr/bin/claude"));
}

/// Add macOS-specific paths
#[cfg(target_os = "macos")]
fn add_macos_paths(paths: &mut Vec<PathBuf>) {
    // Homebrew on Apple Silicon
    paths.push(PathBuf::from("/opt/homebrew/bin/claude"));
}

/// Add Linux-specific paths
#[cfg(target_os = "linux")]
fn add_linux_paths(paths: &mut Vec<PathBuf>) {
    // Linuxbrew
    paths.push(PathBuf::from("/home/linuxbrew/.linuxbrew/bin/claude"));

    // Snap
    paths.push(PathBuf::from("/snap/bin/claude"));

    // Flatpak
    if let Ok(home) = std::env::var("HOME") {
        paths.push(
            PathBuf::from(&home)
                .join(".local")
                .join("share")
                .join("flatpak")
                .join("exports")
                .join("bin")
                .join("claude"),
        );
    }
    paths.push(PathBuf::from("/var/lib/flatpak/exports/bin/claude"));
}

/// Fetch and parse the Claude Code changelog from GitHub
#[tauri::command]
pub async fn fetch_claude_changelog() -> Result<ChangelogResponse, String> {
    let url = "https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md";

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch changelog: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch changelog: HTTP {}",
            response.status()
        ));
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
        let available_version = available
            .version
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let update_available = if available_version != "unknown" && installed_version != "unknown" {
            version_compare(&available_version, &installed_version) == std::cmp::Ordering::Greater
        } else {
            false
        };

        updates.push(PluginUpdateInfo {
            plugin_id: installed.id.clone(),
            plugin_name: installed.manifest.name.clone(),
            marketplace: marketplace_name.clone(),
            installed_version,
            available_version,
            update_available,
        });
    }

    let plugins_with_updates = updates.iter().filter(|u| u.update_available).count();
    let total_plugins = updates.len();

    // Sort: updates first, then alphabetically
    updates.sort_by(|a, b| match (a.update_available, b.update_available) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.plugin_name.cmp(&b.plugin_name),
    });

    Ok(PluginUpdatesResponse {
        updates,
        total_plugins,
        plugins_with_updates,
    })
}

// ============================================================================
// TARS App Updates
// ============================================================================

/// Information about a TARS app update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarsUpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
}

/// Check for TARS app updates
#[tauri::command]
pub async fn check_tars_update(app: AppHandle) -> Result<TarsUpdateInfo, String> {
    let current_version = app.package_info().version.to_string();

    // Check for updates using the updater plugin
    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| format!("Failed to initialize updater: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => Ok(TarsUpdateInfo {
            current_version,
            latest_version: Some(update.version.clone()),
            update_available: true,
            release_notes: update.body.clone(),
            download_url: Some(update.download_url.to_string()),
        }),
        Ok(None) => Ok(TarsUpdateInfo {
            current_version,
            latest_version: None,
            update_available: false,
            release_notes: None,
            download_url: None,
        }),
        Err(e) => {
            // Return current version info even if check fails
            // This can happen if offline or endpoint is unavailable
            eprintln!("Update check failed: {e}");
            Ok(TarsUpdateInfo {
                current_version,
                latest_version: None,
                update_available: false,
                release_notes: None,
                download_url: None,
            })
        }
    }
}

/// Download and install TARS app update
#[tauri::command]
pub async fn install_tars_update(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| format!("Failed to initialize updater: {e}"))?;

    let update = updater
        .check()
        .await
        .map_err(|e| format!("Failed to check for update: {e}"))?
        .ok_or_else(|| "No update available".to_string())?;

    // Download and install the update
    let mut downloaded = 0;
    update
        .download_and_install(
            |chunk_length, content_length| {
                downloaded += chunk_length;
                if let Some(total) = content_length {
                    eprintln!("Download progress: {downloaded}/{total}");
                }
            },
            || {
                eprintln!("Download finished, preparing to install...");
            },
        )
        .await
        .map_err(|e| format!("Failed to download and install update: {e}"))?;

    Ok(())
}

/// Get the current TARS app version
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned AppHandle
pub fn get_tars_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}
