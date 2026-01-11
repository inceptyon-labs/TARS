//! Profile to plugin conversion

use crate::profile::Profile;
use std::path::Path;
use thiserror::Error;

use super::manifest::ExportManifest;
use super::structure::{create_plugin_structure, StructureError};

/// Errors during export
#[derive(Error, Debug)]
pub enum ExportError {
    #[error("Structure error: {0}")]
    Structure(#[from] StructureError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Export a profile as a Claude Code plugin
///
/// # Errors
/// Returns an error if export fails
pub fn export_as_plugin(
    profile: &Profile,
    output_dir: &Path,
    plugin_name: &str,
    version: &str,
) -> Result<(), ExportError> {
    // Create directory structure
    create_plugin_structure(output_dir)?;

    // Create manifest
    let manifest = ExportManifest::new(
        plugin_name.to_string(),
        version.to_string(),
        profile.description.clone().unwrap_or_default(),
    );

    // Write manifest
    let manifest_path = output_dir.join(".claude-plugin").join("plugin.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(manifest_path, manifest_json)?;

    // TODO: Write skills, commands, agents from profile overlays

    Ok(())
}
