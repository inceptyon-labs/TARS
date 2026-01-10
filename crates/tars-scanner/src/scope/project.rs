//! Project scope scanner
//!
//! Scans a project directory for Claude Code configuration including:
//! - CLAUDE.md file
//! - .claude/ directory with settings, skills, commands, agents, hooks
//! - .claude.json MCP configuration
//! - Git repository information

use crate::artifacts::HookInfo;
use crate::error::{ScanError, ScanResult};
use crate::inventory::{GitInfo, ProjectScope, ProjectSettings};
use crate::parser::{parse_mcp_config, parse_settings};
use crate::scope::user::{scan_agents_directory, scan_commands_directory, scan_skills_directory};
use crate::settings::{McpConfig, SettingsFile};
use crate::types::{FileInfo, Scope};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Scan a project directory for Claude Code configuration
///
/// # Errors
/// Returns an error if scanning fails
pub fn scan_project(path: &Path) -> ScanResult<ProjectScope> {
    if !path.exists() {
        return Err(ScanError::InvalidPath(format!(
            "Project path does not exist: {}",
            path.display()
        )));
    }

    if !path.is_dir() {
        return Err(ScanError::InvalidPath(format!(
            "Project path is not a directory: {}",
            path.display()
        )));
    }

    let name = path.file_name().map_or_else(
        || "unnamed".to_string(),
        |n| n.to_string_lossy().to_string(),
    );

    let claude_dir = path.join(".claude");
    let claude_dir_exists = claude_dir.exists() && claude_dir.is_dir();

    let git = scan_git_info(path);
    let claude_md = scan_claude_md(path)?;
    let settings = scan_project_settings(&claude_dir)?;
    let mcp = scan_project_mcp(path)?;
    let skills = if claude_dir_exists {
        scan_skills_directory(&claude_dir.join("skills"), Scope::Project)?
    } else {
        Vec::new()
    };
    let commands = if claude_dir_exists {
        scan_commands_directory(&claude_dir.join("commands"), Scope::Project)?
    } else {
        Vec::new()
    };
    let agents = if claude_dir_exists {
        scan_agents_directory(&claude_dir.join("agents"), Scope::Project)?
    } else {
        Vec::new()
    };
    let hooks = if claude_dir_exists {
        scan_hooks_directory(&claude_dir.join("hooks"))?
    } else {
        Vec::new()
    };

    Ok(ProjectScope {
        path: path.to_path_buf(),
        name,
        git,
        claude_md,
        claude_dir: if claude_dir_exists {
            Some(claude_dir)
        } else {
            None
        },
        settings,
        mcp,
        skills,
        commands,
        agents,
        hooks,
    })
}

fn scan_claude_md(project_path: &Path) -> ScanResult<Option<FileInfo>> {
    let claude_md_path = project_path.join("CLAUDE.md");
    if claude_md_path.exists() {
        let content = fs::read_to_string(&claude_md_path)?;

        Ok(Some(FileInfo {
            path: claude_md_path,
            sha256: hash_content(&content),
        }))
    } else {
        Ok(None)
    }
}

fn scan_project_settings(claude_dir: &Path) -> ScanResult<ProjectSettings> {
    let shared = scan_settings_file(&claude_dir.join("settings.json"))?;
    let local = scan_settings_file(&claude_dir.join("settings.local.json"))?;

    Ok(ProjectSettings { shared, local })
}

fn scan_settings_file(path: &Path) -> ScanResult<Option<SettingsFile>> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        let settings = parse_settings(path, &content)?;
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}

fn scan_project_mcp(project_path: &Path) -> ScanResult<Option<McpConfig>> {
    // Project MCP config is at .claude.json in project root
    let mcp_path = project_path.join(".claude.json");
    if mcp_path.exists() {
        let content = fs::read_to_string(&mcp_path)?;
        let mcp = parse_mcp_config(&mcp_path, &content)?;
        Ok(Some(mcp))
    } else {
        Ok(None)
    }
}

fn scan_git_info(project_path: &Path) -> Option<GitInfo> {
    let git_dir = project_path.join(".git");
    if !git_dir.exists() {
        return None;
    }

    // Use git status -sb to get branch and dirty status in one call
    // Output format: "## branch...tracking [M/A/D indicators]"
    let (branch, is_dirty) = get_git_status(project_path);

    // Get remote URL (still needs separate call, but only if git repo exists)
    let remote = get_git_remote(project_path);

    Some(GitInfo {
        remote,
        branch,
        is_dirty,
    })
}

/// Create a git command with security hardening to prevent malicious hooks/config
fn secure_git_command() -> Command {
    let mut cmd = Command::new("git");
    // Prevent loading of system and global git config to avoid malicious overrides
    cmd.env("GIT_CONFIG_NOSYSTEM", "1");
    // Use /dev/null for global config to prevent loading user's global config
    // which could be manipulated if the user cloned a malicious repo
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
    // Disable advice messages that could leak info
    cmd.env("GIT_ADVICE", "0");
    cmd
}

/// Get branch name and dirty status in a single git call
/// Returns (branch_name, is_dirty)
fn get_git_status(project_path: &Path) -> (String, bool) {
    let output = secure_git_command()
        .args(["status", "-sb"])
        .current_dir(project_path)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = stdout.lines().collect();

            // First line is "## branch" or "## branch...remote"
            let branch = lines
                .first()
                .and_then(|line| {
                    line.strip_prefix("## ").map(|rest| {
                        // Handle "branch...origin/branch" format
                        rest.split("...").next().unwrap_or(rest).to_string()
                    })
                })
                .unwrap_or_else(|| "unknown".to_string());

            // If there are any lines after the first, the repo is dirty
            let is_dirty = lines.len() > 1;

            (branch, is_dirty)
        }
        _ => ("unknown".to_string(), false),
    }
}

fn get_git_remote(project_path: &Path) -> Option<String> {
    let output = secure_git_command()
        .args(["remote", "get-url", "origin"])
        .current_dir(project_path)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn scan_hooks_directory(hooks_dir: &Path) -> ScanResult<Vec<HookInfo>> {
    let mut hooks = Vec::new();

    if !hooks_dir.exists() {
        return Ok(hooks);
    }

    // Look for hooks.json file
    let hooks_json_path = hooks_dir.join("hooks.json");
    if hooks_json_path.exists() {
        match fs::read_to_string(&hooks_json_path) {
            Ok(content) => match parse_hooks_json(&hooks_json_path, &content) {
                Ok(parsed_hooks) => hooks.extend(parsed_hooks),
                Err(e) => {
                    eprintln!("Warning: Failed to parse hooks at {hooks_json_path:?}: {e}");
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to read hooks file {hooks_json_path:?}: {e}");
            }
        }
    }

    Ok(hooks)
}

fn parse_hooks_json(path: &Path, content: &str) -> ScanResult<Vec<HookInfo>> {
    use crate::artifacts::{HookDefinition, HookSource, HookTrigger};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct HooksFile {
        #[serde(default)]
        hooks: Vec<HookEntry>,
    }

    #[derive(Deserialize)]
    struct HookEntry {
        event: String,
        #[serde(default)]
        matcher: Option<String>,
        command: Option<String>,
        prompt: Option<String>,
        agent: Option<String>,
    }

    let hooks_file: HooksFile = serde_json::from_str(content)?;

    Ok(hooks_file
        .hooks
        .into_iter()
        .filter_map(|h| {
            let trigger = match h.event.as_str() {
                "PreToolUse" => HookTrigger::PreToolUse,
                "PostToolUse" => HookTrigger::PostToolUse,
                "PermissionRequest" => HookTrigger::PermissionRequest,
                "UserPromptSubmit" => HookTrigger::UserPromptSubmit,
                "SessionStart" => HookTrigger::SessionStart,
                "SessionEnd" => HookTrigger::SessionEnd,
                "Notification" => HookTrigger::Notification,
                "Stop" => HookTrigger::Stop,
                "SubagentStop" => HookTrigger::SubagentStop,
                "PreCompact" => HookTrigger::PreCompact,
                _ => return None,
            };

            let definition = if let Some(cmd) = h.command {
                HookDefinition::Command { command: cmd }
            } else if let Some(prompt) = h.prompt {
                HookDefinition::Prompt { prompt }
            } else if let Some(agent) = h.agent {
                HookDefinition::Agent { agent }
            } else {
                return None;
            };

            Some(HookInfo {
                source: HookSource::Settings {
                    path: path.to_path_buf(),
                },
                trigger,
                matcher: h.matcher,
                definition,
            })
        })
        .collect())
}

fn hash_content(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}
