//! Profile types and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tars_scanner::types::Scope;
use uuid::Uuid;

/// A profile configuration bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier
    pub id: Uuid,
    /// Profile name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
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
