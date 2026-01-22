//! Config operations Tauri commands
//!
//! Exposes config CRUD operations to the React frontend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

use tars_core::config::{
    ConfigItemData, ConfigScope, McpOps, McpServerConfig, McpServerUpdate, McpTransport,
};

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

    let result = ops
        .move_server(&params.name, from_scope, to_scope, dry_run)
        .map_err(|e| e.to_string())?;

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
    _params: RollbackParams,
    _state: State<'_, AppState>,
) -> Result<RollbackResult, String> {
    // TODO: Implement
    Err("Not yet implemented".into())
}
