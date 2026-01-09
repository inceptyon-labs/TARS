//! Diff display formatting for user review

use crate::diff::{DiffPlan, FileOperation, WarningSeverity};
use std::fmt::Write;

/// Format a diff plan for terminal display
pub fn format_plan_terminal(plan: &DiffPlan) -> String {
    let mut output = String::new();

    // Header
    writeln!(output, "=== Diff Plan ===").unwrap();
    writeln!(output, "Operations: {}", plan.operations.len()).unwrap();
    writeln!(output).unwrap();

    // Warnings first
    if !plan.warnings.is_empty() {
        writeln!(output, "Warnings:").unwrap();
        for warning in &plan.warnings {
            let prefix = match warning.severity {
                WarningSeverity::Info => "[INFO]",
                WarningSeverity::Warning => "[WARN]",
                WarningSeverity::Error => "[ERROR]",
            };
            writeln!(output, "  {} {}", prefix, warning.message).unwrap();
        }
        writeln!(output).unwrap();
    }

    // Operations
    for op in &plan.operations {
        match op {
            FileOperation::Create { path, content } => {
                writeln!(output, "CREATE: {}", path.display()).unwrap();
                writeln!(output, "  Size: {} bytes", content.len()).unwrap();
            }
            FileOperation::Modify { path, diff, .. } => {
                writeln!(output, "MODIFY: {}", path.display()).unwrap();
                // Show diff with indentation
                for line in diff.lines() {
                    writeln!(output, "  {line}").unwrap();
                }
            }
            FileOperation::Delete { path } => {
                writeln!(output, "DELETE: {}", path.display()).unwrap();
            }
        }
        writeln!(output).unwrap();
    }

    output
}

/// Format a diff plan as markdown for documentation/export
pub fn format_plan_markdown(plan: &DiffPlan) -> String {
    let mut output = String::new();

    writeln!(output, "# Diff Plan").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "**Operations:** {}", plan.operations.len()).unwrap();
    writeln!(output).unwrap();

    // Warnings
    if !plan.warnings.is_empty() {
        writeln!(output, "## Warnings").unwrap();
        writeln!(output).unwrap();
        for warning in &plan.warnings {
            let emoji = match warning.severity {
                WarningSeverity::Info => "ℹ️",
                WarningSeverity::Warning => "⚠️",
                WarningSeverity::Error => "❌",
            };
            writeln!(output, "- {} {}", emoji, warning.message).unwrap();
        }
        writeln!(output).unwrap();
    }

    // Operations
    writeln!(output, "## Changes").unwrap();
    writeln!(output).unwrap();

    for op in &plan.operations {
        match op {
            FileOperation::Create { path, content } => {
                writeln!(output, "### ➕ Create `{}`", path.display()).unwrap();
                writeln!(output).unwrap();
                writeln!(output, "New file ({} bytes)", content.len()).unwrap();
            }
            FileOperation::Modify { path, diff, .. } => {
                writeln!(output, "### ✏️ Modify `{}`", path.display()).unwrap();
                writeln!(output).unwrap();
                writeln!(output, "```diff").unwrap();
                writeln!(output, "{diff}").unwrap();
                writeln!(output, "```").unwrap();
            }
            FileOperation::Delete { path } => {
                writeln!(output, "### ➖ Delete `{}`", path.display()).unwrap();
            }
        }
        writeln!(output).unwrap();
    }

    output
}

/// Summary statistics for a diff plan
#[derive(Debug, Default)]
pub struct DiffSummary {
    /// Files to create
    pub creates: usize,
    /// Files to modify
    pub modifies: usize,
    /// Files to delete
    pub deletes: usize,
    /// Total bytes to be written
    pub total_bytes: usize,
}

impl DiffSummary {
    /// Generate summary from a diff plan
    pub fn from_plan(plan: &DiffPlan) -> Self {
        let mut summary = Self::default();

        for op in &plan.operations {
            match op {
                FileOperation::Create { content, .. } => {
                    summary.creates += 1;
                    summary.total_bytes += content.len();
                }
                FileOperation::Modify { new_content, .. } => {
                    summary.modifies += 1;
                    summary.total_bytes += new_content.len();
                }
                FileOperation::Delete { .. } => {
                    summary.deletes += 1;
                }
            }
        }

        summary
    }

    /// Format as a one-line summary
    pub fn one_line(&self) -> String {
        format!(
            "{} create(s), {} modify(s), {} delete(s) - {} bytes total",
            self.creates, self.modifies, self.deletes, self.total_bytes
        )
    }
}
