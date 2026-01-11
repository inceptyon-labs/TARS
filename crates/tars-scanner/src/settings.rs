//! Settings and MCP configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Parsed settings.json content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsFile {
    /// Path to the settings file
    pub path: PathBuf,
    /// SHA256 hash of file content
    pub sha256: String,
    /// Number of hooks defined
    pub hooks_count: usize,
    /// Permissions configuration
    pub permissions: Option<Permissions>,
    /// Enabled plugins map
    #[serde(default)]
    pub enabled_plugins: HashMap<String, bool>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Model override
    pub model: Option<String>,
}

/// Permissions configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Permissions {
    /// Allowed patterns
    #[serde(default)]
    pub allow: Vec<String>,
    /// Denied patterns
    #[serde(default)]
    pub deny: Vec<String>,
    /// Default permission mode
    pub default_mode: Option<String>,
}

/// MCP (Model Context Protocol) configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// Path to the config file
    #[serde(default)]
    pub path: PathBuf,
    /// SHA256 hash of file content
    #[serde(default)]
    pub sha256: String,
    /// Configured servers
    #[serde(default)]
    pub servers: Vec<McpServer>,
    /// Source plugin if this config came from a plugin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_plugin: Option<String>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Server name
    pub name: String,
    /// Transport type
    pub transport: McpTransport,
    /// Command to run (for stdio)
    pub command: Option<String>,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// URL (for http/sse)
    pub url: Option<String>,
}

/// MCP transport types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
}
