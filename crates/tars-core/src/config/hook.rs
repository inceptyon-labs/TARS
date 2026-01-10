//! Hook configuration operations

use serde::{Deserialize, Serialize};

/// Hook trigger types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookTrigger {
    PreToolUse,
    PostToolUse,
    PermissionRequest,
    UserPromptSubmit,
    SessionStart,
    SessionEnd,
    Notification,
    Stop,
    SubagentStop,
    PreCompact,
}

impl HookTrigger {
    /// Get the JSON key name for this trigger
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PermissionRequest => "PermissionRequest",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::Notification => "Notification",
            Self::Stop => "Stop",
            Self::SubagentStop => "SubagentStop",
            Self::PreCompact => "PreCompact",
        }
    }

    /// All available triggers
    pub fn all() -> &'static [HookTrigger] {
        &[
            Self::PreToolUse,
            Self::PostToolUse,
            Self::PermissionRequest,
            Self::UserPromptSubmit,
            Self::SessionStart,
            Self::SessionEnd,
            Self::Notification,
            Self::Stop,
            Self::SubagentStop,
            Self::PreCompact,
        ]
    }
}

impl std::str::FromStr for HookTrigger {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PreToolUse" => Ok(Self::PreToolUse),
            "PostToolUse" => Ok(Self::PostToolUse),
            "PermissionRequest" => Ok(Self::PermissionRequest),
            "UserPromptSubmit" => Ok(Self::UserPromptSubmit),
            "SessionStart" => Ok(Self::SessionStart),
            "SessionEnd" => Ok(Self::SessionEnd),
            "Notification" => Ok(Self::Notification),
            "Stop" => Ok(Self::Stop),
            "SubagentStop" => Ok(Self::SubagentStop),
            "PreCompact" => Ok(Self::PreCompact),
            _ => Err(format!("Unknown hook trigger: {}", s)),
        }
    }
}

impl std::fmt::Display for HookTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Hook definition - what action the hook takes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HookDefinition {
    /// Run a shell command
    Command { command: String },
    /// Inject a prompt
    Prompt { prompt: String },
    /// Invoke an agent
    Agent { agent: String },
}

impl HookDefinition {
    /// Create a command hook
    pub fn command(cmd: impl Into<String>) -> Self {
        Self::Command {
            command: cmd.into(),
        }
    }

    /// Create a prompt hook
    pub fn prompt(prompt: impl Into<String>) -> Self {
        Self::Prompt {
            prompt: prompt.into(),
        }
    }

    /// Create an agent hook
    pub fn agent(agent: impl Into<String>) -> Self {
        Self::Agent {
            agent: agent.into(),
        }
    }

    /// Get a display string
    pub fn display(&self) -> String {
        match self {
            Self::Command { command } => format!("command: {}", command),
            Self::Prompt { prompt } => format!("prompt: {}", truncate(prompt, 50)),
            Self::Agent { agent } => format!("agent: {}", agent),
        }
    }
}

/// Hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// The trigger that activates this hook
    pub trigger: HookTrigger,

    /// Optional matcher pattern (for tool-specific hooks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,

    /// The hook definition (what to do)
    pub definition: HookDefinition,
}

impl HookConfig {
    /// Create a new hook config
    pub fn new(trigger: HookTrigger, definition: HookDefinition) -> Self {
        Self {
            trigger,
            definition,
            matcher: None,
        }
    }

    /// Set matcher pattern
    pub fn with_matcher(mut self, matcher: impl Into<String>) -> Self {
        self.matcher = Some(matcher.into());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        match &self.definition {
            HookDefinition::Command { command } if command.is_empty() => {
                Err("command cannot be empty".into())
            }
            HookDefinition::Prompt { prompt } if prompt.is_empty() => {
                Err("prompt cannot be empty".into())
            }
            HookDefinition::Agent { agent } if agent.is_empty() => {
                Err("agent cannot be empty".into())
            }
            _ => Ok(()),
        }
    }

    /// Convert to the JSON format used in settings.json
    pub fn to_json_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();

        if let Some(ref matcher) = self.matcher {
            obj.insert("matcher".into(), serde_json::Value::String(matcher.clone()));
        }

        match &self.definition {
            HookDefinition::Command { command } => {
                obj.insert("command".into(), serde_json::Value::String(command.clone()));
            }
            HookDefinition::Prompt { prompt } => {
                obj.insert("prompt".into(), serde_json::Value::String(prompt.clone()));
            }
            HookDefinition::Agent { agent } => {
                obj.insert("agent".into(), serde_json::Value::String(agent.clone()));
            }
        }

        serde_json::Value::Object(obj)
    }
}

/// Truncate a string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_trigger_parse() {
        assert_eq!(
            "PreToolUse".parse::<HookTrigger>().unwrap(),
            HookTrigger::PreToolUse
        );
        assert_eq!(
            "SessionStart".parse::<HookTrigger>().unwrap(),
            HookTrigger::SessionStart
        );
        assert!("Invalid".parse::<HookTrigger>().is_err());
    }

    #[test]
    fn test_hook_config() {
        let hook = HookConfig::new(
            HookTrigger::PreToolUse,
            HookDefinition::command("./lint.sh"),
        )
        .with_matcher("Bash");

        assert_eq!(hook.trigger, HookTrigger::PreToolUse);
        assert_eq!(hook.matcher, Some("Bash".into()));
        assert!(hook.validate().is_ok());
    }

    #[test]
    fn test_hook_to_json() {
        let hook = HookConfig::new(
            HookTrigger::UserPromptSubmit,
            HookDefinition::prompt("Think step by step"),
        );

        let json = hook.to_json_value();
        assert_eq!(json["prompt"], "Think step by step");
    }
}
