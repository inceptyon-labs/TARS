//! Profile snapshot creation from current state

use crate::profile::{
    AgentOverlay, ClaudeMdOverlay, CommandOverlay, OverlayMode, Profile, SkillOverlay,
};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors during snapshot creation
#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path not found: {0}")]
    PathNotFound(String),
}

/// Create a profile snapshot from a project directory
///
/// # Errors
/// Returns an error if snapshot creation fails
pub fn snapshot_from_project(project_path: &Path, name: String) -> Result<Profile, SnapshotError> {
    let mut profile = Profile::new(name);

    let claude_dir = project_path.join(".claude");

    // Snapshot CLAUDE.md if it exists
    let claude_md_path = project_path.join("CLAUDE.md");
    if claude_md_path.exists() {
        let content = fs::read_to_string(&claude_md_path)?;
        profile.repo_overlays.claude_md = Some(ClaudeMdOverlay {
            mode: OverlayMode::Replace,
            content,
        });
    }

    // Snapshot skills
    profile.repo_overlays.skills = snapshot_skills(&claude_dir.join("skills"))?;

    // Snapshot commands
    profile.repo_overlays.commands = snapshot_commands(&claude_dir.join("commands"))?;

    // Snapshot agents
    profile.repo_overlays.agents = snapshot_agents(&claude_dir.join("agents"))?;

    Ok(profile)
}

/// Create a profile snapshot from user-level configuration
///
/// # Errors
/// Returns an error if snapshot creation fails
pub fn snapshot_from_user(name: String) -> Result<Profile, SnapshotError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| {
            SnapshotError::PathNotFound("HOME environment variable not set".to_string())
        })?;

    let claude_dir = home.join(".claude");
    let mut profile = Profile::new(name);

    // Snapshot user skills
    profile.user_overlays.skills = snapshot_skills(&claude_dir.join("skills"))?;

    // Snapshot user commands
    profile.user_overlays.commands = snapshot_commands(&claude_dir.join("commands"))?;

    Ok(profile)
}

fn snapshot_skills(skills_dir: &Path) -> Result<Vec<SkillOverlay>, SnapshotError> {
    let mut skills = Vec::new();

    if !skills_dir.exists() {
        return Ok(skills);
    }

    for entry in fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let skill_file = path.join("SKILL.md");
            if skill_file.exists() {
                let content = fs::read_to_string(&skill_file)?;
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                skills.push(SkillOverlay { name, content });
            }
        }
    }

    Ok(skills)
}

fn snapshot_commands(commands_dir: &Path) -> Result<Vec<CommandOverlay>, SnapshotError> {
    let mut commands = Vec::new();

    if !commands_dir.exists() {
        return Ok(commands);
    }

    for entry in fs::read_dir(commands_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            let content = fs::read_to_string(&path)?;
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            commands.push(CommandOverlay { name, content });
        }
    }

    Ok(commands)
}

fn snapshot_agents(agents_dir: &Path) -> Result<Vec<AgentOverlay>, SnapshotError> {
    let mut agents = Vec::new();

    if !agents_dir.exists() {
        return Ok(agents);
    }

    for entry in fs::read_dir(agents_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            let content = fs::read_to_string(&path)?;
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            agents.push(AgentOverlay { name, content });
        }
    }

    Ok(agents)
}
