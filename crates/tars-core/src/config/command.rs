//! Command configuration operations

use serde::{Deserialize, Serialize};

/// Command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    /// Command description
    pub description: String,

    /// Whether to enable thinking mode
    #[serde(default)]
    pub thinking: bool,

    /// Command template body (uses $ARGUMENTS placeholder)
    pub body: String,
}

impl CommandConfig {
    /// Create a new command config
    pub fn new(description: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            thinking: false,
            body: body.into(),
        }
    }

    /// Enable thinking mode
    pub fn with_thinking(mut self, thinking: bool) -> Self {
        self.thinking = thinking;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.description.is_empty() {
            return Err("description is required".into());
        }
        if self.body.is_empty() {
            return Err("body/template is required".into());
        }
        Ok(())
    }

    /// Generate frontmatter YAML
    pub fn to_frontmatter(&self) -> String {
        let mut lines = vec![format!("description: \"{}\"", self.description)];

        if self.thinking {
            lines.push("thinking: true".into());
        }

        lines.join("\n")
    }

    /// Generate full command .md content
    pub fn to_command_md(&self) -> String {
        format!("---\n{}\n---\n\n{}", self.to_frontmatter(), self.body)
    }
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            description: String::new(),
            thinking: false,
            body: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_config() {
        let config =
            CommandConfig::new("Review code", "Review the changes: $ARGUMENTS").with_thinking(true);

        assert_eq!(config.description, "Review code");
        assert!(config.thinking);
        assert!(config.body.contains("$ARGUMENTS"));
    }

    #[test]
    fn test_to_command_md() {
        let config = CommandConfig::new("Test", "Do the thing");
        let md = config.to_command_md();

        assert!(md.starts_with("---\n"));
        assert!(md.contains("description: \"Test\""));
        assert!(md.ends_with("Do the thing"));
    }

    #[test]
    fn test_validation() {
        let valid = CommandConfig::new("Test", "Body");
        assert!(valid.validate().is_ok());

        let no_desc = CommandConfig {
            description: String::new(),
            body: "body".into(),
            ..Default::default()
        };
        assert!(no_desc.validate().is_err());
    }
}
