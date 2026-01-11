//! Project management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::profile::ToolRef;

/// Project-specific tool additions that persist through profile sync
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalOverrides {
    /// Local MCP server references
    #[serde(default)]
    pub mcp_servers: Vec<ToolRef>,
    /// Local skill references
    #[serde(default)]
    pub skills: Vec<ToolRef>,
    /// Local agent references
    #[serde(default)]
    pub agents: Vec<ToolRef>,
    /// Local hook references
    #[serde(default)]
    pub hooks: Vec<ToolRef>,
}

impl LocalOverrides {
    /// Returns true if there are no local overrides
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mcp_servers.is_empty()
            && self.skills.is_empty()
            && self.agents.is_empty()
            && self.hooks.is_empty()
    }

    /// Returns the total count of all local overrides
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.mcp_servers.len() + self.skills.len() + self.agents.len() + self.hooks.len()
    }
}

/// A registered project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier
    pub id: Uuid,
    /// Project path
    pub path: PathBuf,
    /// Project name
    pub name: String,
    /// Git information
    pub git_info: Option<GitInfo>,
    /// When last scanned
    pub last_scanned: Option<DateTime<Utc>>,
    /// Assigned profile ID
    pub assigned_profile_id: Option<Uuid>,
    /// Project-specific tool additions
    #[serde(default)]
    pub local_overrides: LocalOverrides,
    /// When registered
    pub created_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Create a new project from a path
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            path,
            name,
            git_info: None,
            last_scanned: None,
            assigned_profile_id: None,
            local_overrides: LocalOverrides::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the project name
    #[must_use]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    /// Set git info
    #[must_use]
    pub fn with_git_info(mut self, git_info: GitInfo) -> Self {
        self.git_info = Some(git_info);
        self
    }
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

impl GitInfo {
    /// Check if the repository has uncommitted changes
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
}
