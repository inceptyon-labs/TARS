//! Plugin management Tauri commands
//!
//! Commands for managing Claude Code plugins via the CLI.

use super::utils::find_claude_binary;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tars_core::storage::{PluginSubscription, PluginSubscriptionInput, PluginSubscriptionStore};
use tars_scanner::parser::parse_command;
use tars_scanner::plugins::{InstalledPlugin, PluginInventory};
use tars_scanner::types::Scope as ScanScope;
use tars_scanner::CacheCleanupReport;
use tauri::State;

const TARS_MANAGED_CLAUDE_MARKETPLACE: &str = "tars-managed";
const TARS_MANAGED_CLAUDE_MARKETPLACE_DESCRIPTION: &str =
    "Direct plugin subscriptions managed by TARS";

/// Validate a plugin source string (marketplace URL or plugin@marketplace format)
/// Prevents command injection by restricting to safe characters
fn validate_plugin_source(source: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("Source cannot be empty".to_string());
    }
    if source.len() > 500 {
        return Err("Source string too long".to_string());
    }
    // Allow alphanumeric, hyphens, underscores, dots, @, /, :, and common URL chars
    // Reject shell metacharacters and control characters
    let forbidden_chars = [
        '`', '$', '(', ')', '{', '}', '[', ']', '|', ';', '&', '<', '>', '\\', '\n', '\r', '\0',
        '\'', '"', '!', '*', '?',
    ];
    for ch in forbidden_chars {
        if source.contains(ch) {
            return Err(format!("Source contains forbidden character: {ch}"));
        }
    }
    Ok(())
}

/// Validate a plugin name (alphanumeric, hyphens, underscores, dots)
fn validate_plugin_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Plugin name cannot be empty".to_string());
    }
    if name.len() > 200 {
        return Err("Plugin name too long".to_string());
    }
    // Plugin names should be simple identifiers
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("Plugin name contains invalid characters".to_string());
    }
    if name.starts_with('.') || name.starts_with('-') {
        return Err("Plugin name cannot start with dot or hyphen".to_string());
    }
    Ok(())
}

/// Validate a scope string
fn validate_scope(scope: &str) -> Result<(), String> {
    match scope {
        "user" | "project" | "local" => Ok(()),
        _ => Err(format!(
            "Invalid scope: {scope}. Must be user, project, or local"
        )),
    }
}

fn validate_runtime_target(target: &str) -> Result<(), String> {
    match target {
        "claude-code" | "codex" => Ok(()),
        _ => Err(format!(
            "Invalid runtime target: {target}. Must be claude-code or codex"
        )),
    }
}

fn validate_source_kind(kind: &str) -> Result<(), String> {
    match kind {
        "direct" | "marketplace" => Ok(()),
        _ => Err(format!(
            "Invalid plugin source kind: {kind}. Must be direct or marketplace"
        )),
    }
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn looks_like_github_repo(value: &str) -> bool {
    let Some((owner, repo)) = value.split_once('/') else {
        return false;
    };

    !owner.is_empty()
        && !repo.is_empty()
        && !repo.contains('/')
        && owner
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        && repo
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn expand_tilde(path: &str) -> Option<PathBuf> {
    if path == "~" {
        return dirs::home_dir();
    }

    path.strip_prefix("~/")
        .and_then(|rest| dirs::home_dir().map(|home| home.join(rest)))
}

fn resolve_local_plugin_source(source: &str) -> Option<PathBuf> {
    if let Some(expanded) = expand_tilde(source) {
        return Some(expanded);
    }

    let candidate = PathBuf::from(source);
    if candidate.is_absolute()
        || source.starts_with('.')
        || source.starts_with("..")
        || candidate.exists()
    {
        Some(candidate)
    } else {
        None
    }
}

fn sanitize_codex_plugin_name(raw: &str) -> String {
    let sanitized = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches(|ch| ch == '-' || ch == '.')
        .to_string();

    if sanitized.is_empty() {
        "plugin".to_string()
    } else {
        sanitized
    }
}

fn plugin_name_from_source(source: &str) -> String {
    if let Some(path) = resolve_local_plugin_source(source) {
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            return sanitize_codex_plugin_name(name);
        }
    }

    let normalized = if is_http_url(source) {
        source.trim_end_matches('/').trim_end_matches(".git")
    } else if looks_like_github_repo(source) {
        source
    } else {
        source.split('@').next().unwrap_or(source)
    };

    let last_segment = normalized
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(normalized);

    sanitize_codex_plugin_name(last_segment)
}

fn marketplace_name_from_source(source: &str) -> String {
    let normalized = if is_http_url(source) {
        source
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .to_string()
    } else {
        source.to_string()
    };

    let tail = normalized
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(&normalized);

    sanitize_codex_plugin_name(tail)
}

fn github_repo_from_source(source: &str) -> Option<String> {
    if looks_like_github_repo(source) {
        return Some(source.to_string());
    }

    let normalized = source.trim_end_matches('/').trim_end_matches(".git");
    let rest = normalized
        .strip_prefix("https://github.com/")
        .or_else(|| normalized.strip_prefix("http://github.com/"))?;
    let mut parts = rest.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;

    if parts.next().is_some() {
        return None;
    }

    let repo_path = format!("{owner}/{repo}");
    looks_like_github_repo(&repo_path).then_some(repo_path)
}

fn claude_managed_marketplace_dir(home_dir: &Path) -> PathBuf {
    home_dir
        .join(".claude")
        .join("plugins")
        .join("marketplaces")
        .join(TARS_MANAGED_CLAUDE_MARKETPLACE)
}

fn claude_managed_marketplace_plugins_dir(home_dir: &Path) -> PathBuf {
    claude_managed_marketplace_dir(home_dir).join("plugins")
}

fn read_claude_plugin_metadata(plugin_root: &Path) -> (Option<String>, Option<String>) {
    let manifest_paths = [
        plugin_root.join(".claude-plugin").join("plugin.json"),
        plugin_root.join("plugin.json"),
    ];

    for manifest_path in manifest_paths {
        let Ok(content) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(&content) else {
            continue;
        };

        let description = json
            .get("description")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let version = json
            .get("version")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        return (description, version);
    }

    (None, None)
}

fn copy_directory_recursive(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Err(format!(
            "Local plugin path does not exist: {}",
            source.display()
        ));
    }
    if !source.is_dir() {
        return Err(format!(
            "Local plugin source must be a directory: {}",
            source.display()
        ));
    }

    if target.exists() {
        fs::remove_dir_all(target)
            .map_err(|e| format!("Failed to replace managed plugin copy: {e}"))?;
    }

    fs::create_dir_all(target)
        .map_err(|e| format!("Failed to create managed plugin directory: {e}"))?;

    for entry in
        fs::read_dir(source).map_err(|e| format!("Failed to read plugin directory: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read plugin directory entry: {e}"))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect plugin file type: {e}"))?;

        if file_type.is_dir() {
            copy_directory_recursive(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path)
                .map_err(|e| format!("Failed to copy plugin file: {e}"))?;
        } else {
            return Err(format!(
                "Unsupported file type in local plugin source: {}",
                source_path.display()
            ));
        }
    }

    Ok(())
}

fn codex_bridge_metadata_dir(home: &Path) -> PathBuf {
    home.join(".agents")
        .join("skills")
        .join(".tars-claude-bridges")
}

fn codex_bridge_index_path(home: &Path) -> PathBuf {
    codex_bridge_metadata_dir(home).join("bridges.json")
}

fn installed_plugin_scope_name(scope: &ScanScope) -> &'static str {
    match scope {
        ScanScope::User => "user",
        ScanScope::Project => "project",
        ScanScope::Local => "local",
        ScanScope::Managed => "managed",
        ScanScope::Plugin(_) => "plugin",
    }
}

fn codex_bridge_key(
    plugin_name: &str,
    marketplace: Option<&str>,
    scope: &str,
    project_path: Option<&str>,
) -> String {
    format!(
        "{}|{}|{}|{}",
        plugin_name,
        marketplace.unwrap_or("-"),
        scope,
        project_path.unwrap_or("-")
    )
}

fn codex_bridge_dir_prefix(key: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("tars-claude-{}", &digest[..12])
}

fn load_codex_plugin_bridges() -> Result<Vec<CodexPluginBridge>, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let index_path = codex_bridge_index_path(&home);

    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read Codex bridge index: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse Codex bridge index: {e}"))
}

fn save_codex_plugin_bridges(bridges: &[CodexPluginBridge]) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let metadata_dir = codex_bridge_metadata_dir(&home);
    fs::create_dir_all(&metadata_dir)
        .map_err(|e| format!("Failed to create Codex bridge metadata directory: {e}"))?;
    let index_path = codex_bridge_index_path(&home);
    let content = serde_json::to_string_pretty(bridges)
        .map_err(|e| format!("Failed to serialize Codex bridge index: {e}"))?;
    fs::write(&index_path, content)
        .map_err(|e| format!("Failed to write Codex bridge index: {e}"))?;
    Ok(())
}

fn remove_codex_skill_dirs(dir_names: &[String]) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let skills_root = home.join(".agents").join("skills");

    for dir_name in dir_names {
        let path = skills_root.join(dir_name);
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|e| {
                format!(
                    "Failed to remove managed Codex skill directory {}: {e}",
                    path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn resolve_plugin_relative_path(plugin_root: &Path, relative_or_absolute: &Path) -> PathBuf {
    if relative_or_absolute.is_absolute() {
        return relative_or_absolute.to_path_buf();
    }

    let relative = relative_or_absolute
        .to_string_lossy()
        .trim_start_matches("./")
        .to_string();
    plugin_root.join(relative)
}

fn iter_skill_source_dirs(skills_root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    if !skills_root.exists() || !skills_root.is_dir() {
        return Ok(Vec::new());
    }

    if skills_root.join("SKILL.md").exists() {
        let dir_name = skills_root
            .file_name()
            .and_then(|name| name.to_str())
            .map_or_else(|| "skill".to_string(), ToString::to_string);
        return Ok(vec![(dir_name, skills_root.to_path_buf())]);
    }

    let mut skills = Vec::new();
    for entry in fs::read_dir(skills_root)
        .map_err(|e| format!("Failed to read plugin skills directory: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read plugin skill entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").exists() {
            let name = entry.file_name().to_string_lossy().to_string();
            skills.push((name, path));
        }
    }

    Ok(skills)
}

fn collect_plugin_command_paths(plugin: &InstalledPlugin) -> Result<Vec<PathBuf>, String> {
    let mut command_paths = plugin
        .manifest
        .commands
        .iter()
        .map(|path| resolve_plugin_relative_path(&plugin.path, path))
        .collect::<Vec<_>>();

    if command_paths.is_empty() {
        let commands_dir = plugin.path.join("commands");
        if commands_dir.exists() && commands_dir.is_dir() {
            for entry in fs::read_dir(&commands_dir)
                .map_err(|e| format!("Failed to read plugin commands directory: {e}"))?
            {
                let entry =
                    entry.map_err(|e| format!("Failed to read plugin command entry: {e}"))?;
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                    command_paths.push(path);
                }
            }
        }
    }

    command_paths.sort();
    Ok(command_paths)
}

fn plugin_skills_root(plugin: &InstalledPlugin) -> PathBuf {
    plugin.manifest.skills.as_ref().map_or_else(
        || plugin.path.join("skills"),
        |path| resolve_plugin_relative_path(&plugin.path, path),
    )
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
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

fn normalize_skill_frontmatter_header(header: &str) -> String {
    let string_keys = [
        "name",
        "description",
        "version",
        "argument-hint",
        "license",
        "model",
        "context",
        "agent",
    ];

    header
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return line.to_string();
            }

            let Some((key, value)) = line.split_once(':') else {
                return line.to_string();
            };
            let normalized_key = key.trim();
            if !string_keys.contains(&normalized_key) {
                return line.to_string();
            }

            let leading = &line[..line.len() - line.trim_start().len()];
            let value = value.trim();
            if value.is_empty()
                || value.starts_with('"')
                || value.starts_with('\'')
                || value.starts_with('[')
                || value.starts_with('{')
                || value == "true"
                || value == "false"
            {
                return line.to_string();
            }

            format!("{leading}{normalized_key}: {}", yaml_string(value))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn ensure_skill_frontmatter(content: &str, name: &str, description: &str) -> String {
    if let Some((header, body)) = split_frontmatter(content) {
        let header = normalize_skill_frontmatter_header(&header);
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

fn codex_command_skill_name(name: &str) -> String {
    format!("command-{name}")
}

fn convert_command_to_codex_skill(command_path: &Path) -> Result<(String, String), String> {
    let content = fs::read_to_string(command_path).map_err(|e| {
        format!(
            "Failed to read Claude command {}: {e}",
            command_path.display()
        )
    })?;
    let stem = command_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            format!(
                "Invalid Claude command filename: {}",
                command_path.display()
            )
        })?;

    let parsed = parse_command(command_path, &content, ScanScope::Project)
        .map_err(|e| format!("Failed to parse Claude command `{stem}`: {e}"))?;
    let description = parsed.description.unwrap_or_else(|| {
        format!("Converted from the Claude command `/{stem}` so it can be used from Codex.")
    });

    let body = format!(
        "This skill was converted from the Claude command `/{stem}`.\n\nFollow the original command instructions below:\n\n{}\n",
        parsed.body.trim()
    );

    Ok((
        codex_command_skill_name(stem),
        ensure_skill_frontmatter(&body, &codex_command_skill_name(stem), &description),
    ))
}

fn find_installed_plugin<'a>(
    inventory: &'a PluginInventory,
    plugin_name: &str,
    marketplace: Option<&str>,
    scope: &str,
    project_path: Option<&str>,
) -> Option<&'a InstalledPlugin> {
    inventory.installed.iter().find(|plugin| {
        plugin.id == plugin_name
            && plugin.marketplace.as_deref() == marketplace
            && installed_plugin_scope_name(&plugin.scope) == scope
            && plugin.project_path.as_deref() == project_path
    })
}

fn upsert_codex_plugin_bridge(bridge: CodexPluginBridge) -> Result<(), String> {
    let mut bridges = load_codex_plugin_bridges()?;
    bridges.retain(|existing| existing.key != bridge.key);
    bridges.push(bridge);
    bridges.sort_by(|a, b| a.key.cmp(&b.key));
    save_codex_plugin_bridges(&bridges)
}

fn sync_installed_plugin_to_codex(plugin: &InstalledPlugin) -> Result<CodexPluginBridge, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let skills_root = home.join(".agents").join("skills");
    fs::create_dir_all(&skills_root)
        .map_err(|e| format!("Failed to create Codex skills directory: {e}"))?;

    let scope = installed_plugin_scope_name(&plugin.scope).to_string();
    let bridge_key = codex_bridge_key(
        &plugin.id,
        plugin.marketplace.as_deref(),
        &scope,
        plugin.project_path.as_deref(),
    );
    let prefix = codex_bridge_dir_prefix(&bridge_key);
    let existing_bridge = load_codex_plugin_bridges()?
        .into_iter()
        .find(|bridge| bridge.key == bridge_key);

    if let Some(existing) = existing_bridge.as_ref() {
        remove_codex_skill_dirs(&existing.codex_skill_dirs)?;
    }

    let mut created_dirs = Vec::new();
    let mut seen_target_names = HashSet::new();

    for (skill_name, source_dir) in iter_skill_source_dirs(&plugin_skills_root(plugin))? {
        let target_name = format!("{prefix}--{}", sanitize_codex_plugin_name(&skill_name));
        if !seen_target_names.insert(target_name.clone()) {
            continue;
        }
        let target_dir = skills_root.join(&target_name);
        copy_directory_recursive(&source_dir, &target_dir)?;
        let skill_path = target_dir.join("SKILL.md");
        if skill_path.exists() {
            let content = fs::read_to_string(&skill_path).map_err(|e| {
                format!(
                    "Failed to read copied Codex skill {}: {e}",
                    skill_path.display()
                )
            })?;
            let normalized = ensure_skill_frontmatter(&content, &skill_name, &skill_name);
            fs::write(&skill_path, normalized).map_err(|e| {
                format!(
                    "Failed to normalize copied Codex skill {}: {e}",
                    skill_path.display()
                )
            })?;
        }
        created_dirs.push(target_name);
    }

    for command_path in collect_plugin_command_paths(plugin)? {
        let (skill_name, output) = convert_command_to_codex_skill(&command_path)?;
        let target_name = format!("{prefix}--{}", sanitize_codex_plugin_name(&skill_name));
        if !seen_target_names.insert(target_name.clone()) {
            continue;
        }
        let target_dir = skills_root.join(&target_name);
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create Codex command skill directory: {e}"))?;
        fs::write(target_dir.join("SKILL.md"), output)
            .map_err(|e| format!("Failed to write converted Codex skill: {e}"))?;
        created_dirs.push(target_name);
    }

    if created_dirs.is_empty() {
        return Err(format!(
            "No portable skills or commands were found in Claude plugin `{}`.",
            plugin.id
        ));
    }

    let bridge = CodexPluginBridge {
        key: bridge_key,
        plugin_name: plugin.id.clone(),
        marketplace: plugin.marketplace.clone(),
        scope,
        project_path: plugin.project_path.clone(),
        codex_skill_dirs: created_dirs.clone(),
        skill_count: created_dirs.len(),
        updated_at: Utc::now().to_rfc3339(),
    };

    upsert_codex_plugin_bridge(bridge.clone())?;
    Ok(bridge)
}

fn build_claude_managed_marketplace_entry(
    subscription: &PluginSubscription,
    plugins_dir: &Path,
) -> Result<Value, String> {
    let mut entry = serde_json::Map::new();
    entry.insert(
        "name".to_string(),
        Value::String(subscription.plugin_name.clone()),
    );

    if let Some(path) = resolve_local_plugin_source(&subscription.source) {
        let target_dir = plugins_dir.join(&subscription.plugin_name);
        copy_directory_recursive(&path, &target_dir)?;
        let (description, version) = read_claude_plugin_metadata(&target_dir);
        if let Some(description) = description {
            entry.insert("description".to_string(), Value::String(description));
        }
        if let Some(version) = version {
            entry.insert("version".to_string(), Value::String(version));
        }
        entry.insert(
            "source".to_string(),
            Value::String(format!("./plugins/{}", subscription.plugin_name)),
        );
        return Ok(Value::Object(entry));
    }

    if let Some(repo) = github_repo_from_source(&subscription.source) {
        entry.insert(
            "source".to_string(),
            json!({
                "source": "github",
                "repo": repo,
            }),
        );
        return Ok(Value::Object(entry));
    }

    if is_http_url(&subscription.source) {
        entry.insert(
            "source".to_string(),
            json!({
                "source": "url",
                "url": subscription.source,
            }),
        );
        return Ok(Value::Object(entry));
    }

    Err(
        "Unsupported direct Claude source. Use a local path, GitHub owner/repo, or repository URL."
            .to_string(),
    )
}

fn sync_managed_claude_marketplace(
    subscriptions: &[PluginSubscription],
) -> Result<Option<PathBuf>, String> {
    let direct_claude_subscriptions = subscriptions
        .iter()
        .filter(|subscription| {
            subscription.source_kind == "direct"
                && subscription
                    .targets
                    .iter()
                    .any(|target| target == "claude-code")
        })
        .collect::<Vec<_>>();

    let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
    let marketplace_dir = claude_managed_marketplace_dir(&home_dir);

    if direct_claude_subscriptions.is_empty() {
        if marketplace_dir.exists() {
            fs::remove_dir_all(&marketplace_dir)
                .map_err(|e| format!("Failed to remove managed Claude marketplace: {e}"))?;
        }
        return Ok(None);
    }

    let plugins_dir = claude_managed_marketplace_plugins_dir(&home_dir);
    let manifest_dir = marketplace_dir.join(".claude-plugin");
    fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("Failed to create managed Claude plugins directory: {e}"))?;
    fs::create_dir_all(&manifest_dir)
        .map_err(|e| format!("Failed to create managed Claude marketplace manifest dir: {e}"))?;

    let mut local_plugin_names = HashSet::new();
    let mut seen_plugin_names = HashSet::new();
    let mut plugin_entries = Vec::new();

    for subscription in direct_claude_subscriptions {
        if !seen_plugin_names.insert(subscription.plugin_name.clone()) {
            return Err(format!(
                "Managed plugin names must be unique for Claude. `{}` is duplicated.",
                subscription.plugin_name
            ));
        }

        if resolve_local_plugin_source(&subscription.source).is_some() {
            local_plugin_names.insert(subscription.plugin_name.clone());
        }

        plugin_entries.push(build_claude_managed_marketplace_entry(
            subscription,
            &plugins_dir,
        )?);
    }

    if plugins_dir.exists() {
        for entry in fs::read_dir(&plugins_dir)
            .map_err(|e| format!("Failed to read managed Claude plugins directory: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Failed to read managed Claude plugin: {e}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let plugin_name = entry.file_name().to_string_lossy().to_string();
            if !local_plugin_names.contains(&plugin_name) {
                fs::remove_dir_all(&path).map_err(|e| {
                    format!(
                        "Failed to remove stale managed Claude plugin {}: {e}",
                        path.display()
                    )
                })?;
            }
        }
    }

    let manifest = json!({
        "$schema": "https://anthropic.com/claude-code/marketplace.schema.json",
        "name": TARS_MANAGED_CLAUDE_MARKETPLACE,
        "description": TARS_MANAGED_CLAUDE_MARKETPLACE_DESCRIPTION,
        "owner": {
            "name": "TARS"
        },
        "plugins": plugin_entries,
    });

    let manifest_path = manifest_dir.join("marketplace.json");
    let content = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize managed Claude marketplace: {e}"))?;
    fs::write(&manifest_path, content)
        .map_err(|e| format!("Failed to write managed Claude marketplace: {e}"))?;

    Ok(Some(marketplace_dir))
}

fn is_claude_marketplace_registered(name: &str) -> Result<bool, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let marketplaces_file = home
        .join(".claude")
        .join("plugins")
        .join("known_marketplaces.json");

    if !marketplaces_file.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&marketplaces_file)
        .map_err(|e| format!("Failed to read Claude marketplaces file: {e}"))?;
    let json: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse Claude marketplaces file: {e}"))?;

    Ok(json.get(name).is_some())
}

async fn ensure_claude_marketplace_registered(source: String, name: &str) -> Result<(), String> {
    if is_claude_marketplace_registered(name)? {
        return Ok(());
    }

    plugin_marketplace_add(source).await.map(|_| ())
}

fn codex_source_value(source: &str) -> Result<Value, String> {
    if source.contains('@') && !is_http_url(source) && !Path::new(source).exists() {
        return Err(
            "Codex registration needs a direct plugin source like a local path, GitHub repo, or URL — not plugin@marketplace."
                .to_string(),
        );
    }

    if let Some(path) = resolve_local_plugin_source(source) {
        return Ok(json!({
            "source": "local",
            "path": path.display().to_string(),
        }));
    }

    if is_http_url(source) {
        return Ok(json!({
            "source": "url",
            "url": source,
        }));
    }

    if looks_like_github_repo(source) {
        return Ok(json!({
            "source": "url",
            "url": format!("https://github.com/{source}"),
        }));
    }

    Err("Unsupported plugin source. Use a local path, GitHub owner/repo, or full URL.".to_string())
}

fn ensure_object(value: &mut Value) -> Result<&mut serde_json::Map<String, Value>, String> {
    value
        .as_object_mut()
        .ok_or_else(|| "Invalid JSON object".to_string())
}

fn ensure_array<'a>(
    object: &'a mut serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a mut Vec<Value>, String> {
    if !object.contains_key(key) {
        object.insert(key.to_string(), Value::Array(Vec::new()));
    }

    object
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("Invalid `{key}` array"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeTargetResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPluginTargetsResponse {
    pub source: String,
    pub plugin_name: String,
    pub claude: Option<RuntimeTargetResult>,
    pub codex: Option<RuntimeTargetResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePluginSubscriptionResponse {
    pub id: i64,
    pub plugin_name: String,
    pub claude: Option<RuntimeTargetResult>,
    pub codex: Option<RuntimeTargetResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexPluginBridge {
    pub key: String,
    pub plugin_name: String,
    pub marketplace: Option<String>,
    pub scope: String,
    pub project_path: Option<String>,
    pub codex_skill_dirs: Vec<String>,
    pub skill_count: usize,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexPluginBridgeOperationResult {
    pub key: String,
    pub plugin_name: String,
    pub success: bool,
    pub message: String,
    pub skill_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexPluginBridgeSyncResponse {
    pub results: Vec<CodexPluginBridgeOperationResult>,
}

// ============================================================================
// Plugin Inventory Commands
// ============================================================================

/// Plugin manifest for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestInfo {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

/// Plugin scope for frontend display (matches frontend `PluginScope` type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginScopeInfo {
    #[serde(rename = "type")]
    pub scope_type: String,
}

/// Installed plugin for frontend display (matches frontend `InstalledPlugin` interface)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPluginInfo {
    pub id: String,
    pub marketplace: Option<String>,
    pub version: String,
    pub scope: PluginScopeInfo,
    pub enabled: bool,
    pub path: String,
    pub manifest: PluginManifestInfo,
    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
    pub project_path: Option<String>,
}

/// List all installed plugins
#[tauri::command]
pub async fn plugin_list() -> Result<Vec<InstalledPluginInfo>, String> {
    let inventory = PluginInventory::scan().map_err(|e| format!("Failed to scan plugins: {e}"))?;

    let plugins: Vec<InstalledPluginInfo> = inventory
        .installed
        .into_iter()
        .map(|p| InstalledPluginInfo {
            id: p.id,
            marketplace: p.marketplace,
            version: p.version.clone(),
            scope: PluginScopeInfo {
                scope_type: format!("{:?}", p.scope),
            },
            enabled: p.enabled,
            path: p.path.display().to_string(),
            manifest: PluginManifestInfo {
                name: p.manifest.name,
                description: Some(p.manifest.description),
                version: Some(p.version),
            },
            installed_at: p.installed_at,
            last_updated: p.last_updated,
            project_path: p.project_path,
        })
        .collect();

    Ok(plugins)
}

// ============================================================================
// Plugin Marketplace Commands
// ============================================================================

/// Add a plugin marketplace
#[tauri::command]
pub async fn plugin_marketplace_add(source: String) -> Result<String, String> {
    validate_plugin_source(&source)?;

    let claude = find_claude_binary()?;
    let output = Command::new(&claude)
        .args(["plugin", "marketplace", "add", &source])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to add marketplace: {error_msg}"))
    }
}

/// Remove a plugin marketplace
#[tauri::command]
pub async fn plugin_marketplace_remove(name: String) -> Result<String, String> {
    validate_plugin_name(&name)?;

    let claude = find_claude_binary()?;
    let output = Command::new(&claude)
        .args(["plugin", "marketplace", "remove", &name])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to remove marketplace: {error_msg}"))
    }
}

/// Update plugin marketplaces
#[tauri::command]
pub async fn plugin_marketplace_update(name: Option<String>) -> Result<String, String> {
    if let Some(ref n) = name {
        validate_plugin_name(n)?;
    }

    let mut args = vec!["plugin", "marketplace", "update"];
    if let Some(ref n) = name {
        args.push(n);
    }

    let claude = find_claude_binary()?;
    let output = Command::new(&claude)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to update marketplace: {error_msg}"))
    }
}

fn register_codex_plugin_at_user_scope(source: &str) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let marketplace_path = home
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");

    let mut document: Value = if marketplace_path.exists() {
        let content = fs::read_to_string(&marketplace_path)
            .map_err(|e| format!("Failed to read Codex marketplace: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse Codex marketplace: {e}"))?
    } else {
        json!({
            "name": "tars-managed",
            "interface": {
                "displayName": "TARS managed plugins"
            },
            "plugins": []
        })
    };

    let plugin_name = plugin_name_from_source(source);
    let source_value = codex_source_value(source)?;
    let entry = json!({
        "name": plugin_name,
        "source": source_value,
        "policy": {
            "installation": "AVAILABLE",
            "authentication": "ON_INSTALL"
        },
        "category": "Developer Tools"
    });

    let root = ensure_object(&mut document)?;
    if !root.contains_key("name") {
        root.insert(
            "name".to_string(),
            Value::String("tars-managed".to_string()),
        );
    }
    if !root.contains_key("interface") {
        root.insert(
            "interface".to_string(),
            json!({
                "displayName": "TARS managed plugins"
            }),
        );
    }

    let plugins = ensure_array(root, "plugins")?;
    if let Some(existing) = plugins.iter_mut().find(|plugin| {
        plugin
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name == plugin_name)
    }) {
        *existing = entry;
    } else {
        plugins.push(entry);
    }

    if let Some(parent) = marketplace_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Codex marketplace directory: {e}"))?;
    }

    let content = serde_json::to_string_pretty(&document)
        .map_err(|e| format!("Failed to serialize Codex marketplace: {e}"))?;
    fs::write(&marketplace_path, content)
        .map_err(|e| format!("Failed to write Codex marketplace: {e}"))?;

    Ok(format!(
        "Registered {plugin_name} in {}",
        marketplace_path.display()
    ))
}

fn unregister_codex_plugin_at_user_scope(plugin_name: &str) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let marketplace_path = home
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");

    if !marketplace_path.exists() {
        return Ok("Codex marketplace file not found".to_string());
    }

    let content = fs::read_to_string(&marketplace_path)
        .map_err(|e| format!("Failed to read Codex marketplace: {e}"))?;
    let mut document: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse Codex marketplace: {e}"))?;

    let root = ensure_object(&mut document)?;
    let plugins = ensure_array(root, "plugins")?;
    let original_len = plugins.len();
    plugins.retain(|plugin| plugin.get("name").and_then(Value::as_str) != Some(plugin_name));

    if plugins.len() == original_len {
        return Ok(format!("No Codex entry found for {plugin_name}"));
    }

    let updated = serde_json::to_string_pretty(&document)
        .map_err(|e| format!("Failed to serialize Codex marketplace: {e}"))?;
    fs::write(&marketplace_path, updated)
        .map_err(|e| format!("Failed to write Codex marketplace: {e}"))?;

    Ok(format!(
        "Removed {plugin_name} from {}",
        marketplace_path.display()
    ))
}

async fn apply_subscription_targets(
    subscription: &PluginSubscription,
    managed_claude_marketplace: Option<&Path>,
) -> AddPluginTargetsResponse {
    let wants_claude = subscription
        .targets
        .iter()
        .any(|target| target == "claude-code");
    let wants_codex = subscription.targets.iter().any(|target| target == "codex");

    let claude = if wants_claude {
        let claude_result = if subscription.source_kind == "marketplace" {
            if let Some(marketplace_source) = subscription.marketplace_source.as_ref() {
                let _ = plugin_marketplace_add(marketplace_source.clone()).await;
            }

            let marketplace_name = subscription
                .marketplace_name
                .clone()
                .unwrap_or_else(|| "marketplace".to_string());
            let plugin_spec = format!("{}@{}", subscription.plugin_name, marketplace_name);
            plugin_install(plugin_spec, Some("user".to_string()), None).await
        } else {
            match managed_claude_marketplace {
                Some(marketplace_dir) => {
                    let marketplace_source = marketplace_dir.display().to_string();
                    match ensure_claude_marketplace_registered(
                        marketplace_source,
                        TARS_MANAGED_CLAUDE_MARKETPLACE,
                    )
                    .await
                    {
                        Ok(()) => {
                            let plugin_spec = format!(
                                "{}@{}",
                                subscription.plugin_name, TARS_MANAGED_CLAUDE_MARKETPLACE
                            );
                            plugin_install(plugin_spec, Some("user".to_string()), None).await
                        }
                        Err(error) => Err(error),
                    }
                }
                None => Err(
                    "TARS could not prepare the managed Claude marketplace for this direct plugin."
                        .to_string(),
                ),
            }
        };

        match claude_result {
            Ok(output) => Some(RuntimeTargetResult {
                success: true,
                message: if output.trim().is_empty() {
                    format!("Installed {} for Claude Code", subscription.plugin_name)
                } else {
                    output.trim().to_string()
                },
            }),
            Err(error) => Some(RuntimeTargetResult {
                success: false,
                message: error,
            }),
        }
    } else {
        None
    };

    let codex = if wants_codex {
        let codex_source = if subscription.source_kind == "marketplace" {
            subscription.codex_source.as_deref()
        } else {
            Some(subscription.source.as_str())
        };

        match codex_source {
            Some(source) => match register_codex_plugin_at_user_scope(source) {
                Ok(message) => Some(RuntimeTargetResult {
                    success: true,
                    message,
                }),
                Err(error) => Some(RuntimeTargetResult {
                    success: false,
                    message: error,
                }),
            },
            None => Some(RuntimeTargetResult {
                success: false,
                message:
                    "Codex needs a direct plugin source. Add a Codex source path or repo URL for this marketplace-backed plugin."
                        .to_string(),
            }),
        }
    } else {
        None
    };

    AddPluginTargetsResponse {
        source: subscription.source.clone(),
        plugin_name: subscription.plugin_name.clone(),
        claude,
        codex,
    }
}

fn build_subscription_input(
    source_kind: &str,
    source: &str,
    plugin_name: Option<String>,
    marketplace_source: Option<String>,
    marketplace_name: Option<String>,
    codex_source: Option<String>,
    targets: Vec<String>,
) -> Result<PluginSubscriptionInput, String> {
    validate_source_kind(source_kind)?;

    let trimmed_source = source.trim();
    let scope = "user".to_string();

    match source_kind {
        "direct" => {
            validate_plugin_source(trimmed_source)?;
            Ok(PluginSubscriptionInput {
                plugin_name: plugin_name.unwrap_or_else(|| plugin_name_from_source(trimmed_source)),
                source: trimmed_source.to_string(),
                source_kind: "direct".to_string(),
                marketplace_source: None,
                marketplace_name: None,
                codex_source: None,
                scope,
                targets,
            })
        }
        "marketplace" => {
            let marketplace_source = marketplace_source
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Marketplace source is required".to_string())?;
            validate_plugin_source(&marketplace_source)?;

            let plugin_name = plugin_name
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Plugin name is required".to_string())?;
            validate_plugin_name(&plugin_name)?;

            let marketplace_name = marketplace_name
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .or_else(|| Some(marketplace_name_from_source(&marketplace_source)));

            let codex_source = codex_source
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            if let Some(ref value) = codex_source {
                validate_plugin_source(value)?;
            }

            Ok(PluginSubscriptionInput {
                plugin_name: plugin_name.clone(),
                source: format!("{plugin_name}@{marketplace_source}"),
                source_kind: "marketplace".to_string(),
                marketplace_source: Some(marketplace_source),
                marketplace_name,
                codex_source,
                scope,
                targets,
            })
        }
        _ => Err("Unsupported plugin source kind".to_string()),
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_plugin_to_targets(
    state: State<'_, AppState>,
    source_kind: String,
    source: String,
    plugin_name: Option<String>,
    marketplace_source: Option<String>,
    marketplace_name: Option<String>,
    codex_source: Option<String>,
    targets: Vec<String>,
) -> Result<AddPluginTargetsResponse, String> {
    if targets.is_empty() {
        return Err("Select at least one runtime target".to_string());
    }

    for target in &targets {
        validate_runtime_target(target)?;
    }

    let input = build_subscription_input(
        &source_kind,
        &source,
        plugin_name,
        marketplace_source,
        marketplace_name,
        codex_source,
        targets,
    )?;

    let subscription = state.with_db(|db| {
        let store = PluginSubscriptionStore::new(db.connection());
        store
            .upsert(&input)
            .map_err(|e| format!("Failed to save plugin subscription: {e}"))
    })?;

    let all_subscriptions = state.with_db(|db| {
        let store = PluginSubscriptionStore::new(db.connection());
        store
            .list()
            .map_err(|e| format!("Failed to list plugin subscriptions: {e}"))
    })?;
    let managed_claude_marketplace = sync_managed_claude_marketplace(&all_subscriptions)?;

    Ok(apply_subscription_targets(&subscription, managed_claude_marketplace.as_deref()).await)
}

#[tauri::command]
pub async fn list_plugin_subscriptions(
    state: State<'_, AppState>,
) -> Result<Vec<PluginSubscription>, String> {
    state.with_db(|db| {
        let store = PluginSubscriptionStore::new(db.connection());
        store
            .list()
            .map_err(|e| format!("Failed to list plugin subscriptions: {e}"))
    })
}

#[tauri::command]
pub async fn list_codex_plugin_bridges() -> Result<Vec<CodexPluginBridge>, String> {
    load_codex_plugin_bridges()
}

#[tauri::command]
pub async fn bridge_claude_plugin_to_codex(
    plugin_name: String,
    marketplace: Option<String>,
    scope: String,
    project_path: Option<String>,
) -> Result<CodexPluginBridge, String> {
    let inventory = PluginInventory::scan().map_err(|e| format!("Failed to scan plugins: {e}"))?;
    let plugin = find_installed_plugin(
        &inventory,
        &plugin_name,
        marketplace.as_deref(),
        &scope,
        project_path.as_deref(),
    )
    .ok_or_else(|| {
        format!("Installed Claude plugin `{plugin_name}` was not found for scope `{scope}`.")
    })?;

    sync_installed_plugin_to_codex(plugin)
}

#[tauri::command]
pub async fn sync_codex_plugin_bridges() -> Result<CodexPluginBridgeSyncResponse, String> {
    let bridges = load_codex_plugin_bridges()?;
    if bridges.is_empty() {
        return Ok(CodexPluginBridgeSyncResponse {
            results: Vec::new(),
        });
    }

    let inventory = PluginInventory::scan().map_err(|e| format!("Failed to scan plugins: {e}"))?;
    let mut results = Vec::new();
    let mut next_bridges = Vec::new();

    for bridge in bridges {
        if let Some(plugin) = find_installed_plugin(
            &inventory,
            &bridge.plugin_name,
            bridge.marketplace.as_deref(),
            &bridge.scope,
            bridge.project_path.as_deref(),
        ) {
            match sync_installed_plugin_to_codex(plugin) {
                Ok(updated_bridge) => {
                    results.push(CodexPluginBridgeOperationResult {
                        key: updated_bridge.key.clone(),
                        plugin_name: updated_bridge.plugin_name.clone(),
                        success: true,
                        message: format!(
                            "Synced {} skill{} to Codex",
                            updated_bridge.skill_count,
                            if updated_bridge.skill_count == 1 {
                                ""
                            } else {
                                "s"
                            }
                        ),
                        skill_count: updated_bridge.skill_count,
                    });
                    next_bridges.push(updated_bridge);
                }
                Err(error) => {
                    results.push(CodexPluginBridgeOperationResult {
                        key: bridge.key.clone(),
                        plugin_name: bridge.plugin_name.clone(),
                        success: false,
                        message: error,
                        skill_count: 0,
                    });
                    next_bridges.push(bridge);
                }
            }
        } else {
            remove_codex_skill_dirs(&bridge.codex_skill_dirs)?;
            results.push(CodexPluginBridgeOperationResult {
                key: bridge.key.clone(),
                plugin_name: bridge.plugin_name.clone(),
                success: false,
                message: "Claude plugin is no longer installed, so the Codex bridge was removed."
                    .to_string(),
                skill_count: 0,
            });
        }
    }

    save_codex_plugin_bridges(&next_bridges)?;

    Ok(CodexPluginBridgeSyncResponse { results })
}

#[tauri::command]
pub async fn sync_plugin_subscription(
    id: i64,
    state: State<'_, AppState>,
) -> Result<AddPluginTargetsResponse, String> {
    let subscription = state.with_db(|db| {
        let store = PluginSubscriptionStore::new(db.connection());
        store
            .get(id)
            .map_err(|e| format!("Failed to load plugin subscription: {e}"))?
            .ok_or_else(|| "Plugin subscription not found".to_string())
    })?;

    let all_subscriptions = state.with_db(|db| {
        let store = PluginSubscriptionStore::new(db.connection());
        store
            .list()
            .map_err(|e| format!("Failed to list plugin subscriptions: {e}"))
    })?;
    let managed_claude_marketplace = sync_managed_claude_marketplace(&all_subscriptions)?;

    Ok(apply_subscription_targets(&subscription, managed_claude_marketplace.as_deref()).await)
}

#[tauri::command]
pub async fn remove_plugin_subscription(
    id: i64,
    state: State<'_, AppState>,
) -> Result<RemovePluginSubscriptionResponse, String> {
    let (subscription, remaining_subscriptions) = state.with_db(|db| {
        let store = PluginSubscriptionStore::new(db.connection());
        let subscription = store
            .get(id)
            .map_err(|e| format!("Failed to load plugin subscription: {e}"))?
            .ok_or_else(|| "Plugin subscription not found".to_string())?;
        let remaining_subscriptions = store
            .list()
            .map_err(|e| format!("Failed to list plugin subscriptions: {e}"))?
            .into_iter()
            .filter(|candidate| candidate.id != id)
            .collect::<Vec<_>>();

        Ok((subscription, remaining_subscriptions))
    })?;

    let claude = if subscription
        .targets
        .iter()
        .any(|target| target == "claude-code")
    {
        match plugin_uninstall(
            subscription.plugin_name.clone(),
            Some("user".to_string()),
            None,
        )
        .await
        {
            Ok(message) => Some(RuntimeTargetResult {
                success: true,
                message,
            }),
            Err(error) => Some(RuntimeTargetResult {
                success: false,
                message: error,
            }),
        }
    } else {
        None
    };

    let codex = if subscription.targets.iter().any(|target| target == "codex") {
        match unregister_codex_plugin_at_user_scope(&subscription.plugin_name) {
            Ok(message) => Some(RuntimeTargetResult {
                success: true,
                message,
            }),
            Err(error) => Some(RuntimeTargetResult {
                success: false,
                message: error,
            }),
        }
    } else {
        None
    };

    let failure_messages = [claude.as_ref(), codex.as_ref()]
        .into_iter()
        .flatten()
        .filter(|result| !result.success)
        .map(|result| result.message.clone())
        .collect::<Vec<_>>();

    if !failure_messages.is_empty() {
        return Err(format!(
            "Failed to remove managed plugin {}: {}",
            subscription.plugin_name,
            failure_messages.join(" | ")
        ));
    }

    let managed_claude_marketplace = sync_managed_claude_marketplace(&remaining_subscriptions)?;
    if managed_claude_marketplace.is_none()
        && is_claude_marketplace_registered(TARS_MANAGED_CLAUDE_MARKETPLACE)?
    {
        let _ = plugin_marketplace_remove(TARS_MANAGED_CLAUDE_MARKETPLACE.to_string()).await;
    }

    state.with_db(|db| {
        let store = PluginSubscriptionStore::new(db.connection());
        store
            .delete(id)
            .map_err(|e| format!("Failed to delete plugin subscription: {e}"))?;
        Ok(())
    })?;

    Ok(RemovePluginSubscriptionResponse {
        id,
        plugin_name: subscription.plugin_name,
        claude,
        codex,
    })
}

/// Install a plugin
/// For project/local scope, `project_path` must be provided to run CLI from that directory
#[tauri::command]
pub async fn plugin_install(
    plugin: String,
    scope: Option<String>,
    project_path: Option<String>,
) -> Result<String, String> {
    // Validate plugin source (can be name@marketplace format)
    validate_plugin_source(&plugin)?;

    // Validate scope if provided
    if let Some(ref s) = scope {
        validate_scope(s)?;
    }

    let mut args = vec!["plugin", "install"];

    // Add scope if specified (user, project, or local)
    let scope_flag;
    if let Some(ref s) = scope {
        scope_flag = format!("--scope={s}");
        args.push(&scope_flag);
    }

    args.push(&plugin);

    let claude = find_claude_binary()?;
    let mut cmd = Command::new(&claude);
    cmd.args(&args);

    // For project/local scope, run from the project directory
    if let Some(ref path) = project_path {
        cmd.current_dir(path);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to install plugin: {error_msg}"))
    }
}

/// Update an installed plugin
/// For project/local scope, `project_path` must be provided to run CLI from that directory
#[tauri::command]
pub async fn plugin_update(
    plugin: String,
    scope: Option<String>,
    project_path: Option<String>,
) -> Result<String, String> {
    // Validate plugin source (can be name@marketplace format)
    validate_plugin_source(&plugin)?;

    // Validate scope if provided
    if let Some(ref s) = scope {
        validate_scope(s)?;
    }

    let mut args = vec!["plugin", "update"];

    // Add scope if specified (user, project, or local)
    let scope_flag;
    if let Some(ref s) = scope {
        scope_flag = format!("--scope={s}");
        args.push(&scope_flag);
    }

    args.push(&plugin);

    let claude = find_claude_binary()?;
    let mut cmd = Command::new(&claude);
    cmd.args(&args);

    // For project/local scope, run from the project directory
    if let Some(ref path) = project_path {
        cmd.current_dir(path);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        Err(format!("Failed to update plugin: {error_msg}"))
    }
}

/// Uninstall a plugin
/// Note: CLI only accepts plugin name without marketplace
/// For project scope, `project_path` must be provided to run CLI from that directory
#[tauri::command]
pub async fn plugin_uninstall(
    plugin: String,
    scope: Option<String>,
    project_path: Option<String>,
) -> Result<String, String> {
    // Validate plugin source first
    validate_plugin_source(&plugin)?;

    // Validate scope if provided
    if let Some(ref s) = scope {
        validate_scope(s)?;
    }

    let scope_str = scope.as_deref().unwrap_or("user");

    // Workaround for Claude CLI bug #14202: CLI doesn't properly handle
    // project-scoped plugin uninstall - it removes ALL installations instead of
    // just the one for the specified project. Use direct JSON editing for
    // project and local scopes to ensure only the correct installation is removed.
    if (scope_str == "project" || scope_str == "local") && project_path.is_some() {
        return uninstall_plugin_directly(&plugin, Some(scope_str), project_path.as_deref());
    }

    // For user scope, use CLI as it works correctly
    // Extract plugin name (without marketplace) for uninstall
    // Format may be "pluginName@marketplace" - uninstall only wants pluginName
    let plugin_name = plugin.split('@').next().unwrap_or(&plugin);

    let mut args = vec!["plugin", "uninstall"];

    // Add scope if specified
    let scope_flag;
    if let Some(ref s) = scope {
        scope_flag = format!("--scope={s}");
        args.push(&scope_flag);
    }

    args.push(plugin_name);

    let claude = find_claude_binary()?;
    let mut cmd = Command::new(&claude);
    cmd.args(&args);

    // For project scope, run from the project directory
    if let Some(ref path) = project_path {
        cmd.current_dir(path);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };

        // Fall back to direct JSON editing if CLI fails
        if error_msg.contains("not found in installed plugins") {
            return uninstall_plugin_directly(&plugin, scope.as_deref(), project_path.as_deref());
        }

        Err(format!("Failed to uninstall plugin: {error_msg}"))
    }
}

/// Direct uninstall by editing JSON files (workaround for Claude CLI bug #14202)
fn uninstall_plugin_directly(
    plugin: &str,
    scope: Option<&str>,
    project_path: Option<&str>,
) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let installed_file = home
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");

    if !installed_file.exists() {
        return Err("No installed plugins file found".to_string());
    }

    // Read installed_plugins.json
    let content = std::fs::read_to_string(&installed_file)
        .map_err(|e| format!("Failed to read installed plugins: {e}"))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed plugins: {e}"))?;

    let plugins = json
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or("Invalid installed plugins format")?;

    // Find the plugin key (could be "plugin" or "plugin@marketplace")
    let plugin_key = if plugin.contains('@') {
        plugin.to_string()
    } else {
        // Find key that starts with the plugin name
        plugins
            .keys()
            .find(|k| {
                k.starts_with(plugin)
                    && (k.len() == plugin.len() || k.chars().nth(plugin.len()) == Some('@'))
            })
            .cloned()
            .unwrap_or_else(|| plugin.to_string())
    };

    let scope_str = scope.unwrap_or("user");
    let mut removed = false;

    if let Some(installations) = plugins.get_mut(&plugin_key).and_then(|v| v.as_array_mut()) {
        let original_len = installations.len();

        // Filter out the installation matching scope and project_path
        installations.retain(|install| {
            let install_scope = install
                .get("scope")
                .and_then(|s| s.as_str())
                .unwrap_or("user");
            let install_project = install.get("projectPath").and_then(|p| p.as_str());

            // Keep if scope doesn't match
            if install_scope != scope_str {
                return true;
            }

            // For project scope, also check project path
            if scope_str == "project" {
                match (install_project, project_path) {
                    (Some(install_proj), Some(target_proj)) => {
                        // Normalize paths for comparison
                        let install_normalized = install_proj.replace('\\', "/").to_lowercase();
                        let target_normalized = target_proj.replace('\\', "/").to_lowercase();
                        if install_normalized != target_normalized {
                            return true; // Keep - different project
                        }
                        // Paths match - remove this one
                    }
                    (Some(_), None) => {
                        // Project path not provided but entry has one - keep it
                        // (we don't know which project to uninstall from)
                        return true;
                    }
                    (None, _) => {
                        // Entry has no project path - shouldn't happen for project scope, but remove it
                    }
                }
            }

            false // Remove this installation
        });

        removed = installations.len() < original_len;

        // If no installations left, remove the entire plugin entry
        if installations.is_empty() {
            plugins.remove(&plugin_key);
        }
    }

    if !removed {
        return Err(format!("Plugin {plugin} not found for scope {scope_str}"));
    }

    // Write back installed_plugins.json
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize installed plugins: {e}"))?;
    std::fs::write(&installed_file, updated)
        .map_err(|e| format!("Failed to write installed plugins: {e}"))?;

    // Also remove from project settings.json if project scope
    if scope_str == "project" {
        if let Some(proj_path) = project_path {
            let settings_file = std::path::PathBuf::from(proj_path)
                .join(".claude")
                .join("settings.json");
            if settings_file.exists() {
                if let Ok(settings_content) = std::fs::read_to_string(&settings_file) {
                    if let Ok(mut settings_json) =
                        serde_json::from_str::<serde_json::Value>(&settings_content)
                    {
                        if let Some(enabled) = settings_json
                            .get_mut("enabledPlugins")
                            .and_then(|e| e.as_object_mut())
                        {
                            enabled.remove(&plugin_key);
                            // Also try without marketplace suffix
                            let plugin_name = plugin.split('@').next().unwrap_or(plugin);
                            for key in enabled.keys().cloned().collect::<Vec<_>>() {
                                if key.starts_with(plugin_name) {
                                    enabled.remove(&key);
                                }
                            }
                            if let Ok(updated_settings) =
                                serde_json::to_string_pretty(&settings_json)
                            {
                                let _ = std::fs::write(&settings_file, updated_settings);
                            }
                        }
                    }
                }
            }
        }
    }

    // Also remove from user settings.json if user scope
    if scope_str == "user" {
        let user_settings = home.join(".claude").join("settings.json");
        if user_settings.exists() {
            if let Ok(settings_content) = std::fs::read_to_string(&user_settings) {
                if let Ok(mut settings_json) =
                    serde_json::from_str::<serde_json::Value>(&settings_content)
                {
                    if let Some(enabled) = settings_json
                        .get_mut("enabledPlugins")
                        .and_then(|e| e.as_object_mut())
                    {
                        enabled.remove(&plugin_key);
                        if let Ok(updated_settings) = serde_json::to_string_pretty(&settings_json) {
                            let _ = std::fs::write(&user_settings, updated_settings);
                        }
                    }
                }
            }
        }
    }

    Ok(format!("Uninstalled {plugin} (via direct edit workaround)"))
}

/// Move a plugin to a different scope (uninstall + reinstall)
/// Note: uninstall takes just the plugin name, install takes plugin@marketplace
#[tauri::command]
pub async fn plugin_move_scope(
    plugin: String,
    from_scope: String,
    to_scope: String,
) -> Result<String, String> {
    // Validate all inputs
    validate_plugin_source(&plugin)?;
    validate_scope(&from_scope)?;
    validate_scope(&to_scope)?;

    // Extract plugin name (without marketplace) for uninstall
    // Format is "pluginName@marketplace" - uninstall only wants pluginName
    let plugin_name = plugin.split('@').next().unwrap_or(&plugin);

    let claude = find_claude_binary()?;

    // First uninstall from current scope (uses just plugin name)
    let uninstall_output = Command::new(&claude)
        .args([
            "plugin",
            "uninstall",
            &format!("--scope={from_scope}"),
            plugin_name,
        ])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if !uninstall_output.status.success() {
        let stderr = String::from_utf8_lossy(&uninstall_output.stderr);
        let stdout = String::from_utf8_lossy(&uninstall_output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };
        return Err(format!(
            "Failed to uninstall from {from_scope} scope: {error_msg}"
        ));
    }

    // Then reinstall at new scope (uses full plugin@marketplace)
    let install_output = Command::new(&claude)
        .args(["plugin", "install", &format!("--scope={to_scope}"), &plugin])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

    if install_output.status.success() {
        Ok(format!(
            "Moved {plugin_name} from {from_scope} to {to_scope} scope"
        ))
    } else {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };

        // Try to restore original installation if reinstall fails
        let _ = Command::new(&claude)
            .args([
                "plugin",
                "install",
                &format!("--scope={from_scope}"),
                &plugin,
            ])
            .output();

        Err(format!(
            "Failed to install at {to_scope} scope: {error_msg}"
        ))
    }
}

/// Enable a plugin by setting it to true in enabledPlugins
#[tauri::command]
pub async fn plugin_enable(plugin: String) -> Result<String, String> {
    validate_plugin_source(&plugin)?;
    set_plugin_enabled(&plugin, true)
}

/// Disable a plugin by setting it to false in enabledPlugins
#[tauri::command]
pub async fn plugin_disable(plugin: String) -> Result<String, String> {
    validate_plugin_source(&plugin)?;
    set_plugin_enabled(&plugin, false)
}

/// Set a plugin's enabled state in ~/.claude/settings.json (cross-platform)
fn set_plugin_enabled(plugin: &str, enabled: bool) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let settings_file = home.join(".claude").join("settings.json");

    // Read existing settings or create empty object
    let mut settings: serde_json::Value = if settings_file.exists() {
        let content = std::fs::read_to_string(&settings_file)
            .map_err(|_| "Failed to read settings file".to_string())?;
        serde_json::from_str(&content).map_err(|_| "Failed to parse settings file".to_string())?
    } else {
        serde_json::json!({})
    };

    // Ensure enabledPlugins object exists
    if settings.get("enabledPlugins").is_none() {
        settings["enabledPlugins"] = serde_json::json!({});
    }

    // Set the plugin's enabled state
    if let Some(enabled_plugins) = settings
        .get_mut("enabledPlugins")
        .and_then(|p| p.as_object_mut())
    {
        enabled_plugins.insert(plugin.to_string(), serde_json::Value::Bool(enabled));
    }

    // Write back to file
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|_| "Failed to serialize settings".to_string())?;
    std::fs::write(&settings_file, content)
        .map_err(|_| "Failed to write settings file".to_string())?;

    Ok(format!(
        "Plugin {} {}",
        plugin,
        if enabled { "enabled" } else { "disabled" }
    ))
}

/// Toggle auto-update for a marketplace
#[tauri::command]
pub async fn plugin_marketplace_set_auto_update(
    name: String,
    auto_update: bool,
) -> Result<String, String> {
    validate_plugin_name(&name)?;

    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let marketplaces_file = home
        .join(".claude")
        .join("plugins")
        .join("known_marketplaces.json");

    if !marketplaces_file.exists() {
        return Err("Marketplaces file not found".to_string());
    }

    // Read the file
    let content = std::fs::read_to_string(&marketplaces_file)
        .map_err(|_| "Failed to read marketplaces file".to_string())?;

    // Parse as JSON
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|_| "Failed to parse marketplaces file".to_string())?;

    // Update the autoUpdate field for the marketplace
    if let Some(marketplace) = json.get_mut(&name) {
        if let Some(obj) = marketplace.as_object_mut() {
            obj.insert(
                "autoUpdate".to_string(),
                serde_json::Value::Bool(auto_update),
            );
        } else {
            return Err("Invalid marketplace configuration".to_string());
        }
    } else {
        return Err("Marketplace not found".to_string());
    }

    // Write back
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|_| "Failed to serialize marketplaces".to_string())?;

    std::fs::write(&marketplaces_file, updated)
        .map_err(|_| "Failed to write marketplaces file".to_string())?;

    Ok(format!(
        "Auto-update {} for {}",
        if auto_update { "enabled" } else { "disabled" },
        name
    ))
}

/// Stale cache entry for UI display
#[derive(Debug, Clone, Serialize)]
pub struct CacheEntry {
    pub path: String,
    pub plugin_name: String,
    pub marketplace: String,
    pub version: String,
    pub size_bytes: u64,
}

/// Cache status response for UI
#[derive(Debug, Clone, Serialize)]
pub struct CacheStatusResponse {
    pub stale_entries: Vec<CacheEntry>,
    pub total_size_bytes: u64,
    pub total_size_formatted: String,
    pub installed_count: usize,
}

/// Get cache cleanup status
#[tauri::command]
pub async fn cache_status() -> Result<CacheStatusResponse, String> {
    let report = CacheCleanupReport::scan().map_err(|e| format!("Failed to scan cache: {e}"))?;

    Ok(CacheStatusResponse {
        stale_entries: report
            .stale_entries
            .iter()
            .map(|e| CacheEntry {
                path: e.path.display().to_string(),
                plugin_name: e.plugin_name.clone(),
                marketplace: e.marketplace.clone(),
                version: e.version.clone(),
                size_bytes: e.size_bytes,
            })
            .collect(),
        total_size_bytes: report.total_size_bytes,
        total_size_formatted: report.format_size(),
        installed_count: report.installed_count,
    })
}

/// Clean result response for UI
#[derive(Debug, Clone, Serialize)]
pub struct CacheCleanResult {
    pub deleted_count: usize,
    pub deleted_bytes: u64,
    pub deleted_size_formatted: String,
    pub errors: Vec<String>,
}

/// Clean stale cache entries
#[tauri::command]
pub async fn cache_clean() -> Result<CacheCleanResult, String> {
    let report = CacheCleanupReport::scan().map_err(|e| format!("Failed to scan cache: {e}"))?;

    if report.stale_entries.is_empty() {
        return Ok(CacheCleanResult {
            deleted_count: 0,
            deleted_bytes: 0,
            deleted_size_formatted: "0 bytes".to_string(),
            errors: vec![],
        });
    }

    let result = report
        .clean()
        .map_err(|e| format!("Failed to clean cache: {e}"))?;

    Ok(CacheCleanResult {
        deleted_count: result.deleted_count,
        deleted_bytes: result.deleted_bytes,
        deleted_size_formatted: result.format_size(),
        errors: result.errors,
    })
}

/// Open Terminal with Claude Code and run a specific skill/command
/// The skill should be in format "/plugin-name:skill-name"
#[tauri::command]
pub async fn open_claude_with_skill(skill_invocation: String) -> Result<(), String> {
    // Validate the skill invocation format
    if !skill_invocation.starts_with('/') {
        return Err("Skill invocation must start with /".to_string());
    }

    // Validate for shell safety - only allow safe characters
    let forbidden_chars = [
        '`', '$', '(', ')', '{', '}', '[', ']', '|', ';', '&', '<', '>', '\\', '\n', '\r', '\0',
        '\'', '"', '!', '*', '?',
    ];
    for ch in forbidden_chars {
        if skill_invocation.contains(ch) {
            return Err(format!(
                "Skill invocation contains forbidden character: {ch}"
            ));
        }
    }

    // Copy the skill command to clipboard, then open Terminal with Claude
    // Skills are processed by Claude Code's runtime when typed interactively,
    // not via -p flag, so user needs to paste the command

    // First, copy to clipboard
    let copy_script = format!(r#"set the clipboard to "{skill_invocation}""#);
    Command::new("osascript")
        .args(["-e", &copy_script])
        .output()
        .map_err(|_| "Failed to copy to clipboard".to_string())?;

    // Then open Terminal with Claude and instructions
    let terminal_script = r#"tell application "Terminal"
    activate
    do script "echo '📋 Skill command copied to clipboard - paste it after Claude starts\n' && claude"
end tell"#;

    Command::new("osascript")
        .args(["-e", terminal_script])
        .spawn()
        .map_err(|_| "Failed to open Terminal".to_string())?;

    Ok(())
}
