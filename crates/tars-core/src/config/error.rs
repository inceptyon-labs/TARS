//! Error types for config operations

use std::path::PathBuf;
use thiserror::Error;

/// Result type for config operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Errors that can occur during config operations
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Item already exists in the specified scope
    #[error("Item '{name}' already exists in {scope} scope")]
    ItemExists { name: String, scope: String },

    /// Item not found in any scope
    #[error("Item '{name}' not found")]
    ItemNotFound { name: String },

    /// Item exists in multiple scopes (ambiguous)
    #[error(
        "Item '{name}' exists in multiple scopes: {scopes:?}. Specify --scope to disambiguate"
    )]
    AmbiguousItem { name: String, scopes: Vec<String> },

    /// Invalid scope specified
    #[error("Invalid scope: {0}")]
    InvalidScope(String),

    /// Cannot modify managed scope
    #[error("Cannot modify managed scope (read-only)")]
    ManagedScope,

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Conflict during move operation
    #[error("Item '{name}' already exists in target scope {target_scope}")]
    MoveConflict { name: String, target_scope: String },

    /// File I/O error
    #[error("I/O error for {path}: {message}")]
    IoError { path: PathBuf, message: String },

    /// JSON parse error
    #[error("JSON parse error in {path}: {message}")]
    JsonParseError { path: PathBuf, message: String },

    /// YAML/frontmatter parse error
    #[error("Frontmatter parse error in {path}: {message}")]
    FrontmatterError { path: PathBuf, message: String },

    /// Backup creation failed
    #[error("Failed to create backup: {0}")]
    BackupFailed(String),

    /// Rollback failed
    #[error("Failed to rollback: {0}")]
    RollbackFailed(String),

    /// Scanner error
    #[error("Scanner error: {0}")]
    ScannerError(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ConfigError {
    /// Get the error code for CLI/API responses
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::ItemExists { .. } => "ITEM_EXISTS",
            Self::ItemNotFound { .. } => "ITEM_NOT_FOUND",
            Self::AmbiguousItem { .. } => "AMBIGUOUS_ITEM",
            Self::InvalidScope(_) => "INVALID_SCOPE",
            Self::ManagedScope => "PERMISSION_DENIED",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::MoveConflict { .. } => "CONFLICT",
            Self::IoError { .. } => "IO_ERROR",
            Self::JsonParseError { .. } => "PARSE_ERROR",
            Self::FrontmatterError { .. } => "PARSE_ERROR",
            Self::BackupFailed(_) => "BACKUP_FAILED",
            Self::RollbackFailed(_) => "ROLLBACK_FAILED",
            Self::ScannerError(_) => "SCANNER_ERROR",
            Self::MissingField(_) => "VALIDATION_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            path: PathBuf::new(),
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonParseError {
            path: PathBuf::new(),
            message: err.to_string(),
        }
    }
}
