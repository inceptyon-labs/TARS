//! Profile sync operations
//!
//! This module handles syncing profile changes to all assigned projects.

use crate::project::Project;
use crate::storage::db::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use uuid::Uuid;

/// Result of syncing a profile to its assigned projects
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of projects affected by the sync
    pub affected_projects: usize,
    /// When the sync occurred
    pub synced_at: DateTime<Utc>,
}

impl SyncResult {
    /// Create a new sync result
    #[must_use]
    pub fn new(affected_projects: usize) -> Self {
        Self {
            affected_projects,
            synced_at: Utc::now(),
        }
    }
}

/// Sync a profile to all its assigned projects
///
/// This function finds all projects with the given profile assigned
/// and ensures their effective configuration is up-to-date.
///
/// # Errors
/// Returns an error if database operations fail
pub fn sync_profile_to_projects(
    conn: &Connection,
    profile_id: Uuid,
) -> Result<SyncResult, DatabaseError> {
    use crate::storage::projects::ProjectStore;

    let store = ProjectStore::new(conn);

    // Get all projects with this profile assigned
    let projects = store.list_by_profile(profile_id)?;
    let affected = projects.len();

    // For now, sync is implicit - projects always resolve their
    // effective configuration by looking up the profile.
    // This function exists to:
    // 1. Count affected projects for notification
    // 2. Future: trigger any cache invalidation needed

    Ok(SyncResult::new(affected))
}

/// Convert profile tools to local overrides when a profile is deleted
///
/// # Errors
/// Returns an error if database operations fail
pub fn convert_profile_to_local_overrides(
    conn: &Connection,
    profile_id: Uuid,
) -> Result<Vec<Project>, DatabaseError> {
    use crate::storage::profiles::ProfileStore;
    use crate::storage::projects::ProjectStore;

    let profile_store = ProfileStore::new(conn);
    let project_store = ProjectStore::new(conn);

    // Get the profile's tools before deletion
    let profile = profile_store
        .get(profile_id)?
        .ok_or_else(|| DatabaseError::Migration(format!("Profile not found: {profile_id}")))?;

    // Get all projects with this profile
    let projects = project_store.list_by_profile(profile_id)?;
    let mut updated_projects = Vec::new();

    for mut project in projects {
        // Move profile tools to local overrides
        for tool_ref in &profile.tool_refs {
            match tool_ref.tool_type {
                crate::profile::ToolType::Mcp => {
                    project.local_overrides.mcp_servers.push(tool_ref.clone());
                }
                crate::profile::ToolType::Skill => {
                    project.local_overrides.skills.push(tool_ref.clone());
                }
                crate::profile::ToolType::Agent => {
                    project.local_overrides.agents.push(tool_ref.clone());
                }
                crate::profile::ToolType::Hook => {
                    project.local_overrides.hooks.push(tool_ref.clone());
                }
            }
        }

        // Clear the profile assignment
        project.assigned_profile_id = None;
        project.updated_at = Utc::now();

        // Save the updated project
        project_store.update(&project)?;
        updated_projects.push(project);
    }

    Ok(updated_projects)
}
