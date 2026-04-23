//! Codex bundle export and compatibility reporting.

use crate::config::{HookConfig, HookDefinition};
use crate::profile::storage::{get_mcp_server_config, profile_dir, sanitize_tool_name};
use crate::profile::{McpServerOverlay, Profile, ToolRef, ToolType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use tars_scanner::parser::{parse_agent, parse_command, parse_skill};
use tars_scanner::runtime::{Runtime, RuntimeSupport};
use tars_scanner::types::Scope;

use super::convert::ExportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexArtifactKind {
    Skill,
    Command,
    Agent,
    Mcp,
    Hook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCompatibilityFinding {
    pub artifact_kind: CodexArtifactKind,
    pub name: String,
    pub runtime: Runtime,
    pub support: RuntimeSupport,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodexCompatibilityReport {
    #[serde(default)]
    pub findings: Vec<CodexCompatibilityFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexExportResult {
    pub plugin_root: PathBuf,
    pub marketplace_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub agents_dir: Option<PathBuf>,
    pub report: CodexCompatibilityReport,
}

#[derive(Debug, Clone)]
struct ExportSkill {
    name: String,
    content: String,
}

#[derive(Debug, Clone)]
struct ExportCommand {
    name: String,
    content: String,
}

#[derive(Debug, Clone)]
struct ExportAgent {
    name: String,
    content: String,
}

#[derive(Debug, Clone)]
struct ExportMcpServer {
    name: String,
    transport: String,
    command: Option<String>,
    args: Vec<String>,
    env: HashMap<String, String>,
    url: Option<String>,
}

/// Export a profile as a Codex-ready workspace rooted at `output_dir`.
///
/// This creates:
/// - `plugins/<plugin_name>/...` with a `.codex-plugin/plugin.json`
/// - `.agents/plugins/marketplace.json`
/// - `.codex/config.toml` for MCP server configuration
/// - `.codex/agents/*.toml` for custom agents
///
/// # Errors
/// Returns an error if any file cannot be written.
pub fn export_as_codex_bundle(
    profile: &Profile,
    output_dir: &Path,
    plugin_name: &str,
    version: &str,
    marketplace_name: &str,
) -> Result<CodexExportResult, ExportError> {
    let skills = collect_skills(profile)?;
    let commands = collect_commands(profile)?;
    let agents = collect_agents(profile)?;
    let mcp_servers = collect_mcp_servers(profile)?;
    let hook_configs = collect_hooks(profile, &skills, &agents)?;

    let mut report = build_codex_compatibility_report(&skills, &commands, &agents, &mcp_servers);
    append_hook_findings(&mut report, &hook_configs);

    let plugin_root = output_dir.join("plugins").join(plugin_name);
    write_codex_plugin(
        profile,
        &plugin_root,
        plugin_name,
        version,
        &skills,
        &commands,
        &mcp_servers,
    )?;

    let marketplace_path =
        write_codex_marketplace(output_dir, plugin_name, marketplace_name, profile)?;
    let config_path = write_codex_config(output_dir, &mcp_servers)?;
    let agents_dir = write_codex_agents(output_dir, &agents)?;

    Ok(CodexExportResult {
        plugin_root,
        marketplace_path,
        config_path,
        agents_dir,
        report,
    })
}

fn build_codex_compatibility_report(
    skills: &[ExportSkill],
    commands: &[ExportCommand],
    agents: &[ExportAgent],
    mcp_servers: &[ExportMcpServer],
) -> CodexCompatibilityReport {
    let mut findings = Vec::new();

    findings.extend(skills.iter().map(|skill| CodexCompatibilityFinding {
        artifact_kind: CodexArtifactKind::Skill,
        name: skill.name.clone(),
        runtime: Runtime::Codex,
        support: RuntimeSupport::Convertible,
        message:
            "Exported as a Codex skill with required metadata ensured in SKILL.md.".to_string(),
    }));

    findings.extend(commands.iter().map(|command| CodexCompatibilityFinding {
        artifact_kind: CodexArtifactKind::Command,
        name: command.name.clone(),
        runtime: Runtime::Codex,
        support: RuntimeSupport::Convertible,
        message:
            "Converted into a skill-style prompt because Codex does not currently support custom commands."
                .to_string(),
    }));

    findings.extend(agents.iter().map(|agent| CodexCompatibilityFinding {
        artifact_kind: CodexArtifactKind::Agent,
        name: agent.name.clone(),
        runtime: Runtime::Codex,
        support: RuntimeSupport::Convertible,
        message:
            "Converted into a Codex custom agent TOML with required name, description, and developer_instructions fields."
                .to_string(),
    }));

    findings.extend(mcp_servers.iter().map(|server| {
        CodexCompatibilityFinding {
            artifact_kind: CodexArtifactKind::Mcp,
            name: server.name.clone(),
            runtime: Runtime::Codex,
            support: RuntimeSupport::Convertible,
            message:
                "Converted into Codex config.toml [mcp_servers.*] tables and plugin .mcp.json."
                    .to_string(),
        }
    }));

    CodexCompatibilityReport { findings }
}

fn append_hook_findings(report: &mut CodexCompatibilityReport, hook_configs: &[HookConfig]) {
    for (index, hook) in hook_configs.iter().enumerate() {
        report.findings.push(CodexCompatibilityFinding {
            artifact_kind: CodexArtifactKind::Hook,
            name: format!("{}#{index}", hook.trigger.as_str()),
            runtime: Runtime::Codex,
            support: RuntimeSupport::Partial,
            message:
                "Not exported directly because Codex hooks are narrower and still experimental compared with Claude hooks."
                    .to_string(),
        });
    }
}

fn write_codex_plugin(
    profile: &Profile,
    plugin_root: &Path,
    plugin_name: &str,
    version: &str,
    skills: &[ExportSkill],
    commands: &[ExportCommand],
    mcp_servers: &[ExportMcpServer],
) -> Result<(), ExportError> {
    fs::create_dir_all(plugin_root.join(".codex-plugin"))?;

    if !skills.is_empty() || !commands.is_empty() {
        fs::create_dir_all(plugin_root.join("skills"))?;
    }

    for skill in skills {
        let skill_dir = plugin_root
            .join("skills")
            .join(sanitize_tool_name(&skill.name)?);
        fs::create_dir_all(&skill_dir)?;
        let description = skill_description_from_content(&skill.name, &skill.content);
        let output = ensure_skill_frontmatter(&skill.content, &skill.name, &description);
        fs::write(skill_dir.join("SKILL.md"), output)?;
    }

    for command in commands {
        let converted_name = codex_command_skill_name(&command.name);
        let skill_dir = plugin_root
            .join("skills")
            .join(sanitize_tool_name(&converted_name)?);
        fs::create_dir_all(&skill_dir)?;
        let output = convert_command_to_skill(command)?;
        fs::write(skill_dir.join("SKILL.md"), output)?;
    }

    if !mcp_servers.is_empty() {
        let mcp_json_path = plugin_root.join(".mcp.json");
        fs::write(&mcp_json_path, render_codex_plugin_mcp_json(mcp_servers)?)?;
    }

    let mut manifest = json!({
        "name": plugin_name,
        "version": version,
        "description": profile.description.clone().unwrap_or_default(),
        "interface": {
            "displayName": profile.name,
            "shortDescription": profile.description.clone().unwrap_or_default(),
            "category": "Developer Tools"
        }
    });

    if !skills.is_empty() || !commands.is_empty() {
        manifest["skills"] = Value::String("./skills/".to_string());
    }

    if !mcp_servers.is_empty() {
        manifest["mcpServers"] = Value::String("./.mcp.json".to_string());
    }

    let manifest_path = plugin_root.join(".codex-plugin").join("plugin.json");
    fs::write(manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    Ok(())
}

fn write_codex_marketplace(
    output_dir: &Path,
    plugin_name: &str,
    marketplace_name: &str,
    profile: &Profile,
) -> Result<PathBuf, ExportError> {
    let marketplace_dir = output_dir.join(".agents").join("plugins");
    fs::create_dir_all(&marketplace_dir)?;

    let marketplace = json!({
        "name": marketplace_name,
        "interface": {
            "displayName": profile.name
        },
        "plugins": [
            {
                "name": plugin_name,
                "source": {
                    "source": "local",
                    "path": format!("./plugins/{plugin_name}")
                },
                "policy": {
                    "installation": "AVAILABLE",
                    "authentication": "ON_INSTALL"
                },
                "category": "Developer Tools"
            }
        ]
    });

    let marketplace_path = marketplace_dir.join("marketplace.json");
    fs::write(
        &marketplace_path,
        serde_json::to_string_pretty(&marketplace)?,
    )?;
    Ok(marketplace_path)
}

fn write_codex_config(
    output_dir: &Path,
    mcp_servers: &[ExportMcpServer],
) -> Result<Option<PathBuf>, ExportError> {
    if mcp_servers.is_empty() {
        return Ok(None);
    }

    let codex_dir = output_dir.join(".codex");
    fs::create_dir_all(&codex_dir)?;
    let config_path = codex_dir.join("config.toml");
    fs::write(&config_path, render_codex_mcp_toml(mcp_servers))?;
    Ok(Some(config_path))
}

fn write_codex_agents(
    output_dir: &Path,
    agents: &[ExportAgent],
) -> Result<Option<PathBuf>, ExportError> {
    if agents.is_empty() {
        return Ok(None);
    }

    let agents_dir = output_dir.join(".codex").join("agents");
    fs::create_dir_all(&agents_dir)?;

    for agent in agents {
        let file_path = agents_dir.join(format!("{}.toml", sanitize_tool_name(&agent.name)?));
        fs::write(file_path, convert_agent_to_toml(agent)?)?;
    }

    Ok(Some(agents_dir))
}

fn collect_skills(profile: &Profile) -> Result<Vec<ExportSkill>, ExportError> {
    let mut skills = BTreeMap::new();

    for skill in &profile.user_overlays.skills {
        skills.insert(
            skill.name.clone(),
            ExportSkill {
                name: skill.name.clone(),
                content: skill.content.clone(),
            },
        );
    }

    for skill in &profile.repo_overlays.skills {
        skills.insert(
            skill.name.clone(),
            ExportSkill {
                name: skill.name.clone(),
                content: skill.content.clone(),
            },
        );
    }

    let profile_storage = profile_dir(profile.id)?;
    for tool in profile
        .tool_refs
        .iter()
        .filter(|tool| tool.tool_type == ToolType::Skill)
    {
        if let Some(skill) = read_stored_skill(&profile_storage, tool)? {
            skills.insert(skill.name.clone(), skill);
        }
    }

    Ok(skills.into_values().collect())
}

fn collect_commands(profile: &Profile) -> Result<Vec<ExportCommand>, ExportError> {
    let mut commands = BTreeMap::new();

    for command in &profile.user_overlays.commands {
        commands.insert(
            command.name.clone(),
            ExportCommand {
                name: command.name.clone(),
                content: command.content.clone(),
            },
        );
    }

    for command in &profile.repo_overlays.commands {
        commands.insert(
            command.name.clone(),
            ExportCommand {
                name: command.name.clone(),
                content: command.content.clone(),
            },
        );
    }

    let profile_storage = profile_dir(profile.id)?;
    for tool in profile
        .tool_refs
        .iter()
        .filter(|tool| tool.tool_type == ToolType::Hook)
    {
        if let Some(command) = read_stored_command(&profile_storage, tool)? {
            commands.insert(command.name.clone(), command);
        }
    }

    Ok(commands.into_values().collect())
}

fn collect_agents(profile: &Profile) -> Result<Vec<ExportAgent>, ExportError> {
    let mut agents = BTreeMap::new();

    for agent in &profile.repo_overlays.agents {
        agents.insert(
            agent.name.clone(),
            ExportAgent {
                name: agent.name.clone(),
                content: agent.content.clone(),
            },
        );
    }

    let profile_storage = profile_dir(profile.id)?;
    for tool in profile
        .tool_refs
        .iter()
        .filter(|tool| tool.tool_type == ToolType::Agent)
    {
        if let Some(agent) = read_stored_agent(&profile_storage, tool)? {
            agents.insert(agent.name.clone(), agent);
        }
    }

    Ok(agents.into_values().collect())
}

fn collect_mcp_servers(profile: &Profile) -> Result<Vec<ExportMcpServer>, ExportError> {
    let mut servers = BTreeMap::new();

    for server in &profile.repo_overlays.mcp_servers {
        servers.insert(server.name.clone(), convert_overlay_mcp(server));
    }

    for tool in profile
        .tool_refs
        .iter()
        .filter(|tool| tool.tool_type == ToolType::Mcp)
    {
        let value = get_mcp_server_config(profile.id, &tool.name)?;
        servers.insert(tool.name.clone(), parse_mcp_value(&tool.name, &value));
    }

    Ok(servers.into_values().collect())
}

fn collect_hooks(
    profile: &Profile,
    skills: &[ExportSkill],
    agents: &[ExportAgent],
) -> Result<Vec<HookConfig>, ExportError> {
    let mut hooks = Vec::new();

    let profile_storage = profile_dir(profile.id)?;
    let hooks_path = profile_storage.join("hooks.json");
    if hooks_path.exists() {
        let content = fs::read_to_string(hooks_path)?;
        let mut stored_hooks: Vec<HookConfig> = serde_json::from_str(&content)?;
        hooks.append(&mut stored_hooks);
    }

    hooks.extend(collect_skill_embedded_hooks(skills));
    hooks.extend(collect_agent_embedded_hooks(agents));

    Ok(hooks)
}

fn collect_skill_embedded_hooks(skills: &[ExportSkill]) -> Vec<HookConfig> {
    let mut hooks = Vec::new();
    for skill in skills {
        if let Ok(parsed) = parse_skill(Path::new("SKILL.md"), &skill.content, Scope::Project) {
            for (trigger, definitions) in parsed.hooks {
                if let Ok(trigger) = trigger.parse() {
                    hooks.extend(definitions.into_iter().map(|definition| HookConfig {
                        trigger,
                        matcher: None,
                        definition: convert_hook_definition(definition),
                    }));
                }
            }
        }
    }
    hooks
}

fn collect_agent_embedded_hooks(agents: &[ExportAgent]) -> Vec<HookConfig> {
    let mut hooks = Vec::new();
    for agent in agents {
        if let Ok(parsed) = parse_agent(Path::new("agent.md"), &agent.content, Scope::Project) {
            for (trigger, definitions) in parsed.hooks {
                if let Ok(trigger) = trigger.parse() {
                    hooks.extend(definitions.into_iter().map(|definition| HookConfig {
                        trigger,
                        matcher: None,
                        definition: convert_hook_definition(definition),
                    }));
                }
            }
        }
    }
    hooks
}

fn read_stored_skill(
    profile_storage: &Path,
    tool: &ToolRef,
) -> Result<Option<ExportSkill>, ExportError> {
    let path = profile_storage
        .join("skills")
        .join(sanitize_tool_name(&tool.name)?)
        .join("SKILL.md");
    if !path.exists() {
        return Ok(None);
    }

    Ok(Some(ExportSkill {
        name: tool.name.clone(),
        content: fs::read_to_string(path)?,
    }))
}

fn read_stored_command(
    profile_storage: &Path,
    tool: &ToolRef,
) -> Result<Option<ExportCommand>, ExportError> {
    let path = profile_storage
        .join("commands")
        .join(format!("{}.md", sanitize_tool_name(&tool.name)?));
    if !path.exists() {
        return Ok(None);
    }

    Ok(Some(ExportCommand {
        name: tool.name.clone(),
        content: fs::read_to_string(path)?,
    }))
}

fn read_stored_agent(
    profile_storage: &Path,
    tool: &ToolRef,
) -> Result<Option<ExportAgent>, ExportError> {
    let path = profile_storage
        .join("agents")
        .join(format!("{}.md", sanitize_tool_name(&tool.name)?));
    if !path.exists() {
        return Ok(None);
    }

    Ok(Some(ExportAgent {
        name: tool.name.clone(),
        content: fs::read_to_string(path)?,
    }))
}

fn skill_description_from_content(name: &str, content: &str) -> String {
    parse_skill(Path::new("SKILL.md"), content, Scope::Project).map_or_else(
        |_| format!("Imported from the TARS bundle skill `{name}`."),
        |skill| skill.description,
    )
}

fn ensure_skill_frontmatter(content: &str, name: &str, description: &str) -> String {
    if let Some((header, body)) = split_frontmatter(content) {
        let has_name = header
            .lines()
            .any(|line| line.trim_start().starts_with("name:"));
        let has_description = header
            .lines()
            .any(|line| line.trim_start().starts_with("description:"));

        let mut lines = Vec::new();
        if !has_name {
            lines.push(format!("name: {}", yaml_string(name)));
        }
        if !has_description {
            lines.push(format!("description: {}", yaml_string(description)));
        }
        if !header.trim().is_empty() {
            lines.push(header.trim_end().to_string());
        }

        return format!("---\n{}\n---\n{}", lines.join("\n"), body);
    }

    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}",
        yaml_string(name),
        yaml_string(description),
        content.trim_start()
    )
}

fn convert_command_to_skill(command: &ExportCommand) -> Result<String, ExportError> {
    let parsed = parse_command(
        Path::new(&format!("{}.md", command.name)),
        &command.content,
        Scope::Project,
    )
    .map_err(|err| {
        ExportError::Storage(format!("Failed to parse command '{}': {err}", command.name))
    })?;

    let description = parsed.description.unwrap_or_else(|| {
        format!(
            "Converted from the Claude command `/{}.` Use this skill when you want the same prompt behavior.",
            command.name
        )
    });

    let mut body = String::new();
    let _ = write!(
        body,
        "This skill was converted from the Claude command `/{}`.\n\n",
        command.name
    );
    body.push_str("Follow the original command instructions below:\n\n");
    body.push_str(parsed.body.trim());
    body.push('\n');

    Ok(ensure_skill_frontmatter(
        &body,
        &codex_command_skill_name(&command.name),
        &description,
    ))
}

fn codex_command_skill_name(name: &str) -> String {
    format!("command-{name}")
}

fn convert_agent_to_toml(agent: &ExportAgent) -> Result<String, ExportError> {
    let path = PathBuf::from(format!("{}.md", agent.name));
    let parsed = parse_agent(&path, &agent.content, Scope::Project).map_err(|err| {
        ExportError::Storage(format!("Failed to parse agent '{}': {err}", agent.name))
    })?;
    let developer_instructions = agent_body(&agent.content);

    let mut lines = vec![
        format!("name = {}", toml_string(&parsed.name)),
        format!("description = {}", toml_string(&parsed.description)),
        format!(
            "developer_instructions = {}",
            toml_string(&developer_instructions)
        ),
    ];

    if let Some(model) = parsed.model {
        lines.push(format!("model = {}", toml_string(&model)));
    }

    Ok(format!("{}\n", lines.join("\n")))
}

fn agent_body(content: &str) -> String {
    split_frontmatter(content).map_or_else(
        || content.trim().to_string(),
        |(_, body)| body.trim().to_string(),
    )
}

fn render_codex_plugin_mcp_json(mcp_servers: &[ExportMcpServer]) -> Result<String, ExportError> {
    let mut root = serde_json::Map::new();

    for server in mcp_servers {
        let mut value = serde_json::Map::new();
        value.insert("type".to_string(), Value::String(server.transport.clone()));
        if let Some(command) = &server.command {
            value.insert("command".to_string(), Value::String(command.clone()));
        }
        if !server.args.is_empty() {
            value.insert(
                "args".to_string(),
                Value::Array(server.args.iter().cloned().map(Value::String).collect()),
            );
        }
        if !server.env.is_empty() {
            value.insert(
                "env".to_string(),
                Value::Object(
                    server
                        .env
                        .iter()
                        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                        .collect(),
                ),
            );
        }
        if let Some(url) = &server.url {
            value.insert("url".to_string(), Value::String(url.clone()));
        }
        root.insert(server.name.clone(), Value::Object(value));
    }

    Ok(serde_json::to_string_pretty(&Value::Object(root))?)
}

fn render_codex_mcp_toml(mcp_servers: &[ExportMcpServer]) -> String {
    let mut lines = Vec::new();

    for server in mcp_servers {
        lines.push(format!("[mcp_servers.{}]", toml_table_key(&server.name)));

        if let Some(command) = &server.command {
            lines.push(format!("command = {}", toml_string(command)));
        }

        if let Some(url) = &server.url {
            lines.push(format!("url = {}", toml_string(url)));
        }

        if !server.args.is_empty() {
            lines.push(format!("args = {}", toml_array(&server.args)));
        }

        if !server.env.is_empty() {
            lines.push(format!(
                "[mcp_servers.{}.env]",
                toml_table_key(&server.name)
            ));

            let mut env_entries = server.env.iter().collect::<Vec<_>>();
            env_entries.sort_by(|a, b| a.0.cmp(b.0));
            for (key, value) in env_entries {
                lines.push(format!("{key} = {}", toml_string(value)));
            }
        }

        lines.push(String::new());
    }

    lines.join("\n")
}

fn toml_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| toml_string(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn toml_table_key(value: &str) -> String {
    toml_string(value)
}

fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let mut parts = content.split_inclusive('\n');
    let first = parts.next()?;
    let trimmed = first.trim_end_matches(['\n', '\r']);
    if trimmed != "---" {
        return None;
    }

    let mut header = String::new();
    let mut offset = first.len();

    for part in parts {
        offset += part.len();
        if part.trim_end_matches(['\n', '\r']) == "---" {
            return Some((header, content[offset..].to_string()));
        }
        header.push_str(part);
    }

    None
}

fn convert_overlay_mcp(overlay: &McpServerOverlay) -> ExportMcpServer {
    ExportMcpServer {
        name: overlay.name.clone(),
        transport: overlay.transport.clone(),
        command: overlay.command.clone(),
        args: overlay.args.clone(),
        env: overlay.env.clone(),
        url: overlay.url.clone(),
    }
}

fn parse_mcp_value(name: &str, value: &Value) -> ExportMcpServer {
    let transport = value.get("type").and_then(Value::as_str).map_or_else(
        || {
            if value.get("url").is_some() {
                "http".to_string()
            } else {
                "stdio".to_string()
            }
        },
        str::to_string,
    );

    ExportMcpServer {
        name: name.to_string(),
        transport,
        command: value
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_string),
        args: value
            .get("args")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        env: value
            .get("env")
            .and_then(Value::as_object)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        url: value.get("url").and_then(Value::as_str).map(str::to_string),
    }
}

fn convert_hook_definition(definition: tars_scanner::artifacts::HookDefinition) -> HookDefinition {
    match definition {
        tars_scanner::artifacts::HookDefinition::Command { command } => {
            HookDefinition::Command { command }
        }
        tars_scanner::artifacts::HookDefinition::Prompt { prompt } => {
            HookDefinition::Prompt { prompt }
        }
        tars_scanner::artifacts::HookDefinition::Agent { agent } => HookDefinition::Agent { agent },
    }
}
