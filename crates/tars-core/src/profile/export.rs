//! Profile export/import operations
//!
//! This module handles exporting profiles to JSON and importing them back.

use crate::profile::{Profile, ToolRef};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Version of the export format
const EXPORT_FORMAT_VERSION: u32 = 1;

/// Exported profile format (.tars-profile.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExport {
    /// Format version for future compatibility
    pub version: u32,
    /// Profile name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Tool references
    pub tool_refs: Vec<ExportedTool>,
    /// When the profile was originally created
    pub created_at: DateTime<Utc>,
    /// When exported
    pub exported_at: DateTime<Utc>,
}

/// Exported tool reference (simplified for portability)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedTool {
    /// Tool name
    pub name: String,
    /// Tool type (mcp, skill, agent, hook)
    pub tool_type: String,
    /// Optional permission restrictions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<ExportedPermissions>,
}

/// Exported permissions (directory paths excluded for portability)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedPermissions {
    /// Allowed tools
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    /// Disallowed tools
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disallowed_tools: Vec<String>,
}

/// Error type for export/import operations
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Invalid format version
    #[error("Unsupported export format version: {0}")]
    UnsupportedVersion(u32),
}

/// Export a profile to JSON format
///
/// # Errors
/// Returns an error if the file cannot be written
pub fn export_profile(profile: &Profile, output_path: &Path) -> Result<ProfileExport, ExportError> {
    let export = ProfileExport {
        version: EXPORT_FORMAT_VERSION,
        name: profile.name.clone(),
        description: profile.description.clone(),
        tool_refs: profile
            .tool_refs
            .iter()
            .map(|r| ExportedTool {
                name: r.name.clone(),
                tool_type: r.tool_type.to_string(),
                permissions: r.permissions.as_ref().map(|p| ExportedPermissions {
                    allowed_tools: p.allowed_tools.clone(),
                    disallowed_tools: p.disallowed_tools.clone(),
                }),
            })
            .collect(),
        created_at: profile.created_at,
        exported_at: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&export)?;
    fs::write(output_path, json)?;

    Ok(export)
}

/// Preview what would be imported from a file
///
/// # Errors
/// Returns an error if the file cannot be read or parsed
pub fn preview_import(path: &Path) -> Result<ImportPreview, ExportError> {
    let content = fs::read_to_string(path)?;
    let export: ProfileExport = serde_json::from_str(&content)?;

    if export.version > EXPORT_FORMAT_VERSION {
        return Err(ExportError::UnsupportedVersion(export.version));
    }

    Ok(ImportPreview {
        name: export.name,
        description: export.description,
        tool_count: export.tool_refs.len(),
        version: export.version,
        created_at: export.created_at,
        exported_at: export.exported_at,
    })
}

/// Preview of what would be imported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    /// Profile name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Number of tools in the profile
    pub tool_count: usize,
    /// Format version
    pub version: u32,
    /// When originally created
    pub created_at: DateTime<Utc>,
    /// When exported
    pub exported_at: DateTime<Utc>,
}

/// Import a profile from JSON file
///
/// Creates a new profile with a new ID.
///
/// # Errors
/// Returns an error if the file cannot be read or parsed
pub fn import_profile(path: &Path) -> Result<Profile, ExportError> {
    use crate::profile::{ToolPermissions, ToolType};

    let content = fs::read_to_string(path)?;
    let export: ProfileExport = serde_json::from_str(&content)?;

    if export.version > EXPORT_FORMAT_VERSION {
        return Err(ExportError::UnsupportedVersion(export.version));
    }

    // Convert exported tools back to ToolRef
    let tool_refs: Vec<ToolRef> = export
        .tool_refs
        .into_iter()
        .filter_map(|t| {
            let tool_type = match t.tool_type.to_lowercase().as_str() {
                "mcp" => ToolType::Mcp,
                "skill" => ToolType::Skill,
                "agent" => ToolType::Agent,
                "hook" => ToolType::Hook,
                _ => return None, // Skip unknown tool types
            };

            let permissions = t.permissions.map(|p| ToolPermissions {
                allowed_directories: Vec::new(), // Directory paths not exported
                allowed_tools: p.allowed_tools,
                disallowed_tools: p.disallowed_tools,
            });

            Some(ToolRef {
                name: t.name,
                tool_type,
                source_scope: None,
                permissions,
            })
        })
        .collect();

    let mut profile = Profile::new(export.name);
    profile.description = export.description;
    profile.tool_refs = tool_refs;
    // Note: Keep the new created_at for the imported profile, not the original

    Ok(profile)
}
