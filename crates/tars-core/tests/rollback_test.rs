//! Byte-for-byte rollback tests
//!
//! Tests verifying that backup/restore achieves exact byte-for-byte restoration.
//! This is a constitution requirement: apply + rollback = original state.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tars_core::apply::apply_operations;
use tars_core::backup::restore::restore_from_backup;
use tars_core::backup::Backup;
use tars_core::diff::plan::generate_plan;
use tars_core::profile::{ClaudeMdOverlay, CommandOverlay, OverlayMode, Profile, SkillOverlay};
use tempfile::TempDir;
use uuid::Uuid;

/// Compute SHA256 hash of file content
fn hash_file(path: &Path) -> Option<String> {
    if path.is_file() {
        let content = fs::read(path).ok()?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Some(hex::encode(hasher.finalize()))
    } else {
        None
    }
}

/// Create a snapshot of a directory's state (file paths and their hashes)
fn snapshot_directory(dir: &Path) -> HashMap<String, Option<String>> {
    let mut snapshot = HashMap::new();

    fn visit_dir(dir: &Path, base: &Path, snapshot: &mut HashMap<String, Option<String>>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    let relative = path.strip_prefix(base).unwrap().to_string_lossy().to_string();
                    snapshot.insert(relative, hash_file(&path));
                } else if path.is_dir() {
                    visit_dir(&path, base, snapshot);
                }
            }
        }
    }

    visit_dir(dir, dir, &mut snapshot);
    snapshot
}

/// Create a test project with Claude Code configuration
fn create_test_project(temp_dir: &Path) {
    // Create CLAUDE.md
    fs::write(temp_dir.join("CLAUDE.md"), "# Project Instructions\n\nOriginal content.\n")
        .expect("Failed to create CLAUDE.md");

    // Create .claude directory
    let claude_dir = temp_dir.join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude dir");

    // Create settings.json
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"permissions":{"allow":["Read","Write"]}}"#,
    )
    .expect("Failed to create settings.json");

    // Create existing skill
    let skill_dir = claude_dir.join("skills/existing-skill");
    fs::create_dir_all(&skill_dir).expect("Failed to create skill dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: existing-skill\ndescription: Existing skill\n---\n\nOriginal skill content\n",
    )
    .expect("Failed to create existing skill");

    // Create existing command
    let commands_dir = claude_dir.join("commands");
    fs::create_dir_all(&commands_dir).expect("Failed to create commands dir");
    fs::write(
        commands_dir.join("existing-cmd.md"),
        "---\ndescription: Existing command\n---\n\nOriginal command content\n",
    )
    .expect("Failed to create existing command");
}

/// Create a profile that will make various changes
fn create_modifying_profile() -> Profile {
    let mut profile = Profile::new("test-profile".to_string());

    // Add a new skill
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "new-skill".to_string(),
        content: "---\nname: new-skill\ndescription: New skill\n---\n\nNew skill content\n"
            .to_string(),
    });

    // Modify existing skill (by adding one with same name)
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "existing-skill".to_string(),
        content:
            "---\nname: existing-skill\ndescription: Modified skill\n---\n\nModified skill content\n"
                .to_string(),
    });

    // Add new command
    profile.repo_overlays.commands.push(CommandOverlay {
        name: "new-cmd".to_string(),
        content: "---\ndescription: New command\n---\n\nNew command content\n".to_string(),
    });

    // Replace CLAUDE.md
    profile.repo_overlays.claude_md = Some(ClaudeMdOverlay {
        mode: OverlayMode::Replace,
        content: "# New Instructions\n\nReplaced content.\n".to_string(),
    });

    profile
}

// =============================================================================
// Byte-for-byte Rollback Tests
// =============================================================================

#[test]
fn test_rollback_restores_exact_original_state() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    let project_id = Uuid::new_v4();

    // Create initial project state
    create_test_project(project_path);

    // Take snapshot BEFORE any changes
    let before_snapshot = snapshot_directory(project_path);

    // Create profile and generate plan
    let profile = create_modifying_profile();
    let plan = generate_plan(project_id, project_path, &profile).expect("Failed to generate plan");

    // Create backup and apply changes
    let mut backup = Backup::new(project_id, temp_dir.path().join("backup.json"));
    apply_operations(&plan, project_path, &mut backup).expect("Failed to apply operations");

    // Verify changes were applied (state is different)
    let after_apply_snapshot = snapshot_directory(project_path);
    assert_ne!(
        before_snapshot, after_apply_snapshot,
        "State should have changed after apply"
    );

    // Rollback using backup
    restore_from_backup(project_path, &backup).expect("Failed to restore from backup");

    // Take snapshot AFTER rollback
    let after_rollback_snapshot = snapshot_directory(project_path);

    // CRITICAL: Verify byte-for-byte restoration
    assert_eq!(
        before_snapshot, after_rollback_snapshot,
        "State after rollback must match original state exactly"
    );
}

#[test]
fn test_rollback_removes_newly_created_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    let project_id = Uuid::new_v4();

    // Create minimal project
    fs::create_dir_all(project_path.join(".claude")).expect("Failed to create .claude dir");

    // Profile that creates a new skill
    let mut profile = Profile::new("test".to_string());
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "brand-new-skill".to_string(),
        content: "New skill content".to_string(),
    });

    let plan = generate_plan(project_id, project_path, &profile).expect("Failed to generate plan");

    // Apply with backup
    let mut backup = Backup::new(project_id, temp_dir.path().join("backup.json"));
    apply_operations(&plan, project_path, &mut backup).expect("Failed to apply");

    // Verify file was created
    let new_skill_path = project_path.join(".claude/skills/brand-new-skill/SKILL.md");
    assert!(new_skill_path.exists(), "New skill should exist after apply");

    // Rollback
    restore_from_backup(project_path, &backup).expect("Failed to rollback");

    // Verify file was removed
    assert!(
        !new_skill_path.exists(),
        "New skill should be removed after rollback"
    );
}

#[test]
fn test_rollback_restores_modified_file_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    let project_id = Uuid::new_v4();

    // Create CLAUDE.md with specific content
    let original_content = "# Original CLAUDE.md\n\nThis is the original content.\n";
    fs::write(project_path.join("CLAUDE.md"), original_content).expect("Failed to write");

    // Profile that replaces CLAUDE.md
    let mut profile = Profile::new("test".to_string());
    profile.repo_overlays.claude_md = Some(ClaudeMdOverlay {
        mode: OverlayMode::Replace,
        content: "# Replaced Content\n\nCompletely different.\n".to_string(),
    });

    let plan = generate_plan(project_id, project_path, &profile).expect("Failed to generate plan");

    // Apply with backup
    let mut backup = Backup::new(project_id, temp_dir.path().join("backup.json"));
    apply_operations(&plan, project_path, &mut backup).expect("Failed to apply");

    // Verify content changed
    let after_content =
        fs::read_to_string(project_path.join("CLAUDE.md")).expect("Failed to read");
    assert_ne!(after_content, original_content, "Content should have changed");

    // Rollback
    restore_from_backup(project_path, &backup).expect("Failed to rollback");

    // Verify EXACT original content restored
    let restored_content =
        fs::read_to_string(project_path.join("CLAUDE.md")).expect("Failed to read");
    assert_eq!(
        restored_content, original_content,
        "Restored content must match original byte-for-byte"
    );
}

#[test]
fn test_rollback_preserves_untouched_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    let project_id = Uuid::new_v4();

    // Create an unrelated file
    let unrelated_content = "This file should not be touched.\n";
    fs::write(project_path.join("unrelated.txt"), unrelated_content).expect("Failed to write");

    // Create .claude directory
    fs::create_dir_all(project_path.join(".claude")).expect("Failed to create dir");

    // Profile that creates a skill
    let mut profile = Profile::new("test".to_string());
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "test-skill".to_string(),
        content: "Skill content".to_string(),
    });

    let plan = generate_plan(project_id, project_path, &profile).expect("Failed to generate plan");

    // Apply with backup
    let mut backup = Backup::new(project_id, temp_dir.path().join("backup.json"));
    apply_operations(&plan, project_path, &mut backup).expect("Failed to apply");

    // Rollback
    restore_from_backup(project_path, &backup).expect("Failed to rollback");

    // Verify unrelated file is unchanged
    let restored_unrelated =
        fs::read_to_string(project_path.join("unrelated.txt")).expect("Failed to read");
    assert_eq!(restored_unrelated, unrelated_content);
}

#[test]
fn test_multiple_apply_rollback_cycles() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    let project_id = Uuid::new_v4();

    // Create initial state
    create_test_project(project_path);
    let original_snapshot = snapshot_directory(project_path);

    // First cycle: apply and rollback
    let profile1 = create_modifying_profile();
    let plan1 =
        generate_plan(project_id, project_path, &profile1).expect("Failed to generate plan");
    let mut backup1 = Backup::new(project_id, temp_dir.path().join("backup1.json"));
    apply_operations(&plan1, project_path, &mut backup1).expect("Failed to apply");
    restore_from_backup(project_path, &backup1).expect("Failed to rollback");

    // Verify restored to original
    let after_cycle1 = snapshot_directory(project_path);
    assert_eq!(original_snapshot, after_cycle1, "Cycle 1 restore failed");

    // Second cycle: apply different profile and rollback
    let mut profile2 = Profile::new("different-profile".to_string());
    profile2.repo_overlays.claude_md = Some(ClaudeMdOverlay {
        mode: OverlayMode::Append,
        content: "\n## Appended Section\n".to_string(),
    });

    let plan2 =
        generate_plan(project_id, project_path, &profile2).expect("Failed to generate plan");
    let mut backup2 = Backup::new(project_id, temp_dir.path().join("backup2.json"));
    apply_operations(&plan2, project_path, &mut backup2).expect("Failed to apply");
    restore_from_backup(project_path, &backup2).expect("Failed to rollback");

    // Verify restored to original again
    let after_cycle2 = snapshot_directory(project_path);
    assert_eq!(original_snapshot, after_cycle2, "Cycle 2 restore failed");
}

#[test]
fn test_backup_contains_sha256_hashes() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    let project_id = Uuid::new_v4();

    // Create a file
    let content = "Test content for hashing";
    fs::write(project_path.join("CLAUDE.md"), content).expect("Failed to write");

    // Profile that modifies the file
    let mut profile = Profile::new("test".to_string());
    profile.repo_overlays.claude_md = Some(ClaudeMdOverlay {
        mode: OverlayMode::Replace,
        content: "New content".to_string(),
    });

    let plan = generate_plan(project_id, project_path, &profile).expect("Failed to generate plan");

    // Apply with backup
    let mut backup = Backup::new(project_id, temp_dir.path().join("backup.json"));
    apply_operations(&plan, project_path, &mut backup).expect("Failed to apply");

    // Verify backup has SHA256 hash
    assert!(!backup.files.is_empty(), "Backup should have files");
    let backed_up_file = &backup.files[0];
    assert!(
        backed_up_file.sha256.is_some(),
        "Backup file should have SHA256 hash"
    );

    // Compute expected hash
    let expected_hash = {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    };

    assert_eq!(
        backed_up_file.sha256.as_ref().unwrap(),
        &expected_hash,
        "SHA256 hash should match original content"
    );
}

#[test]
fn test_rollback_with_nested_directories() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    let project_id = Uuid::new_v4();

    // Create nested directory structure
    let nested_dir = project_path.join(".claude/skills/deeply/nested/skill");
    fs::create_dir_all(&nested_dir).expect("Failed to create nested dir");
    fs::write(nested_dir.join("SKILL.md"), "Original nested content")
        .expect("Failed to write nested skill");

    let original_snapshot = snapshot_directory(project_path);

    // Profile that creates a new skill (will create new directories)
    let mut profile = Profile::new("test".to_string());
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "another/nested/skill".to_string(),
        content: "New nested skill".to_string(),
    });

    // This should fail due to path traversal protection (slashes in name)
    let result = generate_plan(project_id, project_path, &profile);
    assert!(result.is_err(), "Should reject path with slashes");

    // State should remain unchanged
    let after_snapshot = snapshot_directory(project_path);
    assert_eq!(original_snapshot, after_snapshot);
}
