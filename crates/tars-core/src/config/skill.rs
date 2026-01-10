//! Skill configuration operations

use serde::{Deserialize, Serialize};

/// Skill configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillConfig {
    /// Skill description
    pub description: String,

    /// Whether the skill can be invoked directly by users (e.g., /skill-name)
    #[serde(default, rename = "user-invocable")]
    pub user_invocable: bool,

    /// List of allowed tools for this skill
    #[serde(
        default,
        rename = "allowed-tools",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub allowed_tools: Vec<String>,

    /// Preferred model for this skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Skill instructions/body content
    pub body: String,
}

impl SkillConfig {
    /// Create a new skill config
    pub fn new(description: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            user_invocable: false,
            allowed_tools: Vec::new(),
            model: None,
            body: body.into(),
        }
    }

    /// Set user invocable
    #[must_use]
    pub fn with_user_invocable(mut self, invocable: bool) -> Self {
        self.user_invocable = invocable;
        self
    }

    /// Set allowed tools
    #[must_use]
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Set preferred model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.description.is_empty() {
            return Err("description is required".into());
        }
        if self.body.is_empty() {
            return Err("body/instructions is required".into());
        }
        Ok(())
    }

    /// Generate SKILL.md frontmatter YAML
    #[must_use]
    pub fn to_frontmatter(&self) -> String {
        let mut lines = vec![format!("description: \"{}\"", self.description)];

        if self.user_invocable {
            lines.push("user-invocable: true".into());
        }

        if !self.allowed_tools.is_empty() {
            lines.push(format!(
                "allowed-tools: [{}]",
                self.allowed_tools.join(", ")
            ));
        }

        if let Some(ref model) = self.model {
            lines.push(format!("model: {model}"));
        }

        lines.join("\n")
    }

    /// Generate full SKILL.md content
    #[must_use]
    pub fn to_skill_md(&self) -> String {
        format!("---\n{}\n---\n\n{}", self.to_frontmatter(), self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_config_builder() {
        let config = SkillConfig::new("Code review", "Review the code for issues")
            .with_user_invocable(true)
            .with_allowed_tools(vec!["Read".into(), "Grep".into()])
            .with_model("sonnet");

        assert_eq!(config.description, "Code review");
        assert!(config.user_invocable);
        assert_eq!(config.allowed_tools, vec!["Read", "Grep"]);
        assert_eq!(config.model, Some("sonnet".into()));
    }

    #[test]
    fn test_to_skill_md() {
        let config = SkillConfig::new("Test skill", "Do the thing").with_user_invocable(true);

        let md = config.to_skill_md();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("description: \"Test skill\""));
        assert!(md.contains("user-invocable: true"));
        assert!(md.ends_with("Do the thing"));
    }

    #[test]
    fn test_validation() {
        let valid = SkillConfig::new("Test", "Body");
        assert!(valid.validate().is_ok());

        let no_desc = SkillConfig {
            description: String::new(),
            body: "body".into(),
            ..Default::default()
        };
        assert!(no_desc.validate().is_err());

        let no_body = SkillConfig {
            description: "desc".into(),
            body: String::new(),
            ..Default::default()
        };
        assert!(no_body.validate().is_err());
    }
}
