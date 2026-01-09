//! Inventory types for scan results

use crate::artifacts::{AgentInfo, CommandInfo, HookInfo, SkillInfo};
use crate::collision::CollisionReport;
use crate::plugins::PluginInventory;
use crate::settings::{McpConfig, SettingsFile};
use crate::types::HostInfo;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete inventory from a scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Host system information
    pub host: HostInfo,
    /// User-level scope inventory
    pub user_scope: UserScope,
    /// Managed scope inventory (if present)
    pub managed_scope: Option<ManagedScope>,
    /// Project scope inventories
    pub projects: Vec<ProjectScope>,
    /// Plugin inventory
    pub plugins: PluginInventory,
    /// Collision report
    pub collisions: CollisionReport,
    /// When the scan was performed
    pub scanned_at: DateTime<Utc>,
}

/// User-level scope inventory
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserScope {
    /// User settings file
    pub settings: Option<SettingsFile>,
    /// User MCP configuration
    pub mcp: Option<McpConfig>,
    /// User-level skills
    pub skills: Vec<SkillInfo>,
    /// User-level commands
    pub commands: Vec<CommandInfo>,
    /// User-level agents
    pub agents: Vec<AgentInfo>,
}

/// Managed (IT-deployed) scope inventory
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManagedScope {
    /// Managed settings file
    pub settings: Option<SettingsFile>,
    /// Managed MCP configuration
    pub mcp: Option<McpConfig>,
}

/// Project-level scope inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectScope {
    /// Project directory path
    pub path: PathBuf,
    /// Project name
    pub name: String,
    /// Git information
    pub git: Option<GitInfo>,
    /// CLAUDE.md file info
    pub claude_md: Option<crate::types::FileInfo>,
    /// .claude directory path
    pub claude_dir: Option<PathBuf>,
    /// Project settings
    pub settings: ProjectSettings,
    /// Project MCP configuration
    pub mcp: Option<McpConfig>,
    /// Project skills
    pub skills: Vec<SkillInfo>,
    /// Project commands
    pub commands: Vec<CommandInfo>,
    /// Project agents
    pub agents: Vec<AgentInfo>,
    /// Project hooks
    pub hooks: Vec<HookInfo>,
}

/// Project settings (shared and local)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Shared settings (.claude/settings.json)
    pub shared: Option<SettingsFile>,
    /// Local settings (.claude/settings.local.json)
    pub local: Option<SettingsFile>,
}

/// Git repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    /// Remote URL
    pub remote: Option<String>,
    /// Current branch
    pub branch: String,
    /// Whether there are uncommitted changes
    pub is_dirty: bool,
}
