//! Non-destructive guarantee tests
//!
//! These tests verify that the scanner NEVER modifies any files.
//! This is a critical constitution requirement.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tars_scanner::Scanner;
use tempfile::TempDir;
use walkdir::WalkDir;

/// Compute SHA256 hash of a file
fn hash_file(path: &Path) -> Option<String> {
    let content = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Some(hex::encode(hasher.finalize()))
}

/// Create a snapshot of all files in a directory
fn snapshot_directory(path: &Path) -> HashMap<String, String> {
    let mut snapshot = HashMap::new();

    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let relative_path = entry
                .path()
                .strip_prefix(path)
                .unwrap()
                .to_string_lossy()
                .to_string();
            if let Some(hash) = hash_file(entry.path()) {
                snapshot.insert(relative_path, hash);
            }
        }
    }

    snapshot
}

/// Create a test fixture with various Claude Code configurations
fn create_readonly_test_fixture() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base = temp_dir.path();

    // Create .claude directory
    let claude_dir = base.join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude");

    // Create settings.json
    let settings = r#"{
        "permissions": {"allow": ["Bash(*)"], "deny": []},
        "hooks": {}
    }"#;
    fs::write(claude_dir.join("settings.json"), settings).unwrap();

    // Create settings.local.json
    let local_settings = r#"{"env": {"DEBUG": "true"}}"#;
    fs::write(claude_dir.join("settings.local.json"), local_settings).unwrap();

    // Create skills directory
    let skills_dir = claude_dir.join("skills");
    fs::create_dir_all(skills_dir.join("my-skill")).unwrap();
    let skill = r"---
name: my-skill
description: Test skill
---

Instructions here.
";
    fs::write(skills_dir.join("my-skill").join("SKILL.md"), skill).unwrap();

    // Create commands directory
    let commands_dir = claude_dir.join("commands");
    fs::create_dir_all(&commands_dir).unwrap();
    let command = r"---
description: Test command
---

Do $ARGUMENTS
";
    fs::write(commands_dir.join("my-command.md"), command).unwrap();

    // Create agents directory
    let agents_dir = claude_dir.join("agents");
    fs::create_dir_all(&agents_dir).unwrap();
    let agent = r"---
name: my-agent
description: Test agent
tools:
  - Read
  - Write
---

Agent instructions.
";
    fs::write(agents_dir.join("my-agent.md"), agent).unwrap();

    // Create CLAUDE.md
    fs::write(base.join("CLAUDE.md"), "# Project\n\nInstructions.").unwrap();

    // Create .mcp.json
    let mcp = r#"{"mcpServers": {"test": {"command": "test"}}}"#;
    fs::write(base.join(".mcp.json"), mcp).unwrap();

    // Create some regular project files
    fs::write(base.join("README.md"), "# Test Project").unwrap();
    fs::create_dir_all(base.join("src")).unwrap();
    fs::write(base.join("src").join("main.rs"), "fn main() {}").unwrap();

    temp_dir
}

#[test]
fn test_scanner_is_readonly_single_project() {
    let fixture = create_readonly_test_fixture();

    // Take snapshot before scanning
    let before_snapshot = snapshot_directory(fixture.path());

    // Run the scanner
    let scanner = Scanner::new();
    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok(), "Scan should succeed");

    // Take snapshot after scanning
    let after_snapshot = snapshot_directory(fixture.path());

    // Compare snapshots
    assert_eq!(
        before_snapshot.len(),
        after_snapshot.len(),
        "Number of files should not change"
    );

    for (path, before_hash) in &before_snapshot {
        let after_hash = after_snapshot.get(path);
        assert_eq!(
            Some(before_hash),
            after_hash,
            "File {path} should not be modified"
        );
    }
}

#[test]
fn test_scanner_is_readonly_full_scan() {
    let fixture = create_readonly_test_fixture();

    // Take snapshot before scanning
    let before_snapshot = snapshot_directory(fixture.path());

    // Run full inventory scan
    let scanner = Scanner::new().with_managed(true);
    let result = scanner.scan_all(&[fixture.path()]);
    assert!(result.is_ok(), "Full scan should succeed");

    // Take snapshot after scanning
    let after_snapshot = snapshot_directory(fixture.path());

    // Compare snapshots
    assert_eq!(
        before_snapshot.len(),
        after_snapshot.len(),
        "Number of files should not change after full scan"
    );

    for (path, before_hash) in &before_snapshot {
        let after_hash = after_snapshot.get(path);
        assert_eq!(
            Some(before_hash),
            after_hash,
            "File {path} should not be modified after full scan"
        );
    }
}

#[test]
fn test_scanner_is_readonly_multiple_scans() {
    let fixture = create_readonly_test_fixture();

    // Take initial snapshot
    let initial_snapshot = snapshot_directory(fixture.path());

    let scanner = Scanner::new();

    // Scan multiple times
    for i in 0..5 {
        let result = scanner.scan_project(fixture.path());
        assert!(result.is_ok(), "Scan {i} should succeed");
    }

    // Take final snapshot
    let final_snapshot = snapshot_directory(fixture.path());

    // Compare snapshots
    assert_eq!(
        initial_snapshot.len(),
        final_snapshot.len(),
        "Number of files should not change after multiple scans"
    );

    for (path, initial_hash) in &initial_snapshot {
        let final_hash = final_snapshot.get(path);
        assert_eq!(
            Some(initial_hash),
            final_hash,
            "File {path} should not be modified after multiple scans"
        );
    }
}

#[test]
fn test_scanner_does_not_create_files() {
    let fixture = create_readonly_test_fixture();

    // Count files before
    let count_before: usize = WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();

    // Run scanner
    let scanner = Scanner::new();
    let _ = scanner.scan_all(&[fixture.path()]);

    // Count files after
    let count_after: usize = WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();

    assert_eq!(
        count_before, count_after,
        "Scanner should not create any files"
    );
}

#[test]
fn test_scanner_does_not_create_directories() {
    let fixture = create_readonly_test_fixture();

    // Count directories before
    let count_before: usize = WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
        .count();

    // Run scanner
    let scanner = Scanner::new();
    let _ = scanner.scan_all(&[fixture.path()]);

    // Count directories after
    let count_after: usize = WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
        .count();

    assert_eq!(
        count_before, count_after,
        "Scanner should not create any directories"
    );
}

#[test]
fn test_scanner_preserves_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = create_readonly_test_fixture();

    // Record permissions before
    let mut permissions_before = HashMap::new();
    for entry in WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() {
            let metadata = fs::metadata(entry.path()).unwrap();
            let relative_path = entry
                .path()
                .strip_prefix(fixture.path())
                .unwrap()
                .to_string_lossy()
                .to_string();
            permissions_before.insert(relative_path, metadata.permissions().mode());
        }
    }

    // Run scanner
    let scanner = Scanner::new();
    let _ = scanner.scan_all(&[fixture.path()]);

    // Check permissions after
    for entry in WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() {
            let metadata = fs::metadata(entry.path()).unwrap();
            let relative_path = entry
                .path()
                .strip_prefix(fixture.path())
                .unwrap()
                .to_string_lossy()
                .to_string();
            let before_mode = permissions_before.get(&relative_path);
            assert_eq!(
                before_mode,
                Some(&metadata.permissions().mode()),
                "File {relative_path} permissions should not change"
            );
        }
    }
}

#[test]
fn test_scanner_preserves_timestamps() {
    let fixture = create_readonly_test_fixture();

    // Small delay to ensure any modifications would show a timestamp change
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Record modification times before
    let mut mtimes_before = HashMap::new();
    for entry in WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() {
            let metadata = fs::metadata(entry.path()).unwrap();
            let relative_path = entry
                .path()
                .strip_prefix(fixture.path())
                .unwrap()
                .to_string_lossy()
                .to_string();
            mtimes_before.insert(relative_path, metadata.modified().unwrap());
        }
    }

    // Small delay before scanning
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Run scanner
    let scanner = Scanner::new();
    let _ = scanner.scan_all(&[fixture.path()]);

    // Check modification times after
    for entry in WalkDir::new(fixture.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() {
            let metadata = fs::metadata(entry.path()).unwrap();
            let relative_path = entry
                .path()
                .strip_prefix(fixture.path())
                .unwrap()
                .to_string_lossy()
                .to_string();
            let before_mtime = mtimes_before.get(&relative_path);
            assert_eq!(
                before_mtime,
                Some(&metadata.modified().unwrap()),
                "File {relative_path} modification time should not change"
            );
        }
    }
}
