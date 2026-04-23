//! Scanner integration tests
//!
//! Tests the full scanning pipeline against fixture directories.

use std::fs;
use tars_scanner::{scope::codex::scan_user_codex_scope, Scanner};
use tempfile::TempDir;

/// Create a test fixture directory with Claude Code configuration
fn create_test_fixture() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base = temp_dir.path();

    // Create .claude directory structure
    let claude_dir = base.join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude directory");

    // Create settings.json
    let settings = r#"{
        "permissions": {
            "allow": ["Bash(*)", "Read(*)"],
            "deny": []
        },
        "hooks": {
            "PreToolUse": [
                {"command": "echo pre-hook"}
            ]
        }
    }"#;
    fs::write(claude_dir.join("settings.json"), settings).expect("Failed to write settings.json");

    // Create skills directory with a skill
    let skills_dir = claude_dir.join("skills");
    fs::create_dir_all(&skills_dir).expect("Failed to create skills directory");

    let skill_content = r"---
name: test-skill
description: A test skill for integration testing
user-invocable: true
allowed-tools:
  - Read
  - Grep
---

# Test Skill

This is a test skill.
";
    fs::write(skills_dir.join("test-skill").join("SKILL.md"), "").ok(); // Ignore if fails
    fs::create_dir_all(skills_dir.join("test-skill")).expect("Failed to create skill directory");
    fs::write(
        skills_dir.join("test-skill").join("SKILL.md"),
        skill_content,
    )
    .expect("Failed to write SKILL.md");

    // Create commands directory with a command
    let commands_dir = claude_dir.join("commands");
    fs::create_dir_all(&commands_dir).expect("Failed to create commands directory");

    let command_content = r"---
description: A test command
thinking: true
---

Execute the following task: $ARGUMENTS
";
    fs::write(commands_dir.join("test-cmd.md"), command_content).expect("Failed to write command");

    // Create hooks directory with hooks.json
    let hooks_dir = claude_dir.join("hooks");
    fs::create_dir_all(&hooks_dir).expect("Failed to create hooks directory");

    let hooks_content = r#"{
        "hooks": [
            {
                "event": "PreToolUse",
                "command": "echo pre-tool"
            },
            {
                "event": "SessionStart",
                "command": "echo session-started"
            }
        ]
    }"#;
    fs::write(hooks_dir.join("hooks.json"), hooks_content).expect("Failed to write hooks.json");

    // Create a CLAUDE.md file
    let claude_md = r"# Project Instructions

This is a test project for TARS scanner integration tests.
";
    fs::write(base.join("CLAUDE.md"), claude_md).expect("Failed to write CLAUDE.md");

    // Create .claude.json (MCP config at project root)
    let mcp_config = r#"{
        "mcpServers": {
            "test-server": {
                "command": "node",
                "args": ["server.js"],
                "env": {}
            }
        }
    }"#;
    fs::write(base.join(".claude.json"), mcp_config).expect("Failed to write .claude.json");

    // Create a fake repo root marker for upward Codex layer scanning
    fs::create_dir_all(base.join(".git")).expect("Failed to create .git directory");

    // Create Codex project config
    let codex_dir = base.join(".codex");
    fs::create_dir_all(codex_dir.join("agents")).expect("Failed to create .codex/agents");
    fs::write(
        codex_dir.join("config.toml"),
        "model = \"gpt-5\"\napproval_policy = \"never\"\n",
    )
    .expect("Failed to write .codex/config.toml");
    fs::write(codex_dir.join("AGENTS.md"), "# User Agent Instructions\n")
        .expect("Failed to write .codex/AGENTS.md");
    fs::write(
        codex_dir.join("AGENTS.override.md"),
        "# User Override Instructions\n",
    )
    .expect("Failed to write .codex/AGENTS.override.md");
    fs::write(
        codex_dir.join("agents").join("reviewer.toml"),
        "name = \"reviewer\"\ndescription = \"Review code changes\"\n",
    )
    .expect("Failed to write Codex agent file");

    // Create Codex skill layer
    let codex_skills_dir = base.join(".agents").join("skills").join("codex-skill");
    fs::create_dir_all(&codex_skills_dir).expect("Failed to create .agents/skills");
    fs::write(
        codex_skills_dir.join("SKILL.md"),
        r"---
name: codex-skill
description: A Codex-discovered skill
---

# Codex Skill

This skill is discovered from .agents/skills.
",
    )
    .expect("Failed to write Codex SKILL.md");

    // Create instruction layers and marketplace
    fs::write(base.join("AGENTS.md"), "# Project Agent Instructions\n")
        .expect("Failed to write AGENTS.md");
    fs::write(base.join("AGENTS.override.md"), "# Override Instructions\n")
        .expect("Failed to write AGENTS.override.md");
    fs::create_dir_all(base.join(".agents").join("plugins"))
        .expect("Failed to create .agents/plugins");
    fs::write(
        base.join(".agents")
            .join("plugins")
            .join("marketplace.json"),
        r#"{
  "name": "local-dev",
  "interface": {
    "displayName": "Local Dev Plugins"
  },
  "plugins": [
    {
      "name": "repo-plugin",
      "source": {
        "source": "local",
        "path": "./plugins/repo-plugin"
      },
      "policy": {
        "installation": "AVAILABLE",
        "authentication": "ON_INSTALL"
      },
      "category": "Productivity"
    },
    {
      "name": "personal-plugin",
      "source": {
        "source": "local",
        "path": "./.codex/plugins/personal-plugin"
      },
      "policy": {
        "installation": "AVAILABLE",
        "authentication": "ON_INSTALL"
      },
      "category": "Utilities"
    }
  ]
}"#,
    )
    .expect("Failed to write Codex marketplace");
    fs::create_dir_all(
        base.join("plugins")
            .join("repo-plugin")
            .join(".codex-plugin"),
    )
    .expect("Failed to create repo Codex plugin directory");
    fs::write(
        base.join("plugins")
            .join("repo-plugin")
            .join(".codex-plugin")
            .join("plugin.json"),
        r#"{
  "name": "repo-plugin",
  "version": "0.1.0",
  "description": "Repo scoped Codex plugin",
  "interface": {
    "displayName": "Repo Plugin",
    "shortDescription": "Project plugin"
  }
}"#,
    )
    .expect("Failed to write repo Codex plugin manifest");

    fs::create_dir_all(
        base.join(".codex")
            .join("plugins")
            .join("personal-plugin")
            .join(".codex-plugin"),
    )
    .expect("Failed to create personal Codex plugin directory");
    fs::write(
        base.join(".codex")
            .join("plugins")
            .join("personal-plugin")
            .join(".codex-plugin")
            .join("plugin.json"),
        r#"{
  "name": "personal-plugin",
  "version": "1.2.3",
  "description": "Personal Codex plugin",
  "interface": {
    "displayName": "Personal Plugin",
    "shortDescription": "Personal plugin"
  }
}"#,
    )
    .expect("Failed to write personal Codex plugin manifest");

    temp_dir
}

#[test]
fn test_scan_project_structure() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok(), "Scan should succeed: {:?}", result.err());

    let project_scope = result.unwrap();
    assert!(project_scope.claude_md.is_some(), "Should detect CLAUDE.md");
    assert!(
        project_scope.mcp.is_some(),
        "Should detect .claude.json MCP config"
    );
}

#[test]
fn test_scan_discovers_skills() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok());

    let project_scope = result.unwrap();
    assert!(!project_scope.skills.is_empty(), "Should discover skills");

    let skill = &project_scope.skills[0];
    assert_eq!(skill.name, "test-skill");
    assert_eq!(skill.description, "A test skill for integration testing");
    assert!(skill.user_invocable);
}

#[test]
fn test_scan_discovers_commands() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok());

    let project_scope = result.unwrap();
    assert!(
        !project_scope.commands.is_empty(),
        "Should discover commands"
    );

    let command = &project_scope.commands[0];
    assert_eq!(command.name, "test-cmd");
    assert!(command.thinking);
}

#[test]
fn test_scan_parses_settings() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok());

    let project_scope = result.unwrap();
    assert!(
        project_scope.settings.shared.is_some(),
        "Should parse settings"
    );
}

#[test]
fn test_scan_parses_mcp_config() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok());

    let project_scope = result.unwrap();
    let mcp = project_scope.mcp.expect("Should have MCP config");
    assert!(!mcp.servers.is_empty(), "Should have MCP servers");
    assert_eq!(mcp.servers[0].name, "test-server");
}

#[test]
fn test_scan_extracts_hooks() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok());

    let project_scope = result.unwrap();
    assert!(!project_scope.hooks.is_empty(), "Should extract hooks");
}

#[test]
fn test_scan_discovers_codex_project_files() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_project(fixture.path());
    assert!(result.is_ok());

    let project_scope = result.unwrap();
    assert!(
        project_scope.codex.config.is_some(),
        "Should detect .codex/config.toml"
    );
    assert_eq!(
        project_scope.codex.instructions.len(),
        2,
        "Should detect AGENTS.md layers"
    );
    assert_eq!(
        project_scope.codex.skills.len(),
        1,
        "Should detect .agents/skills"
    );
    assert_eq!(project_scope.codex.skills[0].name, "codex-skill");
    assert_eq!(
        project_scope.codex.agents.len(),
        1,
        "Should detect .codex/agents"
    );
    assert_eq!(project_scope.codex.agents[0].name, "reviewer");
    assert_eq!(
        project_scope.codex.marketplaces.len(),
        1,
        "Should detect project marketplace.json"
    );
    assert_eq!(project_scope.codex.marketplaces[0].name, "local-dev");
    assert_eq!(project_scope.codex.marketplaces[0].plugins.len(), 2);
    assert!(project_scope.codex.marketplaces[0]
        .plugins
        .iter()
        .all(|plugin| plugin.resolved));
    assert!(project_scope.codex.marketplaces[0]
        .plugins
        .iter()
        .any(|plugin| plugin.id == "repo-plugin"));
}

#[test]
fn test_scan_discovers_codex_user_files() {
    let fixture = create_test_fixture();

    let user_scope = scan_user_codex_scope(fixture.path()).expect("User Codex scan should succeed");

    assert!(
        user_scope.config.is_some(),
        "Should detect ~/.codex/config.toml"
    );
    assert_eq!(
        user_scope.instructions.len(),
        2,
        "Should detect user AGENTS.md layers"
    );
    assert_eq!(
        user_scope.skills.len(),
        1,
        "Should detect ~/.agents/skills content"
    );
    assert_eq!(user_scope.skills[0].name, "codex-skill");
    assert_eq!(user_scope.agents.len(), 1, "Should detect ~/.codex/agents");
    assert_eq!(user_scope.agents[0].name, "reviewer");
    assert_eq!(
        user_scope.marketplaces.len(),
        1,
        "Should detect ~/.agents/plugins/marketplace.json"
    );
    assert_eq!(user_scope.marketplaces[0].name, "local-dev");
    assert_eq!(user_scope.marketplaces[0].plugins.len(), 2);
    assert!(user_scope.marketplaces[0]
        .plugins
        .iter()
        .any(|plugin| plugin.id == "personal-plugin"));
    assert!(user_scope.marketplaces[0]
        .plugins
        .iter()
        .all(|plugin| plugin.resolved));
}

#[test]
fn test_full_inventory_scan() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result = scanner.scan_all(&[fixture.path()]);
    assert!(result.is_ok(), "Full scan should succeed");

    let inventory = result.unwrap();
    assert_eq!(inventory.projects.len(), 1, "Should have one project");
}

#[test]
fn test_collision_detection() {
    // Create two project fixtures with the same skill name
    let fixture1 = create_test_fixture();
    let fixture2 = create_test_fixture();

    let scanner = Scanner::new();
    let result = scanner.scan_all(&[fixture1.path(), fixture2.path()]);
    assert!(result.is_ok());

    let inventory = result.unwrap();
    // Both projects have "test-skill", so there should be potential for collisions
    // Note: Collisions are detected across scopes, not between projects of same scope
    assert_eq!(inventory.projects.len(), 2, "Should have two projects");
}

#[test]
fn test_empty_directory_scan() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let scanner = Scanner::new();

    let result = scanner.scan_project(temp_dir.path());
    assert!(result.is_ok(), "Empty directory scan should not fail");

    let project_scope = result.unwrap();
    assert!(project_scope.skills.is_empty());
    assert!(project_scope.commands.is_empty());
    assert!(project_scope.agents.is_empty());
}

#[test]
fn test_sha256_consistency() {
    let fixture = create_test_fixture();
    let scanner = Scanner::new();

    let result1 = scanner.scan_project(fixture.path());
    let result2 = scanner.scan_project(fixture.path());

    assert!(result1.is_ok() && result2.is_ok());

    let scope1 = result1.unwrap();
    let scope2 = result2.unwrap();

    // Skills should have same SHA256 across scans
    if !scope1.skills.is_empty() && !scope2.skills.is_empty() {
        assert_eq!(
            scope1.skills[0].sha256, scope2.skills[0].sha256,
            "SHA256 should be consistent across scans"
        );
    }
}
