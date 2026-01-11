//! Configuration scope handling
//!
//! Defines the hierarchy of scopes where config items can live.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use super::error::{ConfigError, ConfigResult};

/// Configuration scope - where a config item lives
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigScope {
    /// User-global scope (~/.claude/, ~/.claude.json)
    User,
    /// Project scope (.claude/, .mcp.json in repo root)
    Project,
    /// Local scope (.claude/settings.local.json) - gitignored
    Local,
    /// Managed scope (read-only, system-level)
    Managed,
}

impl ConfigScope {
    /// Whether this scope can be modified
    #[must_use]
    pub fn is_writable(&self) -> bool {
        !matches!(self, Self::Managed)
    }

    /// Precedence order (higher = takes priority)
    #[must_use]
    pub fn precedence(&self) -> u8 {
        match self {
            Self::Managed => 4,
            Self::Local => 3,
            Self::Project => 2,
            Self::User => 1,
        }
    }

    /// Get all writable scopes
    #[must_use]
    pub fn writable_scopes() -> &'static [ConfigScope] {
        &[Self::User, Self::Project, Self::Local]
    }

    /// Get the base directory for this scope (cross-platform)
    pub fn base_dir(&self, project_path: Option<&PathBuf>) -> ConfigResult<PathBuf> {
        match self {
            Self::User => dirs::home_dir()
                .ok_or_else(|| ConfigError::Internal("Cannot find home directory".into())),
            Self::Project | Self::Local => project_path.cloned().ok_or_else(|| {
                ConfigError::ValidationError("Project path required for project/local scope".into())
            }),
            Self::Managed => Ok(Self::managed_base_dir()),
        }
    }

    /// Get the managed scope base directory (cross-platform)
    #[must_use]
    fn managed_base_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Library/Application Support/ClaudeCode")
        }
        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/etc/claude")
        }
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(r"C:\ProgramData\ClaudeCode")
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            PathBuf::from("/etc/claude")
        }
    }

    /// Get the .claude directory for this scope
    pub fn claude_dir(&self, project_path: Option<&PathBuf>) -> ConfigResult<PathBuf> {
        let base = self.base_dir(project_path)?;
        match self {
            Self::User => Ok(base.join(".claude")),
            Self::Project | Self::Local => Ok(base.join(".claude")),
            Self::Managed => Ok(base),
        }
    }

    /// Get the settings.json path for this scope
    pub fn settings_path(&self, project_path: Option<&PathBuf>) -> ConfigResult<PathBuf> {
        let claude_dir = self.claude_dir(project_path)?;
        match self {
            Self::User => Ok(claude_dir.join("settings.json")),
            Self::Project => Ok(claude_dir.join("settings.json")),
            Self::Local => Ok(claude_dir.join("settings.local.json")),
            Self::Managed => Ok(claude_dir.join("managed-settings.json")),
        }
    }

    /// Get the MCP config path for this scope
    pub fn mcp_path(&self, project_path: Option<&PathBuf>) -> ConfigResult<PathBuf> {
        match self {
            Self::User => {
                let home = dirs::home_dir()
                    .ok_or_else(|| ConfigError::Internal("Cannot find home directory".into()))?;
                Ok(home.join(".claude.json"))
            }
            Self::Project => {
                let base = project_path
                    .ok_or_else(|| ConfigError::ValidationError("Project path required".into()))?;
                Ok(base.join(".mcp.json"))
            }
            Self::Local => {
                // Local scope uses project .mcp.json (no separate local MCP config)
                let base = project_path
                    .ok_or_else(|| ConfigError::ValidationError("Project path required".into()))?;
                Ok(base.join(".mcp.json"))
            }
            Self::Managed => Err(ConfigError::ManagedScope),
        }
    }

    /// Get the skills directory for this scope
    pub fn skills_dir(&self, project_path: Option<&PathBuf>) -> ConfigResult<PathBuf> {
        let claude_dir = self.claude_dir(project_path)?;
        Ok(claude_dir.join("skills"))
    }

    /// Get the commands directory for this scope
    pub fn commands_dir(&self, project_path: Option<&PathBuf>) -> ConfigResult<PathBuf> {
        let claude_dir = self.claude_dir(project_path)?;
        Ok(claude_dir.join("commands"))
    }

    /// Get the agents directory for this scope
    pub fn agents_dir(&self, project_path: Option<&PathBuf>) -> ConfigResult<PathBuf> {
        let claude_dir = self.claude_dir(project_path)?;
        Ok(claude_dir.join("agents"))
    }
}

impl fmt::Display for ConfigScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Project => write!(f, "project"),
            Self::Local => write!(f, "local"),
            Self::Managed => write!(f, "managed"),
        }
    }
}

impl FromStr for ConfigScope {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" | "global" => Ok(Self::User),
            "project" => Ok(Self::Project),
            "local" => Ok(Self::Local),
            "managed" => Ok(Self::Managed),
            _ => Err(ConfigError::InvalidScope(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_precedence() {
        assert!(ConfigScope::Managed.precedence() > ConfigScope::Local.precedence());
        assert!(ConfigScope::Local.precedence() > ConfigScope::Project.precedence());
        assert!(ConfigScope::Project.precedence() > ConfigScope::User.precedence());
    }

    #[test]
    fn test_scope_writable() {
        assert!(ConfigScope::User.is_writable());
        assert!(ConfigScope::Project.is_writable());
        assert!(ConfigScope::Local.is_writable());
        assert!(!ConfigScope::Managed.is_writable());
    }

    #[test]
    fn test_scope_from_str() {
        assert_eq!(ConfigScope::from_str("user").unwrap(), ConfigScope::User);
        assert_eq!(ConfigScope::from_str("global").unwrap(), ConfigScope::User);
        assert_eq!(
            ConfigScope::from_str("project").unwrap(),
            ConfigScope::Project
        );
        assert_eq!(ConfigScope::from_str("local").unwrap(), ConfigScope::Local);
        assert!(ConfigScope::from_str("invalid").is_err());
    }
}
