//! MCP configuration parser

use crate::error::ScanResult;
use crate::settings::{McpConfig, McpServer, McpTransport};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Raw .mcp.json structure (Claude Code format with mcpServers wrapper)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMcpConfig {
    #[serde(default)]
    mcp_servers: HashMap<String, RawMcpServer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMcpServer {
    #[serde(rename = "type")]
    transport_type: Option<String>,
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    url: Option<String>,
}

/// Convert raw servers map to `McpServer` vec
fn convert_servers(servers: HashMap<String, RawMcpServer>) -> Vec<McpServer> {
    servers
        .into_iter()
        .map(|(name, server)| {
            let transport = match server.transport_type.as_deref() {
                Some("http") => McpTransport::Http,
                Some("sse") => McpTransport::Sse,
                _ => McpTransport::Stdio,
            };

            McpServer {
                name,
                transport,
                command: server.command,
                args: server.args,
                env: server.env,
                url: server.url,
            }
        })
        .collect()
}

/// Parse an MCP configuration file
///
/// Supports two formats:
/// 1. Claude Code format: `{"mcpServers": {"name": {...}}}`
/// 2. Plugin format: `{"name": {...}}`
///
/// # Errors
/// Returns an error if parsing fails
pub fn parse_mcp_config(path: &Path, content: &str) -> ScanResult<McpConfig> {
    let sha256 = compute_sha256(content);

    // First try the Claude Code format with mcpServers wrapper
    if let Ok(raw) = serde_json::from_str::<RawMcpConfig>(content) {
        if !raw.mcp_servers.is_empty() {
            return Ok(McpConfig {
                path: path.to_path_buf(),
                sha256,
                servers: convert_servers(raw.mcp_servers),
                source_plugin: None,
            });
        }
    }

    // Then try the plugin format (flat object with server names as keys)
    if let Ok(servers) = serde_json::from_str::<HashMap<String, RawMcpServer>>(content) {
        return Ok(McpConfig {
            path: path.to_path_buf(),
            sha256,
            servers: convert_servers(servers),
            source_plugin: None,
        });
    }

    // If neither format works, return empty config
    Ok(McpConfig {
        path: path.to_path_buf(),
        sha256,
        servers: Vec::new(),
        source_plugin: None,
    })
}

fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_mcp_config_claude_code_format() {
        let content = r#"{
            "mcpServers": {
                "test-server": {
                    "type": "stdio",
                    "command": "/usr/bin/test",
                    "args": ["--flag"],
                    "env": { "KEY": "value" }
                }
            }
        }"#;

        let result = parse_mcp_config(&PathBuf::from(".mcp.json"), content);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.servers.len(), 1);
        let server = &config.servers[0];
        assert_eq!(server.name, "test-server");
        assert_eq!(server.transport, McpTransport::Stdio);
        assert_eq!(server.command, Some("/usr/bin/test".to_string()));
    }

    #[test]
    fn test_parse_mcp_config_plugin_format() {
        // Plugin format doesn't have mcpServers wrapper
        let content = r#"{
            "supabase": {
                "type": "http",
                "url": "https://mcp.supabase.com/mcp"
            }
        }"#;

        let result = parse_mcp_config(&PathBuf::from(".mcp.json"), content);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.servers.len(), 1);
        let server = &config.servers[0];
        assert_eq!(server.name, "supabase");
        assert_eq!(server.transport, McpTransport::Http);
        assert_eq!(server.url, Some("https://mcp.supabase.com/mcp".to_string()));
    }

    #[test]
    fn test_parse_mcp_config_sse_transport() {
        let content = r#"{
            "asana": {
                "type": "sse",
                "url": "https://mcp.asana.com/sse"
            }
        }"#;

        let result = parse_mcp_config(&PathBuf::from(".mcp.json"), content);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.servers.len(), 1);
        let server = &config.servers[0];
        assert_eq!(server.name, "asana");
        assert_eq!(server.transport, McpTransport::Sse);
        assert_eq!(server.url, Some("https://mcp.asana.com/sse".to_string()));
    }
}
