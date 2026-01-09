//! Config item types and enum

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{
    AgentConfig, CommandConfig, ConfigScope, HookConfig, McpServerConfig, SkillConfig,
};
use super::error::ConfigError;

/// Validate a config item name
///
/// Names must:
/// - Not be empty
/// - Not contain path separators (/, \)
/// - Not contain .. (directory traversal)
/// - Not contain null bytes
/// - Not start or end with whitespace
/// - Be reasonably short (max 128 chars)
pub fn validate_name(name: &str) -> Result<(), ConfigError> {
    if name.is_empty() {
        return Err(ConfigError::ValidationError("name cannot be empty".into()));
    }

    if name.len() > 128 {
        return Err(ConfigError::ValidationError(
            "name cannot exceed 128 characters".into(),
        ));
    }

    if name != name.trim() {
        return Err(ConfigError::ValidationError(
            "name cannot start or end with whitespace".into(),
        ));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(ConfigError::ValidationError(
            "name cannot contain path separators (/ or \\)".into(),
        ));
    }

    if name.contains("..") {
        return Err(ConfigError::ValidationError(
            "name cannot contain '..' (directory traversal)".into(),
        ));
    }

    if name.contains('\0') {
        return Err(ConfigError::ValidationError(
            "name cannot contain null bytes".into(),
        ));
    }

    // Check for other problematic characters
    if name.contains(':') || name.contains('*') || name.contains('?')
        || name.contains('"') || name.contains('<') || name.contains('>')
        || name.contains('|') {
        return Err(ConfigError::ValidationError(
            "name contains invalid characters (:*?\"<>|)".into(),
        ));
    }

    Ok(())
}

/// Type of configuration item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigItemType {
    McpServer,
    Skill,
    Hook,
    Command,
    Agent,
}

impl ConfigItemType {
    /// Get a human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::McpServer => "MCP server",
            Self::Skill => "skill",
            Self::Hook => "hook",
            Self::Command => "command",
            Self::Agent => "agent",
        }
    }

    /// Get the plural form
    pub fn plural(&self) -> &'static str {
        match self {
            Self::McpServer => "MCP servers",
            Self::Skill => "skills",
            Self::Hook => "hooks",
            Self::Command => "commands",
            Self::Agent => "agents",
        }
    }
}

/// A configuration item with its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    /// Unique name/identifier
    pub name: String,
    /// Type of config item
    pub item_type: ConfigItemType,
    /// Which scope this item belongs to
    pub scope: ConfigScope,
    /// File path where this item is defined
    pub file_path: PathBuf,
    /// The actual configuration data
    pub config: ConfigItemData,
}

impl ConfigItem {
    /// Create a new config item
    pub fn new(
        name: String,
        item_type: ConfigItemType,
        scope: ConfigScope,
        file_path: PathBuf,
        config: ConfigItemData,
    ) -> Self {
        Self {
            name,
            item_type,
            scope,
            file_path,
            config,
        }
    }

    /// Get the display name for this item
    pub fn display_name(&self) -> String {
        format!("{} '{}'", self.item_type.display_name(), self.name)
    }
}

/// The actual configuration data for an item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfigItemData {
    McpServer(McpServerConfig),
    Skill(SkillConfig),
    Hook(HookConfig),
    Command(CommandConfig),
    Agent(AgentConfig),
}

impl ConfigItemData {
    /// Get the item type
    pub fn item_type(&self) -> ConfigItemType {
        match self {
            Self::McpServer(_) => ConfigItemType::McpServer,
            Self::Skill(_) => ConfigItemType::Skill,
            Self::Hook(_) => ConfigItemType::Hook,
            Self::Command(_) => ConfigItemType::Command,
            Self::Agent(_) => ConfigItemType::Agent,
        }
    }
}

impl From<McpServerConfig> for ConfigItemData {
    fn from(config: McpServerConfig) -> Self {
        Self::McpServer(config)
    }
}

impl From<SkillConfig> for ConfigItemData {
    fn from(config: SkillConfig) -> Self {
        Self::Skill(config)
    }
}

impl From<HookConfig> for ConfigItemData {
    fn from(config: HookConfig) -> Self {
        Self::Hook(config)
    }
}

impl From<CommandConfig> for ConfigItemData {
    fn from(config: CommandConfig) -> Self {
        Self::Command(config)
    }
}

impl From<AgentConfig> for ConfigItemData {
    fn from(config: AgentConfig) -> Self {
        Self::Agent(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("context7").is_ok());
        assert!(validate_name("my-server").is_ok());
        assert!(validate_name("my_server").is_ok());
        assert!(validate_name("MyServer123").is_ok());
        assert!(validate_name("a").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(129);
        assert!(validate_name(&long_name).is_err());

        let max_name = "a".repeat(128);
        assert!(validate_name(&max_name).is_ok());
    }

    #[test]
    fn test_validate_name_whitespace() {
        assert!(validate_name(" leading").is_err());
        assert!(validate_name("trailing ").is_err());
        assert!(validate_name(" both ").is_err());
        // Internal whitespace is ok
        assert!(validate_name("with space").is_ok());
    }

    #[test]
    fn test_validate_name_path_separators() {
        assert!(validate_name("path/to/thing").is_err());
        assert!(validate_name("path\\to\\thing").is_err());
    }

    #[test]
    fn test_validate_name_directory_traversal() {
        assert!(validate_name("..").is_err());
        assert!(validate_name("../parent").is_err());
        assert!(validate_name("name..ext").is_err());
    }

    #[test]
    fn test_validate_name_null_byte() {
        assert!(validate_name("name\0bad").is_err());
    }

    #[test]
    fn test_validate_name_special_chars() {
        assert!(validate_name("name:bad").is_err());
        assert!(validate_name("name*bad").is_err());
        assert!(validate_name("name?bad").is_err());
        assert!(validate_name("name\"bad").is_err());
        assert!(validate_name("name<bad").is_err());
        assert!(validate_name("name>bad").is_err());
        assert!(validate_name("name|bad").is_err());
    }
}
