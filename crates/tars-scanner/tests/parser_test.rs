//! Frontmatter parser tests
//!
//! Tests for parsing YAML frontmatter in skills, commands, and agents.

use std::path::PathBuf;
use tars_scanner::parser::frontmatter::{parse_agent, parse_command, parse_skill};
use tars_scanner::types::Scope;

// =============================================================================
// Skill Parsing Tests
// =============================================================================

#[test]
fn test_parse_skill_minimal() {
    let content = r"---
name: minimal-skill
description: A minimal skill
---

Instructions.
";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_ok());

    let skill = result.unwrap();
    assert_eq!(skill.name, "minimal-skill");
    assert_eq!(skill.description, "A minimal skill");
    assert!(!skill.user_invocable); // Default is false
    assert!(!skill.disable_model_invocation);
    assert!(skill.allowed_tools.is_empty());
    assert!(skill.model.is_none());
    assert!(skill.context.is_none());
    assert!(skill.agent.is_none());
}

#[test]
fn test_parse_skill_full() {
    let content = r#"---
name: full-skill
description: A fully configured skill
user-invocable: true
disable-model-invocation: true
allowed-tools:
  - Read
  - Write
  - Bash
model: sonnet
context: current-directory
agent: my-agent
hooks:
  pre-tool-use:
    - type: command
      command: echo "before"
---

Full skill instructions.
"#;
    let result = parse_skill(&PathBuf::from("test"), content, Scope::Project);
    assert!(result.is_ok());

    let skill = result.unwrap();
    assert_eq!(skill.name, "full-skill");
    assert_eq!(skill.description, "A fully configured skill");
    assert!(skill.user_invocable);
    assert!(skill.disable_model_invocation);
    assert_eq!(skill.allowed_tools, vec!["Read", "Write", "Bash"]);
    assert_eq!(skill.model, Some("sonnet".to_string()));
    assert_eq!(skill.context, Some("current-directory".to_string()));
    assert_eq!(skill.agent, Some("my-agent".to_string()));
    assert!(!skill.hooks.is_empty());
    assert_eq!(skill.scope, Scope::Project);
}

#[test]
fn test_parse_skill_missing_frontmatter() {
    let content = "# Just markdown, no frontmatter";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_err());
}

#[test]
fn test_parse_skill_missing_required_fields() {
    let content = r"---
description: Missing name field
---

Instructions.
";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_err());
}

#[test]
fn test_parse_skill_invalid_yaml() {
    let content = r"---
name: test
description: [invalid: yaml: syntax:
---

Instructions.
";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_err());
}

#[test]
fn test_parse_skill_preserves_body() {
    let content = r"---
name: body-test
description: Test body preservation
---

# Instructions

This is the body content.

## Section

More content here.
";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_ok());
    // Note: Skills don't have a body field, but commands do
}

#[test]
fn test_parse_skill_sha256_consistency() {
    let content = r"---
name: hash-test
description: Hash consistency test
---

Instructions.
";
    let result1 = parse_skill(&PathBuf::from("test"), content, Scope::User);
    let result2 = parse_skill(&PathBuf::from("test"), content, Scope::User);

    assert!(result1.is_ok() && result2.is_ok());
    assert_eq!(result1.unwrap().sha256, result2.unwrap().sha256);
}

#[test]
fn test_parse_skill_different_content_different_hash() {
    let content1 = r"---
name: hash-test
description: First content
---

Instructions.
";
    let content2 = r"---
name: hash-test
description: Second content
---

Different instructions.
";
    let result1 = parse_skill(&PathBuf::from("test"), content1, Scope::User);
    let result2 = parse_skill(&PathBuf::from("test"), content2, Scope::User);

    assert!(result1.is_ok() && result2.is_ok());
    assert_ne!(result1.unwrap().sha256, result2.unwrap().sha256);
}

// =============================================================================
// Command Parsing Tests
// =============================================================================

#[test]
fn test_parse_command_minimal() {
    let content = r"Execute $ARGUMENTS";
    let result = parse_command(&PathBuf::from("my-cmd.md"), content, Scope::User);
    assert!(result.is_ok());

    let cmd = result.unwrap();
    assert_eq!(cmd.name, "my-cmd");
    assert!(cmd.description.is_none());
    assert!(!cmd.thinking);
    assert_eq!(cmd.body, "Execute $ARGUMENTS");
}

#[test]
fn test_parse_command_with_frontmatter() {
    let content = r"---
description: A helpful command
thinking: true
---

Execute the task: $ARGUMENTS

With detailed steps:
1. First step
2. Second step
";
    let result = parse_command(&PathBuf::from("helpful.md"), content, Scope::Project);
    assert!(result.is_ok());

    let cmd = result.unwrap();
    assert_eq!(cmd.name, "helpful");
    assert_eq!(cmd.description, Some("A helpful command".to_string()));
    assert!(cmd.thinking);
    assert!(cmd.body.contains("Execute the task"));
    assert!(cmd.body.contains("$ARGUMENTS"));
}

#[test]
fn test_parse_command_preserves_body() {
    let content = r"---
description: Test
---

Line 1
Line 2
Line 3
";
    let result = parse_command(&PathBuf::from("test.md"), content, Scope::User);
    assert!(result.is_ok());

    let cmd = result.unwrap();
    assert!(cmd.body.contains("Line 1"));
    assert!(cmd.body.contains("Line 2"));
    assert!(cmd.body.contains("Line 3"));
}

#[test]
fn test_parse_command_name_from_filename() {
    let result = parse_command(
        &PathBuf::from("/path/to/my-command.md"),
        "content",
        Scope::User,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "my-command");
}

#[test]
fn test_parse_command_handles_variables() {
    let content = r"---
description: Variable test
---

Use $1 and $2 as arguments.
Full args: $ARGUMENTS
";
    let result = parse_command(&PathBuf::from("var-test.md"), content, Scope::User);
    assert!(result.is_ok());

    let cmd = result.unwrap();
    assert!(cmd.body.contains("$1"));
    assert!(cmd.body.contains("$2"));
    assert!(cmd.body.contains("$ARGUMENTS"));
}

// =============================================================================
// Agent Parsing Tests
// =============================================================================

#[test]
fn test_parse_agent_minimal() {
    let content = r"---
name: minimal-agent
description: A minimal agent
---

Agent instructions.
";
    let result = parse_agent(&PathBuf::from("agent.md"), content, Scope::User);
    assert!(result.is_ok());

    let agent = result.unwrap();
    assert_eq!(agent.name, "minimal-agent");
    assert_eq!(agent.description, "A minimal agent");
    assert!(agent.tools.is_empty());
    assert!(agent.model.is_none());
    assert_eq!(agent.permission_mode, "default");
    assert!(agent.skills.is_empty());
}

#[test]
fn test_parse_agent_full() {
    let content = r#"---
name: full-agent
description: A fully configured agent
tools:
  - Read
  - Write
  - Bash
  - Glob
model: opus
permission-mode: strict
skills:
  - skill-one
  - skill-two
hooks:
  session-start:
    - type: command
      command: echo "Agent starting"
---

Full agent instructions.
"#;
    let result = parse_agent(&PathBuf::from("agent.md"), content, Scope::Project);
    assert!(result.is_ok());

    let agent = result.unwrap();
    assert_eq!(agent.name, "full-agent");
    assert_eq!(agent.description, "A fully configured agent");
    assert_eq!(agent.tools, vec!["Read", "Write", "Bash", "Glob"]);
    assert_eq!(agent.model, Some("opus".to_string()));
    assert_eq!(agent.permission_mode, "strict");
    assert_eq!(agent.skills, vec!["skill-one", "skill-two"]);
    assert!(!agent.hooks.is_empty());
    assert_eq!(agent.scope, Scope::Project);
}

#[test]
fn test_parse_agent_missing_required_fields() {
    let content = r"---
description: Missing name
---

Instructions.
";
    let result = parse_agent(&PathBuf::from("agent.md"), content, Scope::User);
    assert!(result.is_err());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_parse_empty_frontmatter() {
    let content = r"---
---

Just body.
";
    // Skills require name and description, so this should fail
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_err());
}

#[test]
fn test_parse_unicode_content() {
    let content = r"---
name: unicode-skill
description: A skill with unicode
---

Instructions with unicode: , ,
";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_ok());

    let skill = result.unwrap();
    assert_eq!(skill.name, "unicode-skill");
}

#[test]
fn test_parse_multiline_description() {
    let content = r"---
name: multiline-test
description: |
  This is a multiline
  description that spans
  multiple lines
---

Instructions.
";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_ok());

    let skill = result.unwrap();
    assert!(skill.description.contains("multiline"));
    assert!(skill.description.contains("multiple lines"));
}

#[test]
fn test_parse_scope_preserved() {
    let content = r"---
name: scope-test
description: Scope test
---

Instructions.
";

    let user_result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    let project_result = parse_skill(&PathBuf::from("test"), content, Scope::Project);
    let managed_result = parse_skill(&PathBuf::from("test"), content, Scope::Managed);

    assert!(user_result.is_ok() && project_result.is_ok() && managed_result.is_ok());

    assert_eq!(user_result.unwrap().scope, Scope::User);
    assert_eq!(project_result.unwrap().scope, Scope::Project);
    assert_eq!(managed_result.unwrap().scope, Scope::Managed);
}

#[test]
fn test_parse_handles_windows_line_endings() {
    let content = "---\r\nname: windows-test\r\ndescription: Windows line endings\r\n---\r\n\r\nInstructions.\r\n";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "windows-test");
}

#[test]
fn test_parse_handles_mixed_line_endings() {
    let content =
        "---\nname: mixed-test\r\ndescription: Mixed line endings\n---\r\n\nInstructions.\n";
    let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "mixed-test");
}
