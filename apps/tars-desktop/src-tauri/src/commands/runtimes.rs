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
    pub install_method: Option<String>,
    pub docs_url: String,
    pub summary: String,
    pub paths: Vec<RuntimePathStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRuntimeCoverage {
    pub id: String,
    pub name: String,
    pub support: String,
    pub summary: String,
    pub surfaces: Vec<RuntimePathStatus>,
}

#[tauri::command]
pub async fn get_runtime_statuses() -> Result<Vec<RuntimeStatus>, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;

    Ok(vec![
        claude_status(&home),
        codex_status(&home),
        gemini_status(&home),
    ])
}

#[tauri::command]
pub async fn get_project_runtime_coverage(
    project_path: String,
) -> Result<Vec<ProjectRuntimeCoverage>, String> {
    let project_root = PathBuf::from(&project_path);

    if !project_root.exists() {
        return Err(format!("Project path does not exist: {project_path}"));
    }

    if !project_root.is_dir() {
        return Err(format!("Project path is not a directory: {project_path}"));
    }

    Ok(vec![
        claude_project_coverage(&project_root),
        codex_project_coverage(&project_root),
        gemini_project_coverage(&project_root),
    ])
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
    let install_method = binary_path
        .as_ref()
        .and_then(|path| detect_install_method(path, "claude-code"));

    RuntimeStatus {
        id: "claude-code".to_string(),
        name: "Claude Code".to_string(),
        installed: binary_path.is_some(),
        version,
        binary_path: display_path(binary_path.as_ref()),
        install_method,
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
    let install_method = binary_path
        .as_ref()
        .and_then(|path| detect_install_method(path, "codex"));

    RuntimeStatus {
        id: "codex".to_string(),
        name: "Codex".to_string(),
        installed: binary_path.is_some(),
        version,
        binary_path: display_path(binary_path.as_ref()),
        install_method,
        docs_url: "https://developers.openai.com/codex".to_string(),
        summary: "Codex skills, AGENTS.md guidance, custom agents, MCP servers, and plugins."
            .to_string(),
        paths: vec![
            path_status("User config", home.join(".codex/config.toml"), "file"),
            path_status("Global instructions", home.join(".codex/AGENTS.md"), "file"),
            path_status("Custom agents", home.join(".codex/agents"), "directory"),
            path_status("Plugin cache", home.join(".codex/plugins"), "directory"),
            path_status("User skills", home.join(".agents/skills"), "directory"),
            path_status(
                "Personal marketplace",
                home.join(".agents/plugins/marketplace.json"),
                "file",
            ),
            path_status("System config", codex_system_config_path(), "file"),
            path_status("Managed config", codex_managed_config_path(home), "file"),
        ],
    }
}

fn gemini_status(home: &Path) -> RuntimeStatus {
    let binary_path = find_binary(
        "gemini",
        "gemini.cmd",
        "gemini.exe",
        &gemini_candidates(home),
    );
    let version = binary_path
        .as_ref()
        .and_then(|path| read_version(path, &["--version"]));
    let install_method = binary_path
        .as_ref()
        .and_then(|path| detect_install_method(path, "gemini-cli"));

    RuntimeStatus {
        id: "gemini-cli".to_string(),
        name: "Gemini CLI".to_string(),
        installed: binary_path.is_some(),
        version,
        binary_path: display_path(binary_path.as_ref()),
        install_method,
        docs_url: "https://google-gemini.github.io/gemini-cli/docs/get-started/".to_string(),
        summary: "Gemini CLI settings, project memory, MCP configuration, and terminal workflows."
            .to_string(),
        paths: vec![
            path_status("User settings", home.join(".gemini/settings.json"), "file"),
            path_status("User config directory", home.join(".gemini"), "directory"),
            path_status("System settings", gemini_system_settings_path(), "file"),
            path_status("System defaults", gemini_system_defaults_path(), "file"),
        ],
    }
}

fn claude_project_coverage(project_root: &Path) -> ProjectRuntimeCoverage {
    let surfaces = vec![
        path_status("CLAUDE.md", project_root.join("CLAUDE.md"), "file"),
        path_status(
            "Project settings",
            project_root.join(".claude/settings.json"),
            "file",
        ),
        path_status(
            "Local settings",
            project_root.join(".claude/settings.local.json"),
            "file",
        ),
        path_status(
            "Project skills",
            project_root.join(".claude/skills"),
            "directory",
        ),
        path_status(
            "Project commands",
            project_root.join(".claude/commands"),
            "directory",
        ),
        path_status(
            "Project agents",
            project_root.join(".claude/agents"),
            "directory",
        ),
        path_status("Project MCP", project_root.join(".mcp.json"), "file"),
    ];
    let found_count = found_surface_count(&surfaces);
    let total_count = surfaces.len();
    let summary = if found_count > 0 {
        format!("Detected {found_count} of {total_count} Claude Code project surfaces.")
    } else {
        "No Claude Code project files yet. TARS can scaffold them from this workspace.".to_string()
    };

    ProjectRuntimeCoverage {
        id: "claude-code".to_string(),
        name: "Claude Code".to_string(),
        support: "Native".to_string(),
        summary,
        surfaces,
    }
}

fn codex_project_coverage(project_root: &Path) -> ProjectRuntimeCoverage {
    let surfaces = vec![
        path_status("AGENTS.md", project_root.join("AGENTS.md"), "file"),
        path_status(
            "AGENTS.override.md",
            project_root.join("AGENTS.override.md"),
            "file",
        ),
        path_status(
            "Project config",
            project_root.join(".codex/config.toml"),
            "file",
        ),
        path_status(
            "Custom agents",
            project_root.join(".codex/agents"),
            "directory",
        ),
        path_status(
            "Project skills",
            project_root.join(".agents/skills"),
            "directory",
        ),
        path_status(
            "Project marketplace",
            project_root.join(".agents/plugins/marketplace.json"),
            "file",
        ),
        path_status("Project plugins", project_root.join("plugins"), "directory"),
    ];
    let found_count = found_surface_count(&surfaces);
    let total_count = surfaces.len();
    let (support, summary) = if found_count > 0 {
        (
            "Native".to_string(),
            format!("Detected {found_count} of {total_count} Codex project surfaces."),
        )
    } else {
        (
            "Convertible".to_string(),
            "No Codex project files yet. Existing Claude tools can roll forward into Codex surfaces."
                .to_string(),
        )
    };

    ProjectRuntimeCoverage {
        id: "codex".to_string(),
        name: "Codex".to_string(),
        support,
        summary,
        surfaces,
    }
}

fn gemini_project_coverage(project_root: &Path) -> ProjectRuntimeCoverage {
    let surfaces = vec![
        path_status(
            "Project settings",
            project_root.join(".gemini/settings.json"),
            "file",
        ),
        path_status("Project context", project_root.join("GEMINI.md"), "file"),
        path_status(
            "Project config directory",
            project_root.join(".gemini"),
            "directory",
        ),
    ];
    let found_count = found_surface_count(&surfaces);
    let total_count = surfaces.len();
    let (support, summary) = if found_count > 0 {
        (
            "Native".to_string(),
            format!("Detected {found_count} of {total_count} Gemini CLI project surfaces."),
        )
    } else {
        (
            "Native".to_string(),
            "No Gemini CLI project files yet. TARS can still surface the runtime and its user settings."
                .to_string(),
        )
    };

    ProjectRuntimeCoverage {
        id: "gemini-cli".to_string(),
        name: "Gemini CLI".to_string(),
        support,
        summary,
        surfaces,
    }
}

fn codex_system_config_path() -> PathBuf {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        PathBuf::from("/etc/codex/config.toml")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        PathBuf::new()
    }
}

fn codex_managed_config_path(_home: &Path) -> PathBuf {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        PathBuf::from("/etc/codex/managed_config.toml")
    }
    #[cfg(target_os = "windows")]
    {
        home.join(".codex").join("managed_config.toml")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::new()
    }
}

fn gemini_system_settings_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/GeminiCli/settings.json")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/etc/gemini-cli/settings.json")
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\ProgramData\gemini-cli\settings.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::new()
    }
}

fn gemini_system_defaults_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/GeminiCli/system-defaults.json")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/etc/gemini-cli/system-defaults.json")
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\ProgramData\gemini-cli\system-defaults.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::new()
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

fn path_status(label: &str, path: impl AsRef<Path>, kind: &str) -> RuntimePathStatus {
    let path = path.as_ref();
    RuntimePathStatus {
        label: label.to_string(),
        exists: path.exists(),
        path: path.display().to_string(),
        kind: kind.to_string(),
    }
}

fn found_surface_count(surfaces: &[RuntimePathStatus]) -> usize {
    surfaces.iter().filter(|surface| surface.exists).count()
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

fn gemini_candidates(home: &Path) -> Vec<PathBuf> {
    let mut candidates = common_node_candidates(home, "gemini");
    candidates.extend([
        home.join(".local/bin/gemini"),
        home.join(".cargo/bin/gemini"),
        PathBuf::from("/usr/local/bin/gemini"),
        PathBuf::from("/opt/homebrew/bin/gemini"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin/gemini"),
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

fn detect_install_method(path: &Path, runtime_id: &str) -> Option<String> {
    let normalized = path.to_string_lossy().replace('\\', "/").to_lowercase();
    let resolved = path
        .canonicalize()
        .ok()
        .map(|resolved| resolved.to_string_lossy().replace('\\', "/").to_lowercase());
    let matches = |needle: &str| {
        normalized.contains(needle)
            || resolved
                .as_ref()
                .is_some_and(|candidate| candidate.contains(needle))
    };

    if matches("/opt/homebrew/") || matches("/home/linuxbrew/.linuxbrew/") || matches("/cellar/") {
        return Some("Homebrew".to_string());
    }

    if matches("/appdata/roaming/npm/") || matches("/.npm-global/") || matches("/.local/bin/") {
        return Some("npm".to_string());
    }

    if matches("/.local/share/pnpm/") {
        return Some("pnpm".to_string());
    }

    if matches("/.yarn/") || matches("/.config/yarn/global/node_modules/.bin/") {
        return Some("Yarn".to_string());
    }

    if matches("/.volta/") {
        return Some("Volta".to_string());
    }

    if matches("/.nvm/versions/node/") {
        return Some("npm".to_string());
    }

    if matches("/.asdf/shims/") {
        return Some("asdf".to_string());
    }

    if matches("/.local/share/mise/shims/") {
        return Some("mise".to_string());
    }

    if matches("/snap/bin/") {
        return Some("Snap".to_string());
    }

    if matches("/scoop/shims/") {
        return Some("Scoop".to_string());
    }

    if matches("/chocolatey/bin/") {
        return Some("Chocolatey".to_string());
    }

    if runtime_id == "claude-code"
        && (normalized.contains("/programs/claude/")
            || normalized.contains("/programs/claude code/")
            || normalized.ends_with("/claude.exe"))
    {
        return Some("Standalone installer".to_string());
    }

    None
}
