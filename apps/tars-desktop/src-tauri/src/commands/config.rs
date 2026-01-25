//! Config operations Tauri commands
//!
//! Exposes config CRUD operations to the React frontend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

use tars_core::backup::restore::restore_from_backup;
use tars_core::config::{
    ConfigError, ConfigItemData, ConfigScope, McpOps, McpServerConfig, McpServerUpdate,
    McpTransport,
};
use tars_core::storage::BackupStore;
use uuid::Uuid;

use crate::state::AppState;

// ============================================================================
// MCP Server Commands
// ============================================================================

/// MCP server item for list results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerItem {
    pub name: String,
    pub scope: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// If set, this server comes from a plugin and is read-only
    #[serde(rename = "sourcePlugin", skip_serializing_if = "Option::is_none")]
    pub source_plugin: Option<String>,
    /// Optional documentation/project page URL
    #[serde(rename = "docsUrl", skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

/// Result from `mcp_list` command
#[derive(Debug, Serialize)]
pub struct McpListResult {
    pub servers: Vec<McpServerItem>,
}

/// Parameters for `mcp_add` command
#[derive(Debug, Deserialize)]
pub struct McpAddParams {
    pub name: String,
    pub scope: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    #[serde(rename = "docsUrl")]
    pub docs_url: Option<String>,
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

/// Result from `mcp_add/remove/update` commands
#[derive(Debug, Serialize)]
pub struct McpOperationResult {
    pub success: bool,
    #[serde(rename = "backupId", skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result from `mcp_refresh` command
#[derive(Debug, Serialize)]
pub struct McpRefreshResult {
    pub success: bool,
    #[serde(rename = "serverName")]
    pub server_name: String,
    #[serde(rename = "refreshType")]
    pub refresh_type: String, // "npm_install", "git_pull", "npx_skip", "unknown"
    #[serde(rename = "commandRun", skip_serializing_if = "Option::is_none")]
    pub command_run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// List all MCP servers
#[tauri::command]
pub async fn mcp_list(
    project_path: Option<String>,
    _state: State<'_, AppState>,
) -> Result<McpListResult, String> {
    let ops = McpOps::new(project_path.map(PathBuf::from));

    let items = ops.list().map_err(|e| e.to_string())?;

    let servers: Vec<McpServerItem> = items
        .into_iter()
        .filter_map(|item| {
            if let ConfigItemData::McpServer(config) = item.config {
                Some(McpServerItem {
                    name: item.name,
                    scope: item.scope.to_string(),
                    transport: format!("{:?}", config.transport).to_lowercase(),
                    command: config.command,
                    args: config.args,
                    env: config.env,
                    url: config.url,
                    file_path: item.file_path.display().to_string(),
                    source_plugin: item.source_plugin,
                    docs_url: config.docs_url,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(McpListResult { servers })
}

/// Add a new MCP server
#[tauri::command]
pub async fn mcp_add(
    params: McpAddParams,
    project_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<McpOperationResult, String> {
    // Parse transport type
    let transport = match params.transport.as_str() {
        "stdio" => McpTransport::Stdio,
        "http" => McpTransport::Http,
        "sse" => McpTransport::Sse,
        other => return Err(format!("Invalid transport type: {other}")),
    };

    // Parse scope
    let scope: ConfigScope = params.scope.parse().map_err(|e| format!("{e}"))?;

    // Build server config
    let config = McpServerConfig {
        transport,
        command: params.command,
        args: params.args.unwrap_or_default(),
        env: params.env.unwrap_or_default(),
        url: params.url,
        docs_url: params.docs_url,
    };

    // Validate config
    config.validate().map_err(|e| e.clone())?;

    // Set up backup directory
    let backup_dir = state.data_dir().join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {e}"))?;

    // Create operations manager
    let ops = McpOps::new(project_path.map(PathBuf::from)).with_backup_dir(backup_dir);

    let dry_run = params.dry_run.unwrap_or(false);
    let result = ops
        .add(&params.name, scope, config, dry_run)
        .map_err(|e| e.to_string())?;

    Ok(McpOperationResult {
        success: result.success,
        backup_id: result.backup_id,
        file_path: result
            .files_modified
            .first()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        diff: None,
        error: result.error,
    })
}

/// Parameters for `mcp_remove` command
#[derive(Debug, Deserialize)]
pub struct McpRemoveParams {
    pub name: String,
    pub scope: Option<String>,
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

/// Remove an MCP server
#[tauri::command]
pub async fn mcp_remove(
    params: McpRemoveParams,
    project_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<McpOperationResult, String> {
    // Parse scope if provided
    let scope_filter: Option<ConfigScope> = if let Some(s) = &params.scope {
        Some(s.parse().map_err(|e| format!("{e}"))?)
    } else {
        None
    };

    // Set up backup directory
    let backup_dir = state.data_dir().join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {e}"))?;

    // Create operations manager
    let ops = McpOps::new(project_path.map(PathBuf::from)).with_backup_dir(backup_dir);

    let dry_run = params.dry_run.unwrap_or(false);
    let result = ops
        .remove(&params.name, scope_filter, dry_run)
        .map_err(|e| e.to_string())?;

    Ok(McpOperationResult {
        success: result.success,
        backup_id: result.backup_id,
        file_path: result
            .files_modified
            .first()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        diff: None,
        error: result.error,
    })
}

/// Parameters for `mcp_update` command
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct McpUpdateParams {
    pub name: String,
    pub scope: Option<String>,
    pub updates: HashMap<String, serde_json::Value>,
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

/// Update an MCP server
#[tauri::command]
pub async fn mcp_update(
    params: McpUpdateParams,
    project_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<McpOperationResult, String> {
    // Parse scope if provided
    let scope = params
        .scope
        .as_ref()
        .map(|s| s.parse::<ConfigScope>())
        .transpose()
        .map_err(|e| format!("{e}"))?;

    // Build update struct from HashMap
    let mut update = McpServerUpdate::default();

    // Helper to extract string values
    let get_string = |key: &str| {
        params
            .updates
            .get(key)
            .and_then(|v| v.as_str())
            .map(String::from)
    };

    // Helper to extract string arrays
    let get_string_array = |key: &str| {
        params
            .updates
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
    };

    // Helper to extract object as HashMap
    let get_object = |key: &str| {
        params
            .updates
            .get(key)
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect::<HashMap<_, _>>()
            })
    };

    // Extract fields from updates
    if let Some(command) = get_string("command") {
        update.command = Some(command);
    }
    if let Some(args) = get_string_array("args") {
        if !args.is_empty() {
            update.args = Some(args);
        }
    }
    if let Some(add_args) = get_string_array("addArgs") {
        if !add_args.is_empty() {
            update.add_args = Some(add_args);
        }
    }
    if let Some(env) = get_object("env") {
        if !env.is_empty() {
            update.env = Some(env);
        }
    }
    if let Some(add_env) = get_object("addEnv") {
        if !add_env.is_empty() {
            update.add_env = Some(add_env);
        }
    }
    if let Some(remove_env) = get_string_array("removeEnv") {
        if !remove_env.is_empty() {
            update.remove_env = Some(remove_env);
        }
    }
    if let Some(url) = get_string("url") {
        update.url = Some(url);
    }

    // Set up backup directory
    let backup_dir = state.data_dir().join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {e}"))?;

    // Create operations manager
    let ops = McpOps::new(project_path.map(PathBuf::from)).with_backup_dir(backup_dir);

    let dry_run = params.dry_run.unwrap_or(false);
    let result = ops
        .update(&params.name, scope, update, dry_run)
        .map_err(|e| e.to_string())?;

    Ok(McpOperationResult {
        success: result.success,
        backup_id: result.backup_id,
        file_path: result
            .files_modified
            .first()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        diff: None,
        error: result.error,
    })
}

/// Parameters for `mcp_move` command
#[derive(Debug, Deserialize)]
pub struct McpMoveParams {
    pub name: String,
    #[serde(rename = "fromScope")]
    pub from_scope: Option<String>,
    #[serde(rename = "toScope")]
    pub to_scope: String,
    #[allow(dead_code)]
    pub force: Option<bool>,
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

/// Result from `mcp_move` command
#[derive(Debug, Serialize)]
pub struct McpMoveResult {
    pub success: bool,
    #[serde(rename = "backupId", skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    #[serde(rename = "removedFrom")]
    pub removed_from: String,
    #[serde(rename = "addedTo")]
    pub added_to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Move an MCP server between scopes
#[tauri::command]
pub async fn mcp_move(
    params: McpMoveParams,
    project_path: Option<String>,
    _state: State<'_, AppState>,
) -> Result<McpMoveResult, String> {
    let backup_dir = dirs::home_dir().map_or_else(
        || std::env::temp_dir().join("tars").join("backups"),
        |h| h.join(".tars").join("backups"),
    );

    let ops = McpOps::new(project_path.map(PathBuf::from)).with_backup_dir(backup_dir);

    let from_scope: Option<ConfigScope> = params
        .from_scope
        .as_ref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: tars_core::config::ConfigError| e.to_string())?;

    let to_scope: ConfigScope = params
        .to_scope
        .parse()
        .map_err(|e: tars_core::config::ConfigError| e.to_string())?;

    let dry_run = params.dry_run.unwrap_or(false);
    let force = params.force.unwrap_or(false);

    // Try the move operation
    let result = match ops.move_server(&params.name, from_scope, to_scope, dry_run) {
        Ok(r) => r,
        Err(ConfigError::ItemExists { name: _, scope: _ }) if force && !dry_run => {
            // With --force: remove existing server from target, then retry move
            // Use original params.name to ensure we remove the correct server
            ops.remove(&params.name, Some(to_scope), false)
                .map_err(|e| e.to_string())?;
            ops.move_server(&params.name, from_scope, to_scope, false)
                .map_err(|e| e.to_string())?
        }
        Err(e) => return Err(e.to_string()),
    };

    Ok(McpMoveResult {
        success: result.success,
        backup_id: result.backup_id,
        removed_from: from_scope.map_or_else(|| "auto".to_string(), |s| s.to_string()),
        added_to: to_scope.to_string(),
        diff: None,
        error: result.error,
    })
}

// ============================================================================
// Rollback Command
// ============================================================================

/// Parameters for `config_rollback` command
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RollbackParams {
    #[serde(rename = "backupId")]
    pub backup_id: String,
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
}

/// Result from `config_rollback` command
#[derive(Debug, Serialize)]
pub struct RollbackResult {
    pub success: bool,
    #[serde(rename = "filesRestored")]
    pub files_restored: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Rollback a config operation
#[tauri::command]
pub async fn config_rollback(
    params: RollbackParams,
    state: State<'_, AppState>,
) -> Result<RollbackResult, String> {
    // Parse backup ID
    let backup_id = Uuid::parse_str(&params.backup_id)
        .map_err(|_| format!("Invalid backup ID: {}", params.backup_id))?;

    // Get backup from database
    let backup = state.with_db(|db| {
        let store = BackupStore::new(db.connection());
        store
            .get(backup_id)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| format!("Backup not found: {}", params.backup_id))
    })?;

    // Determine project path
    let project_path = if let Some(path) = params.project_path {
        PathBuf::from(path)
    } else {
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))?
    };

    // Log rollback operation start (audit trail)
    let project_path_display = project_path.display();
    let files_count = backup.files.len();
    eprintln!(
        "[AUDIT] config_rollback: Starting rollback backup_id={backup_id} project_path={project_path_display} files_count={files_count}"
    );

    // Restore from backup
    if let Err(e) = restore_from_backup(&project_path, &backup) {
        eprintln!("[AUDIT] config_rollback: FAILED backup_id={backup_id} error={e}");
        return Err(format!("Restore failed: {e}"));
    }

    // Collect restored files
    let files_restored: Vec<String> = backup
        .files
        .iter()
        .map(|f| f.path.to_string_lossy().into_owned())
        .collect();

    // Log rollback operation completion (audit trail)
    eprintln!(
        "[AUDIT] config_rollback: SUCCESS backup_id={backup_id} files_restored={files_restored:?}"
    );

    Ok(RollbackResult {
        success: true,
        files_restored,
        error: None,
    })
}

// ============================================================================
// MCP Refresh Command
// ============================================================================

/// Strategy for refreshing an MCP server
#[derive(Debug)]
enum McpRefreshStrategy {
    /// npx always fetches the latest version
    NpxSkip,
    /// Run `npm install` in the given directory
    NpmInstall(PathBuf),
    /// Run `git pull` then `npm install` in the given directory
    GitPullNpmInstall(PathBuf),
    /// Unknown server type - cannot refresh
    Unknown,
}

/// Detect the refresh strategy based on command and args
fn detect_refresh_strategy(command: Option<&str>, args: &[String]) -> McpRefreshStrategy {
    let cmd = match command {
        Some(c) => c,
        None => return McpRefreshStrategy::Unknown,
    };

    // Check for npx command
    if cmd == "npx" || cmd.ends_with("/npx") {
        return McpRefreshStrategy::NpxSkip;
    }

    // Check for node running a local file
    if cmd == "node" || cmd.ends_with("/node") {
        if let Some(script_path) = args.first() {
            let path = PathBuf::from(script_path);
            if path.is_absolute() {
                // Find the directory containing package.json
                if let Some(project_dir) = find_npm_project_dir(&path) {
                    // Check if it's a git repo
                    if project_dir.join(".git").exists() {
                        return McpRefreshStrategy::GitPullNpmInstall(project_dir);
                    }
                    return McpRefreshStrategy::NpmInstall(project_dir);
                }
            }
        }
        return McpRefreshStrategy::Unknown;
    }

    // Check for absolute path to a binary
    let cmd_path = PathBuf::from(cmd);
    if cmd_path.is_absolute() && cmd_path.exists() {
        // Find the directory containing package.json
        if let Some(project_dir) = find_npm_project_dir(&cmd_path) {
            // Check if it's a git repo
            if project_dir.join(".git").exists() {
                return McpRefreshStrategy::GitPullNpmInstall(project_dir);
            }
            return McpRefreshStrategy::NpmInstall(project_dir);
        }
    }

    McpRefreshStrategy::Unknown
}

/// Find the nearest directory containing package.json by walking up from the given path
fn find_npm_project_dir(start_path: &PathBuf) -> Option<PathBuf> {
    let mut current = if start_path.is_file() {
        start_path.parent()?.to_path_buf()
    } else {
        start_path.clone()
    };

    // Walk up to 5 levels to find package.json
    for _ in 0..5 {
        if current.join("package.json").exists() {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }

    None
}

/// Refresh an MCP server by running the appropriate update command
#[tauri::command]
pub async fn mcp_refresh(
    name: String,
    project_path: Option<String>,
) -> Result<McpRefreshResult, String> {
    let ops = McpOps::new(project_path.map(PathBuf::from));

    // Find the server by name
    let items = ops.list().map_err(|e| e.to_string())?;
    let server = items
        .into_iter()
        .find(|item| item.name == name)
        .ok_or_else(|| format!("MCP server '{name}' not found"))?;

    // Extract command and args from the config
    let (command, args) = if let ConfigItemData::McpServer(config) = &server.config {
        (config.command.clone(), config.args.clone())
    } else {
        return Err("Invalid server configuration".to_string());
    };

    // Detect refresh strategy
    let strategy = detect_refresh_strategy(command.as_deref(), &args);

    match strategy {
        McpRefreshStrategy::NpxSkip => Ok(McpRefreshResult {
            success: true,
            server_name: name,
            refresh_type: "npx_skip".to_string(),
            command_run: None,
            output: Some("npx always fetches the latest version - no refresh needed".to_string()),
            error: None,
        }),

        McpRefreshStrategy::NpmInstall(dir) => {
            let output = run_command_in_dir("npm", &["install"], &dir)?;
            Ok(McpRefreshResult {
                success: true,
                server_name: name,
                refresh_type: "npm_install".to_string(),
                command_run: Some(format!("npm install (in {})", dir.display())),
                output: Some(output),
                error: None,
            })
        }

        McpRefreshStrategy::GitPullNpmInstall(dir) => {
            // First try git pull
            let git_output = match run_command_in_dir("git", &["pull"], &dir) {
                Ok(out) => out,
                Err(e) => format!("git pull failed: {e}"),
            };

            // Then run npm install regardless of git pull result
            let npm_output = run_command_in_dir("npm", &["install"], &dir)?;

            let combined_output = format!("=== git pull ===\n{git_output}\n\n=== npm install ===\n{npm_output}");

            Ok(McpRefreshResult {
                success: true,
                server_name: name,
                refresh_type: "git_pull".to_string(),
                command_run: Some(format!("git pull && npm install (in {})", dir.display())),
                output: Some(combined_output),
                error: None,
            })
        }

        McpRefreshStrategy::Unknown => Ok(McpRefreshResult {
            success: false,
            server_name: name,
            refresh_type: "unknown".to_string(),
            command_run: None,
            output: None,
            error: Some("Cannot determine how to refresh this MCP server. Only local Node.js projects are supported.".to_string()),
        }),
    }
}

/// Resolve the full path to a command by checking common locations
/// This is needed because GUI apps on macOS don't inherit shell PATH
fn resolve_command(cmd: &str) -> String {
    // If it's already an absolute path, use it directly
    if cmd.starts_with('/') {
        return cmd.to_string();
    }

    // Common locations for npm, node, git on macOS/Linux
    let common_paths = [
        // Homebrew (Apple Silicon)
        "/opt/homebrew/bin",
        // Homebrew (Intel)
        "/usr/local/bin",
        // System paths
        "/usr/bin",
        // Common nvm locations
        &format!(
            "{}/.nvm/versions/node",
            std::env::var("HOME").unwrap_or_default()
        ),
        // fnm
        &format!(
            "{}/.local/share/fnm/aliases/default/bin",
            std::env::var("HOME").unwrap_or_default()
        ),
        // volta
        &format!("{}/.volta/bin", std::env::var("HOME").unwrap_or_default()),
        // asdf
        &format!("{}/.asdf/shims", std::env::var("HOME").unwrap_or_default()),
        // n (node version manager)
        "/usr/local/n/versions/node",
    ];

    // For nvm, we need to find the active version directory
    if let Ok(home) = std::env::var("HOME") {
        let nvm_dir = format!("{home}/.nvm/versions/node");
        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            // Find the most recent node version (they're named like v20.10.0)
            let mut versions: Vec<_> = entries
                .filter_map(std::result::Result::ok)
                .filter(|e| e.path().is_dir())
                .collect();
            versions.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
            if let Some(latest) = versions.first() {
                let bin_path = latest.path().join("bin").join(cmd);
                if bin_path.exists() {
                    return bin_path.to_string_lossy().to_string();
                }
            }
        }
    }

    // Check common paths
    for base in &common_paths {
        let full_path = format!("{base}/{cmd}");
        if std::path::Path::new(&full_path).exists() {
            return full_path;
        }
    }

    // Fall back to just the command name (will use PATH)
    cmd.to_string()
}

/// Run a command in the specified directory and return the combined stdout/stderr
fn run_command_in_dir(cmd: &str, args: &[&str], dir: &PathBuf) -> Result<String, String> {
    use std::process::Command;

    let resolved_cmd = resolve_command(cmd);
    let output = Command::new(&resolved_cmd)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("Failed to execute {cmd} (tried {resolved_cmd}): {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined = if stderr.is_empty() {
        stdout.to_string()
    } else if stdout.is_empty() {
        stderr.to_string()
    } else {
        format!("{stdout}\n{stderr}")
    };

    if output.status.success() {
        Ok(combined)
    } else {
        Err(format!(
            "{} failed with exit code {:?}:\n{}",
            cmd,
            output.status.code(),
            combined
        ))
    }
}
