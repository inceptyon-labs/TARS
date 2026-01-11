//! Shared types for the TARS scanner

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

/// Scope where an artifact was found
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "plugin_id")]
pub enum Scope {
    /// User-level (~/.claude/)
    User,
    /// Project-level (.claude/)
    Project,
    /// Local project overrides (.claude/settings.local.json)
    Local,
    /// IT-managed (/Library/Application Support/ClaudeCode/)
    Managed,
    /// From a plugin
    Plugin(String),
}

impl FromStr for Scope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Scope::User),
            "project" => Ok(Scope::Project),
            "local" => Ok(Scope::Local),
            "managed" => Ok(Scope::Managed),
            _ => Err(format!("Invalid scope: {s}")),
        }
    }
}

/// Information about a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Path to the file
    pub path: PathBuf,
    /// SHA256 hash of the file contents
    pub sha256: String,
}

/// Host system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    /// Operating system
    pub os: String,
    /// Current username
    pub username: String,
    /// Home directory path
    pub home_dir: PathBuf,
}

impl HostInfo {
    /// Create `HostInfo` for the current system
    #[must_use]
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            username: whoami_username(),
            home_dir: dirs_home_dir(),
        }
    }
}

fn whoami_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn dirs_home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" }))
}
