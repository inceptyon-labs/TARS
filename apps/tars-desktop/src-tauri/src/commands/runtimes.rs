//! Runtime detection commands.
//!
//! This keeps the first multi-runtime UI slice read-only and safe. It reports
//! installed client state and the key filesystem locations TARS will manage.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimePathStatus {
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub docs_url: String,
    pub summary: String,
    pub paths: Vec<RuntimePathStatus>,
}

#[tauri::command]
pub async fn get_runtime_statuses() -> Result<Vec<RuntimeStatus>, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;

    Ok(vec![claude_status(&home), codex_status(&home)])
}

fn claude_status(home: &Path) -> RuntimeStatus {
    let binary_path = find_binary(
        "claude",
        "claude.cmd",
        "claude.exe",
        &claude_candidates(home),
    );
    let version = binary_path
        .as_ref()
        .and_then(|path| read_version(path, &["--version"]));

    RuntimeStatus {
        id: "claude-code".to_string(),
        name: "Claude Code".to_string(),
        installed: binary_path.is_some(),
        version,
        binary_path: display_path(binary_path.as_ref()),
        docs_url: "https://code.claude.com/docs/en/settings".to_string(),
        summary: "Claude-native skills, agents, commands, hooks, MCP servers, and plugins."
            .to_string(),
        paths: vec![
            path_status("User settings", home.join(".claude/settings.json"), "file"),
            path_status("User config directory", home.join(".claude"), "directory"),
            path_status(
                "Plugin marketplaces",
                home.join(".claude/plugins/marketplaces"),
                "directory",
            ),
        ],
    }
}

fn codex_status(home: &Path) -> RuntimeStatus {
    let binary_path = find_binary("codex", "codex.cmd", "codex.exe", &codex_candidates(home));
    let version = binary_path
        .as_ref()
        .and_then(|path| read_version(path, &["--version"]));

    RuntimeStatus {
        id: "codex".to_string(),
        name: "Codex".to_string(),
        installed: binary_path.is_some(),
        version,
        binary_path: display_path(binary_path.as_ref()),
        docs_url: "https://developers.openai.com/codex".to_string(),
        summary: "Codex skills, AGENTS.md guidance, custom agents, MCP servers, and plugins."
            .to_string(),
        paths: vec![
            path_status("User config", home.join(".codex/config.toml"), "file"),
            path_status("Global instructions", home.join(".codex/AGENTS.md"), "file"),
            path_status("Custom agents", home.join(".codex/agents"), "directory"),
            path_status("User skills", home.join(".agents/skills"), "directory"),
            path_status(
                "Personal marketplace",
                home.join(".agents/plugins/marketplace.json"),
                "file",
            ),
        ],
    }
}

fn find_binary(
    unix_name: &str,
    _windows_cmd: &str,
    _windows_exe: &str,
    candidates: &[PathBuf],
) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(path) = which::which(_windows_cmd) {
            return Some(path);
        }
        if let Ok(path) = which::which(_windows_exe) {
            return Some(path);
        }
    }

    #[cfg(not(target_os = "windows"))]
    if let Ok(path) = which::which(unix_name) {
        return Some(path);
    }

    candidates
        .iter()
        .find(|path| path.exists() && path.is_file())
        .cloned()
}

fn read_version(binary_path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(binary_path).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    text.split_whitespace()
        .find(|part| part.chars().any(|c| c.is_ascii_digit()))
        .map(|part| {
            part.trim_matches(|c: char| {
                !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
            })
            .to_string()
        })
}

fn display_path(path: Option<&PathBuf>) -> Option<String> {
    path.map(|path| path.display().to_string())
}

fn path_status(label: &str, path: PathBuf, kind: &str) -> RuntimePathStatus {
    RuntimePathStatus {
        label: label.to_string(),
        exists: path.exists(),
        path: path.display().to_string(),
        kind: kind.to_string(),
    }
}

fn claude_candidates(home: &Path) -> Vec<PathBuf> {
    let mut candidates = common_node_candidates(home, "claude");
    candidates.extend([
        home.join(".local/bin/claude"),
        home.join(".cargo/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
    ]);
    candidates
}

fn codex_candidates(home: &Path) -> Vec<PathBuf> {
    let mut candidates = common_node_candidates(home, "codex");
    candidates.extend([
        home.join(".local/bin/codex"),
        home.join(".cargo/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
        PathBuf::from("/opt/homebrew/bin/codex"),
    ]);
    candidates
}

fn common_node_candidates(home: &Path, binary_name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![
        home.join(".npm-global/bin").join(binary_name),
        home.join(".volta/bin").join(binary_name),
        home.join(".local/share/pnpm").join(binary_name),
        home.join(".yarn/bin").join(binary_name),
    ];

    candidates.extend(versioned_node_candidates(
        &home.join(".nvm/versions/node"),
        &["bin"],
        binary_name,
    ));
    candidates.extend(versioned_node_candidates(
        &home.join(".local/share/fnm/node-versions"),
        &["installation", "bin"],
        binary_name,
    ));

    candidates
}

fn versioned_node_candidates(root: &Path, path_parts: &[&str], binary_name: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| {
            path_parts
                .iter()
                .fold(entry.path(), |path, part| path.join(part))
                .join(binary_name)
        })
        .collect()
}
