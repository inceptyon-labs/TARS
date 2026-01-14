//! Project management Tauri commands
//!
//! Commands for managing tracked projects.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tars_core::storage::ProjectStore;
use tars_core::Project;
use tauri::State;

/// Project summary for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<tars_core::storage::projects::ProjectSummary> for ProjectInfo {
    fn from(p: tars_core::storage::projects::ProjectSummary) -> Self {
        Self {
            id: p.id.to_string(),
            name: p.name,
            path: p.path.display().to_string(),
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

/// List all tracked projects
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, String> {
    state.with_db(|db| {
        let store = ProjectStore::new(db.connection());
        let projects = store
            .list()
            .map_err(|e| format!("Failed to list projects: {e}"))?;
        Ok(projects.into_iter().map(ProjectInfo::from).collect())
    })
}

/// Add a new project to track
#[tauri::command]
pub async fn add_project(path: String, state: State<'_, AppState>) -> Result<ProjectInfo, String> {
    let project_path = PathBuf::from(&path);

    if !project_path.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    // Extract project name from path
    let name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let project = Project::new(project_path).with_name(name);

    state.with_db(|db| {
        let store = ProjectStore::new(db.connection());

        // Check if project already exists
        if store
            .get_by_path(&project.path)
            .map_err(|e| format!("Database error: {e}"))?
            .is_some()
        {
            return Err("Project already tracked".to_string());
        }

        store
            .create(&project)
            .map_err(|e| format!("Failed to add project: {e}"))?;

        Ok(ProjectInfo {
            id: project.id.to_string(),
            name: project.name,
            path: project.path.display().to_string(),
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        })
    })
}

/// Get a project by ID
#[tauri::command]
pub async fn get_project(id: String, state: State<'_, AppState>) -> Result<ProjectInfo, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = ProjectStore::new(db.connection());
        let project = store
            .get(uuid)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Project not found".to_string())?;

        Ok(ProjectInfo {
            id: project.id.to_string(),
            name: project.name,
            path: project.path.display().to_string(),
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        })
    })
}

/// Remove a project from tracking
#[tauri::command]
pub async fn remove_project(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {e}"))?;

    state.with_db(|db| {
        let store = ProjectStore::new(db.connection());
        store
            .delete(uuid)
            .map_err(|e| format!("Failed to remove project: {e}"))
    })
}

/// CLAUDE.md content response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMdInfo {
    pub path: String,
    pub content: Option<String>,
    pub exists: bool,
}

/// Read CLAUDE.md from a project
#[tauri::command]
pub async fn read_claude_md(project_path: String) -> Result<ClaudeMdInfo, String> {
    let path = PathBuf::from(&project_path).join("CLAUDE.md");

    if path.exists() {
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read CLAUDE.md: {e}"))?;
        Ok(ClaudeMdInfo {
            path: path.display().to_string(),
            content: Some(content),
            exists: true,
        })
    } else {
        Ok(ClaudeMdInfo {
            path: path.display().to_string(),
            content: None,
            exists: false,
        })
    }
}

/// Save CLAUDE.md to a project
#[tauri::command]
pub async fn save_claude_md(project_path: String, content: String) -> Result<(), String> {
    let path = PathBuf::from(&project_path).join("CLAUDE.md");

    std::fs::write(&path, &content).map_err(|e| format!("Failed to save CLAUDE.md: {e}"))?;

    Ok(())
}

/// Delete CLAUDE.md from a project
#[tauri::command]
pub async fn delete_claude_md(project_path: String) -> Result<(), String> {
    let path = PathBuf::from(&project_path).join("CLAUDE.md");

    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete CLAUDE.md: {e}"))?;
    }

    Ok(())
}

/// Individual item context info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub name: String,
    pub path: String,
    pub chars: usize,
    pub tokens: usize,
    pub scope: String, // "user" or "project"
}

/// MCP server complexity info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpComplexity {
    pub name: String,
    pub server_type: String, // "stdio", "http", "sse", "unknown"
    pub uses_wrapper: bool,  // bash, node, python, etc.
    pub env_var_count: usize,
    pub is_plugin: bool,
    pub tool_count: usize, // from inventory, 0 if unknown
    pub complexity_score: usize,
    pub status: String, // "connected", "disabled", "unknown"
}

/// Context usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub claude_md_chars: usize,
    pub claude_md_tokens: usize,
    pub skills_chars: usize,
    pub skills_tokens: usize,
    pub skills_count: usize,
    pub skills_items: Vec<ContextItem>,
    pub commands_chars: usize,
    pub commands_tokens: usize,
    pub commands_count: usize,
    pub commands_items: Vec<ContextItem>,
    pub agents_chars: usize,
    pub agents_tokens: usize,
    pub agents_count: usize,
    pub agents_items: Vec<ContextItem>,
    pub settings_chars: usize,
    pub settings_tokens: usize,
    pub user_settings_chars: usize,
    pub user_settings_tokens: usize,
    pub project_settings_chars: usize,
    pub project_settings_tokens: usize,
    pub project_local_settings_chars: usize,
    pub project_local_settings_tokens: usize,
    pub mcp_chars: usize,
    pub mcp_tokens: usize,
    pub mcp_servers: Vec<McpComplexity>,
    pub total_chars: usize,
    pub total_tokens: usize,
}

fn estimate_tokens(chars: usize) -> usize {
    // Claude uses ~4 characters per token on average for English text
    // Code tends to be slightly higher, ~3.5 chars per token
    // Using 3.5 as a conservative estimate for mixed content
    (chars as f64 / 3.5).ceil() as usize
}

fn read_file_size(path: &PathBuf) -> usize {
    std::fs::read_to_string(path).map(|s| s.len()).unwrap_or(0)
}

fn collect_directory_items(dir: &PathBuf, extension: &str, scope: &str) -> Vec<ContextItem> {
    let mut items = Vec::new();

    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == extension) {
                    let chars = read_file_size(&path);
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    items.push(ContextItem {
                        name,
                        path: path.display().to_string(),
                        chars,
                        tokens: estimate_tokens(chars),
                        scope: scope.to_string(),
                    });
                }
            }
        }
    }

    // Sort by tokens descending
    items.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    items
}

fn calculate_mcp_complexity(
    _name: &str,
    server_type: &str,
    command: Option<&str>,
    env_count: usize,
    is_plugin: bool,
    tool_count: usize,
) -> usize {
    let mut score = 1; // base

    // +3 for remote transport
    if server_type == "http" || server_type == "sse" {
        score += 3;
    }

    // +2 for wrapper scripts
    if let Some(cmd) = command {
        let cmd_lower = cmd.to_lowercase();
        if cmd_lower.contains("bash")
            || cmd_lower.contains("node")
            || cmd_lower.contains("python")
            || cmd_lower.contains("npx")
            || cmd_lower.contains("bunx")
            || cmd_lower.contains("uvx")
        {
            score += 2;
        }
    }

    // +3 for many env vars
    if env_count >= 6 {
        score += 3;
    } else if env_count >= 3 {
        score += 1;
    }

    // +2 for plugin-installed servers
    if is_plugin {
        score += 2;
    }

    // +1 per 5 tools
    score += tool_count / 5;

    score
}

fn parse_mcp_servers(home: &str, project_path: &PathBuf) -> Vec<McpComplexity> {
    let mut servers = Vec::new();

    // Read user MCP config from ~/.claude.json
    let claude_json_path = PathBuf::from(home).join(".claude.json");
    if let Ok(content) = std::fs::read_to_string(&claude_json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(mcp_servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
                for (name, config) in mcp_servers {
                    let server_type = if config.get("url").is_some() {
                        if config
                            .get("url")
                            .and_then(|u| u.as_str())
                            .is_some_and(|u| u.contains("sse"))
                        {
                            "sse"
                        } else {
                            "http"
                        }
                    } else if config.get("command").is_some() {
                        "stdio"
                    } else {
                        "unknown"
                    };

                    let command = config.get("command").and_then(|c| c.as_str());
                    let env_count = config
                        .get("env")
                        .and_then(|e| e.as_object())
                        .map_or(0, serde_json::Map::len);
                    let is_plugin = name.starts_with("plugin:");

                    let complexity = calculate_mcp_complexity(
                        name,
                        server_type,
                        command,
                        env_count,
                        is_plugin,
                        0, // tool_count unknown from static config
                    );

                    servers.push(McpComplexity {
                        name: name.clone(),
                        server_type: server_type.to_string(),
                        uses_wrapper: command.is_some_and(|c| {
                            let cl = c.to_lowercase();
                            cl.contains("bash")
                                || cl.contains("node")
                                || cl.contains("python")
                                || cl.contains("npx")
                                || cl.contains("bunx")
                                || cl.contains("uvx")
                        }),
                        env_var_count: env_count,
                        is_plugin,
                        tool_count: 0,
                        complexity_score: complexity,
                        status: "unknown".to_string(),
                    });
                }
            }
        }
    }

    // Read project .mcp.json
    let project_mcp = project_path.join(".mcp.json");
    if let Ok(content) = std::fs::read_to_string(&project_mcp) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(mcp_servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
                for (name, config) in mcp_servers {
                    // Skip if already added from user config
                    if servers.iter().any(|s| s.name == *name) {
                        continue;
                    }

                    let server_type = if config.get("url").is_some() {
                        "http"
                    } else if config.get("command").is_some() {
                        "stdio"
                    } else {
                        "unknown"
                    };

                    let command = config.get("command").and_then(|c| c.as_str());
                    let env_count = config
                        .get("env")
                        .and_then(|e| e.as_object())
                        .map_or(0, serde_json::Map::len);

                    let complexity =
                        calculate_mcp_complexity(name, server_type, command, env_count, false, 0);

                    servers.push(McpComplexity {
                        name: name.clone(),
                        server_type: server_type.to_string(),
                        uses_wrapper: command.is_some_and(|c| {
                            let cl = c.to_lowercase();
                            cl.contains("bash")
                                || cl.contains("node")
                                || cl.contains("python")
                                || cl.contains("npx")
                                || cl.contains("bunx")
                                || cl.contains("uvx")
                        }),
                        env_var_count: env_count,
                        is_plugin: false,
                        tool_count: 0,
                        complexity_score: complexity,
                        status: "unknown".to_string(),
                    });
                }
            }
        }
    }

    // Sort by complexity score descending
    servers.sort_by(|a, b| b.complexity_score.cmp(&a.complexity_score));
    servers
}

/// Calculate context usage stats for a project
#[tauri::command]
pub async fn get_context_stats(project_path: String) -> Result<ContextStats, String> {
    let project = PathBuf::from(&project_path);
    let home = dirs::home_dir()
        .ok_or("Cannot find home directory")?
        .display()
        .to_string();
    let user_claude = PathBuf::from(&home).join(".claude");
    let project_claude = project.join(".claude");

    // CLAUDE.md
    let claude_md_chars = read_file_size(&project.join("CLAUDE.md"));

    // Skills: user + project (with per-item breakdown)
    let mut skills_items = collect_directory_items(&user_claude.join("skills"), "md", "user");
    skills_items.extend(collect_directory_items(
        &project_claude.join("skills"),
        "md",
        "project",
    ));
    skills_items.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    let skills_chars: usize = skills_items.iter().map(|i| i.chars).sum();
    let skills_count = skills_items.len();

    // Commands: user + project (with per-item breakdown)
    let mut commands_items = collect_directory_items(&user_claude.join("commands"), "md", "user");
    commands_items.extend(collect_directory_items(
        &project_claude.join("commands"),
        "md",
        "project",
    ));
    commands_items.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    let commands_chars: usize = commands_items.iter().map(|i| i.chars).sum();
    let commands_count = commands_items.len();

    // Agents: user + project (with per-item breakdown)
    let mut agents_items = collect_directory_items(&user_claude.join("agents"), "md", "user");
    agents_items.extend(collect_directory_items(
        &project_claude.join("agents"),
        "md",
        "project",
    ));
    agents_items.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    let agents_chars: usize = agents_items.iter().map(|i| i.chars).sum();
    let agents_count = agents_items.len();

    // Settings files
    let user_settings_chars = read_file_size(&user_claude.join("settings.json"));
    let project_settings_chars = read_file_size(&project_claude.join("settings.json"));
    let project_local_settings_chars = read_file_size(&project_claude.join("settings.local.json"));
    let mcp_chars = read_file_size(&project.join(".mcp.json"));

    let settings_chars =
        user_settings_chars + project_settings_chars + project_local_settings_chars;

    // MCP servers with complexity scoring
    let mcp_servers = parse_mcp_servers(&home, &project);

    let total_chars =
        claude_md_chars + skills_chars + commands_chars + agents_chars + settings_chars + mcp_chars;

    Ok(ContextStats {
        claude_md_chars,
        claude_md_tokens: estimate_tokens(claude_md_chars),
        skills_chars,
        skills_tokens: estimate_tokens(skills_chars),
        skills_count,
        skills_items,
        commands_chars,
        commands_tokens: estimate_tokens(commands_chars),
        commands_count,
        commands_items,
        agents_chars,
        agents_tokens: estimate_tokens(agents_chars),
        agents_count,
        agents_items,
        settings_chars,
        settings_tokens: estimate_tokens(settings_chars),
        user_settings_chars,
        user_settings_tokens: estimate_tokens(user_settings_chars),
        project_settings_chars,
        project_settings_tokens: estimate_tokens(project_settings_chars),
        project_local_settings_chars,
        project_local_settings_tokens: estimate_tokens(project_local_settings_chars),
        mcp_chars,
        mcp_tokens: estimate_tokens(mcp_chars),
        mcp_servers,
        total_chars,
        total_tokens: estimate_tokens(total_chars),
    })
}
