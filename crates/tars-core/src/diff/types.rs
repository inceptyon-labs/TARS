//! Diff generation for profile application

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A plan of file operations for profile application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPlan {
    /// Project being modified
    pub project_id: Uuid,
    /// Profile being applied
    pub profile_id: Uuid,
    /// Operations to perform
    pub operations: Vec<FileOperation>,
    /// Warnings generated
    pub warnings: Vec<Warning>,
}

impl DiffPlan {
    /// Create a new empty diff plan
    #[must_use]
    pub fn new(project_id: Uuid, profile_id: Uuid) -> Self {
        Self {
            project_id,
            profile_id,
            operations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Check if there are any operations
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Check if there are any errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| matches!(w.severity, WarningSeverity::Error))
    }
}

/// A file operation in the diff plan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileOperation {
    /// Create a new file
    Create {
        path: PathBuf,
        content: Vec<u8>,
    },
    /// Modify an existing file
    Modify {
        path: PathBuf,
        diff: String,
        new_content: Vec<u8>,
    },
    /// Delete a file
    Delete {
        path: PathBuf,
    },
}

impl FileOperation {
    /// Get the path affected by this operation
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Create { path, .. }
            | Self::Modify { path, .. }
            | Self::Delete { path } => path,
        }
    }
}

/// A warning or error in the diff plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    /// Severity level
    pub severity: WarningSeverity,
    /// Warning message
    pub message: String,
}

/// Warning severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    /// Informational
    Info,
    /// Warning (proceed with caution)
    Warning,
    /// Error (should not proceed)
    Error,
}
