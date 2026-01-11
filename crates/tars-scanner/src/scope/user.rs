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

/// Get the user's home directory (cross-platform)
fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Scan user-level Claude Code configuration with pre-scanned plugin inventory
///
/// This avoids duplicate plugin scanning by accepting an already-scanned inventory.
///
/// # Errors
/// Returns an error if scanning fails
pub fn scan_user_scope_with_plugins(plugin_inventory: &PluginInventory) -> ScanResult<UserScope> {
    let home = home_dir().ok_or(ScanError::HomeNotFound)?;
    let claude_dir = home.join(".claude");

    let settings = scan_user_settings(&claude_dir)?;

    // Scan user MCP config and merge with plugin-provided MCP configs
    let mut mcp = scan_user_mcp(&home)?;
    let plugin_mcp = extract_plugin_mcp(plugin_inventory)?;
    mcp = merge_mcp_configs(mcp, plugin_mcp);

    // Scan user skills and merge with plugin-provided skills
    let mut skills = scan_user_skills(&claude_dir)?;
    let plugin_skills = extract_plugin_skills(plugin_inventory)?;
    skills.extend(plugin_skills);

    // Scan user commands and merge with plugin-provided commands
    let mut commands = scan_user_commands(&claude_dir)?;
    let plugin_commands = extract_plugin_commands(plugin_inventory)?;
    commands.extend(plugin_commands);

    // Scan user agents and merge with plugin-provided agents
    let mut agents = scan_user_agents(&claude_dir)?;
    let plugin_agents = extract_plugin_agents(plugin_inventory)?;
    agents.extend(plugin_agents);

    Ok(UserScope {
        settings,
        mcp,
        skills,
        commands,
        agents,
    })
}

/// Scan user-level Claude Code configuration
///
/// # Errors
/// Returns an error if scanning fails
pub fn scan_user_scope() -> ScanResult<UserScope> {
    // For standalone usage, scan plugins and pass to the shared function
    let plugin_inventory = PluginInventory::scan()?;
    scan_user_scope_with_plugins(&plugin_inventory)
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

/// Extract skills from a pre-scanned plugin inventory
///
/// Uses the already-scanned plugin inventory to extract skills,
/// avoiding duplicate file system operations.
fn extract_plugin_skills(plugin_inventory: &PluginInventory) -> ScanResult<Vec<SkillInfo>> {
    let mut all_skills = Vec::new();

    // Only scan skills from installed AND enabled plugin paths
    for plugin in &plugin_inventory.installed {
        if !plugin.enabled {
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

/// Extract MCP configs from installed plugins
fn extract_plugin_mcp(plugin_inventory: &PluginInventory) -> ScanResult<Vec<McpConfig>> {
    let mut all_mcp = Vec::new();

    for plugin in &plugin_inventory.installed {
        if !plugin.enabled {
            continue;
        }
        let mcp_path = plugin.path.join(".mcp.json");
        if mcp_path.exists() {
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
            // Don't overwrite user-defined servers
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

/// Extract commands from installed plugins
fn extract_plugin_commands(plugin_inventory: &PluginInventory) -> ScanResult<Vec<CommandInfo>> {
    let mut all_commands = Vec::new();

    for plugin in &plugin_inventory.installed {
        if !plugin.enabled {
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

/// Extract agents from installed plugins
fn extract_plugin_agents(plugin_inventory: &PluginInventory) -> ScanResult<Vec<AgentInfo>> {
    let mut all_agents = Vec::new();

    for plugin in &plugin_inventory.installed {
        if !plugin.enabled {
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
                                eprintln!("Warning: Failed to parse skill at {path:?}: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read skill file {skill_file:?}: {e}");
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
        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            match fs::read_to_string(&path) {
                Ok(content) => match parse_command(&path, &content, scope.clone()) {
                    Ok(cmd) => commands.push(cmd),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse command at {path:?}: {e}");
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read command file {path:?}: {e}");
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
        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            match fs::read_to_string(&path) {
                Ok(content) => match parse_agent(&path, &content, scope.clone()) {
                    Ok(agent) => agents.push(agent),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse agent at {path:?}: {e}");
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read agent file {path:?}: {e}");
                }
            }
        }
    }

    Ok(agents)
}
