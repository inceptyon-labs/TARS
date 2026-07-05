//! Install skill bundles into the tool-neutral external skills directory
//! (`~/.agents/skills`) — the same destination `npx skills add` uses — from a
//! local folder or a cloned git checkout.
//!
//! The command layer performs the clone; everything here is pure filesystem
//! work plus URL parsing, so it can be exercised with tempdirs.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use super::deploy::{copy_dir, make_symlink, SkillDeployError};
use super::scan::scan_source;

/// Where externally-installed standalone skills live.
pub fn external_skills_dir(home: &Path) -> PathBuf {
    home.join(".agents").join("skills")
}

#[derive(Debug, Error)]
pub enum SkillInstallError {
    #[error("no skill found: no SKILL.md in {0} or its subdirectories")]
    NoSkillFound(PathBuf),
    #[error("not a resident skill directory: {0}")]
    NotResident(PathBuf),
    #[error("'{0}' already exists in the library — remove one copy first")]
    AlreadyInLibrary(String),
    #[error("invalid skill name {0:?}")]
    InvalidName(String),
    #[error("unsupported url {0:?}: expected an https git repository url")]
    InvalidUrl(String),
    #[error(transparent)]
    Deploy(#[from] SkillDeployError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// What an install did: bundles copied in, and bundles left alone because a
/// same-named entry already exists (we never overwrite).
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInstallReport {
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
}

/// Find skill bundle directories (containing `SKILL.md`) under `root`.
///
/// Does not descend into a found bundle, dot-directories, `node_modules`, or
/// symlinks; depth-limited so a huge repo can't be walked endlessly. `root`
/// itself may be a bundle. Result is sorted.
pub fn find_skill_bundles(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, 0, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 4 {
        return;
    }
    if dir.join("SKILL.md").is_file() {
        out.push(dir.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        if entry.file_type().is_ok_and(|t| t.is_symlink()) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk(&path, depth + 1, out);
        }
    }
}

/// Copy every skill bundle found under `root` into `dest_root/<name>`.
///
/// Never overwrites — a same-named existing entry lands in `skipped`. Errors
/// if `root` holds no bundle at all.
pub fn install_bundles(
    root: &Path,
    dest_root: &Path,
) -> Result<SkillInstallReport, SkillInstallError> {
    let bundles = find_skill_bundles(root);
    if bundles.is_empty() {
        return Err(SkillInstallError::NoSkillFound(root.to_path_buf()));
    }
    fs::create_dir_all(dest_root)?;

    let mut report = SkillInstallReport::default();
    for bundle in bundles {
        let name = bundle_name(&bundle)?;
        let dest = dest_root.join(&name);
        // symlink_metadata doesn't follow links, so a dangling symlink also
        // counts as occupied.
        if dest.symlink_metadata().is_ok() {
            report.skipped.push(name);
            continue;
        }
        copy_dir(&bundle, &dest)?;
        report.installed.push(name);
    }
    Ok(report)
}

/// The bundle's frontmatter `name`, falling back to its directory name.
/// Must be a plain single path component.
fn bundle_name(bundle: &Path) -> Result<String, SkillInstallError> {
    let name = scan_source(bundle)
        .into_iter()
        .next()
        .map(|s| s.name)
        .or_else(|| bundle.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':']) {
        return Err(SkillInstallError::InvalidName(name));
    }
    Ok(name)
}

/// Adopt a resident skill into the library: move the bundle from an agent's
/// own skills directory into `library_dir`, leaving a symlink behind at the
/// original path so the agent keeps loading it. Returns the new library path.
///
/// Refuses symlinks (already deployed from somewhere), non-bundles, and names
/// already present in the library. Falls back to copy+delete when a plain
/// rename crosses filesystems.
pub fn adopt_resident_skill(
    resident_dir: &Path,
    library_dir: &Path,
) -> Result<PathBuf, SkillInstallError> {
    let meta = resident_dir.symlink_metadata()?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(SkillInstallError::NotResident(resident_dir.to_path_buf()));
    }
    if !resident_dir.join("SKILL.md").is_file() {
        return Err(SkillInstallError::NoSkillFound(resident_dir.to_path_buf()));
    }

    let name = bundle_name(resident_dir)?;
    let dest = library_dir.join(&name);
    if dest.symlink_metadata().is_ok() {
        return Err(SkillInstallError::AlreadyInLibrary(name));
    }
    fs::create_dir_all(library_dir)?;

    if fs::rename(resident_dir, &dest).is_err() {
        // Cross-device move: copy, then remove the original.
        copy_dir(resident_dir, &dest)?;
        fs::remove_dir_all(resident_dir)?;
    }
    make_symlink(&dest, resident_dir)?;
    Ok(dest)
}

/// Clone coordinates parsed from a user-supplied skill URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSkillSource {
    /// The URL to `git clone`.
    pub repo_url: String,
    /// Branch/tag from a `/tree/<ref>` segment, if any.
    pub reference: Option<String>,
    /// Subdirectory within the checkout to install from, if any.
    pub subpath: Option<String>,
}

/// Parse an https git URL, optionally with a GitHub-style
/// `/tree/<ref>[/<subpath>]` suffix, into clone coordinates.
///
/// Only `https://` is accepted (the URL is passed to `git` as a plain
/// argument, never through a shell), and a subpath may not traverse upward.
pub fn parse_git_skill_url(input: &str) -> Result<GitSkillSource, SkillInstallError> {
    let invalid = || SkillInstallError::InvalidUrl(input.to_string());
    let url = input.trim().trim_end_matches('/');
    let rest = url.strip_prefix("https://").ok_or_else(invalid)?;
    if rest.is_empty() || rest.chars().any(char::is_whitespace) {
        return Err(invalid());
    }
    // At least host/repo.
    if rest.split('/').filter(|s| !s.is_empty()).count() < 2 {
        return Err(invalid());
    }

    match rest.split_once("/tree/") {
        Some((repo, tail)) => {
            let (reference, subpath) = match tail.split_once('/') {
                Some((r, sub)) => (r, Some(sub.to_string())),
                None => (tail, None),
            };
            if reference.is_empty() {
                return Err(invalid());
            }
            if let Some(sub) = &subpath {
                if sub.split('/').any(|c| c.is_empty() || c == "..") {
                    return Err(invalid());
                }
            }
            Ok(GitSkillSource {
                repo_url: format!("https://{repo}"),
                reference: Some(reference.to_string()),
                subpath,
            })
        }
        None => Ok(GitSkillSource {
            repo_url: url.to_string(),
            reference: None,
            subpath: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_skill(parent: &Path, dir: &str, name: &str) -> PathBuf {
        let d = parent.join(dir);
        fs::create_dir_all(&d).unwrap();
        fs::write(
            d.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: D\n---\nbody"),
        )
        .unwrap();
        d
    }

    #[test]
    fn finds_bundles_at_root_and_nested_without_descending_into_them() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_skill(root, "alpha", "alpha");
        write_skill(root, "skills/beta", "beta");
        // Inside a bundle -> not listed separately.
        write_skill(root, "alpha/nested", "nested");
        // Skipped locations.
        write_skill(root, ".git/sneaky", "sneaky");
        write_skill(root, "node_modules/dep", "dep");

        let bundles = find_skill_bundles(root);
        assert_eq!(bundles, vec![root.join("alpha"), root.join("skills/beta")]);
    }

    #[test]
    fn root_itself_can_be_a_bundle() {
        let tmp = TempDir::new().unwrap();
        write_skill(tmp.path(), ".", "solo");
        assert_eq!(
            find_skill_bundles(tmp.path()),
            vec![tmp.path().to_path_buf()]
        );
    }

    #[test]
    fn install_copies_new_and_skips_existing() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write_skill(&src, "fresh", "fresh");
        write_skill(&src, "taken", "taken");
        fs::create_dir_all(dest.join("taken")).unwrap();

        let report = install_bundles(&src, &dest).unwrap();
        assert_eq!(report.installed, vec!["fresh"]);
        assert_eq!(report.skipped, vec!["taken"]);
        assert!(dest.join("fresh/SKILL.md").is_file());
    }

    #[test]
    fn install_uses_frontmatter_name_over_dir_name() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write_skill(&src, "some-folder", "real-name");

        let report = install_bundles(&src, &dest).unwrap();
        assert_eq!(report.installed, vec!["real-name"]);
        assert!(dest.join("real-name/SKILL.md").is_file());
    }

    #[test]
    fn install_errors_when_nothing_found() {
        let tmp = TempDir::new().unwrap();
        assert!(matches!(
            install_bundles(tmp.path(), &tmp.path().join("dest")),
            Err(SkillInstallError::NoSkillFound(_))
        ));
    }

    #[test]
    fn adopt_moves_bundle_and_leaves_symlink_behind() {
        let tmp = TempDir::new().unwrap();
        let agent_dir = tmp.path().join("claude-skills");
        let library = tmp.path().join("library");
        let resident = write_skill(&agent_dir, "acp", "acp");
        fs::write(resident.join("extra.md"), "supporting file").unwrap();

        let dest = adopt_resident_skill(&resident, &library).unwrap();
        assert_eq!(dest, library.join("acp"));
        assert!(dest.join("SKILL.md").is_file());
        assert!(dest.join("extra.md").is_file());
        // The original path is now a symlink to the library copy.
        assert!(resident
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::canonicalize(&resident).unwrap(),
            fs::canonicalize(&dest).unwrap()
        );
    }

    #[test]
    fn adopt_refuses_symlinks_and_library_collisions() {
        let tmp = TempDir::new().unwrap();
        let agent_dir = tmp.path().join("claude-skills");
        let library = tmp.path().join("library");

        // A name already in the library -> refused, original untouched.
        let taken = write_skill(&agent_dir, "taken", "taken");
        write_skill(&library, "taken", "taken");
        assert!(matches!(
            adopt_resident_skill(&taken, &library),
            Err(SkillInstallError::AlreadyInLibrary(_))
        ));
        assert!(taken.is_dir());
        assert!(!taken.symlink_metadata().unwrap().file_type().is_symlink());

        // An already-symlinked entry is not a resident.
        let real = write_skill(tmp.path(), "elsewhere", "elsewhere");
        let link = agent_dir.join("linked");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real, &link).unwrap();
        assert!(matches!(
            adopt_resident_skill(&link, &library),
            Err(SkillInstallError::NotResident(_))
        ));
    }

    #[test]
    fn parses_plain_repo_url() {
        let s = parse_git_skill_url("https://github.com/Leonxlnx/taste-skill").unwrap();
        assert_eq!(s.repo_url, "https://github.com/Leonxlnx/taste-skill");
        assert_eq!(s.reference, None);
        assert_eq!(s.subpath, None);
    }

    #[test]
    fn parses_tree_ref_and_subpath() {
        let s = parse_git_skill_url("https://github.com/o/r/tree/main/skills/foo").unwrap();
        assert_eq!(s.repo_url, "https://github.com/o/r");
        assert_eq!(s.reference.as_deref(), Some("main"));
        assert_eq!(s.subpath.as_deref(), Some("skills/foo"));

        let branch_only = parse_git_skill_url("https://github.com/o/r/tree/dev").unwrap();
        assert_eq!(branch_only.reference.as_deref(), Some("dev"));
        assert_eq!(branch_only.subpath, None);
    }

    #[test]
    fn rejects_bad_urls() {
        for bad in [
            "http://github.com/o/r",
            "git@github.com:o/r.git",
            "https://github.com",
            "https://github.com/o/r/tree/main/../../etc",
            "https://github.com/o/r bad",
            "",
        ] {
            assert!(parse_git_skill_url(bad).is_err(), "should reject {bad:?}");
        }
    }
}
