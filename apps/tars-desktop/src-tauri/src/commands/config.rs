//! Config operations Tauri commands
//!
//! Exposes config CRUD operations to the React frontend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

use tars_core::config::{ConfigItemData, ConfigScope, McpOps, McpServerConfig, McpTransport};

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
}

/// Result from mcp_list command
#[derive(Debug, Serialize)]
pub struct McpListResult {
    pub servers: Vec<McpServerItem>,
}

/// Parameters for mcp_add command
#[derive(Debug, Deserialize)]
pub struct McpAddParams {
    pub name: String,
    pub scope: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

/// Result from mcp_add/remove/update commands
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
        other => return Err(format!("Invalid transport type: {}", other)),
    };

    // Parse scope
    let scope: ConfigScope = params.scope.parse().map_err(|e| format!("{}", e))?;

    // Build server config
    let config = McpServerConfig {
        transport,
        command: params.command,
        args: params.args.unwrap_or_default(),
        env: params.env.unwrap_or_default(),
        url: params.url,
    };

    // Validate config
    config.validate().map_err(|e| e.to_string())?;

    // Set up backup directory
    let backup_dir = state.data_dir().join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    // Create operations manager
    let ops = McpOps::new(project_path.map(PathBuf::from))
        .with_backup_dir(backup_dir);

    let dry_run = params.dry_run.unwrap_or(false);
    let result = ops.add(&params.name, scope, config, dry_run)
        .map_err(|e| e.to_string())?;

    Ok(McpOperationResult {
        success: result.success,
        backup_id: result.backup_id,
        file_path: result.files_modified.first()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        diff: None,
        error: result.error,
    })
}

/// Parameters for mcp_remove command
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
        Some(s.parse().map_err(|e| format!("{}", e))?)
    } else {
        None
    };

    // Set up backup directory
    let backup_dir = state.data_dir().join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    // Create operations manager
    let ops = McpOps::new(project_path.map(PathBuf::from))
        .with_backup_dir(backup_dir);

    let dry_run = params.dry_run.unwrap_or(false);
    let result = ops.remove(&params.name, scope_filter, dry_run)
        .map_err(|e| e.to_string())?;

    Ok(McpOperationResult {
        success: result.success,
        backup_id: result.backup_id,
        file_path: result.files_modified.first()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        diff: None,
        error: result.error,
    })
}

/// Parameters for mcp_update command
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
    _params: McpUpdateParams,
    _state: State<'_, AppState>,
) -> Result<McpOperationResult, String> {
    // TODO: Implement in Phase 4
    Err("Not yet implemented".into())
}

/// Parameters for mcp_move command
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

/// Result from mcp_move command
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
    let backup_dir = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".tars").join("backups"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/tars/backups"));

    let ops = McpOps::new(project_path.map(PathBuf::from))
        .with_backup_dir(backup_dir);

    let from_scope: Option<ConfigScope> = params.from_scope
        .as_ref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: tars_core::config::ConfigError| e.to_string())?;

    let to_scope: ConfigScope = params.to_scope.parse()
        .map_err(|e: tars_core::config::ConfigError| e.to_string())?;

    let dry_run = params.dry_run.unwrap_or(false);

    let result = ops.move_server(&params.name, from_scope, to_scope, dry_run)
        .map_err(|e| e.to_string())?;

    Ok(McpMoveResult {
        success: result.success,
        backup_id: result.backup_id,
        removed_from: from_scope.map(|s| s.to_string()).unwrap_or_else(|| "auto".to_string()),
        added_to: to_scope.to_string(),
        diff: None,
        error: result.error,
    })
}

// ============================================================================
// Rollback Command
// ============================================================================

/// Parameters for config_rollback command
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RollbackParams {
    #[serde(rename = "backupId")]
    pub backup_id: String,
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
}

/// Result from config_rollback command
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
