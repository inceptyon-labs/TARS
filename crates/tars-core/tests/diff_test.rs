//! Diff generation tests
//!
//! Tests for generating diff plans from profiles.

use std::fs;
use tars_core::diff::plan::{generate_plan, generate_text_diff};
use tars_core::diff::{DiffPlan, FileOperation};
use tars_core::profile::{
    AgentOverlay, ClaudeMdOverlay, CommandOverlay, OverlayMode, Profile, RepoOverlays, SkillOverlay,
};
use tempfile::TempDir;
use uuid::Uuid;

fn create_test_profile_with_skills(name: &str) -> Profile {
    let mut profile = Profile::new(name.to_string());
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "test-skill".to_string(),
        content: "---\nname: test-skill\ndescription: Test\n---\n\nSkill content".to_string(),
    });
    profile
}

fn create_test_profile_with_commands(name: &str) -> Profile {
    let mut profile = Profile::new(name.to_string());
    profile.repo_overlays.commands.push(CommandOverlay {
        name: "test-cmd".to_string(),
        content: "---\ndescription: Test\n---\n\nExecute $ARGUMENTS".to_string(),
    });
    profile
}

fn create_test_profile_with_agents(name: &str) -> Profile {
    let mut profile = Profile::new(name.to_string());
    profile.repo_overlays.agents.push(AgentOverlay {
        name: "test-agent".to_string(),
        content: "---\nname: test-agent\ndescription: Test\n---\n\nAgent instructions".to_string(),
    });
    profile
}

fn create_test_profile_with_claude_md(name: &str, mode: OverlayMode) -> Profile {
    let mut profile = Profile::new(name.to_string());
    profile.repo_overlays.claude_md = Some(ClaudeMdOverlay {
        mode,
        content: "# Additional Instructions".to_string(),
    });
    profile
}

// =============================================================================
// Text Diff Tests
// =============================================================================

#[test]
fn test_generate_text_diff_no_changes() {
    let text = "line1\nline2\nline3\n";
    let diff = generate_text_diff(text, text);
    // All lines should be marked as equal (space prefix)
    assert!(diff.lines().all(|l| l.starts_with(' ')));
}

#[test]
fn test_generate_text_diff_addition() {
    let old = "line1\nline3\n";
    let new = "line1\nline2\nline3\n";
    let diff = generate_text_diff(old, new);

    assert!(diff.contains("+line2"));
}

#[test]
fn test_generate_text_diff_removal() {
    let old = "line1\nline2\nline3\n";
    let new = "line1\nline3\n";
    let diff = generate_text_diff(old, new);

    assert!(diff.contains("-line2"));
}

#[test]
fn test_generate_text_diff_modification() {
    let old = "line1\nold content\nline3\n";
    let new = "line1\nnew content\nline3\n";
    let diff = generate_text_diff(old, new);

    assert!(diff.contains("-old content"));
    assert!(diff.contains("+new content"));
}

// =============================================================================
// DiffPlan Type Tests
// =============================================================================

#[test]
fn test_diff_plan_new() {
    let project_id = Uuid::new_v4();
    let profile_id = Uuid::new_v4();
    let plan = DiffPlan::new(project_id, profile_id);

    assert_eq!(plan.project_id, project_id);
    assert_eq!(plan.profile_id, profile_id);
    assert!(plan.is_empty());
    assert!(!plan.has_errors());
}

// =============================================================================
// Skill Overlay Tests
// =============================================================================

#[test]
fn test_plan_new_skill() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();
    let profile = create_test_profile_with_skills("skill-test");

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());
    assert_eq!(plan.operations.len(), 1);

    match &plan.operations[0] {
        FileOperation::Create { path, content } => {
            assert!(path.ends_with("test-skill/SKILL.md"));
            assert!(!content.is_empty());
        }
        _ => panic!("Expected Create operation"),
    }
}

#[test]
fn test_plan_modify_existing_skill() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    // Create existing skill
    let skill_dir = temp_dir.path().join(".claude/skills/test-skill");
    fs::create_dir_all(&skill_dir).expect("Failed to create dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Old\n---\n\nOld content",
    )
    .expect("Failed to write");

    let profile = create_test_profile_with_skills("skill-modify-test");

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());

    match &plan.operations[0] {
        FileOperation::Modify { path, diff, .. } => {
            assert!(path.ends_with("test-skill/SKILL.md"));
            assert!(!diff.is_empty());
        }
        _ => panic!("Expected Modify operation"),
    }
}

#[test]
fn test_plan_unchanged_skill() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    // Create existing skill with same content
    let skill_dir = temp_dir.path().join(".claude/skills/test-skill");
    fs::create_dir_all(&skill_dir).expect("Failed to create dir");
    let content = "---\nname: test-skill\ndescription: Test\n---\n\nSkill content";
    fs::write(skill_dir.join("SKILL.md"), content).expect("Failed to write");

    let profile = create_test_profile_with_skills("skill-unchanged-test");

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    // No operations if content is identical
    assert!(plan.is_empty());
}

// =============================================================================
// Command Overlay Tests
// =============================================================================

#[test]
fn test_plan_new_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();
    let profile = create_test_profile_with_commands("command-test");

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());

    match &plan.operations[0] {
        FileOperation::Create { path, .. } => {
            assert!(path.ends_with("test-cmd.md"));
        }
        _ => panic!("Expected Create operation"),
    }
}

// =============================================================================
// Agent Overlay Tests
// =============================================================================

#[test]
fn test_plan_new_agent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();
    let profile = create_test_profile_with_agents("agent-test");

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());

    match &plan.operations[0] {
        FileOperation::Create { path, .. } => {
            assert!(path.ends_with("test-agent.md"));
        }
        _ => panic!("Expected Create operation"),
    }
}

// =============================================================================
// CLAUDE.md Overlay Tests
// =============================================================================

#[test]
fn test_plan_create_claude_md() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();
    let profile = create_test_profile_with_claude_md("claude-md-create", OverlayMode::Replace);

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());

    match &plan.operations[0] {
        FileOperation::Create { path, content } => {
            assert!(path.ends_with("CLAUDE.md"));
            assert_eq!(
                String::from_utf8_lossy(content),
                "# Additional Instructions"
            );
        }
        _ => panic!("Expected Create operation"),
    }
}

#[test]
fn test_plan_replace_claude_md() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    // Create existing CLAUDE.md
    fs::write(temp_dir.path().join("CLAUDE.md"), "# Original Content").expect("Failed to write");

    let profile = create_test_profile_with_claude_md("claude-md-replace", OverlayMode::Replace);

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());

    match &plan.operations[0] {
        FileOperation::Modify { new_content, .. } => {
            assert_eq!(
                String::from_utf8_lossy(new_content),
                "# Additional Instructions"
            );
        }
        _ => panic!("Expected Modify operation"),
    }
}

#[test]
fn test_plan_append_claude_md() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    // Create existing CLAUDE.md
    fs::write(temp_dir.path().join("CLAUDE.md"), "# Original Content").expect("Failed to write");

    let profile = create_test_profile_with_claude_md("claude-md-append", OverlayMode::Append);

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());

    match &plan.operations[0] {
        FileOperation::Modify { new_content, .. } => {
            let content = String::from_utf8_lossy(new_content);
            assert!(content.contains("# Original Content"));
            assert!(content.contains("# Additional Instructions"));
            // Append means new content comes after original
            assert!(content.find("Original").unwrap() < content.find("Additional").unwrap());
        }
        _ => panic!("Expected Modify operation"),
    }
}

#[test]
fn test_plan_prepend_claude_md() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    // Create existing CLAUDE.md
    fs::write(temp_dir.path().join("CLAUDE.md"), "# Original Content").expect("Failed to write");

    let profile = create_test_profile_with_claude_md("claude-md-prepend", OverlayMode::Prepend);

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(!plan.is_empty());

    match &plan.operations[0] {
        FileOperation::Modify { new_content, .. } => {
            let content = String::from_utf8_lossy(new_content);
            assert!(content.contains("# Original Content"));
            assert!(content.contains("# Additional Instructions"));
            // Prepend means new content comes before original
            assert!(content.find("Additional").unwrap() < content.find("Original").unwrap());
        }
        _ => panic!("Expected Modify operation"),
    }
}

// =============================================================================
// Combined Profile Tests
// =============================================================================

#[test]
fn test_plan_full_profile() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    let mut profile = Profile::new("full-profile".to_string());
    profile.repo_overlays = RepoOverlays {
        skills: vec![
            SkillOverlay {
                name: "skill1".to_string(),
                content: "skill1 content".to_string(),
            },
            SkillOverlay {
                name: "skill2".to_string(),
                content: "skill2 content".to_string(),
            },
        ],
        commands: vec![CommandOverlay {
            name: "cmd1".to_string(),
            content: "cmd1 content".to_string(),
        }],
        agents: vec![AgentOverlay {
            name: "agent1".to_string(),
            content: "agent1 content".to_string(),
        }],
        claude_md: Some(ClaudeMdOverlay {
            mode: OverlayMode::Replace,
            content: "# Instructions".to_string(),
        }),
    };

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    // Should have 5 operations: 2 skills + 1 command + 1 agent + 1 CLAUDE.md
    assert_eq!(plan.operations.len(), 5);
}

#[test]
fn test_plan_empty_profile() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();
    let profile = Profile::new("empty-profile".to_string());

    let plan =
        generate_plan(project_id, temp_dir.path(), &profile).expect("Failed to generate plan");

    assert!(plan.is_empty());
}

// =============================================================================
// Path Validation Tests
// =============================================================================

#[test]
fn test_plan_rejects_path_traversal_skill() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    let mut profile = Profile::new("path-traversal".to_string());
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "../../../etc/passwd".to_string(),
        content: "malicious".to_string(),
    });

    let result = generate_plan(project_id, temp_dir.path(), &profile);
    assert!(result.is_err());
}

#[test]
fn test_plan_rejects_path_traversal_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_id = Uuid::new_v4();

    let mut profile = Profile::new("path-traversal".to_string());
    profile.repo_overlays.commands.push(CommandOverlay {
        name: "../../malicious".to_string(),
        content: "malicious".to_string(),
    });

    let result = generate_plan(project_id, temp_dir.path(), &profile);
    assert!(result.is_err());
}
