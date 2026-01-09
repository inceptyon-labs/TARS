//! Plugin export tests
//!
//! Tests for exporting profiles as Claude Code plugins.

use std::fs;
use tars_core::export::export_as_plugin;
use tars_core::profile::{
    AgentOverlay, ClaudeMdOverlay, CommandOverlay, OverlayMode, Profile, SkillOverlay,
};
use tempfile::TempDir;

fn create_minimal_profile(name: &str) -> Profile {
    Profile::new(name.to_string())
}

fn create_full_profile(name: &str) -> Profile {
    let mut profile = Profile::new(name.to_string());
    profile.description = Some("A test profile with all overlays".to_string());

    profile.repo_overlays.skills.push(SkillOverlay {
        name: "test-skill".to_string(),
        content: "---\nname: test-skill\ndescription: Test skill\n---\n\nSkill content".to_string(),
    });

    profile.repo_overlays.commands.push(CommandOverlay {
        name: "test-cmd".to_string(),
        content: "---\ndescription: Test command\n---\n\nRun $ARGUMENTS".to_string(),
    });

    profile.repo_overlays.agents.push(AgentOverlay {
        name: "test-agent".to_string(),
        content: "---\nname: test-agent\ndescription: Test agent\n---\n\nAgent content".to_string(),
    });

    profile.repo_overlays.claude_md = Some(ClaudeMdOverlay {
        mode: OverlayMode::Replace,
        content: "# Instructions".to_string(),
    });

    profile
}

// =============================================================================
// Basic Export Tests
// =============================================================================

#[test]
fn test_export_creates_plugin_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Export failed");

    assert!(output_dir.exists(), "Output directory should exist");
    assert!(
        output_dir.join(".claude-plugin").exists(),
        ".claude-plugin directory should exist"
    );
}

#[test]
fn test_export_creates_manifest() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    assert!(manifest_path.exists(), "plugin.json should exist");

    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");
    assert!(content.contains("\"name\": \"my-plugin\""));
    assert!(content.contains("\"version\": \"1.0.0\""));
}

#[test]
fn test_export_creates_subdirectories() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Export failed");

    assert!(
        output_dir.join("commands").exists(),
        "commands directory should exist"
    );
    assert!(
        output_dir.join("skills").exists(),
        "skills directory should exist"
    );
    assert!(
        output_dir.join("agents").exists(),
        "agents directory should exist"
    );
}

// =============================================================================
// Manifest Content Tests
// =============================================================================

#[test]
fn test_manifest_includes_description() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let mut profile = create_minimal_profile("test");
    profile.description = Some("A helpful plugin".to_string());

    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");
    assert!(content.contains("\"description\": \"A helpful plugin\""));
}

#[test]
fn test_manifest_valid_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let profile = create_full_profile("full-test");
    export_as_plugin(&profile, &output_dir, "full-plugin", "2.0.0").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");

    // Should be valid JSON
    let _: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON in manifest");
}

// =============================================================================
// Version Format Tests
// =============================================================================

#[test]
fn test_export_with_semver_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.2.3").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");
    assert!(content.contains("\"version\": \"1.2.3\""));
}

#[test]
fn test_export_with_prerelease_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0-beta.1").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");
    assert!(content.contains("\"version\": \"1.0.0-beta.1\""));
}

// =============================================================================
// Plugin Name Tests
// =============================================================================

#[test]
fn test_export_with_kebab_case_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-awesome-plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "my-awesome-plugin", "1.0.0").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");
    assert!(content.contains("\"name\": \"my-awesome-plugin\""));
}

#[test]
fn test_export_with_scoped_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "@myorg/my-plugin", "1.0.0").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");
    assert!(content.contains("\"name\": \"@myorg/my-plugin\""));
}

// =============================================================================
// Idempotency Tests
// =============================================================================

#[test]
fn test_export_is_idempotent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let profile = create_full_profile("test");

    // Export twice
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("First export failed");
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Second export failed");

    // Should still have valid structure
    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    assert!(manifest_path.exists());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_export_with_empty_description() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let mut profile = create_minimal_profile("test");
    profile.description = None;

    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");
    // Should have empty description
    assert!(content.contains("\"description\": \"\""));
}

#[test]
fn test_export_to_nested_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("deeply/nested/path/my-plugin");

    let profile = create_minimal_profile("test");
    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Export failed");

    assert!(
        output_dir.join(".claude-plugin/plugin.json").exists(),
        "Should create nested directories"
    );
}

#[test]
fn test_export_preserves_special_chars_in_description() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("my-plugin");

    let mut profile = create_minimal_profile("test");
    profile.description = Some("A plugin with \"quotes\" and 'apostrophes'".to_string());

    export_as_plugin(&profile, &output_dir, "my-plugin", "1.0.0").expect("Export failed");

    let manifest_path = output_dir.join(".claude-plugin/plugin.json");
    let content = fs::read_to_string(manifest_path).expect("Failed to read manifest");

    // Should be valid JSON with escaped quotes
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Invalid JSON with special chars");
    assert!(parsed["description"]
        .as_str()
        .unwrap()
        .contains("\"quotes\""));
}
