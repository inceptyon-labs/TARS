//! Profile update detection
//!
//! This module handles detecting when source files have changed for tools
//! that are tracked (as opposed to pinned) in a profile.

use crate::profile::storage::{compute_dir_hash, compute_file_hash, StorageError};
use crate::profile::types::{Profile, SourceMode, SourceRef, ToolType};
use chrono::Utc;
use std::path::PathBuf;

/// Information about an available update for a tool
#[derive(Debug, Clone)]
pub struct ToolUpdateInfo {
    /// Tool name
    pub name: String,
    /// Tool type
    pub tool_type: ToolType,
    /// Path to the source file/directory
    pub source_path: PathBuf,
    /// Hash at the time the tool was copied
    pub old_hash: String,
    /// Current hash of the source
    pub new_hash: String,
    /// Source tracking mode
    pub mode: SourceMode,
}

/// Result of checking for profile updates
#[derive(Debug, Clone, Default)]
pub struct ProfileUpdateCheck {
    /// Tools with available updates
    pub updates: Vec<ToolUpdateInfo>,
    /// Tools where source no longer exists
    pub missing_sources: Vec<String>,
    /// Total tools checked
    pub total_checked: usize,
}

impl ProfileUpdateCheck {
    /// Check if there are any updates available
    pub fn has_updates(&self) -> bool {
        !self.updates.is_empty()
    }

    /// Get count of available updates
    pub fn update_count(&self) -> usize {
        self.updates.len()
    }
}

/// Check a profile for available updates to tracked tools
///
/// This examines all tools with `source_ref` set to Track mode and compares
/// the current source file hash against the stored hash from copy time.
///
/// # Errors
/// Returns an error if hash computation fails
pub fn check_profile_updates(profile: &Profile) -> Result<ProfileUpdateCheck, StorageError> {
    let mut result = ProfileUpdateCheck::default();

    for tool in &profile.tool_refs {
        // Skip tools without source tracking
        let Some(source_ref) = &tool.source_ref else {
            continue;
        };

        result.total_checked += 1;

        // Skip pinned tools
        if matches!(source_ref.mode, SourceMode::Pin) {
            continue;
        }

        // Check if source still exists
        if !source_ref.source_path.exists() {
            result.missing_sources.push(tool.name.clone());
            continue;
        }

        // Compute current hash based on tool type
        let current_hash = match tool.tool_type {
            ToolType::Skill => {
                // Skills are directories
                compute_dir_hash(&source_ref.source_path)?
            }
            _ => {
                // Other tools are single files
                compute_file_hash(&source_ref.source_path)?
            }
        };

        // Check if hash has changed
        if current_hash != source_ref.source_hash {
            result.updates.push(ToolUpdateInfo {
                name: tool.name.clone(),
                tool_type: tool.tool_type,
                source_path: source_ref.source_path.clone(),
                old_hash: source_ref.source_hash.clone(),
                new_hash: current_hash,
                mode: source_ref.mode,
            });
        }
    }

    Ok(result)
}

/// Create a `SourceRef` for a new tool being added to a profile
///
/// # Errors
/// Returns an error if hash computation fails
pub fn create_source_ref(
    source_path: PathBuf,
    tool_type: ToolType,
    mode: SourceMode,
) -> Result<SourceRef, StorageError> {
    let source_hash = match tool_type {
        ToolType::Skill => compute_dir_hash(&source_path)?,
        _ => compute_file_hash(&source_path)?,
    };

    Ok(SourceRef {
        source_path,
        source_hash,
        mode,
        copied_at: Utc::now().to_rfc3339(),
    })
}

/// Update the hash in a source ref after pulling changes
pub fn update_source_hash(source_ref: &mut SourceRef, new_hash: String) {
    source_ref.source_hash = new_hash;
    source_ref.copied_at = Utc::now().to_rfc3339();
}

/// Change the tracking mode for a source ref
pub fn set_source_mode(source_ref: &mut SourceRef, mode: SourceMode) {
    source_ref.mode = mode;
}

// ============================================================================
// Migration
// ============================================================================

/// Migrate a legacy profile that doesn't have `source_ref` set on tools
///
/// This adds a "legacy" `source_ref` to each tool, marking them as pinned
/// since we don't know where they originally came from.
pub fn migrate_legacy_profile(profile: &mut Profile) {
    for tool in &mut profile.tool_refs {
        if tool.source_ref.is_none() {
            tool.source_ref = Some(SourceRef {
                source_path: PathBuf::new(),
                source_hash: "legacy".to_string(),
                mode: SourceMode::Pin,
                copied_at: Utc::now().to_rfc3339(),
            });
        }
    }
}

/// Check if a profile needs migration
pub fn needs_migration(profile: &Profile) -> bool {
    profile
        .tool_refs
        .iter()
        .any(|tool| tool.source_ref.is_none())
}
