//! Core config operations

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{ConfigItem, ConfigResult, ConfigScope};

/// Operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    Add,
    Remove,
    Update,
    Move,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "add"),
            Self::Remove => write!(f, "remove"),
            Self::Update => write!(f, "update"),
            Self::Move => write!(f, "move"),
        }
    }
}

/// A planned operation before execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPlan {
    /// Type of operation
    pub operation: OperationType,

    /// Item name
    pub name: String,

    /// Target scope
    pub scope: ConfigScope,

    /// Source scope (for move operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_scope: Option<ConfigScope>,

    /// Files that will be modified
    pub affected_files: Vec<PathBuf>,

    /// Human-readable diff preview
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,

    /// Warnings (non-fatal issues)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl OperationPlan {
    /// Create a new add operation plan
    pub fn add(name: impl Into<String>, scope: ConfigScope, file: PathBuf) -> Self {
        Self {
            operation: OperationType::Add,
            name: name.into(),
            scope,
            from_scope: None,
            affected_files: vec![file],
            diff: None,
            warnings: Vec::new(),
        }
    }

    /// Create a new remove operation plan
    pub fn remove(name: impl Into<String>, scope: ConfigScope, file: PathBuf) -> Self {
        Self {
            operation: OperationType::Remove,
            name: name.into(),
            scope,
            from_scope: None,
            affected_files: vec![file],
            diff: None,
            warnings: Vec::new(),
        }
    }

    /// Create a new move operation plan
    pub fn move_item(
        name: impl Into<String>,
        from_scope: ConfigScope,
        to_scope: ConfigScope,
        from_file: PathBuf,
        to_file: PathBuf,
    ) -> Self {
        Self {
            operation: OperationType::Move,
            name: name.into(),
            scope: to_scope,
            from_scope: Some(from_scope),
            affected_files: vec![from_file, to_file],
            diff: None,
            warnings: Vec::new(),
        }
    }

    /// Add a diff preview
    pub fn with_diff(mut self, diff: impl Into<String>) -> Self {
        self.diff = Some(diff.into());
        self
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Result of an executed operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Whether the operation succeeded
    pub success: bool,

    /// Operation that was performed
    pub operation: OperationType,

    /// Item name
    pub name: String,

    /// Scope affected
    pub scope: ConfigScope,

    /// Files that were modified
    pub files_modified: Vec<PathBuf>,

    /// Backup ID (for rollback)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Warnings (non-fatal issues)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl OperationResult {
    /// Create a success result
    pub fn success(
        operation: OperationType,
        name: impl Into<String>,
        scope: ConfigScope,
        files: Vec<PathBuf>,
        backup_id: Option<String>,
    ) -> Self {
        Self {
            success: true,
            operation,
            name: name.into(),
            scope,
            files_modified: files,
            backup_id,
            error: None,
            warnings: Vec::new(),
        }
    }

    /// Create a failure result
    pub fn failure(
        operation: OperationType,
        name: impl Into<String>,
        scope: ConfigScope,
        error: impl Into<String>,
    ) -> Self {
        Self {
            success: false,
            operation,
            name: name.into(),
            scope,
            files_modified: Vec::new(),
            backup_id: None,
            error: Some(error.into()),
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Config operations trait
///
/// This trait defines the core operations for managing config items.
/// Each item type (MCP, Skill, Hook, etc.) implements specific versions.
pub trait ConfigOps {
    /// List all items of this type across scopes
    fn list(&self, project_path: Option<&PathBuf>) -> ConfigResult<Vec<ConfigItem>>;

    /// Add a new item
    fn add(
        &self,
        name: &str,
        scope: ConfigScope,
        config: serde_json::Value,
        project_path: Option<&PathBuf>,
        dry_run: bool,
    ) -> ConfigResult<OperationResult>;

    /// Remove an item
    fn remove(
        &self,
        name: &str,
        scope: Option<ConfigScope>,
        project_path: Option<&PathBuf>,
        dry_run: bool,
    ) -> ConfigResult<OperationResult>;

    /// Update an existing item
    fn update(
        &self,
        name: &str,
        scope: Option<ConfigScope>,
        updates: serde_json::Value,
        project_path: Option<&PathBuf>,
        dry_run: bool,
    ) -> ConfigResult<OperationResult>;

    /// Move an item between scopes
    fn move_item(
        &self,
        name: &str,
        from_scope: Option<ConfigScope>,
        to_scope: ConfigScope,
        project_path: Option<&PathBuf>,
        force: bool,
        dry_run: bool,
    ) -> ConfigResult<OperationResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_plan_add() {
        let plan = OperationPlan::add("context7", ConfigScope::Project, PathBuf::from(".mcp.json"));
        assert_eq!(plan.operation, OperationType::Add);
        assert_eq!(plan.name, "context7");
        assert_eq!(plan.scope, ConfigScope::Project);
    }

    #[test]
    fn test_operation_result_success() {
        let result = OperationResult::success(
            OperationType::Add,
            "context7",
            ConfigScope::Project,
            vec![PathBuf::from(".mcp.json")],
            Some("backup-123".into()),
        );
        assert!(result.success);
        assert_eq!(result.backup_id, Some("backup-123".into()));
    }

    #[test]
    fn test_operation_result_failure() {
        let result = OperationResult::failure(
            OperationType::Add,
            "context7",
            ConfigScope::Project,
            "Item already exists",
        );
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
