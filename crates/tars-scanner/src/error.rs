//! Error types for the TARS scanner

use thiserror::Error;

/// Result type for scanner operations
pub type ScanResult<T> = Result<T, ScanError>;

/// Errors that can occur during scanning
#[derive(Error, Debug)]
pub enum ScanError {
    /// IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse frontmatter
    #[error("Failed to parse frontmatter: {0}")]
    FrontmatterParse(String),

    /// Failed to parse JSON
    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// No frontmatter found in file
    #[error("No frontmatter found in file")]
    NoFrontmatter,

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// CLI command failed
    #[error("CLI command failed: {0}")]
    CliError(String),

    /// Home directory not found
    #[error("Home directory not found")]
    HomeNotFound,
}
