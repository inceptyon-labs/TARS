//! Plugin manifest generation

use serde::Serialize;
use tars_scanner::plugins::Author;

/// Plugin manifest for export
#[derive(Debug, Serialize)]
pub struct ExportManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
}

impl ExportManifest {
    /// Create a new manifest with the given name and version
    #[must_use]
    pub fn new(name: String, version: String, description: String) -> Self {
        Self {
            name,
            version,
            description,
            author: None,
        }
    }
}
