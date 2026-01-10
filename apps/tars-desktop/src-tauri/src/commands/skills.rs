//! Skill management Tauri commands
//!
//! Commands for viewing and editing skills.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tars_core::util::validate_name;

/// Skill information for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub path: String,
    pub content: String,
    pub description: Option<String>,
    pub scope: String, // "user" or "project"
    pub supporting_files: Vec<SupportingFile>,
}

/// A supporting file in a skill directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportingFile {
    /// File name (e.g., "reference.md")
    pub name: String,
    /// Full path to the file
    pub path: String,
    /// File type: "markdown", "script", "other"
    pub file_type: String,
    /// Whether this file is referenced in SKILL.md
    pub is_referenced: bool,
}

/// Read a skill file
#[tauri::command]
pub async fn read_skill(path: String) -> Result<SkillInfo, String> {
    let skill_path = PathBuf::from(&path);

    // Validate the path is within allowed skill directories
    let validated_path = validate_skill_path(&skill_path)?;

    if !validated_path.exists() {
        return Err("Skill file not found".to_string());
    }

    let content =
        std::fs::read_to_string(&validated_path).map_err(|_| "Failed to read skill".to_string())?;

    // Extract name from path
    let name = skill_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Determine scope from path (platform-aware)
    let scope = determine_skill_scope(&validated_path);

    // Try to extract description from frontmatter
    let description = extract_description(&content);

    // Scan for supporting files in the skill directory
    let supporting_files = scan_supporting_files(&validated_path, &content);

    Ok(SkillInfo {
        name,
        path,
        content,
        description,
        scope,
        supporting_files,
    })
}

/// Read a supporting file from a skill directory
#[tauri::command]
pub async fn read_supporting_file(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);

    // Validate the path is within allowed skill directories
    let validated_path = validate_skill_path(&file_path)?;

    if !validated_path.exists() {
        return Err("File not found".to_string());
    }

    std::fs::read_to_string(&validated_path).map_err(|_| "Failed to read file".to_string())
}

/// Save a skill file
#[tauri::command]
pub async fn save_skill(path: String, content: String) -> Result<(), String> {
    let skill_path = PathBuf::from(&path);

    // Validate the path is within allowed skill directories
    let validated_path = validate_skill_path(&skill_path)?;

    // Ensure parent directory exists
    if let Some(parent) = validated_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "Failed to create directory".to_string())?;
    }

    std::fs::write(&validated_path, content).map_err(|_| "Failed to save skill".to_string())?;

    Ok(())
}

/// Create a new skill
#[tauri::command]
pub async fn create_skill(
    name: String,
    scope: String,
    project_path: Option<String>,
) -> Result<SkillInfo, String> {
    // Validate the skill name to prevent path traversal
    validate_name(&name).map_err(|_| "Invalid skill name".to_string())?;

    let base_path = if scope == "user" {
        let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
        PathBuf::from(home).join(".claude/skills")
    } else {
        let project = project_path.ok_or("Project path required for project-scoped skill")?;
        PathBuf::from(project).join(".claude/skills")
    };

    let skill_dir = base_path.join(&name);
    let skill_file = skill_dir.join("SKILL.md");

    // Validate the final path is within allowed directories
    validate_skill_path(&skill_file)?;

    if skill_file.exists() {
        return Err(format!("Skill '{name}' already exists"));
    }

    // Create default skill content
    let content = format!(
        r"---
name: {name}
description: A new skill
---

# {name}

Add your skill instructions here.
",
    );

    std::fs::create_dir_all(&skill_dir)
        .map_err(|_| "Failed to create skill directory".to_string())?;
    std::fs::write(&skill_file, &content).map_err(|_| "Failed to create skill".to_string())?;

    Ok(SkillInfo {
        name,
        path: skill_file.display().to_string(),
        content,
        description: Some("A new skill".to_string()),
        scope,
        supporting_files: Vec::new(),
    })
}

/// Save a supporting file in a skill directory
/// Supports subdirectories like "scripts/helper.py"
#[tauri::command]
pub async fn save_supporting_file(
    skill_path: String,
    file_name: String,
    content: String,
) -> Result<SupportingFile, String> {
    let skill_file = PathBuf::from(&skill_path);

    // Validate the skill path
    let validated_skill = validate_skill_path(&skill_file)?;

    // Get the skill directory
    let skill_dir = validated_skill.parent().ok_or("Invalid skill path")?;

    // Validate file name (no path traversal, but allow forward slashes for subdirs)
    if file_name.contains("..") || file_name.contains('\\') {
        return Err("Invalid file name".to_string());
    }

    // Normalize path separators and build the file path
    let normalized_name = file_name.replace('\\', "/");
    let file_path = skill_dir.join(&normalized_name);

    // Validate the final path is still within the skill directory
    // Use canonicalize on parent to handle the case where file doesn't exist yet
    let canonical_skill_dir = skill_dir
        .canonicalize()
        .map_err(|_| "Invalid skill directory".to_string())?;

    // For new files, we can't canonicalize the full path, so check the parent
    if let Some(parent) = file_path.parent() {
        // Create parent directories if they don't exist
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|_| "Failed to create directory".to_string())?;
        }

        let canonical_parent = parent
            .canonicalize()
            .map_err(|_| "Invalid parent directory".to_string())?;

        if !canonical_parent.starts_with(&canonical_skill_dir) {
            return Err("Path is outside skill directory".to_string());
        }
    }

    std::fs::write(&file_path, &content).map_err(|_| "Failed to save file".to_string())?;

    // Extract just the file name for display (last component)
    let display_name = if normalized_name.contains('/') {
        normalized_name.clone()
    } else {
        file_name.clone()
    };

    let file_type = determine_file_type(&display_name);

    Ok(SupportingFile {
        name: display_name,
        path: file_path.display().to_string(),
        file_type,
        is_referenced: false,
    })
}

/// Delete a supporting file from a skill directory
#[tauri::command]
pub async fn delete_supporting_file(path: String) -> Result<(), String> {
    let file_path = PathBuf::from(&path);

    // Validate the path is within allowed skill directories
    let validated_path = validate_skill_path(&file_path)?;

    if !validated_path.exists() {
        return Err("File not found".to_string());
    }

    // Don't allow deleting SKILL.md
    let file_name = validated_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if file_name == "SKILL.md" {
        return Err("Cannot delete SKILL.md - use delete skill instead".to_string());
    }

    std::fs::remove_file(&validated_path).map_err(|_| "Failed to delete file".to_string())?;

    Ok(())
}

/// Delete a skill
#[tauri::command]
pub async fn delete_skill(path: String) -> Result<(), String> {
    let skill_path = PathBuf::from(&path);

    // Validate the path is within allowed skill directories
    let validated_path = validate_skill_path(&skill_path)?;

    if !validated_path.exists() {
        return Err("Skill not found".to_string());
    }

    // Verify this is actually a SKILL.md file to prevent deleting arbitrary directories
    let file_name = validated_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if file_name != "SKILL.md" {
        return Err("Can only delete skill files (SKILL.md)".to_string());
    }

    // Get the skill directory (parent of SKILL.md)
    let skill_dir = validated_path.parent().ok_or("Invalid skill path")?;

    // Verify the skill directory name is valid and doesn't contain traversal
    let dir_name = skill_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid skill directory")?;

    validate_name(dir_name).map_err(|_| "Invalid skill directory name".to_string())?;

    // Verify the skill directory is directly under a skills/ directory
    let skills_parent = skill_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if skills_parent != "skills" {
        return Err("Skill must be directly under a skills directory".to_string());
    }

    // Now safe to remove the skill directory
    std::fs::remove_dir_all(skill_dir).map_err(|_| "Failed to delete skill".to_string())?;

    Ok(())
}

/// Determine if a skill path is user-scoped or project-scoped
fn determine_skill_scope(path: &Path) -> String {
    // Get user home directory
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();

    if let Some(home_str) = home {
        let home_path = PathBuf::from(&home_str);
        let user_skills_dir = home_path.join(".claude").join("skills");

        // Check if path is under user's .claude/skills directory
        if let Ok(canonical_user_skills) = user_skills_dir.canonicalize() {
            if let Ok(canonical_path) = path.canonicalize() {
                if canonical_path.starts_with(&canonical_user_skills) {
                    return "user".to_string();
                }
            }
        }

        // Also check logical path (for non-existent paths)
        if path.starts_with(&user_skills_dir) {
            return "user".to_string();
        }
    }

    // Default to project scope
    "project".to_string()
}

/// Get allowed skill root directories
fn get_skill_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let claude_dir = PathBuf::from(&home).join(".claude");
        // User scope: ~/.claude/skills/
        roots.push(claude_dir.join("skills"));
        // Plugin cache: ~/.claude/plugins/cache/
        roots.push(claude_dir.join("plugins/cache"));
        // Plugin marketplaces: ~/.claude/plugins/marketplaces/
        roots.push(claude_dir.join("plugins/marketplaces"));
    }

    roots
}

/// Validate that a path is within an allowed skill directory
/// Returns the canonicalized path if valid
fn validate_skill_path(path: &Path) -> Result<PathBuf, String> {
    let roots = get_skill_roots();
    let path_str = path.display().to_string();

    // Security: Reject any paths with parent directory references
    if path_str.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }

    // Security: Reject null bytes
    if path_str.contains('\0') {
        return Err("Invalid path".to_string());
    }

    // First check if the path exists - if so, canonicalize it
    if path.exists() {
        // Security: Reject symlinks to prevent TOCTOU attacks
        if path.is_symlink() {
            return Err("Symlinks not allowed".to_string());
        }

        let canonical = path
            .canonicalize()
            .map_err(|_| "Invalid path".to_string())?;

        // Also canonicalize roots that exist for comparison
        for root in &roots {
            if root.exists() {
                if let Ok(canonical_root) = root.canonicalize() {
                    if canonical.starts_with(&canonical_root) {
                        return Ok(canonical);
                    }
                }
            }
        }

        // Check if path is within a project's .claude/skills/ directory
        let canonical_str = canonical.display().to_string();
        if canonical_str.contains("/.claude/skills/")
            || canonical_str.contains("\\.claude\\skills\\")
        {
            return Ok(canonical);
        }

        // Check if path is within a plugin's skills directory
        if canonical_str.contains("/.claude/plugins/")
            || canonical_str.contains("\\.claude\\plugins\\")
        {
            return Ok(canonical);
        }

        return Err("Path is not within an allowed skills directory".to_string());
    }

    // For non-existent paths, do a logical check
    // Check against allowed root directories
    for root in &roots {
        if path.starts_with(root) {
            return Ok(path.to_path_buf());
        }
    }

    // Check if path is within a project's .claude/skills/ directory
    if path_str.contains("/.claude/skills/") || path_str.contains("\\.claude\\skills\\") {
        return Ok(path.to_path_buf());
    }

    // Check if path is within a plugin's skills directory
    if path_str.contains("/.claude/plugins/") || path_str.contains("\\.claude\\plugins\\") {
        return Ok(path.to_path_buf());
    }

    Err("Path is not within an allowed skills directory".to_string())
}

/// Extract description from YAML frontmatter
fn extract_description(content: &str) -> Option<String> {
    if !content.starts_with("---") {
        return None;
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return None;
    }

    let frontmatter = parts[1];
    for line in frontmatter.lines() {
        let line = line.trim();
        if line.starts_with("description:") {
            return Some(line.trim_start_matches("description:").trim().to_string());
        }
    }

    None
}

/// Scan a skill directory for supporting files (reference.md, examples.md, scripts/, etc.)
fn scan_supporting_files(skill_file_path: &Path, skill_content: &str) -> Vec<SupportingFile> {
    let mut files = Vec::new();

    // Get the skill directory (parent of SKILL.md)
    let skill_dir = match skill_file_path.parent() {
        Some(dir) => dir,
        None => return files,
    };

    // Extract markdown links from SKILL.md to check if files are referenced
    // Pattern: [text](filename) or [text](./filename)
    let referenced_files: Vec<String> = extract_markdown_links(skill_content);

    // Scan the skill directory
    if let Ok(entries) = std::fs::read_dir(skill_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            // Skip SKILL.md itself and hidden files
            if file_name == "SKILL.md" || file_name.starts_with('.') {
                continue;
            }

            if path.is_file() {
                let file_type = determine_file_type(&file_name);
                let is_referenced = is_file_referenced(&file_name, &referenced_files);

                files.push(SupportingFile {
                    name: file_name,
                    path: path.display().to_string(),
                    file_type,
                    is_referenced,
                });
            } else if path.is_dir() {
                // Scan subdirectories like scripts/
                scan_subdirectory(&path, &mut files, &referenced_files);
            }
        }
    }

    // Sort: referenced files first, then by name
    files.sort_by(|a, b| match (a.is_referenced, b.is_referenced) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    files
}

/// Scan a subdirectory for supporting files
fn scan_subdirectory(dir: &Path, files: &mut Vec<SupportingFile>, referenced_files: &[String]) {
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                // Skip hidden files
                if file_name.starts_with('.') {
                    continue;
                }

                let display_name = format!("{dir_name}/{file_name}");
                let file_type = determine_file_type(&file_name);
                let is_referenced = is_file_referenced(&display_name, referenced_files)
                    || is_file_referenced(&file_name, referenced_files);

                files.push(SupportingFile {
                    name: display_name,
                    path: path.display().to_string(),
                    file_type,
                    is_referenced,
                });
            }
        }
    }
}

/// Extract markdown links from content
fn extract_markdown_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();

    // Simple regex-like pattern matching for [text](link)
    // Look for patterns like [text](filename.md) or [text](./filename.md) or [text](scripts/helper.py)
    let mut in_link = false;
    let mut link_start = 0;

    for (i, c) in content.char_indices() {
        if c == ']' && i + 1 < content.len() {
            if content[i + 1..].starts_with('(') {
                in_link = true;
                link_start = i + 2;
            }
        } else if in_link && c == ')' {
            let link = &content[link_start..i];
            // Only include local file references (not http/https)
            if !link.starts_with("http://") && !link.starts_with("https://") {
                // Normalize: remove leading ./ if present
                let normalized = link.trim_start_matches("./");
                links.push(normalized.to_string());
            }
            in_link = false;
        }
    }

    links
}

/// Determine file type from extension
fn determine_file_type(file_name: &str) -> String {
    let extension = file_name.rsplit('.').next().unwrap_or("");

    match extension.to_lowercase().as_str() {
        "md" | "markdown" => "markdown".to_string(),
        "py" | "sh" | "bash" | "js" | "ts" | "rb" | "pl" => "script".to_string(),
        "json" | "yaml" | "yml" | "toml" => "config".to_string(),
        "txt" => "text".to_string(),
        _ => "other".to_string(),
    }
}

/// Check if a file is referenced in the SKILL.md
fn is_file_referenced(file_name: &str, referenced_files: &[String]) -> bool {
    referenced_files
        .iter()
        .any(|link| link == file_name || link.ends_with(&format!("/{file_name}")))
}
