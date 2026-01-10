//! Apply/rollback Tauri commands
//!
//! Commands for applying profiles and rolling back changes.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tars_core::backup::restore::{restore_from_backup, verify_backup_integrity};
use tars_core::diff::display::{format_plan_terminal, DiffSummary};
use tars_core::diff::plan::generate_plan;
use tars_core::storage::{BackupStore, ProfileStore, ProjectStore};
use tars_core::{apply::apply_operations, Backup};
use tauri::State;

/// Diff preview for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPreview {
    pub operations: Vec<OperationPreview>,
    pub summary: String,
    pub warnings: Vec<String>,
    pub terminal_output: String,
}

/// Individual operation preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPreview {
    pub operation_type: String,
    pub path: String,
    pub diff: Option<String>,
    pub size: Option<usize>,
}

/// Backup summary for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub project_id: String,
    pub profile_id: Option<String>,
    pub description: Option<String>,
    pub files_count: usize,
    pub created_at: String,
}

impl From<tars_core::storage::backups::BackupSummary> for BackupInfo {
    fn from(b: tars_core::storage::backups::BackupSummary) -> Self {
        Self {
            id: b.id.to_string(),
            project_id: b.project_id.to_string(),
            profile_id: b.profile_id.map(|id| id.to_string()),
            description: b.description,
            files_count: 0, // Summary doesn't have this
            created_at: b.created_at.to_rfc3339(),
        }
    }
}

/// Preview what applying a profile would do
#[tauri::command]
pub async fn preview_apply(
    profile_id: String,
    project_path: String,
    state: State<'_, AppState>,
) -> Result<DiffPreview, String> {
    let uuid = uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid UUID: {e}"))?;
    let path = PathBuf::from(&project_path);

    if !path.exists() {
        return Err(format!("Path does not exist: {project_path}"));
    }

    state.with_db(|db| {
        let profiles = ProfileStore::new(db.connection());
        let projects = ProjectStore::new(db.connection());

        let profile = profiles
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        // Get or create project ID
        let project_id = match projects.get_by_path(&path) {
            Ok(Some(p)) => p.id,
            _ => uuid::Uuid::new_v4(), // Temporary ID for preview
        };

        let plan = generate_plan(project_id, &path, &profile)
            .map_err(|e| format!("Failed to generate plan: {e}"))?;

        let operations: Vec<OperationPreview> = plan
            .operations
            .iter()
            .map(|op| match op {
                tars_core::diff::FileOperation::Create { path, content } => OperationPreview {
                    operation_type: "create".to_string(),
                    path: path.display().to_string(),
                    diff: None,
                    size: Some(content.len()),
                },
                tars_core::diff::FileOperation::Modify { path, diff, .. } => OperationPreview {
                    operation_type: "modify".to_string(),
                    path: path.display().to_string(),
                    diff: Some(diff.clone()),
                    size: None,
                },
                tars_core::diff::FileOperation::Delete { path } => OperationPreview {
                    operation_type: "delete".to_string(),
                    path: path.display().to_string(),
                    diff: None,
                    size: None,
                },
            })
            .collect();

        let summary = DiffSummary::from_plan(&plan);
        let warnings: Vec<String> = plan.warnings.iter().map(|w| w.message.clone()).collect();
        let terminal_output = format_plan_terminal(&plan);

        Ok(DiffPreview {
            operations,
            summary: summary.one_line(),
            warnings,
            terminal_output,
        })
    })
}

/// Apply a profile to a project
#[tauri::command]
pub async fn apply_profile(
    profile_id: String,
    project_path: String,
    state: State<'_, AppState>,
) -> Result<BackupInfo, String> {
    let uuid = uuid::Uuid::parse_str(&profile_id).map_err(|e| format!("Invalid UUID: {e}"))?;
    let path = PathBuf::from(&project_path);

    if !path.exists() {
        return Err(format!("Path does not exist: {project_path}"));
    }

    let data_dir = state.data_dir().clone();

    state.with_db(|db| {
        let profiles = ProfileStore::new(db.connection());
        let projects = ProjectStore::new(db.connection());
        let backups = BackupStore::new(db.connection());

        let profile = profiles
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Profile not found".to_string())?;

        // Get or create project
        let project = if let Ok(Some(p)) = projects.get_by_path(&path) {
            p
        } else {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let p = tars_core::Project::new(path.clone()).with_name(name);
            projects
                .create(&p)
                .map_err(|e| format!("Failed to create project: {e}"))?;
            p
        };

        let plan = generate_plan(project.id, &path, &profile)
            .map_err(|e| format!("Failed to generate plan: {e}"))?;

        if plan.is_empty() {
            return Err("No changes needed - project already matches profile.".to_string());
        }

        // Create backup directory
        let backup_dir = data_dir.join("backups");
        std::fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {e}"))?;

        let archive_path = backup_dir.join(format!(
            "backup-{}.json",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        let mut backup = Backup::new(project.id, archive_path.clone())
            .with_profile(profile.id)
            .with_description(format!("Before applying profile '{}'", profile.name));

        apply_operations(&plan, &path, &mut backup)
            .map_err(|e| format!("Failed to apply changes: {e}"))?;

        // Save backup
        let backup_json = serde_json::to_string_pretty(&backup)
            .map_err(|e| format!("Failed to serialize backup: {e}"))?;
        std::fs::write(&archive_path, backup_json)
            .map_err(|e| format!("Failed to write backup: {e}"))?;

        backups
            .create(&backup)
            .map_err(|e| format!("Failed to save backup record: {e}"))?;

        Ok(BackupInfo {
            id: backup.id.to_string(),
            project_id: backup.project_id.to_string(),
            profile_id: backup.profile_id.map(|id| id.to_string()),
            description: backup.description,
            files_count: backup.files.len(),
            created_at: backup.created_at.to_rfc3339(),
        })
    })
}

/// List backups for a project
#[tauri::command]
pub async fn list_backups(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<BackupInfo>, String> {
    let uuid = uuid::Uuid::parse_str(&project_id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = BackupStore::new(db.connection());
        let backups = store
            .list_for_project(uuid)
            .map_err(|e| format!("Failed to list backups: {e}"))?;
        Ok(backups.into_iter().map(BackupInfo::from).collect())
    })
}

/// Rollback to a backup
#[tauri::command]
pub async fn rollback(
    backup_id: String,
    project_path: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let uuid = uuid::Uuid::parse_str(&backup_id).map_err(|e| format!("Invalid UUID: {e}"))?;
    let path = PathBuf::from(&project_path);

    state.with_db(|db| {
        let store = BackupStore::new(db.connection());
        let backup = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Backup not found".to_string())?;

        // Verify backup integrity
        verify_backup_integrity(&backup)
            .map_err(|e| format!("Backup integrity check failed: {e}"))?;

        let files_count = backup.files.len();

        // Restore
        restore_from_backup(&path, &backup).map_err(|e| format!("Rollback failed: {e}"))?;

        Ok(files_count)
    })
}
