//! Codex scope discovery helpers.

use crate::artifacts::{CodexAgentInfo, SkillInfo};
use crate::error::ScanResult;
use crate::inventory::CodexScope;
use crate::plugins::{scan_codex_marketplace_file, CodexMarketplace};
use crate::runtime::{codex_agent_runtime_support, codex_skill_runtime_support};
use crate::scope::user::scan_skills_directory;
use crate::types::{FileInfo, Scope};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan_user_codex_scope(home: &Path) -> ScanResult<CodexScope> {
    let codex_dir = home.join(".codex");
    let user_skills_dir = home.join(".agents").join("skills");

    let mut skills = scan_skills_directory(&user_skills_dir, Scope::User)?;
    for skill in &mut skills {
        skill.runtime_support = codex_skill_runtime_support();
    }

    let marketplaces = scan_codex_marketplaces(
        &[home
            .join(".agents")
            .join("plugins")
            .join("marketplace.json")],
        Scope::User,
    )?;

    Ok(CodexScope {
        config: scan_file(&codex_dir.join("config.toml"))?,
        instructions: scan_existing_files(&[
            codex_dir.join("AGENTS.override.md"),
            codex_dir.join("AGENTS.md"),
        ])?,
        skills,
        agents: scan_codex_agents_directory(&codex_dir.join("agents"), Scope::User)?,
        marketplaces,
    })
}

pub fn scan_project_codex_scope(project_path: &Path) -> ScanResult<CodexScope> {
    let repo_root = find_repo_root(project_path);
    let mut skills = scan_upward_skill_layers(project_path, &repo_root)?;
    for skill in &mut skills {
        skill.runtime_support = codex_skill_runtime_support();
    }

    let marketplaces = scan_codex_marketplaces(
        &[project_path
            .join(".agents")
            .join("plugins")
            .join("marketplace.json")],
        Scope::Project,
    )?;

    Ok(CodexScope {
        config: scan_file(&project_path.join(".codex").join("config.toml"))?,
        instructions: scan_instruction_layers(project_path, &repo_root)?,
        skills,
        agents: scan_codex_agents_directory(
            &project_path.join(".codex").join("agents"),
            Scope::Project,
        )?,
        marketplaces,
    })
}

fn scan_upward_skill_layers(start: &Path, repo_root: &Path) -> ScanResult<Vec<SkillInfo>> {
    let mut results = Vec::new();
    let mut seen_paths = HashSet::new();

    for dir in ancestors_to_root(start, repo_root) {
        let skills_dir = dir.join(".agents").join("skills");
        let scanned = scan_skills_directory(&skills_dir, Scope::Project)?;
        for skill in scanned {
            if seen_paths.insert(skill.path.clone()) {
                results.push(skill);
            }
        }
    }

    Ok(results)
}

fn scan_instruction_layers(start: &Path, repo_root: &Path) -> ScanResult<Vec<FileInfo>> {
    let mut layers = Vec::new();
    for dir in ancestors_to_root(start, repo_root) {
        let paths = [dir.join("AGENTS.md"), dir.join("AGENTS.override.md")];
        layers.extend(scan_existing_files(&paths)?);
    }
    Ok(layers)
}

fn scan_existing_files(paths: &[PathBuf]) -> ScanResult<Vec<FileInfo>> {
    let mut files = Vec::new();
    for path in paths {
        if let Some(info) = scan_file(path)? {
            files.push(info);
        }
    }
    Ok(files)
}

fn scan_codex_marketplaces(paths: &[PathBuf], scope: Scope) -> ScanResult<Vec<CodexMarketplace>> {
    let mut marketplaces = Vec::new();

    for path in paths {
        if let Some(marketplace) = scan_codex_marketplace_file(path, scope.clone())? {
            marketplaces.push(marketplace);
        }
    }

    Ok(marketplaces)
}

fn scan_file(path: &Path) -> ScanResult<Option<FileInfo>> {
    if !path.exists() || !path.is_file() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    Ok(Some(FileInfo {
        path: path.to_path_buf(),
        sha256: hash_content(&content),
    }))
}

fn scan_codex_agents_directory(dir: &Path, scope: Scope) -> ScanResult<Vec<CodexAgentInfo>> {
    let mut agents = Vec::new();

    if !dir.exists() {
        return Ok(agents);
    }

    let entries = fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Warning: Failed to read Codex agent file {path:?}: {err}");
                    continue;
                }
            };

            agents.push(CodexAgentInfo {
                name: extract_toml_string(&content, "name").unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                }),
                description: extract_toml_string(&content, "description"),
                path,
                sha256: hash_content(&content),
                runtime_support: codex_agent_runtime_support(),
                scope: scope.clone(),
            });
        }
    }

    agents.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(agents)
}

fn extract_toml_string(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        let prefix = format!("{key} =");
        if !trimmed.starts_with(&prefix) {
            return None;
        }

        let value = trimmed.split_once('=').map(|(_, value)| value.trim())?;
        Some(value.trim_matches('"').to_string())
    })
}

fn ancestors_to_root(start: &Path, repo_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut current = Some(start);

    while let Some(path) = current {
        paths.push(path.to_path_buf());
        if path == repo_root {
            break;
        }
        current = path.parent();
    }

    paths
}

fn find_repo_root(start: &Path) -> PathBuf {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return ancestor.to_path_buf();
        }
    }

    start.to_path_buf()
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}
