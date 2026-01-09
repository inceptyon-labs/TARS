//! Utility functions for TARS

use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Errors related to path validation
#[derive(Error, Debug)]
pub enum PathError {
    #[error("Path traversal attempt detected: {0}")]
    TraversalAttempt(String),

    #[error("Path escapes root directory: {0}")]
    EscapesRoot(String),

    #[error("Invalid path component: {0}")]
    InvalidComponent(String),

    #[error("Symlink not allowed: {0}")]
    SymlinkNotAllowed(String),
}

/// Validate that a path does not escape a root directory via `..` components
/// Returns the canonicalized path if safe
///
/// # Errors
/// Returns an error if the path would escape the root directory
pub fn safe_join(root: &Path, untrusted_path: &Path) -> Result<PathBuf, PathError> {
    // First, normalize the untrusted path by removing redundant components
    let normalized = normalize_path(untrusted_path)?;

    // Join with root
    let joined = root.join(&normalized);

    // Verify the joined path is still under root
    // We can't use canonicalize() because the file may not exist yet
    // Instead, check that the path doesn't escape via ..
    verify_under_root(root, &joined)?;

    Ok(joined)
}

/// Normalize a path by removing . and .. components where possible
/// Also validates for path traversal attempts
fn normalize_path(path: &Path) -> Result<PathBuf, PathError> {
    let mut normalized = PathBuf::new();
    let mut depth: i32 = 0;

    for component in path.components() {
        match component {
            Component::Normal(c) => {
                // Check for dangerous characters that could be misinterpreted
                let s = c.to_string_lossy();
                if s.contains('\0') {
                    return Err(PathError::InvalidComponent(
                        "Null byte in path".to_string(),
                    ));
                }
                normalized.push(c);
                depth += 1;
            }
            Component::CurDir => {
                // Skip . components
            }
            Component::ParentDir => {
                if depth > 0 {
                    normalized.pop();
                    depth -= 1;
                } else {
                    // Attempting to go above root
                    return Err(PathError::TraversalAttempt(path.display().to_string()));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                // Absolute paths not allowed in untrusted input
                return Err(PathError::InvalidComponent(
                    "Absolute path not allowed".to_string(),
                ));
            }
        }
    }

    Ok(normalized)
}

/// Verify that a path is under the given root
///
/// This uses logical path comparison after normalization, since canonicalization
/// requires paths to exist and can have issues with symlinks.
fn verify_under_root(root: &Path, path: &Path) -> Result<(), PathError> {
    // For security, we rely on the normalize_path function having already
    // rejected any .. components that would escape. At this point, we just
    // need to verify the path starts with the root prefix.

    // If both paths exist, use canonicalization for the most accurate check
    if root.exists() && path.exists() {
        let canonical_root = root.canonicalize()
            .map_err(|_| PathError::EscapesRoot(path.display().to_string()))?;
        let canonical_path = path.canonicalize()
            .map_err(|_| PathError::EscapesRoot(path.display().to_string()))?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(PathError::EscapesRoot(path.display().to_string()));
        }
    } else {
        // For non-existent paths, we've already validated via normalize_path
        // that there are no escaping .. components. Just verify the logical
        // path structure.
        if !path.starts_with(root) {
            return Err(PathError::EscapesRoot(path.display().to_string()));
        }
    }

    Ok(())
}

/// Validate a name (skill, command, agent) for use in paths
/// Names must not contain path separators or .. sequences
///
/// # Errors
/// Returns an error if the name is invalid
pub fn validate_name(name: &str) -> Result<(), PathError> {
    if name.is_empty() {
        return Err(PathError::InvalidComponent("Empty name".to_string()));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(PathError::TraversalAttempt(format!(
            "Name contains path separator: {name}"
        )));
    }

    if name.contains("..") {
        return Err(PathError::TraversalAttempt(format!(
            "Name contains parent directory reference: {name}"
        )));
    }

    if name.starts_with('.') && name != ".claude" {
        return Err(PathError::InvalidComponent(format!(
            "Name cannot start with dot: {name}"
        )));
    }

    if name.contains('\0') {
        return Err(PathError::InvalidComponent(
            "Name contains null byte".to_string(),
        ));
    }

    Ok(())
}

/// Check if a path is a symlink
///
/// # Errors
/// Returns an error if the path is a symlink
pub fn reject_symlink(path: &Path) -> Result<(), PathError> {
    if path.is_symlink() {
        return Err(PathError::SymlinkNotAllowed(path.display().to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_join_normal() {
        let root = PathBuf::from("/tmp/project");
        let result = safe_join(&root, Path::new("file.txt")).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/project/file.txt"));
    }

    #[test]
    fn test_safe_join_nested() {
        let root = PathBuf::from("/tmp/project");
        let result = safe_join(&root, Path::new("dir/subdir/file.txt")).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/project/dir/subdir/file.txt"));
    }

    #[test]
    fn test_safe_join_rejects_traversal() {
        let root = PathBuf::from("/tmp/project");
        let result = safe_join(&root, Path::new("../etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_join_rejects_absolute() {
        let root = PathBuf::from("/tmp/project");
        let result = safe_join(&root, Path::new("/etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_name_normal() {
        assert!(validate_name("my-skill").is_ok());
        assert!(validate_name("my_command").is_ok());
        assert!(validate_name("MyAgent123").is_ok());
    }

    #[test]
    fn test_validate_name_rejects_traversal() {
        assert!(validate_name("../etc").is_err());
        assert!(validate_name("foo/bar").is_err());
        assert!(validate_name("foo\\bar").is_err());
    }

    #[test]
    fn test_validate_name_rejects_hidden() {
        assert!(validate_name(".hidden").is_err());
        assert!(validate_name("..").is_err());
    }
}
