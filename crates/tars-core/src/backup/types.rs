//! Backup and rollback functionality

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A backup bundle for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    /// Unique identifier
    pub id: Uuid,
    /// Project this backup is for
    pub project_id: Uuid,
    /// Profile that was applied (if any)
    pub profile_id: Option<Uuid>,
    /// Description of what triggered this backup
    pub description: Option<String>,
    /// Path to the backup archive file
    pub archive_path: PathBuf,
    /// Backed up files
    pub files: Vec<BackupFile>,
    /// When created
    pub created_at: DateTime<Utc>,
}

impl Backup {
    /// Create a new backup
    #[must_use]
    pub fn new(project_id: Uuid, archive_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            profile_id: None,
            description: None,
            archive_path,
            files: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Set the profile ID that triggered this backup
    #[must_use]
    pub fn with_profile(mut self, profile_id: Uuid) -> Self {
        self.profile_id = Some(profile_id);
        self
    }

    /// Set a description for this backup
    #[must_use]
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a file to the backup
    pub fn add_file(&mut self, file: BackupFile) {
        self.files.push(file);
    }
}

/// A file in a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFile {
    /// Path to the file (relative to project root)
    pub path: PathBuf,
    /// Original content (None if file didn't exist)
    pub original_content: Option<Vec<u8>>,
    /// SHA256 hash of original content
    pub sha256: Option<String>,
}

impl BackupFile {
    /// Create a backup entry for an existing file
    #[must_use]
    pub fn existing(path: PathBuf, content: Vec<u8>, sha256: String) -> Self {
        Self {
            path,
            original_content: Some(content),
            sha256: Some(sha256),
        }
    }

    /// Create a backup entry for a file that didn't exist
    #[must_use]
    pub fn new_file(path: PathBuf) -> Self {
        Self {
            path,
            original_content: None,
            sha256: None,
        }
    }

    /// Check if this was a new file (didn't exist before)
    #[must_use]
    pub fn was_new(&self) -> bool {
        self.original_content.is_none()
    }
}
