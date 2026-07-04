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

/// Scan one source directory for immediate `<name>/SKILL.md` bundles.
///
/// Entries without a parseable `SKILL.md` (missing file, no frontmatter, or a
/// missing required `name`/`description`) are silently skipped, matching how
/// the scanner treats malformed skills. Result is sorted by name.
pub fn scan_source(source_dir: &Path) -> Vec<CatalogSkill> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(source_dir) else {
        return out;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let skill_md = dir.join("SKILL.md");
        let Ok(content) = fs::read_to_string(&skill_md) else {
            continue;
        };
        if let Ok(info) = parse_skill(&skill_md, &content, ScanScope::User) {
            out.push(CatalogSkill {
                name: info.name,
                description: info.description,
                source_dir: dir,
                sha256: info.sha256,
            });
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
