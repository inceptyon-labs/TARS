//! Project scope scanner
//!
//! Scans a project directory for Claude Code configuration including:
//! - CLAUDE.md file
//! - .claude/ directory with settings, skills, commands, agents, hooks
//! - .claude.json MCP configuration
//! - Git repository information
//! - Project-scoped plugins (skills, agents, MCP, commands from plugins installed for this project)

use crate::artifacts::{AgentInfo, CommandInfo, HookInfo, SkillInfo};
use crate::error::{ScanError, ScanResult};
use crate::inventory::{GitInfo, ProjectScope, ProjectSettings};
use crate::parser::{parse_mcp_config, parse_settings};
use crate::plugins::PluginInventory;
use crate::scope::user::{scan_agents_directory, scan_commands_directory, scan_skills_directory};
use crate::settings::{McpConfig, SettingsFile};
use crate::types::{FileInfo, Scope};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Scan a project directory for Claude Code configuration
///
/// This is a standalone scan that does not include project-scoped plugins.
/// Use `scan_project_with_plugins` when you have a pre-scanned plugin inventory.
///
/// # Errors
/// Returns an error if scanning fails
pub fn scan_project(path: &Path) -> ScanResult<ProjectScope> {
    // For standalone usage, scan plugins and pass to the shared function
    let plugin_inventory = PluginInventory::scan()?;
    scan_project_with_plugins(path, &plugin_inventory)
}

/// Scan a project directory for Claude Code configuration with pre-scanned plugin inventory
///
/// This includes tools from plugins installed at project scope for this specific project.
///
/// # Errors
/// Returns an error if scanning fails
pub fn scan_project_with_plugins(
    path: &Path,
    plugin_inventory: &PluginInventory,
) -> ScanResult<ProjectScope> {
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

    // Scan project MCP config and merge with project-scoped plugin MCP configs
    let mut mcp = scan_project_mcp(path)?;
    let plugin_mcp = extract_project_plugin_mcp(plugin_inventory, path)?;
    mcp = merge_mcp_configs(mcp, plugin_mcp);

    // Scan project skills and merge with project-scoped plugin skills
    let mut skills = if claude_dir_exists {
        scan_skills_directory(&claude_dir.join("skills"), Scope::Project)?
    } else {
        Vec::new()
    };
    let plugin_skills = extract_project_plugin_skills(plugin_inventory, path)?;
    skills.extend(plugin_skills);

    // Scan project commands and merge with project-scoped plugin commands
    let mut commands = if claude_dir_exists {
        scan_commands_directory(&claude_dir.join("commands"), Scope::Project)?
    } else {
        Vec::new()
    };
    let plugin_commands = extract_project_plugin_commands(plugin_inventory, path)?;
    commands.extend(plugin_commands);

    // Scan project agents and merge with project-scoped plugin agents
    let mut agents = if claude_dir_exists {
        scan_agents_directory(&claude_dir.join("agents"), Scope::Project)?
    } else {
        Vec::new()
    };
    let plugin_agents = extract_project_plugin_agents(plugin_inventory, path)?;
    agents.extend(plugin_agents);

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

/// Check if a plugin is scoped to a specific project
fn is_plugin_for_project(plugin: &crate::plugins::InstalledPlugin, project_path: &Path) -> bool {
    // Plugin must be project-scoped or local-scoped
    if !matches!(plugin.scope, Scope::Project | Scope::Local) {
        return false;
    }

    // Plugin must be enabled
    if !plugin.enabled {
        return false;
    }

    // Plugin must have a project_path that matches this project
    match &plugin.project_path {
        Some(pp) => {
            // Normalize paths for comparison
            let plugin_project = std::path::Path::new(pp);
            plugin_project == project_path
                || plugin_project
                    .canonicalize()
                    .ok()
                    .zip(project_path.canonicalize().ok())
                    .is_some_and(|(a, b)| a == b)
        }
        None => false,
    }
}

/// Extract skills from plugins installed at project scope for a specific project
fn extract_project_plugin_skills(
    plugin_inventory: &PluginInventory,
    project_path: &Path,
) -> ScanResult<Vec<SkillInfo>> {
    let mut all_skills = Vec::new();

    for plugin in &plugin_inventory.installed {
        if !is_plugin_for_project(plugin, project_path) {
            continue;
        }

        let skills_dir = plugin.path.join("skills");
        if skills_dir.exists() {
            let plugin_id = match &plugin.marketplace {
                Some(marketplace) => format!("{}@{}", plugin.id, marketplace),
                None => plugin.id.clone(),
            };
            let scope = Scope::Plugin(plugin_id);
            let dir_skills = scan_skills_directory(&skills_dir, scope)?;
            all_skills.extend(dir_skills);
        }
    }

    Ok(all_skills)
}

/// Extract commands from plugins installed at project scope for a specific project
fn extract_project_plugin_commands(
    plugin_inventory: &PluginInventory,
    project_path: &Path,
) -> ScanResult<Vec<CommandInfo>> {
    let mut all_commands = Vec::new();

    for plugin in &plugin_inventory.installed {
        if !is_plugin_for_project(plugin, project_path) {
            continue;
        }

        let commands_dir = plugin.path.join("commands");
        if commands_dir.exists() {
            let plugin_id = match &plugin.marketplace {
                Some(marketplace) => format!("{}@{}", plugin.id, marketplace),
                None => plugin.id.clone(),
            };
            let scope = Scope::Plugin(plugin_id);
            let dir_commands = scan_commands_directory(&commands_dir, scope)?;
            all_commands.extend(dir_commands);
        }
    }

    Ok(all_commands)
}

/// Extract agents from plugins installed at project scope for a specific project
fn extract_project_plugin_agents(
    plugin_inventory: &PluginInventory,
    project_path: &Path,
) -> ScanResult<Vec<AgentInfo>> {
    let mut all_agents = Vec::new();

    for plugin in &plugin_inventory.installed {
        if !is_plugin_for_project(plugin, project_path) {
            continue;
        }

        let agents_dir = plugin.path.join("agents");
        if agents_dir.exists() {
            let plugin_id = match &plugin.marketplace {
                Some(marketplace) => format!("{}@{}", plugin.id, marketplace),
                None => plugin.id.clone(),
            };
            let scope = Scope::Plugin(plugin_id);
            let dir_agents = scan_agents_directory(&agents_dir, scope)?;
            all_agents.extend(dir_agents);
        }
    }

    Ok(all_agents)
}

/// Extract MCP configs from plugins installed at project scope for a specific project
fn extract_project_plugin_mcp(
    plugin_inventory: &PluginInventory,
    project_path: &Path,
) -> ScanResult<Vec<McpConfig>> {
    let mut all_mcp = Vec::new();

    for plugin in &plugin_inventory.installed {
        if !is_plugin_for_project(plugin, project_path) {
            continue;
        }

        if let Some(mcp_path) = resolve_plugin_mcp_path(plugin) {
            if let Ok(content) = fs::read_to_string(&mcp_path) {
                if let Ok(mut mcp) = parse_mcp_config(&mcp_path, &content) {
                    // Tag the MCP servers with their plugin source
                    let plugin_id = match &plugin.marketplace {
                        Some(marketplace) => format!("{}@{}", plugin.id, marketplace),
                        None => plugin.id.clone(),
                    };
                    mcp.source_plugin = Some(plugin_id);
                    all_mcp.push(mcp);
                }
            }
        }
    }

    Ok(all_mcp)
}

fn resolve_plugin_mcp_path(plugin: &crate::plugins::InstalledPlugin) -> Option<PathBuf> {
    if let Some(path) = plugin.manifest.mcp_servers.as_ref() {
        if path.is_absolute() && path.exists() {
            return Some(path.clone());
        }
        let relative_candidates = [
            plugin.path.join(path),
            plugin.path.join(".claude-plugin").join(path),
        ];
        if let Some(found) = relative_candidates
            .into_iter()
            .find(|candidate| candidate.exists())
        {
            return Some(found);
        }
    }

    let candidates = [
        plugin.path.join(".mcp.json"),
        plugin.path.join(".claude-plugin").join("mcp.json"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

/// Merge multiple MCP configs into a single Option<McpConfig>
fn merge_mcp_configs(base: Option<McpConfig>, plugin_configs: Vec<McpConfig>) -> Option<McpConfig> {
    if plugin_configs.is_empty() {
        return base;
    }

    let mut merged = base.unwrap_or_default();

    // Get existing server names to avoid duplicates
    let existing_names: std::collections::HashSet<_> =
        merged.servers.iter().map(|s| s.name.clone()).collect();

    for config in plugin_configs {
        // Merge servers from plugin configs
        for server in config.servers {
            // Don't overwrite project-defined servers
            if !existing_names.contains(&server.name) {
                merged.servers.push(server);
            }
        }
    }

    if merged.servers.is_empty() && merged.path.as_os_str().is_empty() {
        None
    } else {
        Some(merged)
    }
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
    // Project MCP config can be at .mcp.json or .claude.json in project root
    let mcp_json_path = project_path.join(".mcp.json");
    let claude_json_path = project_path.join(".claude.json");

    // Prefer .mcp.json, fall back to .claude.json
    let mcp_path = if mcp_json_path.exists() {
        mcp_json_path
    } else if claude_json_path.exists() {
        claude_json_path
    } else {
        return Ok(None);
    };

    let content = fs::read_to_string(&mcp_path)?;
    let mcp = parse_mcp_config(&mcp_path, &content)?;
    Ok(Some(mcp))
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
/// Returns (`branch_name`, `is_dirty`)
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
