//! Managed scope scanner
//!
//! Scans IT-deployed (managed) configuration from /etc/claude/

use crate::error::ScanResult;
use crate::inventory::ManagedScope;
use crate::parser::{parse_mcp_config, parse_settings};
use crate::settings::{McpConfig, SettingsFile};
use std::fs;
use std::path::{Path, PathBuf};

/// Get the managed configuration directory
fn managed_dir() -> PathBuf {
    PathBuf::from("/etc/claude")
}

/// Scan managed (IT-deployed) Claude Code configuration
///
/// Managed configuration is deployed by IT administrators and takes
/// highest precedence in the scope hierarchy.
///
/// # Errors
/// Returns an error if scanning fails
pub fn scan_managed_scope() -> ScanResult<Option<ManagedScope>> {
    let managed_dir = managed_dir();

    // If the managed directory doesn't exist, there's no managed config
    if !managed_dir.exists() {
        return Ok(None);
    }

    let settings = scan_managed_settings(&managed_dir)?;
    let mcp = scan_managed_mcp(&managed_dir)?;

    // Only return Some if there's actual managed configuration
    if settings.is_some() || mcp.is_some() {
        Ok(Some(ManagedScope { settings, mcp }))
    } else {
        Ok(None)
    }
}

fn scan_managed_settings(managed_dir: &Path) -> ScanResult<Option<SettingsFile>> {
    let settings_path = managed_dir.join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let settings = parse_settings(&settings_path, &content)?;
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}

fn scan_managed_mcp(managed_dir: &Path) -> ScanResult<Option<McpConfig>> {
    let mcp_path = managed_dir.join("mcp.json");
    if mcp_path.exists() {
        let content = fs::read_to_string(&mcp_path)?;
        let mcp = parse_mcp_config(&mcp_path, &content)?;
        Ok(Some(mcp))
    } else {
        Ok(None)
    }
}
