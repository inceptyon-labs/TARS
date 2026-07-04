//! Materialize a library skill into an agent's skills directory, and remove it.
//!
//! A skill is a directory containing a `SKILL.md`. Both Claude Code and Codex
//! discover skills by directory presence and follow symlinks, so a deployment
//! is just a symlink (default) — or a copy, for shared repos where a
//! machine-local symlink would be useless to teammates — from the agent's
//! skills directory to the canonical skill folder in the library.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

/// Target agent for a deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    Claude,
    Codex,
}

/// Target scope for a deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    Project,
}

/// How a skill is materialized into a target directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    Symlink,
    Copy,
}

impl Agent {
    /// String form stored in the `skill_deployments.agent` column.
    pub fn as_str(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "claude" => Some(Agent::Claude),
            "codex" => Some(Agent::Codex),
            _ => None,
        }
    }
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Project => "project",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Scope::User),
            "project" => Some(Scope::Project),
            _ => None,
        }
    }
}

impl LinkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkKind::Symlink => "symlink",
            LinkKind::Copy => "copy",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "symlink" => Some(LinkKind::Symlink),
            "copy" => Some(LinkKind::Copy),
            _ => None,
        }
    }
}

/// Errors from materializing or removing a deployment.
#[derive(Debug, thiserror::Error)]
pub enum SkillDeployError {
    #[error("skill source not found: {0}")]
    SourceNotFound(PathBuf),
    #[error("a skill named '{name}' already exists at {path} (not created by TARS)")]
    TargetExists { name: String, path: PathBuf },
    #[error("refusing to remove {0}: expected a TARS-created symlink but found a real file/dir")]
    NotASymlink(PathBuf),
    #[error("a project root is required for project scope")]
    ProjectRootRequired,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

/// Outcome of a successful [`deploy`], ready to persist as a
/// [`SkillDeploymentInput`](crate::storage::skill_library::SkillDeploymentInput).
#[derive(Debug, Clone)]
pub struct DeployResult {
    pub skill_name: String,
    pub source_path: PathBuf,
    pub link_path: PathBuf,
    pub link_kind: LinkKind,
    pub sha256: Option<String>,
}

/// Resolve the agent's skills directory for a target.
///
/// Claude uses `.claude/skills`; Codex user scope is detected
/// ([`codex_user_skills_dir`]) and Codex project scope is the tool-neutral
/// `.agents/skills`. `home` is the user's home directory (injected for tests).
pub fn resolve_skills_dir(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    home: &Path,
) -> Result<PathBuf, SkillDeployError> {
    match scope {
        Scope::User => Ok(match agent {
            Agent::Claude => home.join(".claude").join("skills"),
            Agent::Codex => codex_user_skills_dir(home),
        }),
        Scope::Project => {
            let root = project_root.ok_or(SkillDeployError::ProjectRootRequired)?;
            Ok(match agent {
                Agent::Claude => root.join(".claude").join("skills"),
                Agent::Codex => root.join(".agents").join("skills"),
            })
        }
    }
}

/// Detect the Codex user skills directory.
///
/// Codex builds disagree on the path: current docs use `~/.agents/skills`, but
/// shipped desktop builds read `~/.codex/skills`. Prefer whichever already
/// exists; otherwise pick `~/.codex/skills` when a Codex home is present, else
/// fall back to the documented `~/.agents/skills`.
pub fn codex_user_skills_dir(home: &Path) -> PathBuf {
    let codex = home.join(".codex").join("skills");
    let agents = home.join(".agents").join("skills");
    if codex.is_dir() {
        codex
    } else if agents.is_dir() {
        agents
    } else if home.join(".codex").is_dir() {
        codex
    } else {
        agents
    }
}

/// Materialize `source_skill_dir` into the target agent's skills directory as
/// `skill_name`.
///
/// Refuses to overwrite an existing entry (so a pre-existing skill the user
/// placed by hand is never clobbered). The caller is responsible for recording
/// the returned [`DeployResult`] in the deployment store.
pub fn deploy(
    source_skill_dir: &Path,
    skill_name: &str,
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    link_kind: LinkKind,
    home: &Path,
) -> Result<DeployResult, SkillDeployError> {
    if !source_skill_dir.is_dir() {
        return Err(SkillDeployError::SourceNotFound(
            source_skill_dir.to_path_buf(),
        ));
    }

    let dir = resolve_skills_dir(agent, scope, project_root, home)?;
    fs::create_dir_all(&dir)?;
    let link_path = dir.join(skill_name);

    // symlink_metadata does not follow the link, so a dangling symlink also
    // counts as "exists" — we never silently replace anything.
    if link_path.symlink_metadata().is_ok() {
        return Err(SkillDeployError::TargetExists {
            name: skill_name.to_string(),
            path: link_path,
        });
    }

    match link_kind {
        LinkKind::Symlink => make_symlink(source_skill_dir, &link_path)?,
        LinkKind::Copy => copy_dir(source_skill_dir, &link_path)?,
    }

    // Only copies can drift from the source; symlinks are the source.
    let sha256 = match link_kind {
        LinkKind::Copy => hash_skill_md(source_skill_dir),
        LinkKind::Symlink => None,
    };

    Ok(DeployResult {
        skill_name: skill_name.to_string(),
        source_path: source_skill_dir.to_path_buf(),
        link_path,
        link_kind,
        sha256,
    })
}

/// Remove a materialized deployment at `link_path`.
///
/// Idempotent: a missing path is treated as already removed. For symlink
/// deployments this refuses to delete anything that is not a symlink, so a real
/// skill directory can never be destroyed by an "off" toggle.
pub fn undeploy(link_path: &Path, link_kind: LinkKind) -> Result<(), SkillDeployError> {
    let meta = match link_path.symlink_metadata() {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    if meta.file_type().is_symlink() {
        remove_symlink(link_path)?;
    } else if link_kind == LinkKind::Copy && meta.is_dir() {
        fs::remove_dir_all(link_path)?;
    } else {
        return Err(SkillDeployError::NotASymlink(link_path.to_path_buf()));
    }

    Ok(())
}

#[cfg(unix)]
fn make_symlink(source: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[cfg(windows)]
fn make_symlink(source: &Path, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(source, link)
}

#[cfg(unix)]
fn remove_symlink(link: &Path) -> io::Result<()> {
    fs::remove_file(link)
}

#[cfg(windows)]
fn remove_symlink(link: &Path) -> io::Result<()> {
    fs::remove_dir(link)
}

/// Recursively copy a skill directory, skipping any nested symlinks.
fn copy_dir(src: &Path, dst: &Path) -> Result<(), SkillDeployError> {
    for entry in WalkDir::new(src).follow_links(false) {
        let entry = entry.map_err(io::Error::other)?;
        let rel = entry.path().strip_prefix(src).map_err(io::Error::other)?;
        let target = dst.join(rel);
        let file_type = entry.file_type();
        if file_type.is_dir() {
            fs::create_dir_all(&target)?;
        } else if file_type.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
        // Symlinks inside a skill bundle are intentionally skipped.
    }
    Ok(())
}

fn hash_skill_md(skill_dir: &Path) -> Option<String> {
    let content = fs::read(skill_dir.join("SKILL.md")).ok()?;
    Some(hex::encode(Sha256::digest(&content)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_skill(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: x\ndescription: y\n---\nbody\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn symlink_roundtrip_leaves_source_intact() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let lib = tmp.path().join("lib");
        fs::create_dir_all(&home).unwrap();
        let src = make_skill(&lib, "deep-research");

        let res = deploy(
            &src,
            "deep-research",
            Agent::Claude,
            Scope::User,
            None,
            LinkKind::Symlink,
            &home,
        )
        .unwrap();

        assert_eq!(
            res.link_path,
            home.join(".claude").join("skills").join("deep-research")
        );
        assert!(res
            .link_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        // Reading through the link reaches the source content.
        assert!(res.link_path.join("SKILL.md").exists());
        assert!(res.sha256.is_none());

        undeploy(&res.link_path, LinkKind::Symlink).unwrap();
        assert!(res.link_path.symlink_metadata().is_err());
        // Source survives the undeploy.
        assert!(src.join("SKILL.md").exists());
    }

    #[test]
    fn copy_roundtrip_hashes_and_removes_copy() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let lib = tmp.path().join("lib");
        fs::create_dir_all(&home).unwrap();
        let src = make_skill(&lib, "handoff");

        let res = deploy(
            &src,
            "handoff",
            Agent::Codex,
            Scope::User,
            None,
            LinkKind::Copy,
            &home,
        )
        .unwrap();

        // Nothing pre-exists, so Codex user scope falls back to .agents/skills.
        assert_eq!(
            res.link_path,
            home.join(".agents").join("skills").join("handoff")
        );
        assert!(!res
            .link_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(res.link_path.join("SKILL.md").exists());
        assert!(res.sha256.is_some());

        undeploy(&res.link_path, LinkKind::Copy).unwrap();
        assert!(!res.link_path.exists());
        assert!(src.join("SKILL.md").exists());
    }

    fn make_skill_with_extras(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        fs::create_dir_all(dir.join("scripts")).unwrap();
        fs::create_dir_all(dir.join("references")).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: x\ndescription: y\n---\nbody\n",
        )
        .unwrap();
        fs::write(dir.join("scripts").join("run.sh"), "#!/bin/sh\necho hi\n").unwrap();
        fs::write(dir.join("references").join("guide.md"), "# guide\n").unwrap();
        dir
    }

    #[test]
    fn deploy_materializes_the_whole_skill_bundle() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let lib = tmp.path().join("lib");
        fs::create_dir_all(&home).unwrap();
        let src = make_skill_with_extras(&lib, "rich");

        // Symlink: the whole tree is reachable through the link.
        let link = deploy(
            &src,
            "rich",
            Agent::Claude,
            Scope::User,
            None,
            LinkKind::Symlink,
            &home,
        )
        .unwrap();
        assert!(link.link_path.join("scripts/run.sh").exists());
        assert!(link.link_path.join("references/guide.md").exists());
        undeploy(&link.link_path, LinkKind::Symlink).unwrap();

        // Copy: supporting files are copied recursively, contents intact.
        let copied = deploy(
            &src,
            "rich",
            Agent::Codex,
            Scope::User,
            None,
            LinkKind::Copy,
            &home,
        )
        .unwrap();
        assert!(copied.link_path.join("references/guide.md").exists());
        assert_eq!(
            fs::read_to_string(copied.link_path.join("scripts/run.sh")).unwrap(),
            "#!/bin/sh\necho hi\n"
        );
    }

    #[test]
    fn refuses_to_clobber_existing_target() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let lib = tmp.path().join("lib");
        fs::create_dir_all(&home).unwrap();
        let src = make_skill(&lib, "dupe");

        deploy(
            &src,
            "dupe",
            Agent::Claude,
            Scope::User,
            None,
            LinkKind::Symlink,
            &home,
        )
        .unwrap();
        let again = deploy(
            &src,
            "dupe",
            Agent::Claude,
            Scope::User,
            None,
            LinkKind::Symlink,
            &home,
        );
        assert!(matches!(again, Err(SkillDeployError::TargetExists { .. })));
    }

    #[test]
    fn undeploy_refuses_to_delete_a_real_dir_for_symlink_kind() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join(".claude").join("skills").join("hand-made");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("SKILL.md"), "real").unwrap();

        let err = undeploy(&real, LinkKind::Symlink).unwrap_err();
        assert!(matches!(err, SkillDeployError::NotASymlink(_)));
        // The real directory is untouched.
        assert!(real.join("SKILL.md").exists());
    }

    #[test]
    fn project_scope_requires_root() {
        let home = Path::new("/tmp/home");
        assert!(matches!(
            resolve_skills_dir(Agent::Claude, Scope::Project, None, home),
            Err(SkillDeployError::ProjectRootRequired)
        ));
    }

    #[test]
    fn resolves_expected_dirs() {
        let home = Path::new("/home/j");
        let repo = Path::new("/repo");
        assert_eq!(
            resolve_skills_dir(Agent::Claude, Scope::User, None, home).unwrap(),
            Path::new("/home/j/.claude/skills")
        );
        assert_eq!(
            resolve_skills_dir(Agent::Claude, Scope::Project, Some(repo), home).unwrap(),
            Path::new("/repo/.claude/skills")
        );
        assert_eq!(
            resolve_skills_dir(Agent::Codex, Scope::Project, Some(repo), home).unwrap(),
            Path::new("/repo/.agents/skills")
        );
    }

    #[test]
    fn codex_dir_detection_precedence() {
        let tmp = TempDir::new().unwrap();

        // Empty home → documented .agents/skills default.
        let empty = tmp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        assert_eq!(codex_user_skills_dir(&empty), empty.join(".agents/skills"));

        // Codex home present but no skills dir → .codex/skills.
        let codex_home = tmp.path().join("codex");
        fs::create_dir_all(codex_home.join(".codex")).unwrap();
        assert_eq!(
            codex_user_skills_dir(&codex_home),
            codex_home.join(".codex/skills")
        );

        // Existing .codex/skills wins outright.
        let existing = tmp.path().join("existing");
        fs::create_dir_all(existing.join(".codex/skills")).unwrap();
        fs::create_dir_all(existing.join(".agents/skills")).unwrap();
        assert_eq!(
            codex_user_skills_dir(&existing),
            existing.join(".codex/skills")
        );
    }
}
