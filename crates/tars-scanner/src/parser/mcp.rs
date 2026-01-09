//! MCP configuration parser

use crate::error::{ScanError, ScanResult};
use crate::settings::{McpConfig, McpServer, McpTransport};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Raw .mcp.json structure
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

/// Parse an MCP configuration file
///
/// # Errors
/// Returns an error if parsing fails
pub fn parse_mcp_config(path: &Path, content: &str) -> ScanResult<McpConfig> {
    let raw: RawMcpConfig =
        serde_json::from_str(content).map_err(ScanError::JsonParse)?;

    let sha256 = compute_sha256(content);

    let servers = raw
        .mcp_servers
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
        .collect();

    Ok(McpConfig {
        path: path.to_path_buf(),
        sha256,
        servers,
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
    fn test_parse_mcp_config() {
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
}
