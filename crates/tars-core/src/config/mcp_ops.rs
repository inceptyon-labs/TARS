//! MCP Server CRUD operations
//!
//! Implements surgical add/remove/update/move operations for MCP servers.

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde_json::{json, Value};
use tars_scanner::plugins::PluginInventory;

use super::error::{ConfigError, ConfigResult};
use super::item::{validate_name, ConfigItem, ConfigItemData, ConfigItemType};
use super::mcp::{McpServerConfig, McpServerUpdate, McpTransport};
use super::ops::{OperationResult, OperationType};
use super::scope::ConfigScope;

/// MCP operations manager
pub struct McpOps {
    /// Optional project path (required for project/local scope operations)
    project_path: Option<PathBuf>,
    /// Backup directory for simple file backups
    backup_dir: Option<PathBuf>,
}

impl McpOps {
    /// Create a new MCP operations manager
    #[must_use]
    pub fn new(project_path: Option<PathBuf>) -> Self {
        Self {
            project_path,
            backup_dir: None,
        }
    }

    /// Enable file backups to a directory
    #[must_use]
    pub fn with_backup_dir(mut self, dir: PathBuf) -> Self {
        self.backup_dir = Some(dir);
        self
    }

    /// List all MCP servers across scopes (including plugins)
    pub fn list(&self) -> ConfigResult<Vec<ConfigItem>> {
        let mut items = Vec::new();

        // Read from user scope (~/.claude.json)
        if let Some(path) = self.get_mcp_path(ConfigScope::User)? {
            if path.exists() {
                items.extend(self.read_servers_from_file(&path, ConfigScope::User)?);
            }
        }

        // Read from project scope (.mcp.json)
        if let Some(path) = self.get_mcp_path(ConfigScope::Project)? {
            if path.exists() {
                items.extend(self.read_servers_from_file(&path, ConfigScope::Project)?);
            }
        }

        // Read from installed plugins
        items.extend(self.read_plugin_servers()?);

        Ok(items)
    }

    /// Read MCP servers from installed plugins
    /// Only includes:
    /// - User-scoped plugins (always)
    /// - Project-scoped plugins (only if their `project_path` matches current project)
    fn read_plugin_servers(&self) -> ConfigResult<Vec<ConfigItem>> {
        use tars_scanner::types::Scope;

        let mut items = Vec::new();

        // Scan plugins - ignore errors (plugins are optional)
        let Ok(plugin_inventory) = PluginInventory::scan() else {
            return Ok(items);
        };

        for plugin in &plugin_inventory.installed {
            if !plugin.enabled {
                continue;
            }

            // Filter based on plugin scope:
            // - User scope: always include
            // - Project scope: only if project_path matches
            // - Local/Managed: skip for now
            let include_plugin = match &plugin.scope {
                Scope::User => true,
                Scope::Project => {
                    // Only include if we're viewing the same project
                    match (&self.project_path, &plugin.project_path) {
                        (Some(current), Some(plugin_proj)) => {
                            // Normalize paths for comparison
                            let current_normalized =
                                current.to_string_lossy().replace('\\', "/").to_lowercase();
                            let plugin_normalized = plugin_proj.replace('\\', "/").to_lowercase();
                            current_normalized == plugin_normalized
                        }
                        _ => false, // No project context or no plugin project path
                    }
                }
                _ => false, // Local, Managed, Plugin scopes - skip
            };

            if !include_plugin {
                continue;
            }

            let mcp_path = plugin.path.join(".mcp.json");
            if !mcp_path.exists() {
                continue;
            }

            let Ok(content) = fs::read_to_string(&mcp_path) else {
                continue;
            };

            let Ok(json): Result<Value, _> = serde_json::from_str(&content) else {
                continue;
            };

            // Plugin format is flat: { "serverName": { "type": "...", ... } }
            let Some(servers) = json.as_object() else {
                continue;
            };

            let plugin_id = match &plugin.marketplace {
                Some(marketplace) => format!("{}@{}", plugin.id, marketplace),
                None => plugin.id.clone(),
            };

            // Determine the scope to display based on plugin scope
            let display_scope = match &plugin.scope {
                Scope::User => ConfigScope::User,
                Scope::Project => ConfigScope::Project,
                _ => ConfigScope::User,
            };

            for (name, value) in servers {
                // Skip mcpServers wrapper if present
                if name == "mcpServers" {
                    if let Some(inner) = value.as_object() {
                        for (inner_name, inner_value) in inner {
                            if let Some(item) = self.parse_plugin_server(
                                inner_name,
                                inner_value,
                                &mcp_path,
                                &plugin_id,
                                display_scope,
                            ) {
                                items.push(item);
                            }
                        }
                    }
                    continue;
                }

                if let Some(item) =
                    self.parse_plugin_server(name, value, &mcp_path, &plugin_id, display_scope)
                {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// Parse a single MCP server from plugin config
    fn parse_plugin_server(
        &self,
        name: &str,
        value: &Value,
        file_path: &PathBuf,
        plugin_id: &str,
        scope: ConfigScope,
    ) -> Option<ConfigItem> {
        // Parse the transport type
        let transport_str = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("stdio");
        let transport = match transport_str {
            "http" => McpTransport::Http,
            "sse" => McpTransport::Sse,
            _ => McpTransport::Stdio,
        };

        let config = McpServerConfig {
            transport,
            command: value
                .get("command")
                .and_then(|v| v.as_str())
                .map(String::from),
            args: value
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            env: value
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default(),
            url: value.get("url").and_then(|v| v.as_str()).map(String::from),
            docs_url: value
                .get("docsUrl")
                .and_then(|v| v.as_str())
                .map(String::from),
        };

        // Skip invalid configs
        if config.validate().is_err() {
            return None;
        }

        // Create item with the appropriate scope
        let mut item = ConfigItem::new(
            name.to_string(),
            ConfigItemType::McpServer,
            scope,
            file_path.clone(),
            ConfigItemData::McpServer(config),
        );

        // Add plugin source info to distinguish from regular servers
        item.source_plugin = Some(plugin_id.to_string());

        Some(item)
    }

    /// List MCP servers from a specific scope only
    pub fn list_scope(&self, scope: ConfigScope) -> ConfigResult<Vec<ConfigItem>> {
        if let Some(path) = self.get_mcp_path(scope)? {
            if path.exists() {
                return self.read_servers_from_file(&path, scope);
            }
        }
        Ok(Vec::new())
    }

    /// Add a new MCP server
    pub fn add(
        &self,
        name: &str,
        scope: ConfigScope,
        config: McpServerConfig,
        dry_run: bool,
    ) -> ConfigResult<OperationResult> {
        // Validate name
        validate_name(name)?;

        // Validate config
        config.validate().map_err(ConfigError::ValidationError)?;

        // Check scope is writable
        if !scope.is_writable() {
            return Err(ConfigError::ManagedScope);
        }

        // Get target file path
        let file_path = self.get_mcp_path(scope)?.ok_or_else(|| {
            ConfigError::ValidationError("Project path required for project scope".into())
        })?;

        // Check if server already exists in this scope
        if file_path.exists() {
            let servers = self.read_servers_from_file(&file_path, scope)?;
            if servers.iter().any(|s| s.name == name) {
                return Err(ConfigError::ItemExists {
                    name: name.to_string(),
                    scope: scope.to_string(),
                });
            }
        }

        // Generate diff preview (for dry-run display)
        let _diff = self.generate_add_diff(name, &config, &file_path)?;

        if dry_run {
            return Ok(OperationResult {
                success: true,
                operation: OperationType::Add,
                name: name.to_string(),
                scope,
                files_modified: vec![file_path],
                backup_id: None,
                error: None,
                warnings: Vec::new(),
            });
        }

        // Create backup before modifying
        let backup_id = self.create_backup_if_exists(&file_path)?;

        // Perform the add operation
        self.add_server_to_file(name, &config, &file_path, scope)?;

        Ok(OperationResult::success(
            OperationType::Add,
            name,
            scope,
            vec![file_path],
            backup_id,
        ))
    }

    /// Remove an MCP server
    pub fn remove(
        &self,
        name: &str,
        scope: Option<ConfigScope>,
        dry_run: bool,
    ) -> ConfigResult<OperationResult> {
        // Find the server
        let (found_scope, file_path) = self.find_server(name, scope)?;

        // Check scope is writable
        if !found_scope.is_writable() {
            return Err(ConfigError::ManagedScope);
        }

        if dry_run {
            return Ok(OperationResult {
                success: true,
                operation: OperationType::Remove,
                name: name.to_string(),
                scope: found_scope,
                files_modified: vec![file_path],
                backup_id: None,
                error: None,
                warnings: Vec::new(),
            });
        }

        // Create backup before modifying
        let backup_id = self.create_backup_if_exists(&file_path)?;

        // Perform the remove operation
        self.remove_server_from_file(name, &file_path, found_scope)?;

        Ok(OperationResult::success(
            OperationType::Remove,
            name,
            found_scope,
            vec![file_path],
            backup_id,
        ))
    }

    /// Move an MCP server to a different scope
    pub fn move_server(
        &self,
        name: &str,
        from_scope: Option<ConfigScope>,
        to_scope: ConfigScope,
        dry_run: bool,
    ) -> ConfigResult<OperationResult> {
        // Validate target scope is writable
        if !to_scope.is_writable() {
            return Err(ConfigError::ManagedScope);
        }

        // Find the server
        let (found_scope, source_path) = self.find_server(name, from_scope)?;

        // Can't move to the same scope
        if found_scope == to_scope {
            return Err(ConfigError::ValidationError(format!(
                "Server '{name}' is already in {to_scope} scope"
            )));
        }

        // Get target file path
        let target_path = self.get_mcp_path(to_scope)?.ok_or_else(|| {
            ConfigError::ValidationError("Project path required for project/local scope".into())
        })?;

        // Read the server config from source
        let servers = self.read_servers_from_file(&source_path, found_scope)?;
        let server =
            servers
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| ConfigError::ItemNotFound {
                    name: name.to_string(),
                })?;

        let config = match &server.config {
            ConfigItemData::McpServer(cfg) => cfg.clone(),
            _ => return Err(ConfigError::Internal("Invalid config type".into())),
        };

        // Check if server already exists in target scope
        if target_path.exists() {
            let target_servers = self.read_servers_from_file(&target_path, to_scope)?;
            if target_servers.iter().any(|s| s.name == name) {
                return Err(ConfigError::ItemExists {
                    name: name.to_string(),
                    scope: to_scope.to_string(),
                });
            }
        }

        // Dry run - return preview
        if dry_run {
            return Ok(OperationResult {
                success: true,
                operation: OperationType::Move,
                name: name.to_string(),
                scope: to_scope,
                files_modified: vec![source_path, target_path],
                backup_id: None,
                error: None,
                warnings: Vec::new(),
            });
        }

        // Create backups before modifying
        let source_backup = self.create_backup_if_exists(&source_path)?;
        let target_backup = self.create_backup_if_exists(&target_path)?;

        // Remove from source
        self.remove_server_from_file(name, &source_path, found_scope)?;

        // Add to target
        self.add_server_to_file(name, &config, &target_path, to_scope)?;

        Ok(OperationResult {
            success: true,
            operation: OperationType::Move,
            name: name.to_string(),
            scope: to_scope,
            files_modified: vec![source_path, target_path],
            backup_id: source_backup.or(target_backup),
            error: None,
            warnings: Vec::new(),
        })
    }

    /// Update an MCP server configuration
    pub fn update(
        &self,
        name: &str,
        scope: Option<ConfigScope>,
        updates: McpServerUpdate,
        dry_run: bool,
    ) -> ConfigResult<OperationResult> {
        // Validate name
        validate_name(name)?;

        // Find the server
        let (found_scope, path) = self.find_server(name, scope)?;

        // Validate scope is writable
        if !found_scope.is_writable() {
            return Err(ConfigError::ManagedScope);
        }

        // Read the server config
        let servers = self.read_servers_from_file(&path, found_scope)?;
        let item =
            servers
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| ConfigError::ItemNotFound {
                    name: name.to_string(),
                })?;

        // Clone and apply updates to config
        let mut config = match item.config {
            ConfigItemData::McpServer(ref cfg) => cfg.clone(),
            _ => {
                return Err(ConfigError::ValidationError(
                    "Expected MCP server config".into(),
                ))
            }
        };

        // Apply updates
        if let Some(command) = updates.command {
            config.command = Some(command);
        }
        if let Some(args) = updates.args {
            config.args = args;
        }
        if let Some(add_args) = updates.add_args {
            config.args.extend(add_args);
        }
        if let Some(env) = updates.env {
            config.env = env;
        }
        if let Some(add_env) = updates.add_env {
            for (key, value) in add_env {
                config.env.insert(key, value);
            }
        }
        if let Some(remove_env) = updates.remove_env {
            for key in remove_env {
                config.env.remove(&key);
            }
        }
        if let Some(url) = updates.url {
            config.url = Some(url);
        }

        // Validate merged config
        config.validate().map_err(ConfigError::ValidationError)?;

        // Return early for dry-run (no backup or file write)
        if dry_run {
            return Ok(OperationResult {
                success: true,
                operation: OperationType::Update,
                name: name.to_string(),
                scope: found_scope,
                files_modified: vec![path],
                backup_id: None,
                error: None,
                warnings: vec!["This is a dry-run; no changes were applied".into()],
            });
        }

        // Create backup if backup directory is configured
        let backup_id = self.create_backup_if_exists(&path)?;

        // Write updated config to file
        self.update_server_in_file(&path, name, config)?;

        Ok(OperationResult {
            success: true,
            operation: OperationType::Update,
            name: name.to_string(),
            scope: found_scope,
            files_modified: vec![path],
            backup_id,
            error: None,
            warnings: Vec::new(),
        })
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    fn get_mcp_path(&self, scope: ConfigScope) -> ConfigResult<Option<PathBuf>> {
        match scope {
            ConfigScope::User => {
                let home = dirs::home_dir()
                    .ok_or_else(|| ConfigError::Internal("Cannot find home directory".into()))?;
                Ok(Some(home.join(".claude.json")))
            }
            ConfigScope::Project | ConfigScope::Local => {
                Ok(self.project_path.as_ref().map(|p| p.join(".mcp.json")))
            }
            ConfigScope::Managed => Ok(None),
        }
    }

    fn read_servers_from_file(
        &self,
        path: &PathBuf,
        scope: ConfigScope,
    ) -> ConfigResult<Vec<ConfigItem>> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError {
            path: path.clone(),
            message: e.to_string(),
        })?;

        let json: Value =
            serde_json::from_str(&content).map_err(|e| ConfigError::JsonParseError {
                path: path.clone(),
                message: e.to_string(),
            })?;

        // MCP config is a flat dict: { "serverName": { "type": "stdio", ... } }
        let servers = json
            .as_object()
            .ok_or_else(|| ConfigError::JsonParseError {
                path: path.clone(),
                message: "Expected JSON object".into(),
            })?;

        let mut items = Vec::new();
        for (name, value) in servers {
            // Skip mcpServers wrapper if present (some formats nest it)
            if name == "mcpServers" {
                if let Some(inner) = value.as_object() {
                    for (inner_name, inner_value) in inner {
                        if let Ok(config) =
                            serde_json::from_value::<McpServerConfig>(inner_value.clone())
                        {
                            // Only include if config is valid (has required fields)
                            if config.validate().is_ok() {
                                items.push(ConfigItem::new(
                                    inner_name.clone(),
                                    ConfigItemType::McpServer,
                                    scope,
                                    path.clone(),
                                    ConfigItemData::McpServer(config),
                                ));
                            }
                        }
                    }
                }
                continue;
            }

            if let Ok(config) = serde_json::from_value::<McpServerConfig>(value.clone()) {
                // Only include if config is valid (has required fields)
                if config.validate().is_ok() {
                    items.push(ConfigItem::new(
                        name.clone(),
                        ConfigItemType::McpServer,
                        scope,
                        path.clone(),
                        ConfigItemData::McpServer(config),
                    ));
                }
            }
        }

        Ok(items)
    }

    fn find_server(
        &self,
        name: &str,
        scope: Option<ConfigScope>,
    ) -> ConfigResult<(ConfigScope, PathBuf)> {
        let scopes_to_check = match scope {
            Some(s) => vec![s],
            None => vec![ConfigScope::Project, ConfigScope::User],
        };

        let mut found_in: Vec<(ConfigScope, PathBuf)> = Vec::new();

        for s in scopes_to_check {
            if let Some(path) = self.get_mcp_path(s)? {
                if path.exists() {
                    let servers = self.read_servers_from_file(&path, s)?;
                    if servers.iter().any(|srv| srv.name == name) {
                        found_in.push((s, path));
                    }
                }
            }
        }

        match found_in.len() {
            0 => Err(ConfigError::ItemNotFound {
                name: name.to_string(),
            }),
            1 => Ok(found_in.remove(0)),
            _ => Err(ConfigError::AmbiguousItem {
                name: name.to_string(),
                scopes: found_in.iter().map(|(s, _)| s.to_string()).collect(),
            }),
        }
    }

    fn generate_add_diff(
        &self,
        name: &str,
        config: &McpServerConfig,
        file_path: &PathBuf,
    ) -> ConfigResult<String> {
        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| ConfigError::Internal(e.to_string()))?;

        Ok(format!(
            "--- {}\n+++ {}\n+ \"{}\": {}",
            file_path.display(),
            file_path.display(),
            name,
            config_json.replace('\n', "\n+ ")
        ))
    }

    fn create_backup_if_exists(&self, path: &PathBuf) -> ConfigResult<Option<String>> {
        if !path.exists() {
            return Ok(None);
        }

        if let Some(ref backup_dir) = self.backup_dir {
            // Create backup directory if it doesn't exist
            fs::create_dir_all(backup_dir).map_err(|e| ConfigError::IoError {
                path: backup_dir.clone(),
                message: e.to_string(),
            })?;

            let backup_id = uuid::Uuid::new_v4().to_string();
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let file_name = path
                .file_name()
                .map_or_else(|| "config".to_string(), |n| n.to_string_lossy().to_string());
            let backup_name = format!("{}_{}.{}.bak", file_name, timestamp, &backup_id[..8]);
            let backup_path = backup_dir.join(&backup_name);

            // Copy the file to backup
            fs::copy(path, &backup_path).map_err(|e| {
                ConfigError::BackupFailed(format!("Failed to backup {}: {}", path.display(), e))
            })?;

            Ok(Some(backup_id))
        } else {
            // No backup configured - return None but allow operation
            Ok(None)
        }
    }

    fn add_server_to_file(
        &self,
        name: &str,
        config: &McpServerConfig,
        file_path: &PathBuf,
        _scope: ConfigScope,
    ) -> ConfigResult<()> {
        // Read existing or create new
        let mut json: Value = if file_path.exists() {
            let content = fs::read_to_string(file_path).map_err(|e| ConfigError::IoError {
                path: file_path.clone(),
                message: e.to_string(),
            })?;
            serde_json::from_str(&content).map_err(|e| ConfigError::JsonParseError {
                path: file_path.clone(),
                message: e.to_string(),
            })?
        } else {
            json!({})
        };

        // Add the new server
        let config_value =
            serde_json::to_value(config).map_err(|e| ConfigError::Internal(e.to_string()))?;

        let root = json
            .as_object_mut()
            .ok_or_else(|| ConfigError::JsonParseError {
                path: file_path.clone(),
                message: "Expected JSON object".into(),
            })?;

        // Both user scope (~/.claude.json) and project scope (.mcp.json) use mcpServers wrapper
        // Ensure mcpServers object exists
        if !root.contains_key("mcpServers") {
            root.insert("mcpServers".to_string(), json!({}));
        }
        root.get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| ConfigError::JsonParseError {
                path: file_path.clone(),
                message: "mcpServers is not an object".into(),
            })?
            .insert(name.to_string(), config_value);

        // Write back
        self.write_json_file(file_path, &json)
    }

    fn remove_server_from_file(
        &self,
        name: &str,
        file_path: &PathBuf,
        scope: ConfigScope,
    ) -> ConfigResult<()> {
        let content = fs::read_to_string(file_path).map_err(|e| ConfigError::IoError {
            path: file_path.clone(),
            message: e.to_string(),
        })?;

        let mut json: Value =
            serde_json::from_str(&content).map_err(|e| ConfigError::JsonParseError {
                path: file_path.clone(),
                message: e.to_string(),
            })?;

        let root = json
            .as_object_mut()
            .ok_or_else(|| ConfigError::JsonParseError {
                path: file_path.clone(),
                message: "Expected JSON object".into(),
            })?;

        // For user scope, remove from mcpServers object
        // For project scope, check both mcpServers and root level
        if scope == ConfigScope::User {
            if let Some(mcp_servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                mcp_servers.remove(name);
            }
        } else {
            // For project scope, try mcpServers first, then root
            let removed_from_mcp = root
                .get_mut("mcpServers")
                .and_then(|v| v.as_object_mut())
                .is_some_and(|obj| obj.remove(name).is_some());

            if !removed_from_mcp {
                root.remove(name);
            }
        }

        // Write back
        self.write_json_file(file_path, &json)
    }

    fn update_server_in_file(
        &self,
        file_path: &PathBuf,
        name: &str,
        config: McpServerConfig,
    ) -> ConfigResult<()> {
        let content = fs::read_to_string(file_path).map_err(|e| ConfigError::IoError {
            path: file_path.clone(),
            message: e.to_string(),
        })?;

        let mut json: Value =
            serde_json::from_str(&content).map_err(|e| ConfigError::JsonParseError {
                path: file_path.clone(),
                message: e.to_string(),
            })?;

        let root = json
            .as_object_mut()
            .ok_or_else(|| ConfigError::JsonParseError {
                path: file_path.clone(),
                message: "Expected JSON object".into(),
            })?;

        // Serialize the updated config
        let config_json = serde_json::to_value(&config).map_err(|e| {
            ConfigError::ValidationError(format!("Failed to serialize config: {e}"))
        })?;

        // Try to update in mcpServers first, then root
        let updated =
            if let Some(mcp_servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                mcp_servers.insert(name.to_string(), config_json);
                true
            } else if root.get(name).is_some() {
                root.insert(name.to_string(), config_json);
                true
            } else {
                false
            };

        if !updated {
            return Err(ConfigError::ItemNotFound {
                name: name.to_string(),
            });
        }

        // Write back
        self.write_json_file(file_path, &json)
    }

    fn write_json_file(&self, path: &PathBuf, value: &Value) -> ConfigResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::IoError {
                path: path.clone(),
                message: e.to_string(),
            })?;
        }

        let content = serde_json::to_string_pretty(value)
            .map_err(|e| ConfigError::Internal(e.to_string()))?;

        fs::write(path, content).map_err(|e| ConfigError::IoError {
            path: path.clone(),
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_mcp_file(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join(".mcp.json");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_list_empty() {
        let dir = TempDir::new().unwrap();
        let ops = McpOps::new(Some(dir.path().to_path_buf()));
        // Use list_scope to only check project scope (not user's real ~/.claude.json)
        let items = ops.list_scope(ConfigScope::Project).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_list_servers() {
        let dir = TempDir::new().unwrap();
        create_test_mcp_file(
            &dir,
            r#"{
            "context7": {
                "type": "stdio",
                "command": "npx",
                "args": ["-y", "@context7/mcp"]
            },
            "neon": {
                "type": "stdio",
                "command": "npx",
                "args": ["@neondatabase/mcp-server"]
            }
        }"#,
        );

        let ops = McpOps::new(Some(dir.path().to_path_buf()));
        // Use list_scope to only check project scope
        let items = ops.list_scope(ConfigScope::Project).unwrap();
        assert_eq!(items.len(), 2);

        let names: Vec<_> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"context7"));
        assert!(names.contains(&"neon"));
    }

    #[test]
    fn test_add_server() {
        let dir = TempDir::new().unwrap();
        let ops = McpOps::new(Some(dir.path().to_path_buf()));

        let config = McpServerConfig::stdio("npx", vec!["-y".into(), "@context7/mcp".into()]);
        let result = ops
            .add("context7", ConfigScope::Project, config, false)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.name, "context7");
        assert_eq!(result.scope, ConfigScope::Project);

        // Verify it was written (use list_scope for isolation)
        let items = ops.list_scope(ConfigScope::Project).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "context7");
    }

    #[test]
    fn test_add_server_dry_run() {
        let dir = TempDir::new().unwrap();
        let ops = McpOps::new(Some(dir.path().to_path_buf()));

        let config = McpServerConfig::stdio("npx", vec![]);
        let result = ops.add("test", ConfigScope::Project, config, true).unwrap();

        assert!(result.success);
        assert!(result.backup_id.is_none());

        // Verify nothing was written (use list_scope for isolation)
        let items = ops.list_scope(ConfigScope::Project).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_add_server_already_exists() {
        let dir = TempDir::new().unwrap();
        create_test_mcp_file(&dir, r#"{"context7": {"type": "stdio", "command": "npx"}}"#);

        let ops = McpOps::new(Some(dir.path().to_path_buf()));
        let config = McpServerConfig::stdio("npx", vec![]);

        let result = ops.add("context7", ConfigScope::Project, config, false);
        assert!(matches!(result, Err(ConfigError::ItemExists { .. })));
    }

    #[test]
    fn test_remove_server() {
        let dir = TempDir::new().unwrap();
        create_test_mcp_file(
            &dir,
            r#"{
            "context7": {"type": "stdio", "command": "npx"},
            "neon": {"type": "stdio", "command": "npx"}
        }"#,
        );

        let ops = McpOps::new(Some(dir.path().to_path_buf()));
        let result = ops
            .remove("context7", Some(ConfigScope::Project), false)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.name, "context7");

        // Verify it was removed (use list_scope for isolation)
        let items = ops.list_scope(ConfigScope::Project).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "neon");
    }

    #[test]
    fn test_remove_server_not_found() {
        let dir = TempDir::new().unwrap();
        create_test_mcp_file(&dir, r#"{"neon": {"type": "stdio", "command": "npx"}}"#);

        let ops = McpOps::new(Some(dir.path().to_path_buf()));
        let result = ops.remove("nonexistent", Some(ConfigScope::Project), false);

        assert!(matches!(result, Err(ConfigError::ItemNotFound { .. })));
    }

    #[test]
    fn test_validate_invalid_name() {
        let dir = TempDir::new().unwrap();
        let ops = McpOps::new(Some(dir.path().to_path_buf()));
        let config = McpServerConfig::stdio("npx", vec![]);

        let result = ops.add("path/traversal", ConfigScope::Project, config, false);
        assert!(matches!(result, Err(ConfigError::ValidationError(_))));
    }

    #[test]
    fn test_update_server_not_found() {
        let dir = TempDir::new().unwrap();
        let ops = McpOps::new(Some(dir.path().to_path_buf()));

        let update = McpServerUpdate {
            command: Some("new".into()),
            ..Default::default()
        };
        let result = ops.update("nonexistent", Some(ConfigScope::Project), update, false);

        assert!(matches!(result, Err(ConfigError::ItemNotFound { .. })));
    }

    #[test]
    fn test_update_dry_run() {
        let dir = TempDir::new().unwrap();
        let ops = McpOps::new(Some(dir.path().to_path_buf()));

        // Add initial server
        let config = McpServerConfig::stdio("npx", vec!["arg1".into()]);
        ops.add("test", ConfigScope::Project, config, false)
            .unwrap();

        // Dry-run update
        let update = McpServerUpdate {
            args: Some(vec!["arg2".into()]),
            ..Default::default()
        };
        let result = ops
            .update("test", Some(ConfigScope::Project), update, true)
            .unwrap();

        assert!(result.success);
        assert!(result.backup_id.is_none());
        assert!(result
            .warnings
            .contains(&"This is a dry-run; no changes were applied".into()));

        // Verify nothing changed - check we still have 1 server
        let items = ops.list_scope(ConfigScope::Project).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "test");
    }

    #[test]
    fn test_update_command() {
        let dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();
        let ops = McpOps::new(Some(dir.path().to_path_buf()))
            .with_backup_dir(backup_dir.path().to_path_buf());

        // Add initial server
        let config = McpServerConfig::stdio("npx", vec!["@old/mcp".into()]);
        ops.add("test", ConfigScope::Project, config, false)
            .unwrap();

        // Update command
        let update = McpServerUpdate {
            command: Some("node".into()),
            ..Default::default()
        };
        let result = ops
            .update("test", Some(ConfigScope::Project), update, false)
            .unwrap();

        assert!(result.success);
        assert!(result.backup_id.is_some()); // Backup created

        // Verify update by checking server was written
        let items = ops.list_scope(ConfigScope::Project).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "test");
    }
}
