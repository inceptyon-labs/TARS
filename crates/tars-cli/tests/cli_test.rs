//! CLI integration tests using assert_cmd
//!
//! These tests verify the CLI commands work correctly end-to-end.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get a command instance for the tars binary
fn tars_cmd() -> Command {
    Command::cargo_bin("tars").expect("Failed to find tars binary")
}

#[test]
fn test_help_command() {
    tars_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "TARS - Claude Code configuration manager",
        ));
}

#[test]
fn test_version_command() {
    tars_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("tars"));
}

#[test]
fn test_scan_help() {
    tars_cmd()
        .arg("scan")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Scan Claude Code configuration"));
}

#[test]
fn test_profile_help() {
    tars_cmd()
        .arg("profile")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage profiles"));
}

#[test]
fn test_mcp_help() {
    tars_cmd()
        .arg("mcp")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage MCP servers"));
}

#[test]
fn test_cache_help() {
    tars_cmd()
        .arg("cache")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage plugin cache"));
}

#[test]
fn test_profile_list_empty() {
    // Use a temporary directory for the database
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles found"));
}

#[test]
fn test_profile_show_not_found() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("show")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Profile not found"));
}

#[test]
fn test_profile_create_from_project() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    // Create a minimal Claude config in the project
    let claude_dir = project_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("create")
        .arg("test-profile")
        .arg("--source")
        .arg(project_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created profile"));
}

#[test]
fn test_profile_create_with_description() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    let claude_dir = project_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("create")
        .arg("my-profile")
        .arg("--source")
        .arg(project_dir.path())
        .arg("--description")
        .arg("A test profile with description")
        .assert()
        .success()
        .stdout(predicate::str::contains("my-profile"));
}

#[test]
fn test_profile_create_validates_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    // Try to create with invalid name containing path separator
    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("create")
        .arg("../bad-name")
        .arg("--source")
        .arg(project_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("path separators"));
}

#[test]
fn test_profile_create_rejects_dot_prefix() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("create")
        .arg(".hidden-profile")
        .arg("--source")
        .arg(project_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot start with"));
}

#[test]
fn test_scan_creates_inventory_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = TempDir::new().expect("Failed to create output dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("scan")
        .arg(project_dir.path())
        .arg("--output")
        .arg(output_dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("Scan complete"));

    // Verify inventory.json was created
    assert!(output_dir.path().join("inventory.json").exists());
}

#[test]
fn test_scan_creates_inventory_markdown() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = TempDir::new().expect("Failed to create output dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("scan")
        .arg(project_dir.path())
        .arg("--output")
        .arg(output_dir.path())
        .arg("--format")
        .arg("markdown")
        .assert()
        .success();

    assert!(output_dir.path().join("inventory.md").exists());
}

#[test]
fn test_scan_creates_both_formats() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = TempDir::new().expect("Failed to create output dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("scan")
        .arg(project_dir.path())
        .arg("--output")
        .arg(output_dir.path())
        .arg("--format")
        .arg("both")
        .assert()
        .success();

    assert!(output_dir.path().join("inventory.json").exists());
    assert!(output_dir.path().join("inventory.md").exists());
}

#[test]
fn test_cache_status() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("cache")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plugin Cache Status"));
}

#[test]
fn test_cache_status_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("cache")
        .arg("status")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
fn test_cache_clean_dry_run() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("cache")
        .arg("clean")
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_profile_apply_nonexistent_target() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_dir = TempDir::new().expect("Failed to create project dir");

    // First create a profile
    let claude_dir = project_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("create")
        .arg("test-profile")
        .arg("--source")
        .arg(project_dir.path())
        .assert()
        .success();

    // Try to apply to nonexistent target
    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("apply")
        .arg("test-profile")
        .arg("/nonexistent/path")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_profile_apply_dry_run() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = TempDir::new().expect("Failed to create source dir");
    let target_dir = TempDir::new().expect("Failed to create target dir");

    // Create a profile from source
    let claude_dir = source_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude dir");

    // Add a skill to make it interesting
    let skills_dir = claude_dir.join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).expect("Failed to create skills dir");
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: A test skill\n---\n\nSkill content",
    )
    .expect("Failed to write skill");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("create")
        .arg("test-profile")
        .arg("--source")
        .arg(source_dir.path())
        .assert()
        .success();

    // Apply with dry-run
    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("apply")
        .arg("test-profile")
        .arg(target_dir.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn test_profile_backups_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("backups")
        .assert()
        .success()
        .stdout(predicate::str::contains("No backups found"));
}

#[test]
fn test_profile_delete_nonexistent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("delete")
        .arg("nonexistent-profile")
        .arg("--force")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Profile not found"));
}

#[test]
fn test_profile_export_creates_plugin() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let source_dir = TempDir::new().expect("Failed to create source dir");
    let output_dir = TempDir::new().expect("Failed to create output dir");

    // Create a profile
    let claude_dir = source_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("create")
        .arg("export-test")
        .arg("--source")
        .arg(source_dir.path())
        .assert()
        .success();

    // Export as plugin
    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("profile")
        .arg("export")
        .arg("export-test")
        .arg("--output")
        .arg(output_dir.path())
        .arg("--version")
        .arg("1.0.0")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created plugin"));

    // Verify plugin was created
    assert!(output_dir.path().join("export-test-1.0.0").exists());
}

#[test]
fn test_unknown_subcommand_fails() {
    tars_cmd()
        .arg("unknown-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_scan_with_include_managed() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = TempDir::new().expect("Failed to create output dir");

    tars_cmd()
        .env("HOME", temp_dir.path())
        .arg("scan")
        .arg("--output")
        .arg(output_dir.path())
        .arg("--include-managed")
        .assert()
        .success();
}
