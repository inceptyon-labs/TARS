//! Artifact types: Skills, Commands, Agents, Hooks

use crate::types::Scope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill information parsed from SKILL.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    /// Path to the skill directory
    pub path: PathBuf,
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Whether the skill can be invoked by users
    #[serde(default)]
    pub user_invocable: bool,
    /// Whether model invocation is disabled
    #[serde(default)]
    pub disable_model_invocation: bool,
    /// Allowed tools
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Model override
    pub model: Option<String>,
    /// Context setting
    pub context: Option<String>,
    /// Agent to use
    pub agent: Option<String>,
    /// Embedded hooks
    #[serde(default)]
    pub hooks: HashMap<String, Vec<HookDefinition>>,
    /// SHA256 hash of SKILL.md content
    pub sha256: String,
    /// Scope where found
    pub scope: Scope,
}

/// Command information parsed from .md file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    /// Path to the command file
    pub path: PathBuf,
    /// Command name (derived from filename)
    pub name: String,
    /// Command description
    pub description: Option<String>,
    /// Whether extended thinking is enabled
    #[serde(default)]
    pub thinking: bool,
    /// Command body (template)
    pub body: String,
    /// SHA256 hash of file content
    pub sha256: String,
    /// Scope where found
    pub scope: Scope,
}

/// Agent information parsed from .md file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Path to the agent file
    pub path: PathBuf,
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Allowed tools
    #[serde(default)]
    pub tools: Vec<String>,
    /// Model override
    pub model: Option<String>,
    /// Permission mode
    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,
    /// Skills this agent can use
    #[serde(default)]
    pub skills: Vec<String>,
    /// Embedded hooks
    #[serde(default)]
    pub hooks: HashMap<String, Vec<HookDefinition>>,
    /// SHA256 hash of file content
    pub sha256: String,
    /// Scope where found
    pub scope: Scope,
}

fn default_permission_mode() -> String {
    "default".to_string()
}

/// Hook information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInfo {
    /// Where the hook is defined
    pub source: HookSource,
    /// Hook trigger event
    pub trigger: HookTrigger,
    /// Optional matcher pattern
    pub matcher: Option<String>,
    /// Hook definition
    pub definition: HookDefinition,
}

/// Source of a hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HookSource {
    /// From a settings file
    Settings { path: PathBuf },
    /// From a skill
    Skill { name: String },
    /// From an agent
    Agent { name: String },
    /// From a plugin
    Plugin { plugin_id: String, path: PathBuf },
}

/// Hook trigger events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Hook action definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HookDefinition {
    /// Run a shell command
    #[serde(rename = "command")]
    Command { command: String },
    /// Send a prompt
    #[serde(rename = "prompt")]
    Prompt { prompt: String },
    /// Invoke an agent
    #[serde(rename = "agent")]
    Agent { agent: String },
}
