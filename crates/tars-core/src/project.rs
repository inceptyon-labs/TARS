//! Project management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

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
