//! Project statistics commands
//!
//! Provides code metrics like LoC, file counts, dependencies, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Language statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageStats {
    pub files: usize,
    pub lines: usize,
    pub code: usize,
    pub comments: usize,
    pub blanks: usize,
}

/// Test coverage info (if available)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageInfo {
    pub source: String,
    pub line_coverage: Option<f64>,
    pub branch_coverage: Option<f64>,
    pub lines_covered: Option<usize>,
    pub lines_total: Option<usize>,
}

/// Dependency info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub source: String,
    pub production: usize,
    pub development: usize,
}

/// Project statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub languages: HashMap<String, LanguageStats>,
    pub total_files: usize,
    pub total_lines: usize,
    pub total_code: usize,
    pub coverage: Option<CoverageInfo>,
    pub dependencies: Vec<DependencyInfo>,
    pub todo_count: usize,
    pub fixme_count: usize,
}

// File extensions to language mapping
fn get_language(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        "rs" => Some("Rust"),
        "ts" | "tsx" => Some("TypeScript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript"),
        "dart" => Some("Dart"),
        "py" => Some("Python"),
        "go" => Some("Go"),
        "java" => Some("Java"),
        "kt" | "kts" => Some("Kotlin"),
        "swift" => Some("Swift"),
        "c" | "h" => Some("C"),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("C++"),
        "cs" => Some("C#"),
        "rb" | "erb" => Some("Ruby"),
        "php" => Some("PHP"),
        "html" | "htm" => Some("HTML"),
        "css" | "scss" | "sass" | "less" => Some("CSS"),
        "json" => Some("JSON"),
        "yaml" | "yml" => Some("YAML"),
        "toml" => Some("TOML"),
        "xml" => Some("XML"),
        "md" | "markdown" => Some("Markdown"),
        "sql" => Some("SQL"),
        "sh" | "bash" | "zsh" => Some("Shell"),
        "vue" => Some("Vue"),
        "svelte" => Some("Svelte"),
        _ => None,
    }
}

// Check if path should be ignored
fn should_ignore(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Common directories to ignore
    let ignore_dirs = [
        "node_modules",
        "target",
        "dist",
        "build",
        ".git",
        ".next",
        ".nuxt",
        "__pycache__",
        ".pytest_cache",
        "venv",
        ".venv",
        "vendor",
        "coverage",
        ".coverage",
        "htmlcov",
    ];

    for dir in ignore_dirs {
        if path_str.contains(&format!("/{dir}/")) || path_str.ends_with(&format!("/{dir}")) {
            return true;
        }
    }

    false
}

// Count lines in a file, separating code, comments, and blanks
fn count_lines(path: &Path, ext: &str) -> (usize, usize, usize, usize) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (0, 0, 0, 0),
    };

    let mut total = 0;
    let mut code = 0;
    let mut comments = 0;
    let mut blanks = 0;
    let mut in_block_comment = false;

    let (line_comment, block_start, block_end) = match ext {
        "rs" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "go" | "java" | "kt" | "kts"
        | "swift" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "cs" | "php" | "vue"
        | "svelte" | "scss" | "less" | "dart" => ("//", "/*", "*/"),
        "py" | "rb" | "erb" | "sh" | "bash" | "zsh" | "yaml" | "yml" | "toml" => {
            ("#", "\"\"\"", "\"\"\"")
        }
        "html" | "htm" | "xml" | "md" | "markdown" => ("", "<!--", "-->"),
        "css" | "sass" => ("", "/*", "*/"),
        "sql" => ("--", "/*", "*/"),
        _ => ("", "", ""),
    };

    for line in content.lines() {
        total += 1;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            blanks += 1;
            continue;
        }

        // Handle block comments
        if in_block_comment {
            comments += 1;
            if !block_end.is_empty() && trimmed.contains(block_end) {
                in_block_comment = false;
            }
            continue;
        }

        if !block_start.is_empty() && trimmed.starts_with(block_start) {
            in_block_comment = true;
            comments += 1;
            if !block_end.is_empty() && trimmed.contains(block_end) {
                in_block_comment = false;
            }
            continue;
        }

        // Handle line comments
        if !line_comment.is_empty() && trimmed.starts_with(line_comment) {
            comments += 1;
            continue;
        }

        code += 1;
    }

    (total, code, comments, blanks)
}

// Scan for TODO and FIXME markers in comments only
fn count_todos(path: &Path, ext: &str) -> (usize, usize) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (0, 0),
    };

    let (line_comment, block_start, block_end) = match ext {
        "rs" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "go" | "java" | "kt" | "kts"
        | "swift" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "cs" | "php" | "vue"
        | "svelte" | "scss" | "less" | "dart" => ("//", "/*", "*/"),
        "py" | "rb" | "erb" | "sh" | "bash" | "zsh" | "yaml" | "yml" | "toml" => {
            ("#", "\"\"\"", "\"\"\"")
        }
        "html" | "htm" | "xml" | "md" | "markdown" => ("", "<!--", "-->"),
        "css" | "sass" => ("", "/*", "*/"),
        "sql" => ("--", "/*", "*/"),
        _ => ("", "", ""),
    };

    let mut todos = 0;
    let mut fixmes = 0;
    let mut in_block_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        // Track block comment state
        if in_block_comment {
            if upper.contains("TODO") {
                todos += 1;
            }
            if upper.contains("FIXME") {
                fixmes += 1;
            }
            if !block_end.is_empty() && trimmed.contains(block_end) {
                in_block_comment = false;
            }
            continue;
        }

        // Check for block comment start
        if !block_start.is_empty() && trimmed.contains(block_start) {
            in_block_comment = true;
            if upper.contains("TODO") {
                todos += 1;
            }
            if upper.contains("FIXME") {
                fixmes += 1;
            }
            if !block_end.is_empty() && trimmed.contains(block_end) {
                in_block_comment = false;
            }
            continue;
        }

        // Check line comments
        if !line_comment.is_empty() && trimmed.contains(line_comment) {
            // Only check the part after the comment marker
            if let Some(idx) = trimmed.find(line_comment) {
                let comment_part = &trimmed[idx..].to_uppercase();
                if comment_part.contains("TODO") {
                    todos += 1;
                }
                if comment_part.contains("FIXME") {
                    fixmes += 1;
                }
            }
        }
    }

    (todos, fixmes)
}

// Walk directory and collect stats
fn collect_language_stats(root: &Path) -> (HashMap<String, LanguageStats>, usize, usize) {
    let mut stats: HashMap<String, LanguageStats> = HashMap::new();
    let mut total_todos = 0;
    let mut total_fixmes = 0;

    fn walk(
        dir: &Path,
        stats: &mut HashMap<String, LanguageStats>,
        todos: &mut usize,
        fixmes: &mut usize,
    ) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if should_ignore(&path) {
                continue;
            }

            if path.is_dir() {
                walk(&path, stats, todos, fixmes);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if let Some(lang) = get_language(ext) {
                        let (total, code, comments, blanks) = count_lines(&path, ext);
                        let (t, f) = count_todos(&path, ext);

                        *todos += t;
                        *fixmes += f;

                        let entry = stats.entry(lang.to_string()).or_default();
                        entry.files += 1;
                        entry.lines += total;
                        entry.code += code;
                        entry.comments += comments;
                        entry.blanks += blanks;
                    }
                }
            }
        }
    }

    walk(root, &mut stats, &mut total_todos, &mut total_fixmes);
    (stats, total_todos, total_fixmes)
}

// Find all package.json files recursively (excluding node_modules)
fn find_package_jsons(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();

    fn walk(dir: &Path, results: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if name == "node_modules" || name == ".git" || name == "target" {
                    continue;
                }

                if path.is_dir() {
                    walk(&path, results);
                } else if name == "package.json" {
                    results.push(path);
                }
            }
        }
    }

    walk(root, &mut results);
    results
}

// Parse all package.json files for dependencies
fn parse_package_jsons(project: &Path) -> Option<DependencyInfo> {
    let package_jsons = find_package_jsons(project);

    let mut total_prod = 0;
    let mut total_dev = 0;
    let mut count = 0;

    for path in package_jsons {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let prod = json
                    .get("dependencies")
                    .and_then(|d| d.as_object())
                    .map_or(0, |o| o.len());

                let dev = json
                    .get("devDependencies")
                    .and_then(|d| d.as_object())
                    .map_or(0, |o| o.len());

                if prod > 0 || dev > 0 {
                    total_prod += prod;
                    total_dev += dev;
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        let source = if count == 1 {
            "package.json".to_string()
        } else {
            format!("{} package.json files", count)
        };
        Some(DependencyInfo {
            source,
            production: total_prod,
            development: total_dev,
        })
    } else {
        None
    }
}

// Parse Cargo.toml for dependencies
fn parse_cargo_toml(project: &Path) -> Option<DependencyInfo> {
    let path = project.join("Cargo.toml");
    let content = fs::read_to_string(&path).ok()?;
    let toml: toml::Value = content.parse().ok()?;

    let prod = toml
        .get("dependencies")
        .and_then(|d| d.as_table())
        .map_or(0, |t| t.len());

    let dev = toml
        .get("dev-dependencies")
        .and_then(|d| d.as_table())
        .map_or(0, |t| t.len());

    let build = toml
        .get("build-dependencies")
        .and_then(|d| d.as_table())
        .map_or(0, |t| t.len());

    if prod > 0 || dev > 0 {
        Some(DependencyInfo {
            source: "Cargo.toml".to_string(),
            production: prod,
            development: dev + build,
        })
    } else {
        None
    }
}

// Parse requirements.txt for Python dependencies
fn parse_requirements_txt(project: &Path) -> Option<DependencyInfo> {
    let path = project.join("requirements.txt");
    let content = fs::read_to_string(&path).ok()?;

    let count = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .count();

    if count > 0 {
        Some(DependencyInfo {
            source: "requirements.txt".to_string(),
            production: count,
            development: 0,
        })
    } else {
        None
    }
}

// Parse pyproject.toml for Python dependencies
fn parse_pyproject_toml(project: &Path) -> Option<DependencyInfo> {
    let path = project.join("pyproject.toml");
    let content = fs::read_to_string(&path).ok()?;
    let toml: toml::Value = content.parse().ok()?;

    // Check for Poetry dependencies
    let poetry_deps = toml
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
        .map_or(0, |t| t.len().saturating_sub(1)); // Subtract 1 for python version

    let poetry_dev = toml
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dev-dependencies"))
        .and_then(|d| d.as_table())
        .map_or(0, |t| t.len());

    // Check for PEP 621 dependencies
    let pep_deps = toml
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
        .map_or(0, |a: &Vec<toml::Value>| a.len());

    let prod = poetry_deps.max(pep_deps);
    let dev = poetry_dev;

    if prod > 0 || dev > 0 {
        Some(DependencyInfo {
            source: "pyproject.toml".to_string(),
            production: prod,
            development: dev,
        })
    } else {
        None
    }
}

// Parse go.mod for Go dependencies
fn parse_go_mod(project: &Path) -> Option<DependencyInfo> {
    let path = project.join("go.mod");
    let content = fs::read_to_string(&path).ok()?;

    let count = content
        .lines()
        .filter(|line| {
            line.trim().starts_with("require")
                || (line.starts_with('\t') && !line.contains("indirect"))
        })
        .count();

    if count > 0 {
        Some(DependencyInfo {
            source: "go.mod".to_string(),
            production: count,
            development: 0,
        })
    } else {
        None
    }
}

// Parse pubspec.yaml for Flutter/Dart dependencies
fn parse_pubspec_yaml(project: &Path) -> Option<DependencyInfo> {
    let path = project.join("pubspec.yaml");
    let content = fs::read_to_string(&path).ok()?;

    // Simple line-based parsing for YAML dependencies
    // Count lines under dependencies: and dev_dependencies: sections
    let mut in_deps = false;
    let mut in_dev_deps = false;
    let mut prod = 0;
    let mut dev = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for section headers
        if trimmed == "dependencies:" {
            in_deps = true;
            in_dev_deps = false;
            continue;
        } else if trimmed == "dev_dependencies:" {
            in_deps = false;
            in_dev_deps = true;
            continue;
        } else if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
            // New top-level section, stop counting
            in_deps = false;
            in_dev_deps = false;
            continue;
        }

        // Count dependencies (lines that start with a package name, not comments)
        if (in_deps || in_dev_deps) && !trimmed.is_empty() && !trimmed.starts_with('#') {
            // Check if this is a direct dependency (has : after name)
            if trimmed.contains(':') && !trimmed.starts_with('-') {
                let name = trimmed.split(':').next().unwrap_or("");
                // Skip flutter sdk references and nested properties
                if !name.is_empty() && name != "sdk" && !name.starts_with(' ') {
                    if in_deps {
                        prod += 1;
                    } else {
                        dev += 1;
                    }
                }
            }
        }
    }

    if prod > 0 || dev > 0 {
        Some(DependencyInfo {
            source: "pubspec.yaml".to_string(),
            production: prod,
            development: dev,
        })
    } else {
        None
    }
}

// Parse Gemfile for Ruby dependencies
fn parse_gemfile(project: &Path) -> Option<DependencyInfo> {
    let path = project.join("Gemfile");
    let content = fs::read_to_string(&path).ok()?;

    let count = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("gem ") || trimmed.starts_with("gem(")
        })
        .count();

    if count > 0 {
        Some(DependencyInfo {
            source: "Gemfile".to_string(),
            production: count,
            development: 0,
        })
    } else {
        None
    }
}

// Collect all dependencies
fn collect_dependencies(project: &Path) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();

    if let Some(d) = parse_package_jsons(project) {
        deps.push(d);
    }
    if let Some(d) = parse_cargo_toml(project) {
        deps.push(d);
    }
    if let Some(d) = parse_pubspec_yaml(project) {
        deps.push(d);
    }
    if let Some(d) = parse_gemfile(project) {
        deps.push(d);
    }
    if let Some(d) = parse_requirements_txt(project) {
        deps.push(d);
    }
    if let Some(d) = parse_pyproject_toml(project) {
        deps.push(d);
    }
    if let Some(d) = parse_go_mod(project) {
        deps.push(d);
    }

    deps
}

// Find coverage files recursively
fn find_coverage_files(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();

    fn walk(dir: &Path, results: &mut Vec<PathBuf>, depth: usize) {
        // Limit depth to avoid going too deep
        if depth > 5 {
            return;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip common non-project directories
                if name == "node_modules" || name == ".git" || name == "target" || name == "vendor"
                {
                    continue;
                }

                if path.is_dir() {
                    // Check for coverage files in this directory
                    let lcov = path.join("lcov.info");
                    if lcov.exists() {
                        results.push(lcov);
                    }

                    // Check coverage subdirectory
                    let coverage_dir = path.join("coverage");
                    if coverage_dir.is_dir() {
                        let cov_lcov = coverage_dir.join("lcov.info");
                        if cov_lcov.exists() {
                            results.push(cov_lcov);
                        }
                    }

                    walk(&path, results, depth + 1);
                } else if name == "lcov.info" {
                    results.push(path);
                }
            }
        }
    }

    // Check root level first
    let root_lcov = root.join("lcov.info");
    if root_lcov.exists() {
        results.push(root_lcov);
    }

    let root_coverage = root.join("coverage/lcov.info");
    if root_coverage.exists() {
        results.push(root_coverage);
    }

    walk(root, &mut results, 0);
    results
}

// Parse LCOV coverage report - searches recursively
fn parse_lcov(project: &Path) -> Option<CoverageInfo> {
    let coverage_files = find_coverage_files(project);

    // Aggregate all coverage data
    let mut total_lines_found = 0usize;
    let mut total_lines_hit = 0usize;
    let mut total_branches_found = 0usize;
    let mut total_branches_hit = 0usize;
    let mut files_found = 0;

    for path in &coverage_files {
        if let Ok(content) = fs::read_to_string(path) {
            let mut lines_found = 0usize;
            let mut lines_hit = 0usize;
            let mut branches_found = 0usize;
            let mut branches_hit = 0usize;

            for line in content.lines() {
                if let Some(val) = line.strip_prefix("LF:") {
                    lines_found += val.trim().parse::<usize>().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("LH:") {
                    lines_hit += val.trim().parse::<usize>().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("BRF:") {
                    branches_found += val.trim().parse::<usize>().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("BRH:") {
                    branches_hit += val.trim().parse::<usize>().unwrap_or(0);
                }
            }

            if lines_found > 0 {
                total_lines_found += lines_found;
                total_lines_hit += lines_hit;
                total_branches_found += branches_found;
                total_branches_hit += branches_hit;
                files_found += 1;
            }
        }
    }

    if total_lines_found > 0 {
        let line_cov = (total_lines_hit as f64 / total_lines_found as f64) * 100.0;
        let branch_cov = if total_branches_found > 0 {
            Some((total_branches_hit as f64 / total_branches_found as f64) * 100.0)
        } else {
            None
        };

        let source = if files_found == 1 {
            "lcov.info".to_string()
        } else {
            format!("{} coverage reports", files_found)
        };

        return Some(CoverageInfo {
            source,
            line_coverage: Some(line_cov),
            branch_coverage: branch_cov,
            lines_covered: Some(total_lines_hit),
            lines_total: Some(total_lines_found),
        });
    }

    None
}

// Parse tarpaulin coverage (Rust)
fn parse_tarpaulin(project: &Path) -> Option<CoverageInfo> {
    let paths = [
        project.join("tarpaulin-report.json"),
        project.join("coverage/tarpaulin-report.json"),
    ];

    for path in paths {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let covered = json.get("covered").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let total = json.get("coverable").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                if total > 0 {
                    return Some(CoverageInfo {
                        source: "tarpaulin".to_string(),
                        line_coverage: Some((covered as f64 / total as f64) * 100.0),
                        branch_coverage: None,
                        lines_covered: Some(covered),
                        lines_total: Some(total),
                    });
                }
            }
        }
    }

    None
}

// Collect coverage info
fn collect_coverage(project: &Path) -> Option<CoverageInfo> {
    parse_lcov(project).or_else(|| parse_tarpaulin(project))
}

/// Get project statistics
#[tauri::command]
pub async fn get_project_stats(project_path: String) -> Result<ProjectStats, String> {
    let project = PathBuf::from(&project_path);

    if !project.exists() {
        return Err("Project path does not exist".to_string());
    }

    let (languages, todo_count, fixme_count) = collect_language_stats(&project);

    let total_files: usize = languages.values().map(|s| s.files).sum();
    let total_lines: usize = languages.values().map(|s| s.lines).sum();
    let total_code: usize = languages.values().map(|s| s.code).sum();

    let coverage = collect_coverage(&project);
    let dependencies = collect_dependencies(&project);

    Ok(ProjectStats {
        languages,
        total_files,
        total_lines,
        total_code,
        coverage,
        dependencies,
        todo_count,
        fixme_count,
    })
}
