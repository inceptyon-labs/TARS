//! Scan registered library source directories into a catalog, and probe what
//! is physically present at a deployment target.
//!
//! Scanning reuses the scanner's `SKILL.md` frontmatter parser so the catalog
//! agrees with what Claude/Codex would actually load. Probing lets the command
//! layer reconcile TARS's deployment records against the filesystem — in
//! particular, adopting symlinks a user created by hand (e.g. an existing
//! `~/.codex/skills` pointing into a library) as already-deployed.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tars_scanner::parser::parse_skill;
use tars_scanner::types::Scope as ScanScope;

/// A skill discovered in a library source directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSkill {
    pub name: String,
    pub description: String,
    /// The `<name>/` directory holding `SKILL.md` — the symlink/copy source.
    pub source_dir: PathBuf,
    pub sha256: String,
}

/// Read a single skill bundle (`<dir>/SKILL.md`) into a [`CatalogSkill`].
/// Returns `None` if there's no parseable `SKILL.md`.
fn read_skill_dir(dir: &Path) -> Option<CatalogSkill> {
    let skill_md = dir.join("SKILL.md");
    let content = fs::read_to_string(&skill_md).ok()?;
    let info = parse_skill(&skill_md, &content, ScanScope::User).ok()?;
    Some(CatalogSkill {
        name: info.name,
        description: info.description,
        source_dir: dir.to_path_buf(),
        sha256: info.sha256,
    })
}

/// Scan a source directory into a catalog.
///
/// Handles both shapes: a directory that *is* a single skill bundle
/// (`<dir>/SKILL.md`), and a directory that *contains* skill bundles
/// (`<dir>/<name>/SKILL.md`). Entries without a parseable `SKILL.md` (missing
/// file, no frontmatter, or a missing required `name`/`description`) are
/// silently skipped. Result is sorted by name.
pub fn scan_source(source_dir: &Path) -> Vec<CatalogSkill> {
    // The source directory may itself be a single skill bundle.
    if source_dir.join("SKILL.md").is_file() {
        return read_skill_dir(source_dir).into_iter().collect();
    }

    // Otherwise, each immediate subdirectory with a SKILL.md is a skill.
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(source_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if dir.is_dir() {
            if let Some(skill) = read_skill_dir(&dir) {
                out.push(skill);
            }
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Directory-name prefixes TARS itself materializes inside the external
/// skills dir (Codex plugin bridges) — managed artifacts, not resident skills.
pub const TARS_MANAGED_PREFIXES: [&str; 2] = ["tars-claude-", "tars-skill-"];

/// Scan an externally-managed skills directory (`~/.agents/skills`) — where
/// `npx skills add` and hand-copies land — into a catalog.
///
/// Unlike [`scan_source`], this skips symlinked entries (deploys pointing back
/// into a library), dot-directories, and TARS's own managed bridge dirs
/// ([`TARS_MANAGED_PREFIXES`]), so only skills that genuinely *live* here
/// surface. Result is sorted by name.
pub fn scan_external_dir(dir: &Path) -> Vec<CatalogSkill> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || TARS_MANAGED_PREFIXES.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let is_symlink = entry.file_type().is_ok_and(|t| t.is_symlink());
        if is_symlink {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            if let Some(skill) = read_skill_dir(&path) {
                out.push(skill);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Scan multiple source directories into one catalog, sorted by name.
pub fn scan_sources(dirs: &[PathBuf]) -> Vec<CatalogSkill> {
    let mut out: Vec<CatalogSkill> = dirs.iter().flat_map(|d| scan_source(d)).collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// What is physically present at a deployment target path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetProbe {
    /// Nothing there.
    Absent,
    /// A symlink; `target` is resolved to an absolute path (best effort).
    Symlink { target: PathBuf },
    /// A real directory — a copied deploy, or a hand-made / foreign skill.
    Directory,
    /// Something else (a plain file, etc.).
    Other,
}

/// Inspect what occupies a target path without following symlinks.
pub fn probe_target(link_path: &Path) -> TargetProbe {
    let Ok(meta) = link_path.symlink_metadata() else {
        return TargetProbe::Absent;
    };

    if meta.file_type().is_symlink() {
        let raw = fs::read_link(link_path).unwrap_or_default();
        let target = if raw.is_relative() {
            link_path
                .parent()
                .map_or_else(|| raw.clone(), |parent| parent.join(&raw))
        } else {
            raw
        };
        TargetProbe::Symlink { target }
    } else if meta.is_dir() {
        TargetProbe::Directory
    } else {
        TargetProbe::Other
    }
}

/// True if `link_path` is a symlink resolving to `expected_source`.
///
/// Used to recognize already-deployed skills (whether TARS or the user created
/// the link). Compares canonicalized paths, falling back to a literal compare
/// when either side cannot be canonicalized.
pub fn symlink_points_to(link_path: &Path, expected_source: &Path) -> bool {
    let TargetProbe::Symlink { target } = probe_target(link_path) else {
        return false;
    };
    match (fs::canonicalize(&target), fs::canonicalize(expected_source)) {
        (Ok(a), Ok(b)) => a == b,
        _ => target == expected_source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_skill(parent: &Path, name: &str, frontmatter: &str) -> PathBuf {
        let dir = parent.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), frontmatter).unwrap();
        dir
    }

    #[test]
    fn scans_valid_skills_and_skips_malformed() {
        let tmp = TempDir::new().unwrap();
        let lib = tmp.path();
        write_skill(lib, "beta", "---\nname: beta\ndescription: B\n---\nbody");
        write_skill(lib, "alpha", "---\nname: alpha\ndescription: A\n---\nbody");
        // No frontmatter -> skipped.
        write_skill(lib, "broken", "just prose, no frontmatter");
        // A plain dir with no SKILL.md -> skipped.
        fs::create_dir_all(lib.join("notaskill")).unwrap();

        let catalog = scan_source(lib);
        let names: Vec<_> = catalog.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["alpha", "beta"]); // sorted, malformed dropped
        assert_eq!(catalog[0].source_dir, lib.join("alpha"));
        assert_eq!(catalog[0].description, "A");
    }

    #[test]
    fn scans_a_dir_that_is_itself_a_skill() {
        let tmp = TempDir::new().unwrap();
        // The source dir holds SKILL.md directly (a single-skill bundle).
        fs::write(
            tmp.path().join("SKILL.md"),
            "---\nname: humanizer\ndescription: H\n---\nbody",
        )
        .unwrap();

        let catalog = scan_source(tmp.path());
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].name, "humanizer");
        assert_eq!(catalog[0].source_dir, tmp.path());
    }

    #[test]
    fn scan_sources_flattens_and_sorts() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("srcA");
        let b = tmp.path().join("srcB");
        write_skill(&a, "zed", "---\nname: zed\ndescription: Z\n---\n");
        write_skill(&b, "ada", "---\nname: ada\ndescription: A\n---\n");

        let catalog = scan_sources(&[a, b]);
        let names: Vec<_> = catalog.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["ada", "zed"]);
    }

    #[test]
    fn external_scan_skips_symlinks_dotdirs_and_tars_bridges() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let real = write_skill(
            dir,
            "resident",
            "---\nname: resident\ndescription: R\n---\n",
        );
        write_skill(
            dir,
            "tars-claude-abc123--bridged",
            "---\nname: bridged\ndescription: B\n---\n",
        );
        write_skill(dir, ".hidden", "---\nname: hidden\ndescription: H\n---\n");

        // A symlinked entry (a TARS deploy pointing back into a library).
        let lib_skill = write_skill(
            &tmp.path().join("lib"),
            "linked",
            "---\nname: linked\ndescription: L\n---\n",
        );
        #[cfg(unix)]
        std::os::unix::fs::symlink(&lib_skill, dir.join("linked")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&lib_skill, dir.join("linked")).unwrap();

        let catalog = scan_external_dir(dir);
        let names: Vec<_> = catalog.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["resident"]);
        assert_eq!(catalog[0].source_dir, real);
    }

    #[test]
    fn probe_distinguishes_absent_symlink_and_dir() {
        let tmp = TempDir::new().unwrap();
        let src = write_skill(tmp.path(), "real", "---\nname: real\ndescription: R\n---\n");
        let link = tmp.path().join("link");

        assert_eq!(probe_target(&link), TargetProbe::Absent);

        #[cfg(unix)]
        std::os::unix::fs::symlink(&src, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&src, &link).unwrap();

        assert!(matches!(probe_target(&link), TargetProbe::Symlink { .. }));
        assert!(symlink_points_to(&link, &src));
        assert!(!symlink_points_to(&link, tmp.path())); // points elsewhere
        assert_eq!(probe_target(&src), TargetProbe::Directory);
    }
}
