//! Add skills to the external library (`~/.agents/skills`): import a local
//! folder, or clone a git repo — the `npx skills add <url>` flow, natively.
//!
//! Cloned repos are only ever *copied* from — nothing in a fetched repo is
//! executed. The URL is validated and passed to `git` as a plain argument
//! (never through a shell), after a `--` terminator.

use std::path::PathBuf;

use tars_core::skills::{
    adopt_resident_skill, external_skills_dir, install_bundles, parse_git_skill_url,
    SkillInstallReport,
};

/// Copy the skill bundle(s) found in a user-picked folder into
/// `~/.agents/skills`. Never overwrites an existing skill.
#[tauri::command]
pub async fn import_skill_folder(path: String) -> Result<SkillInstallReport, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    install_bundles(&dir, &external_skills_dir(&home)).map_err(|e| e.to_string())
}

/// Move a resident skill out of an agent's own skills dir into
/// `~/.agents/skills`, leaving a symlink behind so the agent keeps loading
/// it. Returns the new library path.
#[tauri::command]
pub async fn adopt_skill(source_dir: String) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let dest = adopt_resident_skill(
        std::path::Path::new(&source_dir),
        &external_skills_dir(&home),
    )
    .map_err(|e| e.to_string())?;
    Ok(dest.display().to_string())
}

/// Shallow-clone an https git repo (optionally a `/tree/<ref>/<subpath>` URL)
/// and install every skill bundle found into `~/.agents/skills`.
#[tauri::command]
pub async fn install_skill_from_git(url: String) -> Result<SkillInstallReport, String> {
    let source = parse_git_skill_url(&url).map_err(|e| e.to_string())?;
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;

    let tmp = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let checkout = tmp.path().join("repo");

    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1").arg("--quiet");
    if let Some(reference) = &source.reference {
        cmd.arg("--branch").arg(reference);
    }
    cmd.arg("--").arg(&source.repo_url).arg(&checkout);

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run git: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {}", stderr.trim()));
    }

    let root = match &source.subpath {
        Some(sub) => checkout.join(sub),
        None => checkout.clone(),
    };
    if !root.is_dir() {
        return Err(format!(
            "Path not found in repository: {}",
            source.subpath.as_deref().unwrap_or("")
        ));
    }

    install_bundles(&root, &external_skills_dir(&home)).map_err(|e| e.to_string())
}
