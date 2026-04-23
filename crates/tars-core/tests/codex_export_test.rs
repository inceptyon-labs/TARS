use std::fs;
use tars_core::config::{HookConfig, HookDefinition, HookTrigger};
use tars_core::export::export_as_codex_bundle;
use tars_core::profile::storage::{
    copy_agent_to_profile, copy_command_to_profile, copy_skill_to_profile, ensure_profile_dir,
    profile_dir, store_mcp_server,
};
use tars_core::profile::{Profile, ToolRef, ToolType};
use tempfile::TempDir;

struct ProfileStorageGuard(uuid::Uuid);

impl Drop for ProfileStorageGuard {
    fn drop(&mut self) {
        if let Ok(path) = profile_dir(self.0) {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[test]
fn test_export_as_codex_bundle_generates_plugin_marketplace_and_report() {
    let output_dir = TempDir::new().expect("Failed to create output dir");
    let source_dir = TempDir::new().expect("Failed to create source dir");

    let mut profile = Profile::new("Team Bundle".to_string());
    profile.description = Some("Portable coding workflow".to_string());
    let _storage_guard = ProfileStorageGuard(profile.id);

    let stored_skill_dir = source_dir.path().join("stored-skill");
    fs::create_dir_all(&stored_skill_dir).expect("Failed to create stored skill dir");
    fs::write(
        stored_skill_dir.join("SKILL.md"),
        "---\ndescription: Stored skill without an explicit name\n---\n\nUse this skill carefully.\n",
    )
    .expect("Failed to write stored skill");
    copy_skill_to_profile(profile.id, "stored-skill", &stored_skill_dir)
        .expect("Failed to copy stored skill");

    let stored_agent_path = source_dir.path().join("reviewer.md");
    fs::write(
        &stored_agent_path,
        "---\nname: reviewer\ndescription: Review code for correctness\nmodel: gpt-5.4\n---\n\nReview code like an owner.\n",
    )
    .expect("Failed to write stored agent");
    copy_agent_to_profile(profile.id, "reviewer", &stored_agent_path)
        .expect("Failed to copy stored agent");

    let stored_command_path = source_dir.path().join("ship-it.md");
    fs::write(
        &stored_command_path,
        "---\ndescription: Prepare a release summary\n---\n\nSummarize the release notes and highlight risks.\n",
    )
    .expect("Failed to write stored command");
    copy_command_to_profile(profile.id, "ship-it", &stored_command_path)
        .expect("Failed to copy stored command");

    store_mcp_server(
        profile.id,
        "context7",
        &serde_json::json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "@upstash/context7-mcp"],
            "env": {
                "LOCAL_TOKEN": "secret"
            }
        }),
    )
    .expect("Failed to store MCP config");

    profile.tool_refs = vec![
        ToolRef {
            name: "stored-skill".to_string(),
            tool_type: ToolType::Skill,
            source_scope: None,
            permissions: None,
            source_ref: None,
        },
        ToolRef {
            name: "reviewer".to_string(),
            tool_type: ToolType::Agent,
            source_scope: None,
            permissions: None,
            source_ref: None,
        },
        ToolRef {
            name: "ship-it".to_string(),
            tool_type: ToolType::Hook,
            source_scope: None,
            permissions: None,
            source_ref: None,
        },
        ToolRef {
            name: "context7".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: None,
            permissions: None,
            source_ref: None,
        },
    ];

    profile.repo_overlays.skills.push(tars_core::profile::SkillOverlay {
        name: "overlay-skill".to_string(),
        content: "---\nname: overlay-skill\ndescription: Overlay skill with an embedded hook\nhooks:\n  SessionStart:\n    - type: prompt\n      prompt: Warm up the session\n---\n\nStay focused.\n"
            .to_string(),
    });

    let hooks_dir = ensure_profile_dir(profile.id).expect("Failed to create profile storage dir");
    fs::write(
        hooks_dir.join("hooks.json"),
        serde_json::to_string_pretty(&vec![HookConfig::new(
            HookTrigger::PreToolUse,
            HookDefinition::command("npm test"),
        )])
        .expect("Failed to serialize hooks"),
    )
    .expect("Failed to write hooks");

    let result = export_as_codex_bundle(
        &profile,
        output_dir.path(),
        "team-bundle",
        "1.2.3",
        "team-marketplace",
    )
    .expect("Codex export failed");

    let manifest_path = result.plugin_root.join(".codex-plugin").join("plugin.json");
    let manifest = fs::read_to_string(&manifest_path).expect("Failed to read plugin manifest");
    assert!(manifest.contains("\"name\": \"team-bundle\""));
    assert!(manifest.contains("\"skills\": \"./skills/\""));
    assert!(manifest.contains("\"mcpServers\": \"./.mcp.json\""));

    let injected_skill = fs::read_to_string(
        result
            .plugin_root
            .join("skills")
            .join("stored-skill")
            .join("SKILL.md"),
    )
    .expect("Failed to read injected skill");
    assert!(injected_skill.contains("name: \"stored-skill\""));

    let converted_command = fs::read_to_string(
        result
            .plugin_root
            .join("skills")
            .join("command-ship-it")
            .join("SKILL.md"),
    )
    .expect("Failed to read converted command skill");
    assert!(
        converted_command.contains("This skill was converted from the Claude command `/ship-it`.")
    );

    let agent_toml = fs::read_to_string(
        result
            .agents_dir
            .clone()
            .expect("Expected agents dir")
            .join("reviewer.toml"),
    )
    .expect("Failed to read agent TOML");
    assert!(agent_toml.contains("name = \"reviewer\""));
    assert!(agent_toml.contains("developer_instructions = "));
    assert!(agent_toml.contains("model = \"gpt-5.4\""));

    let config_toml = fs::read_to_string(result.config_path.clone().expect("Expected config path"))
        .expect("Failed to read config TOML");
    assert!(config_toml.contains("[mcp_servers.\"context7\"]"));
    assert!(config_toml.contains("command = \"npx\""));

    let marketplace =
        fs::read_to_string(&result.marketplace_path).expect("Failed to read marketplace");
    assert!(marketplace.contains("\"name\": \"team-marketplace\""));
    assert!(marketplace.contains("\"path\": \"./plugins/team-bundle\""));

    let plugin_mcp =
        fs::read_to_string(result.plugin_root.join(".mcp.json")).expect("Failed to read .mcp.json");
    assert!(plugin_mcp.contains("\"context7\""));

    assert!(result.report.findings.iter().any(|finding| {
        finding.artifact_kind == tars_core::export::CodexArtifactKind::Command
            && finding.name == "ship-it"
            && finding.support == tars_core::tars_scanner::runtime::RuntimeSupport::Convertible
    }));
    assert!(result.report.findings.iter().any(|finding| {
        finding.artifact_kind == tars_core::export::CodexArtifactKind::Hook
            && finding.support == tars_core::tars_scanner::runtime::RuntimeSupport::Partial
    }));
}
