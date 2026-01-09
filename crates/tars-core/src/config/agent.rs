//! Agent configuration operations

use serde::{Deserialize, Serialize};

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent description
    pub description: String,

    /// Allowed tools for this agent
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,

    /// Preferred model for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Permission mode (ask, auto, deny)
    #[serde(default, rename = "permission-mode", skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,

    /// Available skills for this agent
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,

    /// Agent instructions/body content
    pub body: String,
}

impl AgentConfig {
    /// Create a new agent config
    pub fn new(description: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            tools: Vec::new(),
            model: None,
            permission_mode: None,
            skills: Vec::new(),
            body: body.into(),
        }
    }

    /// Set allowed tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// Set preferred model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set permission mode
    pub fn with_permission_mode(mut self, mode: impl Into<String>) -> Self {
        self.permission_mode = Some(mode.into());
        self
    }

    /// Set available skills
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
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
        if let Some(ref mode) = self.permission_mode {
            if !["ask", "auto", "deny"].contains(&mode.as_str()) {
                return Err(format!("invalid permission-mode: {}", mode));
            }
        }
        Ok(())
    }

    /// Generate frontmatter YAML
    pub fn to_frontmatter(&self) -> String {
        let mut lines = vec![format!("description: \"{}\"", self.description)];

        if !self.tools.is_empty() {
            lines.push(format!("tools: [{}]", self.tools.join(", ")));
        }

        if let Some(ref model) = self.model {
            lines.push(format!("model: {}", model));
        }

        if let Some(ref mode) = self.permission_mode {
            lines.push(format!("permission-mode: {}", mode));
        }

        if !self.skills.is_empty() {
            lines.push(format!("skills: [{}]", self.skills.join(", ")));
        }

        lines.join("\n")
    }

    /// Generate full agent .md content
    pub fn to_agent_md(&self) -> String {
        format!("---\n{}\n---\n\n{}", self.to_frontmatter(), self.body)
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            description: String::new(),
            tools: Vec::new(),
            model: None,
            permission_mode: None,
            skills: Vec::new(),
            body: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_builder() {
        let config = AgentConfig::new("Security reviewer", "Review code for security issues")
            .with_tools(vec!["Read".into(), "Grep".into()])
            .with_model("sonnet")
            .with_permission_mode("ask");

        assert_eq!(config.description, "Security reviewer");
        assert_eq!(config.tools, vec!["Read", "Grep"]);
        assert_eq!(config.model, Some("sonnet".into()));
        assert_eq!(config.permission_mode, Some("ask".into()));
    }

    #[test]
    fn test_to_agent_md() {
        let config = AgentConfig::new("Test agent", "Do security things")
            .with_tools(vec!["Read".into()]);

        let md = config.to_agent_md();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("description: \"Test agent\""));
        assert!(md.contains("tools: [Read]"));
        assert!(md.ends_with("Do security things"));
    }

    #[test]
    fn test_validation() {
        let valid = AgentConfig::new("Test", "Body");
        assert!(valid.validate().is_ok());

        let invalid_mode = AgentConfig::new("Test", "Body")
            .with_permission_mode("invalid");
        assert!(invalid_mode.validate().is_err());
    }
}
