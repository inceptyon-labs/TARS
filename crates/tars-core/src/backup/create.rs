//! Backup creation for rollback support

use crate::backup::{Backup, BackupFile};
use crate::diff::{DiffPlan, FileOperation};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Errors during backup creation
#[derive(Error, Debug)]
pub enum BackupCreateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to create backup directory: {0}")]
    DirectoryCreation(String),
}

/// Create a backup before applying a diff plan
///
/// # Errors
/// Returns an error if backup creation fails
pub fn create_backup(
    project_id: Uuid,
    project_path: &Path,
    plan: &DiffPlan,
    backup_dir: &Path,
) -> Result<Backup, BackupCreateError> {
    // Create backup directory if needed
    fs::create_dir_all(backup_dir)?;

    // Generate archive path
    let archive_name = format!("backup-{}.tar.gz", plan.profile_id);
    let archive_path = backup_dir.join(&archive_name);

    let mut backup = Backup::new(project_id, archive_path.clone())
        .with_profile(plan.profile_id)
        .with_description(format!(
            "Backup before applying profile {}",
            plan.profile_id
        ));

    // Collect files to backup
    for op in &plan.operations {
        match op {
            FileOperation::Create { path, .. } => {
                // File doesn't exist yet, record it as a new file
                let relative = path
                    .strip_prefix(project_path)
                    .unwrap_or(path)
                    .to_path_buf();
                backup.add_file(BackupFile::new_file(relative));
            }
            FileOperation::Modify { path, .. } => {
                // File exists, backup its current content
                if path.exists() {
                    let content = fs::read(path)?;
                    let hash = hash_content(&content);
                    let relative = path
                        .strip_prefix(project_path)
                        .unwrap_or(path)
                        .to_path_buf();
                    backup.add_file(BackupFile::existing(relative, content, hash));
                }
            }
            FileOperation::Delete { path } => {
                // File will be deleted, backup its content
                if path.exists() {
                    let content = fs::read(path)?;
                    let hash = hash_content(&content);
                    let relative = path
                        .strip_prefix(project_path)
                        .unwrap_or(path)
                        .to_path_buf();
                    backup.add_file(BackupFile::existing(relative, content, hash));
                }
            }
        }
    }

    // Write backup data to archive (simplified - just JSON for now)
    let backup_json = serde_json::to_string_pretty(&backup)
        .map_err(|e| BackupCreateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    fs::write(&archive_path, backup_json)?;

    Ok(backup)
}

/// Create a full backup of Claude configuration in a project
///
/// # Errors
/// Returns an error if backup creation fails
pub fn create_full_backup(
    project_id: Uuid,
    project_path: &Path,
    backup_dir: &Path,
) -> Result<Backup, BackupCreateError> {
    fs::create_dir_all(backup_dir)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let archive_name = format!("full-backup-{timestamp}.json");
    let archive_path = backup_dir.join(&archive_name);

    let mut backup =
        Backup::new(project_id, archive_path.clone()).with_description("Full backup".to_string());

    // Backup CLAUDE.md
    let claude_md = project_path.join("CLAUDE.md");
    if claude_md.exists() {
        let content = fs::read(&claude_md)?;
        let hash = hash_content(&content);
        backup.add_file(BackupFile::existing(
            PathBuf::from("CLAUDE.md"),
            content,
            hash,
        ));
    }

    // Backup .claude directory contents
    let claude_dir = project_path.join(".claude");
    if claude_dir.exists() {
        backup_directory(&claude_dir, &PathBuf::from(".claude"), &mut backup)?;
    }

    // Write backup data
    let backup_json = serde_json::to_string_pretty(&backup)
        .map_err(|e| BackupCreateError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    fs::write(&archive_path, backup_json)?;

    Ok(backup)
}

fn backup_directory(
    dir: &Path,
    relative_base: &Path,
    backup: &mut Backup,
) -> Result<(), BackupCreateError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative = relative_base.join(entry.file_name());

        // Skip symlinks to prevent potential security issues
        if path.is_symlink() {
            continue;
        }

        if path.is_file() {
            let content = fs::read(&path)?;
            let hash = hash_content(&content);
            backup.add_file(BackupFile::existing(relative, content, hash));
        } else if path.is_dir() {
            backup_directory(&path, &relative, backup)?;
        }
    }

    Ok(())
}

fn hash_content(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}
