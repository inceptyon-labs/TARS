//! Diff plan generation for profile application

use crate::diff::{DiffPlan, FileOperation, Warning, WarningSeverity};
use crate::profile::{
    AgentOverlay, ClaudeMdOverlay, CommandOverlay, OverlayMode, Profile, SkillOverlay,
};
use crate::util::{validate_name, PathError};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

/// Errors during diff plan generation
#[derive(Error, Debug)]
pub enum PlanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Invalid overlay name: {0}")]
    InvalidName(#[from] PathError),
}

/// Generate a diff plan for applying a profile to a project
///
/// # Errors
/// Returns an error if plan generation fails
pub fn generate_plan(
    project_id: Uuid,
    project_path: &Path,
    profile: &Profile,
) -> Result<DiffPlan, PlanError> {
    let mut plan = DiffPlan::new(project_id, profile.id);

    // Process CLAUDE.md overlay
    if let Some(claude_md) = &profile.repo_overlays.claude_md {
        plan_claude_md(project_path, claude_md, &mut plan)?;
    }

    // Process repo skill overlays
    for skill in &profile.repo_overlays.skills {
        plan_skill(project_path, skill, &mut plan)?;
    }

    // Process repo command overlays
    for cmd in &profile.repo_overlays.commands {
        plan_command(project_path, cmd, &mut plan)?;
    }

    // Process repo agent overlays
    for agent in &profile.repo_overlays.agents {
        plan_agent(project_path, agent, &mut plan)?;
    }

    Ok(plan)
}

fn plan_claude_md(
    project_path: &Path,
    overlay: &ClaudeMdOverlay,
    plan: &mut DiffPlan,
) -> Result<(), PlanError> {
    let claude_md_path = project_path.join("CLAUDE.md");

    let new_content = if claude_md_path.exists() {
        let existing = fs::read_to_string(&claude_md_path)?;
        match overlay.mode {
            OverlayMode::Replace => overlay.content.clone(),
            OverlayMode::Prepend => format!("{}\n\n{}", overlay.content, existing),
            OverlayMode::Append => format!("{}\n\n{}", existing, overlay.content),
        }
    } else {
        overlay.content.clone()
    };

    if claude_md_path.exists() {
        let existing = fs::read_to_string(&claude_md_path)?;
        if existing != new_content {
            let diff = generate_text_diff(&existing, &new_content);
            plan.operations.push(FileOperation::Modify {
                path: claude_md_path,
                diff,
                new_content: new_content.into_bytes(),
            });
        }
    } else {
        plan.operations.push(FileOperation::Create {
            path: claude_md_path,
            content: new_content.into_bytes(),
        });
    }

    Ok(())
}

fn plan_skill(
    project_path: &Path,
    skill: &SkillOverlay,
    plan: &mut DiffPlan,
) -> Result<(), PlanError> {
    // Validate skill name to prevent path traversal
    validate_name(&skill.name)?;

    let skill_dir = project_path
        .join(".claude")
        .join("skills")
        .join(&skill.name);
    let skill_file = skill_dir.join("SKILL.md");

    if skill_file.exists() {
        let existing = fs::read_to_string(&skill_file)?;
        if existing != skill.content {
            let diff = generate_text_diff(&existing, &skill.content);
            plan.operations.push(FileOperation::Modify {
                path: skill_file,
                diff,
                new_content: skill.content.clone().into_bytes(),
            });
        }
    } else {
        // Need to create directory and file
        plan.operations.push(FileOperation::Create {
            path: skill_file,
            content: skill.content.clone().into_bytes(),
        });
    }

    Ok(())
}

fn plan_command(
    project_path: &Path,
    cmd: &CommandOverlay,
    plan: &mut DiffPlan,
) -> Result<(), PlanError> {
    // Validate command name to prevent path traversal
    validate_name(&cmd.name)?;

    let cmd_path = project_path
        .join(".claude")
        .join("commands")
        .join(format!("{}.md", cmd.name));

    if cmd_path.exists() {
        let existing = fs::read_to_string(&cmd_path)?;
        if existing != cmd.content {
            let diff = generate_text_diff(&existing, &cmd.content);
            plan.operations.push(FileOperation::Modify {
                path: cmd_path,
                diff,
                new_content: cmd.content.clone().into_bytes(),
            });
        }
    } else {
        plan.operations.push(FileOperation::Create {
            path: cmd_path,
            content: cmd.content.clone().into_bytes(),
        });
    }

    Ok(())
}

fn plan_agent(
    project_path: &Path,
    agent: &AgentOverlay,
    plan: &mut DiffPlan,
) -> Result<(), PlanError> {
    // Validate agent name to prevent path traversal
    validate_name(&agent.name)?;

    let agent_path = project_path
        .join(".claude")
        .join("agents")
        .join(format!("{}.md", agent.name));

    if agent_path.exists() {
        let existing = fs::read_to_string(&agent_path)?;
        if existing != agent.content {
            let diff = generate_text_diff(&existing, &agent.content);
            plan.operations.push(FileOperation::Modify {
                path: agent_path,
                diff,
                new_content: agent.content.clone().into_bytes(),
            });
        }
    } else {
        plan.operations.push(FileOperation::Create {
            path: agent_path,
            content: agent.content.clone().into_bytes(),
        });
    }

    Ok(())
}

/// Generate a unified diff between two strings
#[must_use]
pub fn generate_text_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        output.push_str(sign);
        output.push_str(change.value());
        if !change.value().ends_with('\n') {
            output.push('\n');
        }
    }

    output
}

/// Check if applying a profile would cause issues
#[must_use]
pub fn check_git_dirty(project_path: &Path) -> Option<Warning> {
    let git_dir = project_path.join(".git");
    if !git_dir.exists() {
        return None;
    }

    use std::process::Command;
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_path)
        .output();

    match output {
        Ok(out) if out.status.success() && !out.stdout.is_empty() => Some(Warning {
            severity: WarningSeverity::Warning,
            message: "Repository has uncommitted changes. Consider committing before applying."
                .to_string(),
        }),
        _ => None,
    }
}
