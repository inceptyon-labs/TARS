//! User scope scanner

use crate::artifacts::{AgentInfo, CommandInfo, SkillInfo};
use crate::error::{ScanError, ScanResult};
use crate::inventory::UserScope;
use crate::parser::{parse_agent, parse_command, parse_mcp_config, parse_settings, parse_skill};
use crate::plugins::PluginInventory;
use crate::settings::{McpConfig, SettingsFile};
use crate::types::Scope;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the user's home directory
fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Scan user-level Claude Code configuration
///
/// # Errors
/// Returns an error if scanning fails
pub fn scan_user_scope() -> ScanResult<UserScope> {
    let home = home_dir();
    let claude_dir = home.join(".claude");

    let settings = scan_user_settings(&claude_dir)?;
    let mcp = scan_user_mcp(&home)?;

    // Scan user skills
    let mut skills = scan_user_skills(&claude_dir)?;

    // Also scan plugin skills (uses installed_plugins.json, not claude_dir)
    let plugin_skills = scan_plugin_skills()?;
    skills.extend(plugin_skills);

    let commands = scan_user_commands(&claude_dir)?;
    let agents = scan_user_agents(&claude_dir)?;

    Ok(UserScope {
        settings,
        mcp,
        skills,
        commands,
        agents,
    })
}

fn scan_user_settings(claude_dir: &Path) -> ScanResult<Option<SettingsFile>> {
    let settings_path = claude_dir.join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let settings = parse_settings(&settings_path, &content)?;
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}

fn scan_user_mcp(home: &Path) -> ScanResult<Option<McpConfig>> {
    // User MCP config is at ~/.claude.json (not in .claude/)
    let mcp_path = home.join(".claude.json");
    if mcp_path.exists() {
        let content = fs::read_to_string(&mcp_path)?;
        let mcp = parse_mcp_config(&mcp_path, &content)?;
        Ok(Some(mcp))
    } else {
        Ok(None)
    }
}

fn scan_user_skills(claude_dir: &Path) -> ScanResult<Vec<SkillInfo>> {
    let skills_dir = claude_dir.join("skills");
    scan_skills_directory(&skills_dir, Scope::User)
}

/// Scan plugin directories for skills (only installed plugins)
///
/// Reads installed_plugins.json to determine which plugins are installed,
/// then scans only those plugin paths for skills.
fn scan_plugin_skills() -> ScanResult<Vec<SkillInfo>> {
    let plugin_inventory = PluginInventory::scan()?;
    let mut all_skills = Vec::new();

    // Only scan skills from installed plugin paths
    for plugin in &plugin_inventory.installed {
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

fn scan_user_commands(claude_dir: &Path) -> ScanResult<Vec<CommandInfo>> {
    let commands_dir = claude_dir.join("commands");
    scan_commands_directory(&commands_dir, Scope::User)
}

fn scan_user_agents(claude_dir: &Path) -> ScanResult<Vec<AgentInfo>> {
    let agents_dir = claude_dir.join("agents");
    scan_agents_directory(&agents_dir, Scope::User)
}

/// Scan a directory for skill folders (each containing SKILL.md)
pub fn scan_skills_directory(dir: &Path, scope: Scope) -> ScanResult<Vec<SkillInfo>> {
    let mut skills = Vec::new();

    if !dir.exists() {
        return Ok(skills);
    }

    let entries = fs::read_dir(dir).map_err(ScanError::Io)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skill_file = path.join("SKILL.md");
            if skill_file.exists() {
                match fs::read_to_string(&skill_file) {
                    Ok(content) => {
                        // Pass the SKILL.md file path, not the directory
                        match parse_skill(&skill_file, &content, scope.clone()) {
                            Ok(skill) => skills.push(skill),
                            Err(e) => {
                                // Log warning but continue scanning
                                eprintln!("Warning: Failed to parse skill at {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read skill file {:?}: {}", skill_file, e);
                    }
                }
            }
        }
    }

    Ok(skills)
}

/// Scan a directory for command files (.md files)
pub fn scan_commands_directory(dir: &Path, scope: Scope) -> ScanResult<Vec<CommandInfo>> {
    let mut commands = Vec::new();

    if !dir.exists() {
        return Ok(commands);
    }

    let entries = fs::read_dir(dir).map_err(ScanError::Io)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            match fs::read_to_string(&path) {
                Ok(content) => match parse_command(&path, &content, scope.clone()) {
                    Ok(cmd) => commands.push(cmd),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse command at {:?}: {}", path, e);
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read command file {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(commands)
}

/// Scan a directory for agent files (.md files)
pub fn scan_agents_directory(dir: &Path, scope: Scope) -> ScanResult<Vec<AgentInfo>> {
    let mut agents = Vec::new();

    if !dir.exists() {
        return Ok(agents);
    }

    let entries = fs::read_dir(dir).map_err(ScanError::Io)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            match fs::read_to_string(&path) {
                Ok(content) => match parse_agent(&path, &content, scope.clone()) {
                    Ok(agent) => agents.push(agent),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse agent at {:?}: {}", path, e);
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read agent file {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(agents)
}
