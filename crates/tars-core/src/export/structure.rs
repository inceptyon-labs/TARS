//! Plugin directory structure creation

use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors during structure creation
#[derive(Error, Debug)]
pub enum StructureError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Create the plugin directory structure
///
/// # Errors
/// Returns an error if directory creation fails
pub fn create_plugin_structure(output_dir: &Path) -> Result<(), StructureError> {
    // Create main directories
    fs::create_dir_all(output_dir.join(".claude-plugin"))?;
    fs::create_dir_all(output_dir.join("commands"))?;
    fs::create_dir_all(output_dir.join("skills"))?;
    fs::create_dir_all(output_dir.join("agents"))?;

    Ok(())
}
