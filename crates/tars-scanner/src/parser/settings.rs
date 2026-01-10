//! Settings.json parser

use crate::error::{ScanError, ScanResult};
use crate::settings::{Permissions, SettingsFile};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Raw settings.json structure for parsing
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSettings {
    #[serde(default)]
    env: HashMap<String, String>,
    permissions: Option<RawPermissions>,
    #[serde(default)]
    hooks: HashMap<String, serde_json::Value>,
    model: Option<String>,
    #[serde(default)]
    enabled_plugins: HashMap<String, bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPermissions {
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    deny: Vec<String>,
    default_mode: Option<String>,
}

/// Parse a settings.json file
///
/// # Errors
/// Returns an error if parsing fails
pub fn parse_settings(path: &Path, content: &str) -> ScanResult<SettingsFile> {
    let raw: RawSettings = serde_json::from_str(content).map_err(ScanError::JsonParse)?;

    let sha256 = compute_sha256(content);
    let hooks_count = raw.hooks.len();

    let permissions = raw.permissions.map(|p| Permissions {
        allow: p.allow,
        deny: p.deny,
        default_mode: p.default_mode,
    });

    Ok(SettingsFile {
        path: path.to_path_buf(),
        sha256,
        hooks_count,
        permissions,
        enabled_plugins: raw.enabled_plugins,
        env: raw.env,
        model: raw.model,
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
    fn test_parse_settings() {
        let content = r#"{
            "env": { "FOO": "bar" },
            "permissions": {
                "allow": ["Bash(npm:*)"],
                "deny": ["Read(.env)"],
                "defaultMode": "acceptEdits"
            },
            "hooks": {
                "PostToolUse": []
            },
            "model": "opus",
            "enabledPlugins": { "test@marketplace": true }
        }"#;

        let result = parse_settings(&PathBuf::from("settings.json"), content);
        assert!(result.is_ok());
        let settings = result.unwrap();
        assert_eq!(settings.hooks_count, 1);
        assert_eq!(settings.model, Some("opus".to_string()));
        assert!(settings.permissions.is_some());
        let perms = settings.permissions.unwrap();
        assert_eq!(perms.allow, vec!["Bash(npm:*)"]);
        assert_eq!(perms.deny, vec!["Read(.env)"]);
    }
}
