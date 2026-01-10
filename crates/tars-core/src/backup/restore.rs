//! Rollback restore functionality

use crate::backup::Backup;
use crate::util::{safe_join, PathError};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors during restore
#[derive(Error, Debug)]
pub enum RestoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Backup not found: {0}")]
    BackupNotFound(String),

    #[error("Invalid backup format: {0}")]
    InvalidBackup(String),

    #[error("Hash mismatch for {0}: expected {1}, got {2}")]
    HashMismatch(String, String, String),

    #[error("Path security error: {0}")]
    PathSecurity(#[from] PathError),
}

/// Restore a project from a backup (byte-for-byte rollback)
///
/// # Errors
/// Returns an error if restore fails
pub fn restore_from_backup(project_path: &Path, backup: &Backup) -> Result<(), RestoreError> {
    for file in &backup.files {
        // Validate path doesn't escape project directory
        let target_path = safe_join(project_path, &file.path)?;

        if file.was_new() {
            // File didn't exist before, delete it if it exists now
            if target_path.exists() {
                fs::remove_file(&target_path)?;

                // Try to remove empty parent directories
                if let Some(parent) = target_path.parent() {
                    let _ = remove_empty_dirs(parent, project_path);
                }
            }
        } else {
            // File existed, restore original content
            if let Some(content) = &file.original_content {
                // Ensure parent directory exists
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target_path, content)?;
            }
        }
    }

    Ok(())
}

/// Verify a restore would succeed without actually performing it
///
/// # Errors
/// Returns an error if verification fails
pub fn verify_restore(project_path: &Path, backup: &Backup) -> Result<(), RestoreError> {
    for file in &backup.files {
        // Validate path doesn't escape project directory
        let target_path = safe_join(project_path, &file.path)?;

        if !file.was_new() {
            // For files that should be restored, verify we can write to them
            if let Some(parent) = target_path.parent() {
                if !parent.exists() {
                    // We'll need to create the directory
                    continue;
                }
                // Check parent is writable
                let metadata = fs::metadata(parent)?;
                if metadata.permissions().readonly() {
                    return Err(RestoreError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("Cannot write to directory: {}", parent.display()),
                    )));
                }
            }
        }
    }

    Ok(())
}

/// Load a backup from its archive file
///
/// # Errors
/// Returns an error if loading fails
pub fn load_backup(archive_path: &Path) -> Result<Backup, RestoreError> {
    if !archive_path.exists() {
        return Err(RestoreError::BackupNotFound(
            archive_path.display().to_string(),
        ));
    }

    let content = fs::read_to_string(archive_path)?;
    let backup: Backup = serde_json::from_str(&content)
        .map_err(|e| RestoreError::InvalidBackup(format!("Failed to parse backup: {e}")))?;

    Ok(backup)
}

/// Verify backup integrity using SHA256 hashes
///
/// # Errors
/// Returns an error if verification fails
pub fn verify_backup_integrity(backup: &Backup) -> Result<(), RestoreError> {
    use sha2::{Digest, Sha256};

    for file in &backup.files {
        if let (Some(content), Some(expected_hash)) = (&file.original_content, &file.sha256) {
            let mut hasher = Sha256::new();
            hasher.update(content);
            let actual_hash = hex::encode(hasher.finalize());

            if &actual_hash != expected_hash {
                return Err(RestoreError::HashMismatch(
                    file.path.display().to_string(),
                    expected_hash.clone(),
                    actual_hash,
                ));
            }
        }
    }

    Ok(())
}

/// Try to remove empty directories up to a boundary
fn remove_empty_dirs(dir: &Path, boundary: &Path) -> Result<(), std::io::Error> {
    let mut current = dir;

    while current != boundary && current.starts_with(boundary) {
        // Check if directory is empty
        let is_empty = fs::read_dir(current)?.next().is_none();
        if is_empty {
            fs::remove_dir(current)?;
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(())
}
