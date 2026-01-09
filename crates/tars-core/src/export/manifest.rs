//! Plugin manifest generation

use serde::Serialize;
use tars_scanner::plugins::Author;

/// Plugin manifest for export
#[derive(Debug, Serialize)]
pub struct ExportManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<String>,
    #[serde(rename = "mcpServers", skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<String>,
}

impl ExportManifest {
    /// Create a new manifest with the given name and version
    #[must_use]
    pub fn new(name: String, version: String, description: String) -> Self {
        Self {
            name,
            version,
            description,
            author: None,
            commands: Vec::new(),
            agents: None,
            skills: None,
            hooks: None,
            mcp_servers: None,
        }
    }
}
