//! Architecture diagram commands
//!
//! Lists Archify diagrams generated under a project's `.archify/` directory.
//! Each Archify run writes `.archify/<type>-<slug>-<YYYYMMDD-HHMMSS>/` with a
//! `candidate.json` spec and the rendered `<slug>.html`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

const ARCHIFY_DIR: &str = ".archify";

/// Latest version of one Archify diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDiagram {
    pub title: String,
    pub diagram_type: Option<String>,
    pub html_path: String,
    /// Unix seconds of the HTML file's last modification
    pub modified_at: u64,
    /// Number of runs found for this diagram (including the latest)
    pub versions: usize,
}

#[derive(Deserialize, Default)]
struct Candidate {
    diagram_type: Option<String>,
    #[serde(default)]
    meta: CandidateMeta,
}

#[derive(Deserialize, Default)]
struct CandidateMeta {
    title: Option<String>,
    output: Option<String>,
}

/// Strip the trailing `-YYYYMMDD-HHMMSS` run stamp so reruns group together.
fn strip_run_stamp(name: &str) -> &str {
    let bytes = name.as_bytes();
    let Some(start) = bytes.len().checked_sub(16) else {
        return name;
    };
    let is_stamp = bytes[start..].iter().enumerate().all(|(i, c)| match i {
        0 | 9 => *c == b'-',
        _ => c.is_ascii_digit(),
    });
    if is_stamp && start > 0 {
        &name[..start]
    } else {
        name
    }
}

fn find_diagram_html(dir: &Path, output: Option<&str>) -> Option<PathBuf> {
    if let Some(file_name) = output.and_then(|o| Path::new(o).file_name()) {
        let path = dir.join(file_name);
        if path.is_file() {
            return Some(path);
        }
    }
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "html"))
        .min()
}

fn modified_secs(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

fn scan_diagrams(project: &Path) -> Vec<ProjectDiagram> {
    let Ok(entries) = fs::read_dir(project.join(ARCHIFY_DIR)) else {
        return Vec::new();
    };

    let mut latest: HashMap<String, ProjectDiagram> = HashMap::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let candidate: Candidate = fs::read_to_string(dir.join("candidate.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let Some(html) = find_diagram_html(&dir, candidate.meta.output.as_deref()) else {
            continue;
        };

        let diagram = ProjectDiagram {
            title: candidate.meta.title.unwrap_or_else(|| {
                html.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(name)
                    .to_string()
            }),
            diagram_type: candidate.diagram_type,
            modified_at: modified_secs(&html),
            html_path: html.to_string_lossy().to_string(),
            versions: 1,
        };

        match latest.get_mut(strip_run_stamp(name)) {
            Some(existing) => {
                let versions = existing.versions + 1;
                if diagram.modified_at > existing.modified_at {
                    *existing = diagram;
                }
                existing.versions = versions;
            }
            None => {
                latest.insert(strip_run_stamp(name).to_string(), diagram);
            }
        }
    }

    let mut diagrams: Vec<ProjectDiagram> = latest.into_values().collect();
    diagrams.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.title.cmp(&b.title))
    });
    diagrams
}

/// List the latest version of each Archify diagram in a project
#[tauri::command]
pub async fn list_project_diagrams(project_path: String) -> Result<Vec<ProjectDiagram>, String> {
    Ok(scan_diagrams(Path::new(&project_path)))
}

/// Open a project's Archify diagram in the default browser
#[tauri::command]
pub async fn open_project_diagram(
    app: AppHandle,
    project_path: String,
    html_path: String,
) -> Result<(), String> {
    let archify_root = Path::new(&project_path)
        .join(ARCHIFY_DIR)
        .canonicalize()
        .map_err(|e| format!("No diagrams folder: {e}"))?;
    let html = Path::new(&html_path)
        .canonicalize()
        .map_err(|e| format!("Diagram not found: {e}"))?;

    if !html.starts_with(&archify_root) || html.extension().is_none_or(|ext| ext != "html") {
        return Err("Path is not an Archify diagram in this project".to_string());
    }

    app.opener()
        .open_path(html.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Failed to open diagram: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_run_stamp() {
        assert_eq!(
            strip_run_stamp("architecture-newman-overview-20261003-190235"),
            "architecture-newman-overview"
        );
        assert_eq!(strip_run_stamp("custom-folder"), "custom-folder");
        assert_eq!(strip_run_stamp("-20261003-190235"), "-20261003-190235");
        assert_eq!(
            strip_run_stamp("é-overview-2026100x-190235"),
            "é-overview-2026100x-190235"
        );
    }

    #[test]
    fn test_scan_keeps_latest_run_per_diagram() {
        let tmp = tempfile::tempdir().unwrap();
        let archify = tmp.path().join(ARCHIFY_DIR);
        let write_run = |folder: &str, title: &str| {
            let dir = archify.join(folder);
            fs::create_dir_all(dir.join("visual-check")).unwrap();
            fs::write(
                dir.join("candidate.json"),
                format!(
                    r#"{{"diagram_type":"architecture","meta":{{"title":"{title}","output":".archify/{folder}/app-overview.html"}}}}"#
                ),
            )
            .unwrap();
            fs::write(dir.join("app-overview.html"), "<html></html>").unwrap();
            fs::write(dir.join("visual-check/check.html"), "<html></html>").unwrap();
        };

        write_run("architecture-app-overview-20261001-090000", "Old");
        write_run("architecture-app-overview-20261003-190235", "New");
        fs::File::options()
            .write(true)
            .open(archify.join("architecture-app-overview-20261001-090000/app-overview.html"))
            .unwrap()
            .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000))
            .unwrap();
        fs::create_dir_all(archify.join("empty-run")).unwrap();

        let diagrams = scan_diagrams(tmp.path());
        assert_eq!(diagrams.len(), 1);
        assert_eq!(diagrams[0].title, "New");
        assert_eq!(diagrams[0].versions, 2);
        assert_eq!(diagrams[0].diagram_type.as_deref(), Some("architecture"));
        assert!(diagrams[0]
            .html_path
            .ends_with("architecture-app-overview-20261003-190235/app-overview.html"));
    }

    #[test]
    fn test_scan_without_archify_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(scan_diagrams(tmp.path()).is_empty());
    }
}
