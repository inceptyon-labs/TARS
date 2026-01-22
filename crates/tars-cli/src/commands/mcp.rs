//! MCP server CLI commands
//!
//! Handles: tars mcp add/remove/update/move/list

use clap::{Args, Subcommand};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

use tars_core::config::{
    ConfigItemData, ConfigScope, McpOps, McpServerConfig, McpServerUpdate, McpTransport,
};

/// MCP server commands
#[derive(Subcommand)]
pub enum McpCommands {
    /// List all MCP servers
    List {
        /// Filter by scope
        #[arg(long)]
        scope: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a new MCP server
    Add(McpAddArgs),
    /// Remove an MCP server
    Remove {
        /// Server name
        name: String,
        /// Scope to remove from (auto-detect if not specified)
        #[arg(long)]
        scope: Option<String>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Update an MCP server configuration
    Update(McpUpdateArgs),
    /// Move an MCP server between scopes
    Move {
        /// Server name
        name: String,
        /// Source scope (auto-detect if not specified)
        #[arg(long)]
        from: Option<String>,
        /// Target scope
        #[arg(long)]
        to: String,
        /// Overwrite if exists in target
        #[arg(long)]
        force: bool,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Arguments for `tars mcp add`
#[derive(Args)]
pub struct McpAddArgs {
    /// Server name (must be unique in scope)
    pub name: String,

    /// Target scope (user, project)
    #[arg(long, default_value = "project")]
    pub scope: String,

    /// Transport type (stdio, http, sse)
    #[arg(long, value_name = "TYPE", default_value = "stdio")]
    pub r#type: String,

    /// Command for stdio transport
    #[arg(long)]
    pub command: Option<String>,

    /// Command arguments (can specify multiple times)
    #[arg(long = "args", value_name = "ARG")]
    pub args: Vec<String>,

    /// Environment variables (KEY=value, can specify multiple times)
    #[arg(long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,

    /// URL for http/sse transport
    #[arg(long)]
    pub url: Option<String>,

    /// Preview changes without applying
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl McpAddArgs {
    /// Convert to `McpServerConfig`
    pub fn to_config(&self) -> Result<McpServerConfig, String> {
        let transport = match self.r#type.as_str() {
            "stdio" => McpTransport::Stdio,
            "http" => McpTransport::Http,
            "sse" => McpTransport::Sse,
            other => return Err(format!("Invalid transport type: {other}")),
        };

        let mut env = HashMap::new();
        for e in &self.env {
            let (key, value) = e
                .split_once('=')
                .ok_or_else(|| format!("Invalid env format: {e} (expected KEY=value)"))?;
            env.insert(key.to_string(), value.to_string());
        }

        let config = McpServerConfig {
            transport,
            command: self.command.clone(),
            args: self.args.clone(),
            env,
            url: self.url.clone(),
            docs_url: None, // CLI doesn't support docs_url yet
        };

        config.validate()?;
        Ok(config)
    }

    /// Parse scope
    pub fn parse_scope(&self) -> Result<ConfigScope, String> {
        self.scope.parse().map_err(|e| format!("{e}"))
    }
}

/// Arguments for `tars mcp update`
#[derive(Args)]
pub struct McpUpdateArgs {
    /// Server name
    pub name: String,

    /// Scope to update in (auto-detect if not specified)
    #[arg(long)]
    pub scope: Option<String>,

    /// New command
    #[arg(long)]
    pub command: Option<String>,

    /// Replace all arguments
    #[arg(long = "args", value_name = "ARG")]
    pub args: Vec<String>,

    /// Add to existing arguments
    #[arg(long = "add-arg", value_name = "ARG")]
    pub add_args: Vec<String>,

    /// Replace all environment variables
    #[arg(long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,

    /// Add environment variables
    #[arg(long = "add-env", value_name = "KEY=VALUE")]
    pub add_env: Vec<String>,

    /// Remove environment variables
    #[arg(long = "remove-env", value_name = "KEY")]
    pub remove_env: Vec<String>,

    /// New URL
    #[arg(long)]
    pub url: Option<String>,

    /// Preview changes without applying
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Execute MCP command
pub fn execute(
    cmd: McpCommands,
    project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        McpCommands::List { scope, json } => execute_list(scope, json, project_path),
        McpCommands::Add(args) => execute_add(args, project_path),
        McpCommands::Remove {
            name,
            scope,
            dry_run,
            json,
        } => execute_remove(&name, scope, dry_run, json, project_path),
        McpCommands::Update(args) => execute_update(args, project_path),
        McpCommands::Move {
            name,
            from,
            to,
            force,
            dry_run,
            json,
        } => execute_move(&name, from, &to, force, dry_run, json, project_path),
    }
}

fn execute_list(
    scope: Option<String>,
    json_output: bool,
    project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ops = McpOps::new(project_path.cloned());

    let items = if let Some(scope_str) = &scope {
        let scope: ConfigScope = scope_str.parse()?;
        ops.list_scope(scope)?
    } else {
        ops.list()?
    };

    if json_output {
        // Build JSON output using serde_json::Value
        let servers: Vec<serde_json::Value> = items
            .iter()
            .filter_map(|item| {
                if let ConfigItemData::McpServer(config) = &item.config {
                    Some(json!({
                        "name": item.name,
                        "scope": item.scope.to_string(),
                        "transport": format!("{:?}", config.transport).to_lowercase(),
                        "command": config.command,
                        "args": config.args,
                        "url": config.url,
                        "env": config.env,
                    }))
                } else {
                    None
                }
            })
            .collect();

        let output = json!({
            "count": servers.len(),
            "servers": servers,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if items.is_empty() {
            println!("No MCP servers found.");
            return Ok(());
        }

        // Group by scope for display
        let mut by_scope: HashMap<String, Vec<_>> = HashMap::new();
        for item in &items {
            by_scope
                .entry(item.scope.to_string())
                .or_default()
                .push(item);
        }

        for (scope_name, servers) in &by_scope {
            println!("\n[{scope_name}]");
            for server in servers {
                if let ConfigItemData::McpServer(config) = &server.config {
                    let transport = format!("{:?}", config.transport).to_lowercase();
                    let detail = config
                        .command
                        .clone()
                        .or_else(|| config.url.clone())
                        .unwrap_or_default();
                    println!("  {} ({}) - {}", server.name, transport, detail);
                }
            }
        }
        println!();
    }

    Ok(())
}

fn execute_add(
    args: McpAddArgs,
    project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ops = McpOps::new(project_path.cloned());
    let config = args.to_config()?;
    let scope = args.parse_scope()?;

    let result = ops.add(&args.name, scope, config, args.dry_run)?;

    if args.json {
        let output = json!({
            "success": result.success,
            "operation": "add",
            "server": args.name,
            "scope": scope.to_string(),
            "dry_run": args.dry_run,
            "message": result.error,
            "backup_id": result.backup_id,
            "files_modified": result.files_modified.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if args.dry_run {
        println!(
            "Dry run: Would add MCP server '{}' to {} scope",
            args.name, scope
        );
        if !result.files_modified.is_empty() {
            println!("Would modify: {}", result.files_modified[0].display());
        }
    } else if result.success {
        println!("Added MCP server '{}' to {} scope", args.name, scope);
        if let Some(backup_id) = &result.backup_id {
            println!("Backup created: {backup_id}");
        }
    } else {
        eprintln!(
            "Failed to add MCP server: {}",
            result.error.unwrap_or_default()
        );
        std::process::exit(1);
    }

    Ok(())
}

fn execute_remove(
    name: &str,
    scope: Option<String>,
    dry_run: bool,
    json_output: bool,
    project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ops = McpOps::new(project_path.cloned());

    let scope_filter = if let Some(s) = &scope {
        Some(s.parse()?)
    } else {
        None
    };

    let result = ops.remove(name, scope_filter, dry_run)?;

    if json_output {
        let output = json!({
            "success": result.success,
            "operation": "remove",
            "server": name,
            "scope": result.scope.to_string(),
            "dry_run": dry_run,
            "message": result.error,
            "backup_id": result.backup_id,
            "files_modified": result.files_modified.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if dry_run {
        println!(
            "Dry run: Would remove MCP server '{}' from {} scope",
            name, result.scope
        );
        if !result.files_modified.is_empty() {
            println!("Would modify: {}", result.files_modified[0].display());
        }
    } else if result.success {
        println!("Removed MCP server '{}' from {} scope", name, result.scope);
        if let Some(backup_id) = &result.backup_id {
            println!("Backup created: {backup_id}");
        }
    } else {
        eprintln!(
            "Failed to remove MCP server: {}",
            result.error.unwrap_or_default()
        );
        std::process::exit(1);
    }

    Ok(())
}

fn execute_update(
    args: McpUpdateArgs,
    project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ops = McpOps::new(project_path.cloned());

    // Parse scope if provided
    let scope = args.scope.as_deref().and_then(|s| match s {
        "user" => Some(ConfigScope::User),
        "project" => Some(ConfigScope::Project),
        "local" => Some(ConfigScope::Local),
        _ => {
            eprintln!("Invalid scope: {}. Use 'user', 'project', or 'local'.", s);
            None
        }
    });

    // Build update struct
    let mut env: Option<HashMap<String, String>> = None;
    if !args.env.is_empty() {
        let mut map = HashMap::new();
        for pair in &args.env {
            if let Some((key, value)) = pair.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            } else {
                eprintln!("Invalid env format: '{}'. Use KEY=VALUE.", pair);
                std::process::exit(1);
            }
        }
        env = Some(map);
    }

    let mut add_env: Option<HashMap<String, String>> = None;
    if !args.add_env.is_empty() {
        let mut map = HashMap::new();
        for pair in &args.add_env {
            if let Some((key, value)) = pair.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            } else {
                eprintln!("Invalid add-env format: '{}'. Use KEY=VALUE.", pair);
                std::process::exit(1);
            }
        }
        add_env = Some(map);
    }

    let update = McpServerUpdate {
        command: args.command,
        args: if args.args.is_empty() {
            None
        } else {
            Some(args.args)
        },
        add_args: if args.add_args.is_empty() {
            None
        } else {
            Some(args.add_args)
        },
        env,
        add_env,
        remove_env: if args.remove_env.is_empty() {
            None
        } else {
            Some(args.remove_env)
        },
        url: args.url,
    };

    // Perform update
    let result = ops.update(&args.name, scope, update, args.dry_run)?;

    if args.json {
        println!(
            "{}",
            json!({
                "success": result.success,
                "name": result.name,
                "scope": result.scope.to_string(),
                "backupId": result.backup_id,
                "warnings": result.warnings,
                "dry_run": args.dry_run
            })
        );
    } else {
        if args.dry_run {
            println!("[DRY RUN] Would update MCP server '{}'", args.name);
        } else {
            println!("✓ Updated MCP server '{}'", args.name);
            if let Some(backup_id) = result.backup_id {
                println!("  Backup ID: {}", backup_id);
            }
        }

        if !result.warnings.is_empty() {
            for warning in result.warnings {
                println!("  ⚠ {}", warning);
            }
        }
    }

    Ok(())
}

fn execute_move(
    _name: &str,
    _from: Option<String>,
    _to: &str,
    _force: bool,
    _dry_run: bool,
    _json: bool,
    _project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement in Phase 4
    println!("MCP move not yet implemented");
    Ok(())
}
