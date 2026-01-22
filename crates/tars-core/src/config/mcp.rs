//! MCP Server configuration operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP server transport type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum McpTransport {
    #[default]
    Stdio,
    Http,
    Sse,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Transport type (stdio, http, sse)
    #[serde(rename = "type", default)]
    pub transport: McpTransport,

    /// Command to execute (for stdio transport)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Command arguments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// Environment variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// URL (for http/sse transport)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Documentation/project page URL (TARS-specific, ignored by Claude Code)
    #[serde(rename = "docsUrl", skip_serializing_if = "Option::is_none", default)]
    pub docs_url: Option<String>,
}

impl McpServerConfig {
    /// Create a new stdio MCP server config
    pub fn stdio(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            transport: McpTransport::Stdio,
            command: Some(command.into()),
            args,
            env: HashMap::new(),
            url: None,
            docs_url: None,
        }
    }

    /// Create a new HTTP MCP server config
    pub fn http(url: impl Into<String>) -> Self {
        Self {
            transport: McpTransport::Http,
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some(url.into()),
            docs_url: None,
        }
    }

    /// Create a new SSE MCP server config
    pub fn sse(url: impl Into<String>) -> Self {
        Self {
            transport: McpTransport::Sse,
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some(url.into()),
            docs_url: None,
        }
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Add a documentation URL
    pub fn with_docs_url(mut self, url: impl Into<String>) -> Self {
        self.docs_url = Some(url.into());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        match self.transport {
            McpTransport::Stdio => {
                if self.command.is_none() {
                    return Err("stdio transport requires 'command' field".into());
                }
            }
            McpTransport::Http | McpTransport::Sse => {
                if self.url.is_none() {
                    return Err(format!(
                        "{:?} transport requires 'url' field",
                        self.transport
                    ));
                }
            }
        }
        Ok(())
    }

    /// Get a display string for this config
    #[must_use]
    pub fn display(&self) -> String {
        match self.transport {
            McpTransport::Stdio => {
                let cmd = self.command.as_deref().unwrap_or("");
                let args = self.args.join(" ");
                if args.is_empty() {
                    cmd.to_string()
                } else {
                    format!("{cmd} {args}")
                }
            }
            McpTransport::Http | McpTransport::Sse => self.url.as_deref().unwrap_or("").to_string(),
        }
    }
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            transport: McpTransport::Stdio,
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: None,
            docs_url: None,
        }
    }
}

/// Updates to apply to an MCP server configuration
#[derive(Debug, Clone, Default)]
pub struct McpServerUpdate {
    /// Replace command entirely
    pub command: Option<String>,
    /// Replace all arguments
    pub args: Option<Vec<String>>,
    /// Add arguments to existing list
    pub add_args: Option<Vec<String>>,
    /// Replace all environment variables
    pub env: Option<HashMap<String, String>>,
    /// Add environment variables to existing map
    pub add_env: Option<HashMap<String, String>>,
    /// Remove specific environment variable keys
    pub remove_env: Option<Vec<String>>,
    /// Replace URL
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_config() {
        let config = McpServerConfig::stdio("npx", vec!["-y".into(), "@context7/mcp".into()]);
        assert_eq!(config.transport, McpTransport::Stdio);
        assert_eq!(config.command, Some("npx".into()));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_http_config() {
        let config = McpServerConfig::http("https://api.example.com/mcp");
        assert_eq!(config.transport, McpTransport::Http);
        assert_eq!(config.url, Some("https://api.example.com/mcp".into()));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_stdio_config() {
        let config = McpServerConfig {
            transport: McpTransport::Stdio,
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: None,
            docs_url: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_with_env() {
        let config = McpServerConfig::stdio("npx", vec![])
            .with_env("API_KEY", "secret")
            .with_env("DEBUG", "true");
        assert_eq!(config.env.get("API_KEY"), Some(&"secret".to_string()));
        assert_eq!(config.env.get("DEBUG"), Some(&"true".to_string()));
    }
}
