//! Profile types and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tars_scanner::types::Scope;
use uuid::Uuid;

/// Source tracking mode for profile tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceMode {
    /// Frozen at copied version - ignore source changes
    Pin,
    /// Follow source changes - detect updates
    #[default]
    Track,
}

/// Reference to the original source of a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    /// Path to the original source file/directory
    pub source_path: PathBuf,
    /// SHA256 hash of content at copy time
    pub source_hash: String,
    /// Whether to track updates from source
    pub mode: SourceMode,
    /// When the tool was copied to the profile
    pub copied_at: String,
}

/// Type of tool that can be referenced in a profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    /// MCP server
    Mcp,
    /// Skill
    Skill,
    /// Agent
    Agent,
    /// Hook
    Hook,
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolType::Mcp => write!(f, "mcp"),
            ToolType::Skill => write!(f, "skill"),
            ToolType::Agent => write!(f, "agent"),
            ToolType::Hook => write!(f, "hook"),
        }
    }
}

/// Permission restrictions for a tool in a profile
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolPermissions {
    /// Directories the tool can access (relative paths resolved against project root)
    #[serde(default)]
    pub allowed_directories: Vec<PathBuf>,
    /// Tools this agent/skill can use
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Tools this agent/skill cannot use
    #[serde(default)]
    pub disallowed_tools: Vec<String>,
}

/// A reference to a tool (MCP server, skill, agent, or hook) with optional permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRef {
    /// Tool identifier/name
    pub name: String,
    /// Type of tool
    pub tool_type: ToolType,
    /// Where the tool was discovered
    #[serde(default)]
    pub source_scope: Option<Scope>,
    /// Optional permission restrictions
    #[serde(default)]
    pub permissions: Option<ToolPermissions>,
    /// Source tracking reference (for detecting updates)
    #[serde(default)]
    pub source_ref: Option<SourceRef>,
}

/// A profile configuration bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier
    pub id: Uuid,
    /// Profile name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Tool references for this profile
    #[serde(default)]
    pub tool_refs: Vec<ToolRef>,
    /// Plugin configuration
    pub plugin_set: PluginSet,
    /// Repository-level overlays
    pub repo_overlays: RepoOverlays,
    /// User-level overlays
    pub user_overlays: UserOverlays,
    /// Adapter settings
    pub adapters: Adapters,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
}

impl Profile {
    /// Create a new profile with the given name
    #[must_use]
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            tool_refs: Vec::new(),
            plugin_set: PluginSet::default(),
            repo_overlays: RepoOverlays::default(),
            user_overlays: UserOverlays::default(),
            adapters: Adapters::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Plugin set configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginSet {
    /// Marketplaces to add
    #[serde(default)]
    pub marketplaces: Vec<MarketplaceRef>,
    /// Plugins to install
    #[serde(default)]
    pub plugins: Vec<PluginRef>,
}

/// Reference to a marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceRef {
    /// Marketplace name
    pub name: String,
    /// Source type and location
    pub source: MarketplaceSourceRef,
}

/// Marketplace source reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MarketplaceSourceRef {
    GitHub { owner: String, repo: String },
    Url { url: String },
    Local { path: PathBuf },
}

/// Reference to a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRef {
    /// Plugin identifier
    pub id: String,
    /// Marketplace it comes from
    pub marketplace: Option<String>,
    /// Installation scope
    pub scope: Scope,
    /// Whether to enable
    pub enabled: bool,
}

/// Repository-level overlays
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoOverlays {
    /// MCP servers to add
    #[serde(default)]
    pub mcp_servers: Vec<McpServerOverlay>,
    /// Skills to add
    #[serde(default)]
    pub skills: Vec<SkillOverlay>,
    /// Commands to add
    #[serde(default)]
    pub commands: Vec<CommandOverlay>,
    /// Agents to add
    #[serde(default)]
    pub agents: Vec<AgentOverlay>,
    /// CLAUDE.md overlay
    pub claude_md: Option<ClaudeMdOverlay>,
}

/// User-level overlays
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserOverlays {
    /// Skills to add
    #[serde(default)]
    pub skills: Vec<SkillOverlay>,
    /// Commands to add
    #[serde(default)]
    pub commands: Vec<CommandOverlay>,
}

/// MCP server overlay content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerOverlay {
    /// Server name
    pub name: String,
    /// Transport type (stdio, http, sse)
    pub transport: String,
    /// Command for stdio transport
    pub command: Option<String>,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// URL for http/sse transport
    pub url: Option<String>,
}

/// Skill overlay content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOverlay {
    /// Skill name
    pub name: String,
    /// Full SKILL.md content
    pub content: String,
}

/// Command overlay content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOverlay {
    /// Command name
    pub name: String,
    /// Full command file content
    pub content: String,
}

/// Agent overlay content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOverlay {
    /// Agent name
    pub name: String,
    /// Full agent file content
    pub content: String,
}

/// CLAUDE.md overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMdOverlay {
    /// How to apply the overlay
    pub mode: OverlayMode,
    /// Content to apply
    pub content: String,
}

/// Overlay application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayMode {
    /// Replace existing content
    Replace,
    /// Prepend to existing content
    Prepend,
    /// Append to existing content
    Append,
}

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adapters {
    /// Where to write MCP config
    pub mcp_location: McpLocation,
    /// Merge strategies per artifact type
    #[serde(default)]
    pub merge_strategies: HashMap<String, MergeStrategy>,
}

impl Default for Adapters {
    fn default() -> Self {
        Self {
            mcp_location: McpLocation::ProjectRoot,
            merge_strategies: HashMap::new(),
        }
    }
}

/// MCP config file location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpLocation {
    /// At project root (.mcp.json)
    ProjectRoot,
    /// In .claude directory (.claude/mcp.json)
    ClaudeDir,
}

/// Merge strategy for artifacts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Replace existing
    Replace,
    /// Merge with existing
    Merge,
    /// Skip if exists
    Skip,
}
