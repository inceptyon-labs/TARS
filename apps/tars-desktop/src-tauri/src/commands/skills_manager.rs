//! Cross-agent standalone skills manager commands.
//!
//! Manages a library of standalone (non-plugin) skills and deploys them to
//! Claude Code and/or Codex, per user/project scope. A deployment is a symlink
//! (default) or copy; its presence is the on/off state — nothing is written to
//! `settings.json` or `config.toml`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use tars_scanner::plugins::PluginInventory;

use tars_core::skills::{
    deploy, hash_bundle, probe_target, repoint_symlink, resolve_skills_dir, resync_copy,
    scan_source, scan_sources, undeploy, Agent, CatalogSkill, LinkKind, Scope, TargetProbe,
};
use tars_core::storage::skill_library::{
    SkillDeployment, SkillDeploymentInput, SkillDeploymentStore, SkillSource, SkillSourceStore,
};
use tars_core::storage::{Database, ProjectStore};

use crate::commands::plugins::plugin_skills_root;
use crate::state::AppState;

/// Resolve a project's absolute root path from its UUID.
fn project_root_for(db: &Database, project_id: &str) -> Result<PathBuf, String> {
    let uuid = uuid::Uuid::parse_str(project_id).map_err(|e| format!("Invalid project id: {e}"))?;
    let project = ProjectStore::new(db.connection())
        .get(uuid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {project_id}"))?;
    Ok(project.path)
}

#[tauri::command]
pub async fn list_skill_sources(state: State<'_, AppState>) -> Result<Vec<SkillSource>, String> {
    state.with_db(|db| {
        SkillSourceStore::new(db.connection())
            .list()
            .map_err(|e| format!("Failed to list skill sources: {e}"))
    })
}

#[tauri::command]
pub async fn add_skill_source(
    path: String,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<SkillSource, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    let canonical = dir
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path: {e}"))?
        .display()
        .to_string();

    state.with_db(|db| {
        let store = SkillSourceStore::new(db.connection());
        if let Some(existing) = store.get_by_path(&canonical).map_err(|e| e.to_string())? {
            return Ok(existing);
        }
        store
            .create(&canonical, label.as_deref())
            .map_err(|e| format!("Failed to add source: {e}"))
    })
}

#[tauri::command]
pub async fn remove_skill_source(id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    state.with_db(|db| {
        SkillSourceStore::new(db.connection())
            .delete(id)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub async fn scan_skill_library(state: State<'_, AppState>) -> Result<Vec<CatalogSkill>, String> {
    let dirs = source_dirs(&state)?;
    Ok(scan_sources(&dirs))
}

/// Load the registered source directories from the DB.
fn source_dirs(state: &State<'_, AppState>) -> Result<Vec<PathBuf>, String> {
    state.with_db(|db| {
        Ok(SkillSourceStore::new(db.connection())
            .list()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|s| PathBuf::from(s.path))
            .collect())
    })
}

/// Payload for [`deploy_skill`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploySkillInput {
    pub skill_name: String,
    pub source_dir: String,
    pub agent: Agent,
    pub scope: Scope,
    pub project_id: Option<String>,
    pub link_kind: LinkKind,
}

#[tauri::command]
pub async fn deploy_skill(
    input: DeploySkillInput,
    state: State<'_, AppState>,
) -> Result<SkillDeployment, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let source_dir = PathBuf::from(&input.source_dir);

    state.with_db(|db| {
        let project_root = match (input.scope, input.project_id.as_deref()) {
            (Scope::Project, Some(pid)) => Some(project_root_for(db, pid)?),
            (Scope::Project, None) => return Err("Project scope requires a project id".into()),
            (Scope::User, _) => None,
        };

        let dep_store = SkillDeploymentStore::new(db.connection());
        if dep_store
            .get_target(
                input.agent.as_str(),
                input.scope.as_str(),
                input.project_id.as_deref(),
                &input.skill_name,
            )
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err(format!(
                "'{}' is already deployed to this target",
                input.skill_name
            ));
        }

        let link_path =
            resolve_skills_dir(input.agent, input.scope, project_root.as_deref(), &home)
                .map_err(|e| e.to_string())?
                .join(&input.skill_name);

        // Adopt any existing symlink at the target (regardless of where it
        // points — e.g. a hand-made link to a plugin's repo) instead of
        // colliding; otherwise materialize a fresh deployment.
        let (final_link_path, link_kind, sha256) =
            if let TargetProbe::Symlink { .. } = probe_target(&link_path) {
                (link_path, LinkKind::Symlink, None)
            } else {
                let result = deploy(
                    &source_dir,
                    &input.skill_name,
                    input.agent,
                    input.scope,
                    project_root.as_deref(),
                    input.link_kind,
                    &home,
                )
                .map_err(|e| e.to_string())?;
                (result.link_path, result.link_kind, result.sha256)
            };

        let record = SkillDeploymentInput {
            skill_name: input.skill_name.clone(),
            source_path: input.source_dir.clone(),
            agent: input.agent.as_str().to_string(),
            scope: input.scope.as_str().to_string(),
            project_id: input.project_id.clone(),
            link_path: final_link_path.display().to_string(),
            link_kind: link_kind.as_str().to_string(),
            sha256,
        };
        dep_store
            .create(&record)
            .map_err(|e| format!("Failed to record deployment: {e}"))
    })
}

#[tauri::command]
pub async fn undeploy_skill(id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    state.with_db(|db| {
        let store = SkillDeploymentStore::new(db.connection());
        let Some(row) = store.get(id).map_err(|e| e.to_string())? else {
            return Ok(false);
        };
        let link_kind = LinkKind::from_db_str(&row.link_kind).unwrap_or(LinkKind::Symlink);
        undeploy(Path::new(&row.link_path), link_kind).map_err(|e| e.to_string())?;

        // Clear any lingering skillOverrides entry so the settings file never
        // references a skill that no longer exists (best effort).
        if row.mute_state.is_some() && row.agent == Agent::Claude.as_str() {
            if let (Some(home), Some(scope)) = (dirs::home_dir(), Scope::from_db_str(&row.scope)) {
                let project_root = match (scope, row.project_id.as_deref()) {
                    (Scope::Project, Some(pid)) => project_root_for(db, pid).ok(),
                    _ => None,
                };
                if let Ok(path) = claude_settings_path(scope, project_root.as_deref(), &home) {
                    let skill_name = row.skill_name.clone();
                    let _ = edit_claude_settings(&path, |root| {
                        set_skill_override(root, &skill_name, None);
                    });
                }
            }
        }

        store.delete(id).map_err(|e| e.to_string())
    })
}

/// A skill's on/off state for one agent at the selected scope.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)] // a UI cell DTO; each flag is an independent axis
pub struct SkillCell {
    /// `"on"` | `"off"` | `"adopted"` | `"collision"` | `"plugin"`.
    pub status: String,
    pub deployed: bool,
    /// Whether TARS has a deployment record (vs. a hand-made symlink).
    pub tracked: bool,
    pub link_kind: Option<String>,
    pub deployment_id: Option<i64>,
    pub link_path: String,
    /// Set when this agent receives the skill from a plugin (status `"plugin"`).
    pub plugin_id: Option<String>,
    /// Copy deploy whose on-disk source bundle no longer matches the hash
    /// captured at deploy time (the deployed copy is stale). Always false for
    /// symlink deploys — they are the source.
    pub drifted: bool,
    /// Muting middle-state: `None`/`"on"` = fully visible; `"name-only"`,
    /// `"user-invocable-only"`, `"off"` mirror Claude `skillOverrides`.
    pub mute_state: Option<String>,
    /// Whether this (agent, scope, kind) can actually be muted on the installed
    /// agent build. The UI must not render a mute control when this is false.
    pub mute_supported: bool,
}

/// One catalog skill with its per-agent state for the selected scope.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMatrixRow {
    pub name: String,
    pub description: String,
    pub source_dir: String,
    pub claude: SkillCell,
    pub codex: SkillCell,
}

/// A group of skills in the Library: an installed plugin (auto-listed from the
/// Marketplace) or a registered standalone source directory.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGroup {
    /// `"plugin"` or `"source"`.
    pub kind: String,
    pub label: String,
    pub plugin_id: Option<String>,
    /// Marketplace the plugin came from, so the frontend can form the
    /// `id@marketplace` key used by `enabledPlugins` (plugin groups only).
    pub plugin_marketplace: Option<String>,
    /// Whether this plugin is disabled for the current project scope via
    /// `enabledPlugins` in the project settings (plugin groups only).
    pub plugin_disabled_here: bool,
    pub source_root: Option<String>,
    /// True when the source directory itself is a single skill bundle (its
    /// `SKILL.md` is at the root), rather than a folder that contains skills.
    pub single_skill: bool,
    pub skills: Vec<SkillMatrixRow>,
}

#[tauri::command]
pub async fn get_project_skill_matrix(
    project_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SkillGroup>, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let scope = if project_id.is_some() {
        Scope::Project
    } else {
        Scope::User
    };

    // Installed, enabled plugins become auto-listed groups. We also build a
    // name -> plugin-id map so the Claude column always badges a skill a plugin
    // already provides (even one that also exists in a standalone source).
    let inventory = PluginInventory::scan().map_err(|e| format!("Failed to scan plugins: {e}"))?;
    let mut plugin_catalog: Vec<PluginGroupData> = Vec::new();
    let mut claude_plugin_by_name: HashMap<String, String> = HashMap::new();
    // skill name -> the plugin's CURRENT skills-root source dir, used to repair
    // Codex symlinks that still point at a superseded version-pinned cache dir.
    let mut plugin_skill_source: HashMap<String, PathBuf> = HashMap::new();
    for plugin in inventory.installed {
        if !plugin.enabled {
            continue;
        }
        let skills = scan_source(&plugin_skills_root(&plugin));
        if skills.is_empty() {
            continue;
        }
        for skill in &skills {
            claude_plugin_by_name
                .entry(skill.name.clone())
                .or_insert_with(|| plugin.id.clone());
            plugin_skill_source
                .entry(skill.name.clone())
                .or_insert_with(|| skill.source_dir.clone());
        }
        let label = if plugin.manifest.name.is_empty() {
            plugin.id.clone()
        } else {
            plugin.manifest.name.clone()
        };
        plugin_catalog.push(PluginGroupData {
            id: plugin.id.clone(),
            marketplace: plugin.marketplace.clone(),
            label,
            skills,
        });
    }

    // Muting is only real on Claude Code >= 2.1.129; below that skillOverrides
    // is a silent no-op, so we must not offer the control.
    let claude_mute_supported = claude_supports_skill_overrides();

    let (sources, project_root, deployments) = state.with_db(|db| {
        let sources = SkillSourceStore::new(db.connection())
            .list()
            .map_err(|e| e.to_string())?;
        let root = match project_id.as_deref() {
            Some(pid) => Some(project_root_for(db, pid)?),
            None => None,
        };
        let store = SkillDeploymentStore::new(db.connection());
        let mut deps = match project_id.as_deref() {
            Some(pid) => store.list_for_project(pid).map_err(|e| e.to_string())?,
            None => store.list_user_scope().map_err(|e| e.to_string())?,
        };
        // Pin-following: repair plugin-sourced symlinks orphaned by a plugin
        // version bump (its skills live under a version-pinned cache dir).
        repair_plugin_deployments(&store, &mut deps, &plugin_skill_source)?;
        Ok((sources, root, deps))
    })?;

    // Plugins disabled for this project via `enabledPlugins` (project scope only).
    let disabled_plugins = match project_root.as_deref() {
        Some(root) => read_disabled_plugins(&root.join(".claude").join("settings.json")),
        None => HashSet::new(),
    };

    let build_row = |skill: &CatalogSkill| -> SkillMatrixRow {
        let claude = match claude_plugin_by_name.get(&skill.name) {
            Some(pid) => plugin_cell(pid),
            None => cell_for(
                Agent::Claude,
                scope,
                project_root.as_deref(),
                &home,
                skill,
                &deployments,
                claude_mute_supported,
            ),
        };
        // Codex has no working per-project file-based mute, so never offer it.
        let codex = cell_for(
            Agent::Codex,
            scope,
            project_root.as_deref(),
            &home,
            skill,
            &deployments,
            false,
        );
        SkillMatrixRow {
            name: skill.name.clone(),
            description: skill.description.clone(),
            source_dir: skill.source_dir.display().to_string(),
            claude,
            codex,
        }
    };

    let mut groups: Vec<SkillGroup> = Vec::new();

    // Plugin groups first (auto, from the Marketplace).
    for pg in &plugin_catalog {
        let key = plugin_key(&pg.id, pg.marketplace.as_deref());
        groups.push(SkillGroup {
            kind: "plugin".to_string(),
            label: pg.label.clone(),
            plugin_id: Some(pg.id.clone()),
            plugin_marketplace: pg.marketplace.clone(),
            plugin_disabled_here: disabled_plugins.contains(&key),
            source_root: None,
            single_skill: false,
            skills: pg.skills.iter().map(&build_row).collect(),
        });
    }

    // Standalone source groups.
    for source in &sources {
        let dir = PathBuf::from(&source.path);
        let skills = scan_source(&dir);
        if skills.is_empty() {
            continue;
        }
        let label = source
            .label
            .clone()
            .unwrap_or_else(|| short_path(&source.path));
        groups.push(SkillGroup {
            kind: "source".to_string(),
            label,
            plugin_id: None,
            plugin_marketplace: None,
            plugin_disabled_here: false,
            source_root: Some(source.path.clone()),
            // The source itself is a skill when its SKILL.md is at the root.
            single_skill: dir.join("SKILL.md").is_file(),
            skills: skills.iter().map(&build_row).collect(),
        });
    }

    Ok(groups)
}

/// A Claude cell for a skill provided by an installed plugin.
fn plugin_cell(plugin_id: &str) -> SkillCell {
    SkillCell {
        status: "plugin".to_string(),
        deployed: false,
        tracked: false,
        link_kind: None,
        deployment_id: None,
        link_path: String::new(),
        plugin_id: Some(plugin_id.to_string()),
        drifted: false,
        mute_state: None,
        mute_supported: false,
    }
}

fn cell_for(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    home: &Path,
    skill: &CatalogSkill,
    deployments: &[SkillDeployment],
    mute_supported: bool,
) -> SkillCell {
    // A tracked deployment row wins outright.
    if let Some(dep) = deployments
        .iter()
        .find(|d| d.agent == agent.as_str() && d.skill_name == skill.name)
    {
        // A copy deploy drifts when the source bundle changed since deploy.
        let drifted = dep.link_kind == "copy"
            && dep.sha256.as_deref().is_some_and(|stored| {
                hash_bundle(Path::new(&dep.source_path)).is_some_and(|current| current != stored)
            });
        return SkillCell {
            status: "on".to_string(),
            deployed: true,
            tracked: true,
            link_kind: Some(dep.link_kind.clone()),
            deployment_id: Some(dep.id),
            link_path: dep.link_path.clone(),
            plugin_id: None,
            drifted,
            mute_state: dep.mute_state.clone(),
            mute_supported,
        };
    }

    // No record: adopt any symlink at the target (by name), else off/collision.
    let Ok(dir) = resolve_skills_dir(agent, scope, project_root, home) else {
        return off_cell(String::new());
    };
    let link_path = dir.join(&skill.name);
    let link_str = link_path.display().to_string();

    match probe_target(&link_path) {
        TargetProbe::Symlink { .. } => SkillCell {
            status: "adopted".to_string(),
            deployed: true,
            tracked: false,
            link_kind: Some("symlink".to_string()),
            deployment_id: None,
            link_path: link_str,
            plugin_id: None,
            drifted: false,
            mute_state: None,
            mute_supported: false,
        },
        TargetProbe::Absent => off_cell(link_str),
        _ => SkillCell {
            status: "collision".to_string(),
            deployed: false,
            tracked: false,
            link_kind: None,
            deployment_id: None,
            link_path: link_str,
            plugin_id: None,
            drifted: false,
            mute_state: None,
            mute_supported: false,
        },
    }
}

fn off_cell(link_path: String) -> SkillCell {
    SkillCell {
        status: "off".to_string(),
        deployed: false,
        tracked: false,
        link_kind: None,
        deployment_id: None,
        link_path,
        plugin_id: None,
        drifted: false,
        mute_state: None,
        mute_supported: false,
    }
}

/// Last two path segments, for a compact source label.
fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 2 {
        path.to_string()
    } else {
        format!("…/{}", parts[parts.len() - 2..].join("/"))
    }
}

/// An installed plugin's data as a Library group.
struct PluginGroupData {
    id: String,
    marketplace: Option<String>,
    label: String,
    skills: Vec<CatalogSkill>,
}

/// Form the `id@marketplace` key `enabledPlugins` uses (bare id if no marketplace).
fn plugin_key(id: &str, marketplace: Option<&str>) -> String {
    match marketplace {
        Some(mp) if !mp.is_empty() => format!("{id}@{mp}"),
        _ => id.to_string(),
    }
}

/// True if the installed Claude Code honors per-skill `skillOverrides` in
/// user/project settings. This was a silent no-op until it was fixed in
/// 2.1.129, so below that version we must not offer a mute control.
fn claude_supports_skill_overrides() -> bool {
    let Ok(output) = std::process::Command::new("claude")
        .arg("--version")
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_semver(&text).is_some_and(|v| v >= (2, 1, 129))
}

/// Extract the first `major.minor.patch` triple from a version string,
/// tolerating a leading `v` (e.g. `v1.2.3`).
fn parse_semver(text: &str) -> Option<(u32, u32, u32)> {
    let token = text.split_whitespace().find_map(|t| {
        let t = t.strip_prefix('v').unwrap_or(t);
        (t.split('.').count() >= 3 && t.starts_with(|c: char| c.is_ascii_digit())).then_some(t)
    })?;
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    // The patch field may carry a trailing suffix (e.g. build metadata).
    let patch: u32 = parts
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

/// True if `path` lives under the Claude plugin cache (version-pinned dirs).
fn is_plugin_cache_path(path: &str) -> bool {
    path.replace('\\', "/").contains("/plugins/cache/")
}

/// Repoint plugin-sourced symlinks orphaned by a plugin version bump.
///
/// A plugin's skills live under a version-pinned cache dir; when the plugin
/// updates, the old dir disappears and any deployed symlink dangles. For each
/// tracked symlink deployment whose recorded source is under the plugin cache
/// and either dangles or points at a superseded version, recreate the link
/// against the plugin's current skills dir and update the row. Only ever
/// touches symlinks (never real directories).
fn repair_plugin_deployments(
    store: &SkillDeploymentStore,
    deployments: &mut [SkillDeployment],
    plugin_skill_source: &HashMap<String, PathBuf>,
) -> Result<(), String> {
    for dep in deployments.iter_mut() {
        if dep.link_kind != "symlink" || !is_plugin_cache_path(&dep.source_path) {
            continue;
        }
        // Only repair skills a currently-installed plugin still provides.
        let Some(current) = plugin_skill_source.get(&dep.skill_name) else {
            continue;
        };
        let current_str = current.display().to_string();
        let link_resolves = Path::new(&dep.link_path).exists();
        if current_str == dep.source_path && link_resolves {
            continue;
        }
        repoint_symlink(current, Path::new(&dep.link_path))
            .map_err(|e| format!("Failed to repair plugin link for {}: {e}", dep.skill_name))?;
        store
            .update_source_path(dep.id, &current_str)
            .map_err(|e| e.to_string())?;
        dep.source_path = current_str;
    }
    Ok(())
}

/// Plugin keys (`id@marketplace`) explicitly disabled via
/// `enabledPlugins: { key: false }` in a settings file.
fn read_disabled_plugins(settings_path: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    let Ok(text) = std::fs::read_to_string(settings_path) else {
        return out;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return out;
    };
    if let Some(map) = value.get("enabledPlugins").and_then(|v| v.as_object()) {
        for (key, enabled) in map {
            if enabled.as_bool() == Some(false) {
                out.insert(key.clone());
            }
        }
    }
    out
}

/// Resolve the `.claude/settings.json` path for a scope.
fn claude_settings_path(
    scope: Scope,
    project_root: Option<&Path>,
    home: &Path,
) -> Result<PathBuf, String> {
    match scope {
        Scope::User => Ok(home.join(".claude").join("settings.json")),
        Scope::Project => project_root
            .map(|r| r.join(".claude").join("settings.json"))
            .ok_or_else(|| "Project scope requires a project root".to_string()),
    }
}

/// Read a `.claude/settings.json` object, apply `mutate`, write it back.
/// A missing or empty file starts from `{}`; unrelated keys are preserved.
fn edit_claude_settings<F>(path: &Path, mutate: F) -> Result<(), String>
where
    F: FnOnce(&mut serde_json::Map<String, serde_json::Value>),
{
    let mut root: serde_json::Map<String, serde_json::Value> = if path.exists() {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        if text.trim().is_empty() {
            serde_json::Map::new()
        } else {
            serde_json::from_str(&text)
                .map_err(|e| format!("{} is not a JSON object: {e}", path.display()))?
        }
    } else {
        serde_json::Map::new()
    };

    mutate(&mut root);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {e}"))?;
    }
    let serialized = serde_json::to_string_pretty(&serde_json::Value::Object(root))
        .map_err(|e| e.to_string())?;
    std::fs::write(path, format!("{serialized}\n"))
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(())
}

/// Set or clear a single skill's `skillOverrides` entry.
fn set_skill_override(
    root: &mut serde_json::Map<String, serde_json::Value>,
    skill: &str,
    state: Option<&str>,
) {
    match state {
        Some(s) => {
            let overrides = root
                .entry("skillOverrides")
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(obj) = overrides.as_object_mut() {
                obj.insert(skill.to_string(), serde_json::Value::String(s.to_string()));
            }
        }
        None => {
            if let Some(obj) = root
                .get_mut("skillOverrides")
                .and_then(|v| v.as_object_mut())
            {
                obj.remove(skill);
                if obj.is_empty() {
                    root.remove("skillOverrides");
                }
            }
        }
    }
}

/// Set or clear a single plugin's `enabledPlugins` entry.
fn set_enabled_plugin(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    enabled: bool,
) {
    if enabled {
        // Re-enabling = drop the explicit `false` (back to default-on).
        if let Some(obj) = root
            .get_mut("enabledPlugins")
            .and_then(|v| v.as_object_mut())
        {
            obj.remove(key);
            if obj.is_empty() {
                root.remove("enabledPlugins");
            }
        }
    } else {
        let obj = root
            .entry("enabledPlugins")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(map) = obj.as_object_mut() {
            map.insert(key.to_string(), serde_json::Value::Bool(false));
        }
    }
}

/// Set (or clear, with `None`/`"on"`) the muting state for a Claude deployment.
///
/// Writes the skill's `skillOverrides` entry in the deployment's scope settings
/// file and records the state on the row. Claude standalone skills only — Codex
/// has no working per-project file mute, and plugin skills are muted via
/// [`set_project_plugin_enabled`] instead.
#[tauri::command]
pub async fn set_skill_mute(
    deployment_id: i64,
    mute_state: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let normalized = match mute_state.as_deref() {
        None | Some("on" | "") => None,
        Some("name-only" | "user-invocable-only" | "off") => mute_state.clone(),
        Some(other) => return Err(format!("Invalid mute state: {other}")),
    };

    state.with_db(|db| {
        let store = SkillDeploymentStore::new(db.connection());
        let dep = store
            .get(deployment_id)
            .map_err(|e| e.to_string())?
            .ok_or("Deployment not found")?;
        if dep.agent != Agent::Claude.as_str() {
            return Err("Muting is only supported for Claude skills".into());
        }
        let scope = Scope::from_db_str(&dep.scope).ok_or("Invalid deployment scope")?;
        let project_root = match (scope, dep.project_id.as_deref()) {
            (Scope::Project, Some(pid)) => Some(project_root_for(db, pid)?),
            (Scope::Project, None) => return Err("Project deployment missing project id".into()),
            (Scope::User, _) => None,
        };
        let path = claude_settings_path(scope, project_root.as_deref(), &home)?;
        let skill_name = dep.skill_name.clone();
        let target = normalized.clone();
        edit_claude_settings(&path, |root| {
            set_skill_override(root, &skill_name, target.as_deref());
        })?;
        store
            .set_mute_state(deployment_id, normalized.as_deref())
            .map_err(|e| e.to_string())
    })
}

/// Re-copy a drifted COPY deployment from its source and refresh the hash.
#[tauri::command]
pub async fn resync_skill_deployment(
    deployment_id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.with_db(|db| {
        let store = SkillDeploymentStore::new(db.connection());
        let dep = store
            .get(deployment_id)
            .map_err(|e| e.to_string())?
            .ok_or("Deployment not found")?;
        if dep.link_kind != "copy" {
            return Err("Only copy deployments can be re-synced".into());
        }
        let new_hash = resync_copy(Path::new(&dep.source_path), Path::new(&dep.link_path))
            .map_err(|e| e.to_string())?;
        store
            .update_sha256(deployment_id, new_hash.as_deref())
            .map_err(|e| e.to_string())?;
        Ok(true)
    })
}

/// Enable or disable a whole plugin for a project via `enabledPlugins` in the
/// project `.claude/settings.json`. This is the only per-project lever for
/// plugin-provided skills (Claude ignores `skillOverrides` for them).
#[tauri::command]
pub async fn set_project_plugin_enabled(
    project_id: String,
    plugin_key: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.with_db(|db| {
        let root = project_root_for(db, &project_id)?;
        let path = root.join(".claude").join("settings.json");
        edit_claude_settings(&path, |settings| {
            set_enabled_plugin(settings, &plugin_key, enabled);
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn skill_override_set_change_and_clear() {
        let mut root = serde_json::Map::new();
        set_skill_override(&mut root, "deep-research", Some("off"));
        assert_eq!(root["skillOverrides"], json!({ "deep-research": "off" }));

        // Switching state overwrites in place.
        set_skill_override(&mut root, "deep-research", Some("name-only"));
        assert_eq!(root["skillOverrides"]["deep-research"], json!("name-only"));

        // Clearing the last entry removes the whole skillOverrides object.
        set_skill_override(&mut root, "deep-research", None);
        assert!(!root.contains_key("skillOverrides"));
    }

    #[test]
    fn enabled_plugin_disable_then_reenable() {
        let mut root = serde_json::Map::new();
        set_enabled_plugin(&mut root, "pasiv@tars-profiles", false);
        assert_eq!(
            root["enabledPlugins"],
            json!({ "pasiv@tars-profiles": false })
        );

        // Re-enabling drops the explicit false (back to default-on) and cleans up.
        set_enabled_plugin(&mut root, "pasiv@tars-profiles", true);
        assert!(!root.contains_key("enabledPlugins"));
    }

    #[test]
    fn edits_preserve_unrelated_keys() {
        let mut root = map(json!({
            "model": "opus",
            "permissions": { "deny": ["Bash(rm *)"] },
        }));
        set_skill_override(&mut root, "handoff", Some("off"));
        assert_eq!(root["model"], json!("opus"));
        assert_eq!(root["permissions"]["deny"], json!(["Bash(rm *)"]));
        assert_eq!(root["skillOverrides"]["handoff"], json!("off"));
    }

    #[test]
    fn edit_claude_settings_roundtrips_via_disk() {
        let dir = std::env::temp_dir().join(format!("tars-mute-test-{}", std::process::id()));
        let path = dir.join(".claude").join("settings.json");
        let _ = std::fs::remove_dir_all(&dir);

        // Writing into a missing file creates it from `{}`.
        edit_claude_settings(&path, |root| {
            set_skill_override(root, "denoise", Some("off"));
        })
        .unwrap();
        let read: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(read["skillOverrides"]["denoise"], json!("off"));

        // A second edit merges, not clobbers.
        edit_claude_settings(&path, |root| {
            set_enabled_plugin(root, "x@y", false);
        })
        .unwrap();
        let read: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(read["skillOverrides"]["denoise"], json!("off"));
        assert_eq!(read["enabledPlugins"]["x@y"], json!(false));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn semver_gate_matches_2_1_129_fix() {
        assert_eq!(parse_semver("2.1.201 (Claude Code)"), Some((2, 1, 201)));
        assert_eq!(parse_semver("v0.142.5"), Some((0, 142, 5)));
        // The mute fix landed in 2.1.129.
        assert!(parse_semver("2.1.201 (Claude Code)").unwrap() >= (2, 1, 129));
        assert!(parse_semver("2.1.129").unwrap() >= (2, 1, 129));
        assert!(parse_semver("2.1.128").unwrap() < (2, 1, 129));
        assert!(parse_semver("2.0.999").unwrap() < (2, 1, 129));
        assert!(parse_semver("3.0.0").unwrap() >= (2, 1, 129));
    }

    #[test]
    fn detects_plugin_cache_paths() {
        assert!(is_plugin_cache_path(
            "/Users/j/.claude/plugins/cache/mkt/pasiv/1.2.0/skills/kick"
        ));
        assert!(!is_plugin_cache_path("/Users/j/skills-lib/deep-research"));
    }
}
