//! Cross-agent standalone skills manager commands.
//!
//! Manages a library of standalone (non-plugin) skills and deploys them to
//! Claude Code and/or Codex, per user/project scope. A deployment is a symlink
//! (default) or copy; its presence is the on/off state — nothing is written to
//! `settings.json` or `config.toml`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use tars_scanner::plugins::PluginInventory;

use tars_core::skills::{
    deploy, probe_target, resolve_skills_dir, scan_source, scan_sources, undeploy, Agent,
    CatalogSkill, LinkKind, Scope, TargetProbe,
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
        let (final_link_path, link_kind, sha256) = match probe_target(&link_path) {
            TargetProbe::Symlink { .. } => (link_path, LinkKind::Symlink, None),
            _ => {
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
            }
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
        store.delete(id).map_err(|e| e.to_string())
    })
}

/// A skill's on/off state for one agent at the selected scope.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
    pub source_root: Option<String>,
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
    let mut plugin_catalog: Vec<(String, String, Vec<CatalogSkill>)> = Vec::new();
    let mut claude_plugin_by_name: HashMap<String, String> = HashMap::new();
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
        }
        let label = if plugin.manifest.name.is_empty() {
            plugin.id.clone()
        } else {
            plugin.manifest.name.clone()
        };
        plugin_catalog.push((plugin.id.clone(), label, skills));
    }

    let (sources, project_root, deployments) = state.with_db(|db| {
        let sources = SkillSourceStore::new(db.connection())
            .list()
            .map_err(|e| e.to_string())?;
        let root = match project_id.as_deref() {
            Some(pid) => Some(project_root_for(db, pid)?),
            None => None,
        };
        let store = SkillDeploymentStore::new(db.connection());
        let deps = match project_id.as_deref() {
            Some(pid) => store.list_for_project(pid).map_err(|e| e.to_string())?,
            None => store.list_user_scope().map_err(|e| e.to_string())?,
        };
        Ok((sources, root, deps))
    })?;

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
            ),
        };
        let codex = cell_for(
            Agent::Codex,
            scope,
            project_root.as_deref(),
            &home,
            skill,
            &deployments,
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
    for (id, label, skills) in &plugin_catalog {
        groups.push(SkillGroup {
            kind: "plugin".to_string(),
            label: label.clone(),
            plugin_id: Some(id.clone()),
            source_root: None,
            skills: skills.iter().map(&build_row).collect(),
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
            source_root: Some(source.path.clone()),
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
    }
}

fn cell_for(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    home: &Path,
    skill: &CatalogSkill,
    deployments: &[SkillDeployment],
) -> SkillCell {
    // A tracked deployment row wins outright.
    if let Some(dep) = deployments
        .iter()
        .find(|d| d.agent == agent.as_str() && d.skill_name == skill.name)
    {
        return SkillCell {
            status: "on".to_string(),
            deployed: true,
            tracked: true,
            link_kind: Some(dep.link_kind.clone()),
            deployment_id: Some(dep.id),
            link_path: dep.link_path.clone(),
            plugin_id: None,
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
