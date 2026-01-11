//! Utility Tauri commands
//!
//! Commands for file dialogs, path operations, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Directory info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub path: String,
    pub name: String,
    pub has_claude_config: bool,
    pub is_git_repo: bool,
}

/// Check if a directory exists
#[tauri::command]
pub async fn directory_exists(path: String) -> bool {
    let p = PathBuf::from(&path);
    p.exists() && p.is_dir()
}

/// Get directory info
#[tauri::command]
pub async fn get_directory_info(path: String) -> Result<DirectoryInfo, String> {
    let p = PathBuf::from(&path);

    if !p.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    if !p.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }

    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let has_claude_config = p.join(".claude").exists() || p.join("CLAUDE.md").exists();
    let is_git_repo = p.join(".git").exists();

    Ok(DirectoryInfo {
        path,
        name,
        has_claude_config,
        is_git_repo,
    })
}

/// Get home directory path
#[tauri::command]
pub async fn get_home_dir() -> Result<String, String> {
    std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())
}

/// List subdirectories in a path
#[tauri::command]
pub async fn list_subdirectories(path: String) -> Result<Vec<DirectoryInfo>, String> {
    let p = PathBuf::from(&path);

    if !p.exists() || !p.is_dir() {
        return Err(format!("Invalid directory: {path}"));
    }

    let mut dirs = Vec::new();

    let entries = std::fs::read_dir(&p).map_err(|e| format!("Failed to read directory: {e}"))?;

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            // Skip hidden directories (except .claude)
            let name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if name.starts_with('.') && name != ".claude" {
                continue;
            }

            let has_claude_config =
                entry_path.join(".claude").exists() || entry_path.join("CLAUDE.md").exists();
            let is_git_repo = entry_path.join(".git").exists();

            dirs.push(DirectoryInfo {
                path: entry_path.display().to_string(),
                name,
                has_claude_config,
                is_git_repo,
            });
        }
    }

    // Sort by name
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(dirs)
}

/// Get app version
#[tauri::command]
pub async fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Platform information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub display: String,
}

/// Get platform info (OS and architecture)
#[tauri::command]
pub async fn get_platform_info() -> PlatformInfo {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let display = match (os, arch) {
        ("macos", "aarch64") => "macOS (Apple Silicon)".to_string(),
        ("macos", "x86_64") => "macOS (Intel)".to_string(),
        ("windows", "x86_64") => "Windows (x64)".to_string(),
        ("windows", "aarch64") => "Windows (ARM64)".to_string(),
        ("linux", "x86_64") => "Linux (x64)".to_string(),
        ("linux", "aarch64") => "Linux (ARM64)".to_string(),
        _ => format!("{os} ({arch})"),
    };

    PlatformInfo {
        os: os.to_string(),
        arch: arch.to_string(),
        display,
    }
}

// ============================================================================
// Claude Code Usage Stats
// ============================================================================

/// Daily activity from Claude Code stats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyActivity {
    pub date: String,
    pub message_count: u64,
    pub session_count: u64,
    pub tool_call_count: u64,
}

/// Daily token usage by model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyModelTokens {
    pub date: String,
    pub tokens_by_model: HashMap<String, u64>,
}

/// Lifetime usage stats for a model
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)] // Matches external JSON format from Claude
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

/// Claude Code usage statistics from ~/.claude/stats-cache.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageStats {
    pub total_sessions: u64,
    pub total_messages: u64,
    pub first_session_date: Option<String>,
    pub last_computed_date: Option<String>,
    pub daily_activity: Vec<DailyActivity>,
    pub daily_model_tokens: Vec<DailyModelTokens>,
    pub model_usage: HashMap<String, ModelUsage>,
    pub hour_counts: HashMap<String, u64>,
}

/// Raw stats cache format (matches the JSON file structure)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStatsCache {
    #[serde(default)]
    #[allow(dead_code)]
    version: u32,
    last_computed_date: Option<String>,
    #[serde(default)]
    daily_activity: Vec<RawDailyActivity>,
    #[serde(default)]
    daily_model_tokens: Vec<RawDailyModelTokens>,
    #[serde(default)]
    model_usage: HashMap<String, RawModelUsage>,
    #[serde(default)]
    total_sessions: u64,
    #[serde(default)]
    total_messages: u64,
    first_session_date: Option<String>,
    #[serde(default)]
    hour_counts: HashMap<String, u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDailyActivity {
    date: String,
    #[serde(default)]
    message_count: u64,
    #[serde(default)]
    session_count: u64,
    #[serde(default)]
    tool_call_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDailyModelTokens {
    date: String,
    #[serde(default)]
    tokens_by_model: HashMap<String, u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)] // Matches external JSON format from Claude
struct RawModelUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
}

/// Get Claude Code usage statistics from the local stats cache
#[tauri::command]
pub async fn get_claude_usage_stats() -> Result<ClaudeUsageStats, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    let stats_path = PathBuf::from(&home)
        .join(".claude")
        .join("stats-cache.json");

    if !stats_path.exists() {
        return Err("Claude Code stats file not found. Have you used Claude Code yet?".to_string());
    }

    let content = std::fs::read_to_string(&stats_path)
        .map_err(|e| format!("Failed to read stats file: {e}"))?;

    let raw: RawStatsCache =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse stats file: {e}"))?;

    // Convert raw format to our output format
    let daily_activity: Vec<DailyActivity> = raw
        .daily_activity
        .into_iter()
        .map(|d| DailyActivity {
            date: d.date,
            message_count: d.message_count,
            session_count: d.session_count,
            tool_call_count: d.tool_call_count,
        })
        .collect();

    let daily_model_tokens: Vec<DailyModelTokens> = raw
        .daily_model_tokens
        .into_iter()
        .map(|d| DailyModelTokens {
            date: d.date,
            tokens_by_model: d.tokens_by_model,
        })
        .collect();

    let model_usage: HashMap<String, ModelUsage> = raw
        .model_usage
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                ModelUsage {
                    input_tokens: v.input_tokens,
                    output_tokens: v.output_tokens,
                    cache_read_input_tokens: v.cache_read_input_tokens,
                    cache_creation_input_tokens: v.cache_creation_input_tokens,
                },
            )
        })
        .collect();

    Ok(ClaudeUsageStats {
        total_sessions: raw.total_sessions,
        total_messages: raw.total_messages,
        first_session_date: raw.first_session_date,
        last_computed_date: raw.last_computed_date,
        daily_activity,
        daily_model_tokens,
        model_usage,
        hour_counts: raw.hour_counts,
    })
}
