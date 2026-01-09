//! File write operations

use crate::backup::{Backup, BackupFile};
use crate::diff::{DiffPlan, FileOperation};
use crate::util::{safe_join, PathError};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during apply
#[derive(Error, Debug)]
pub enum ApplyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File already exists: {0}")]
    FileExists(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Path security error: {0}")]
    PathSecurity(#[from] PathError),
}

/// Apply file operations from a diff plan
///
/// # Errors
/// Returns an error if any operation fails
pub fn apply_operations(
    plan: &DiffPlan,
    project_root: &Path,
    backup: &mut Backup,
) -> Result<(), ApplyError> {
    for operation in &plan.operations {
        apply_operation(operation, project_root, backup)?;
    }
    Ok(())
}

fn apply_operation(
    operation: &FileOperation,
    project_root: &Path,
    backup: &mut Backup,
) -> Result<(), ApplyError> {
    match operation {
        FileOperation::Create { path, content } => {
            // Get relative path for validation
            let relative_path = path
                .strip_prefix(project_root)
                .unwrap_or(path);

            // Validate path doesn't escape project directory
            let full_path = safe_join(project_root, relative_path)?;

            // Backup: file didn't exist
            backup.add_file(BackupFile::new_file(relative_path.to_path_buf()));

            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write the file
            fs::write(&full_path, content)?;
        }
        FileOperation::Modify { path, new_content, .. } => {
            // Get relative path for validation
            let relative_path = path
                .strip_prefix(project_root)
                .unwrap_or(path);

            // Validate path doesn't escape project directory
            let full_path = safe_join(project_root, relative_path)?;

            // Backup: save original content
            let original = fs::read(&full_path)?;
            let sha256 = compute_sha256(&original);
            backup.add_file(BackupFile::existing(relative_path.to_path_buf(), original, sha256));

            // Write the new content
            fs::write(&full_path, new_content)?;
        }
        FileOperation::Delete { path } => {
            // Get relative path for validation
            let relative_path = path
                .strip_prefix(project_root)
                .unwrap_or(path);

            // Validate path doesn't escape project directory
            let full_path = safe_join(project_root, relative_path)?;

            // Backup: save original content
            let original = fs::read(&full_path)?;
            let sha256 = compute_sha256(&original);
            backup.add_file(BackupFile::existing(relative_path.to_path_buf(), original, sha256));

            // Delete the file
            fs::remove_file(&full_path)?;
        }
    }
    Ok(())
}

fn compute_sha256(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}
