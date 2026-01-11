//! Profile export/import round-trip tests
//!
//! Tests for exporting profiles to JSON and importing them back.

use std::path::PathBuf;
use tars_core::profile::{export, Profile, ToolPermissions, ToolRef, ToolType};
use tempfile::tempdir;

fn create_profile_with_tools() -> Profile {
    let mut profile = Profile::new("test-export".to_string());
    profile.description = Some("A profile for testing export/import".to_string());
    profile.tool_refs = vec![
        ToolRef {
            name: "context7".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: None,
            permissions: Some(ToolPermissions {
                allowed_directories: vec![PathBuf::from("/home/user/projects")],
                allowed_tools: vec!["query-docs".to_string(), "resolve-library-id".to_string()],
                disallowed_tools: vec!["dangerous-tool".to_string()],
            }),
        },
        ToolRef {
            name: "my-skill".to_string(),
            tool_type: ToolType::Skill,
            source_scope: None,
            permissions: None,
        },
        ToolRef {
            name: "test-agent".to_string(),
            tool_type: ToolType::Agent,
            source_scope: None,
            permissions: Some(ToolPermissions {
                allowed_directories: vec![],
                allowed_tools: vec!["read".to_string()],
                disallowed_tools: vec![],
            }),
        },
    ];
    profile
}

#[test]
fn test_export_import_roundtrip() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let original = create_profile_with_tools();

    // Export
    let exported = export::export_profile(&original, &export_path).expect("Failed to export");
    assert_eq!(exported.name, "test-export");
    assert_eq!(exported.tool_refs.len(), 3);

    // Import
    let imported = export::import_profile(&export_path).expect("Failed to import");

    // Verify basic fields
    assert_eq!(imported.name, original.name);
    assert_eq!(imported.description, original.description);
    assert_eq!(imported.tool_refs.len(), original.tool_refs.len());

    // Note: ID is regenerated on import, so don't compare IDs
    assert_ne!(imported.id, original.id);
}

#[test]
fn test_export_preserves_tool_types() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let original = create_profile_with_tools();
    export::export_profile(&original, &export_path).expect("Failed to export");
    let imported = export::import_profile(&export_path).expect("Failed to import");

    // Verify all tool types are preserved
    let mcp_tool = imported.tool_refs.iter().find(|t| t.name == "context7");
    assert!(mcp_tool.is_some());
    assert_eq!(mcp_tool.unwrap().tool_type, ToolType::Mcp);

    let skill_tool = imported.tool_refs.iter().find(|t| t.name == "my-skill");
    assert!(skill_tool.is_some());
    assert_eq!(skill_tool.unwrap().tool_type, ToolType::Skill);

    let agent_tool = imported.tool_refs.iter().find(|t| t.name == "test-agent");
    assert!(agent_tool.is_some());
    assert_eq!(agent_tool.unwrap().tool_type, ToolType::Agent);
}

#[test]
fn test_export_preserves_allowed_tools_permissions() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let original = create_profile_with_tools();
    export::export_profile(&original, &export_path).expect("Failed to export");
    let imported = export::import_profile(&export_path).expect("Failed to import");

    // Find the MCP tool with permissions
    let mcp_tool = imported
        .tool_refs
        .iter()
        .find(|t| t.name == "context7")
        .expect("MCP tool not found");

    let perms = mcp_tool.permissions.as_ref().expect("Expected permissions");

    // allowed_tools should be preserved
    assert_eq!(perms.allowed_tools.len(), 2);
    assert!(perms.allowed_tools.contains(&"query-docs".to_string()));
    assert!(perms
        .allowed_tools
        .contains(&"resolve-library-id".to_string()));

    // disallowed_tools should be preserved
    assert_eq!(perms.disallowed_tools.len(), 1);
    assert!(perms
        .disallowed_tools
        .contains(&"dangerous-tool".to_string()));
}

#[test]
fn test_export_excludes_directories_for_portability() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let original = create_profile_with_tools();
    export::export_profile(&original, &export_path).expect("Failed to export");
    let imported = export::import_profile(&export_path).expect("Failed to import");

    // Find the MCP tool with permissions
    let mcp_tool = imported
        .tool_refs
        .iter()
        .find(|t| t.name == "context7")
        .expect("MCP tool not found");

    let perms = mcp_tool.permissions.as_ref().expect("Expected permissions");

    // allowed_directories should be empty (not exported for portability)
    assert!(perms.allowed_directories.is_empty());
}

#[test]
fn test_export_tool_without_permissions() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let original = create_profile_with_tools();
    export::export_profile(&original, &export_path).expect("Failed to export");
    let imported = export::import_profile(&export_path).expect("Failed to import");

    // Find the skill tool (no permissions)
    let skill_tool = imported
        .tool_refs
        .iter()
        .find(|t| t.name == "my-skill")
        .expect("Skill tool not found");

    // Should have no permissions
    assert!(skill_tool.permissions.is_none());
}

#[test]
fn test_preview_import() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let original = create_profile_with_tools();
    export::export_profile(&original, &export_path).expect("Failed to export");

    // Preview the import
    let preview = export::preview_import(&export_path).expect("Failed to preview");

    assert_eq!(preview.name, "test-export");
    assert_eq!(
        preview.description,
        Some("A profile for testing export/import".to_string())
    );
    assert_eq!(preview.tool_count, 3);
    assert_eq!(preview.version, 1);
}

#[test]
fn test_export_empty_profile() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("empty.tars-profile.json");

    let original = Profile::new("empty-profile".to_string());
    export::export_profile(&original, &export_path).expect("Failed to export");

    let imported = export::import_profile(&export_path).expect("Failed to import");

    assert_eq!(imported.name, "empty-profile");
    assert!(imported.description.is_none());
    assert!(imported.tool_refs.is_empty());
}

#[test]
fn test_export_json_is_human_readable() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let original = create_profile_with_tools();
    export::export_profile(&original, &export_path).expect("Failed to export");

    // Read the raw JSON
    let json_content = std::fs::read_to_string(&export_path).expect("Failed to read file");

    // Should be pretty-printed (contains newlines and indentation)
    assert!(json_content.contains('\n'));
    assert!(json_content.contains("  ")); // 2-space indentation

    // Should contain expected fields
    assert!(json_content.contains("\"name\""));
    assert!(json_content.contains("\"tool_refs\""));
    assert!(json_content.contains("\"test-export\""));
}

#[test]
fn test_import_invalid_file_fails() {
    let dir = tempdir().expect("Failed to create temp dir");
    let invalid_path = dir.path().join("invalid.json");

    // Write invalid JSON
    std::fs::write(&invalid_path, "{ invalid json }").expect("Failed to write file");

    let result = export::import_profile(&invalid_path);
    assert!(result.is_err());
}

#[test]
fn test_import_nonexistent_file_fails() {
    let dir = tempdir().expect("Failed to create temp dir");
    let nonexistent = dir.path().join("does-not-exist.json");

    let result = export::import_profile(&nonexistent);
    assert!(result.is_err());
}

#[test]
fn test_export_preserves_description() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let mut profile = Profile::new("described-profile".to_string());
    profile.description = Some("This is a detailed description\nwith multiple lines".to_string());

    export::export_profile(&profile, &export_path).expect("Failed to export");
    let imported = export::import_profile(&export_path).expect("Failed to import");

    assert_eq!(imported.description, profile.description);
}

#[test]
fn test_all_tool_types_roundtrip() {
    let dir = tempdir().expect("Failed to create temp dir");
    let export_path = dir.path().join("profile.tars-profile.json");

    let mut profile = Profile::new("all-types".to_string());
    profile.tool_refs = vec![
        ToolRef {
            name: "mcp-tool".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: None,
            permissions: None,
        },
        ToolRef {
            name: "skill-tool".to_string(),
            tool_type: ToolType::Skill,
            source_scope: None,
            permissions: None,
        },
        ToolRef {
            name: "agent-tool".to_string(),
            tool_type: ToolType::Agent,
            source_scope: None,
            permissions: None,
        },
        ToolRef {
            name: "hook-tool".to_string(),
            tool_type: ToolType::Hook,
            source_scope: None,
            permissions: None,
        },
    ];

    export::export_profile(&profile, &export_path).expect("Failed to export");
    let imported = export::import_profile(&export_path).expect("Failed to import");

    assert_eq!(imported.tool_refs.len(), 4);

    // Verify each type is correctly preserved
    assert!(imported
        .tool_refs
        .iter()
        .any(|t| t.tool_type == ToolType::Mcp));
    assert!(imported
        .tool_refs
        .iter()
        .any(|t| t.tool_type == ToolType::Skill));
    assert!(imported
        .tool_refs
        .iter()
        .any(|t| t.tool_type == ToolType::Agent));
    assert!(imported
        .tool_refs
        .iter()
        .any(|t| t.tool_type == ToolType::Hook));
}
