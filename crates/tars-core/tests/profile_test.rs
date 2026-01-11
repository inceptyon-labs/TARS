//! Profile CRUD tests
//!
//! Tests for profile storage operations.

#![allow(clippy::similar_names)]

use tars_core::profile::{
    Adapters, AgentOverlay, ClaudeMdOverlay, CommandOverlay, McpLocation, OverlayMode, PluginRef,
    PluginSet, Profile, RepoOverlays, SkillOverlay, UserOverlays,
};
use tars_core::storage::db::Database;
use tars_core::storage::profiles::ProfileStore;
use tars_scanner::types::Scope;

fn create_test_profile(name: &str) -> Profile {
    Profile::new(name.to_string())
}

fn create_full_profile(name: &str) -> Profile {
    let mut profile = Profile::new(name.to_string());
    profile.description = Some("A comprehensive test profile".to_string());

    // Add plugins
    profile.plugin_set = PluginSet {
        marketplaces: vec![],
        plugins: vec![PluginRef {
            id: "test-plugin".to_string(),
            marketplace: Some("test-marketplace".to_string()),
            scope: Scope::User,
            enabled: true,
        }],
    };

    // Add repo overlays
    profile.repo_overlays = RepoOverlays {
        skills: vec![SkillOverlay {
            name: "test-skill".to_string(),
            content: "---\nname: test-skill\ndescription: Test\n---\n\nSkill content".to_string(),
        }],
        commands: vec![CommandOverlay {
            name: "test-cmd".to_string(),
            content: "Execute $ARGUMENTS".to_string(),
        }],
        agents: vec![AgentOverlay {
            name: "test-agent".to_string(),
            content: "---\nname: test-agent\ndescription: Test\n---\n\nAgent content".to_string(),
        }],
        claude_md: Some(ClaudeMdOverlay {
            mode: OverlayMode::Append,
            content: "# Additional Instructions".to_string(),
        }),
    };

    // Add user overlays
    profile.user_overlays = UserOverlays {
        skills: vec![SkillOverlay {
            name: "user-skill".to_string(),
            content: "User skill content".to_string(),
        }],
        commands: vec![],
    };

    // Add adapters
    profile.adapters = Adapters {
        mcp_location: McpLocation::ClaudeDir,
        merge_strategies: std::collections::HashMap::new(),
    };

    profile
}

#[test]
fn test_create_and_get_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let profile = create_test_profile("test-profile");
    let profile_id = profile.id;

    // Create
    store.create(&profile).expect("Failed to create profile");

    // Get by ID
    let retrieved = store
        .get(profile_id)
        .expect("Failed to get profile")
        .expect("Profile not found");

    assert_eq!(retrieved.id, profile_id);
    assert_eq!(retrieved.name, "test-profile");
}

#[test]
fn test_create_full_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let profile = create_full_profile("full-profile");
    let profile_id = profile.id;

    store.create(&profile).expect("Failed to create profile");

    let retrieved = store
        .get(profile_id)
        .expect("Failed to get profile")
        .expect("Profile not found");

    assert_eq!(retrieved.name, "full-profile");
    assert_eq!(
        retrieved.description,
        Some("A comprehensive test profile".to_string())
    );

    // Verify plugin set
    assert_eq!(retrieved.plugin_set.plugins.len(), 1);
    assert_eq!(retrieved.plugin_set.plugins[0].id, "test-plugin");

    // Verify repo overlays
    assert_eq!(retrieved.repo_overlays.skills.len(), 1);
    assert_eq!(retrieved.repo_overlays.commands.len(), 1);
    assert_eq!(retrieved.repo_overlays.agents.len(), 1);
    assert!(retrieved.repo_overlays.claude_md.is_some());

    // Verify user overlays
    assert_eq!(retrieved.user_overlays.skills.len(), 1);

    // Verify adapters
    assert_eq!(retrieved.adapters.mcp_location, McpLocation::ClaudeDir);
}

#[test]
fn test_get_by_name() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let profile = create_test_profile("named-profile");
    store.create(&profile).expect("Failed to create profile");

    // Get by name
    let retrieved = store
        .get_by_name("named-profile")
        .expect("Failed to get profile")
        .expect("Profile not found");

    assert_eq!(retrieved.name, "named-profile");
}

#[test]
fn test_get_nonexistent_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let result = store.get(uuid::Uuid::new_v4()).expect("Failed to query");
    assert!(result.is_none());
}

#[test]
fn test_get_by_name_nonexistent() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let result = store.get_by_name("nonexistent").expect("Failed to query");
    assert!(result.is_none());
}

#[test]
fn test_list_profiles() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    // Create multiple profiles
    let profile1 = create_test_profile("alpha-profile");
    let profile2 = create_test_profile("beta-profile");
    let profile3 = create_test_profile("gamma-profile");

    store.create(&profile1).expect("Failed to create profile 1");
    store.create(&profile2).expect("Failed to create profile 2");
    store.create(&profile3).expect("Failed to create profile 3");

    // List all
    let profiles = store.list().expect("Failed to list profiles");
    assert_eq!(profiles.len(), 3);

    // Should be sorted by name
    assert_eq!(profiles[0].name, "alpha-profile");
    assert_eq!(profiles[1].name, "beta-profile");
    assert_eq!(profiles[2].name, "gamma-profile");
}

#[test]
fn test_update_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let mut profile = create_test_profile("update-test");
    let profile_id = profile.id;

    store.create(&profile).expect("Failed to create profile");

    // Update
    profile.name = "updated-name".to_string();
    profile.description = Some("Updated description".to_string());
    profile.updated_at = chrono::Utc::now();

    store.update(&profile).expect("Failed to update profile");

    // Verify update
    let retrieved = store
        .get(profile_id)
        .expect("Failed to get profile")
        .expect("Profile not found");

    assert_eq!(retrieved.name, "updated-name");
    assert_eq!(
        retrieved.description,
        Some("Updated description".to_string())
    );
}

#[test]
fn test_update_profile_overlays() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let mut profile = create_test_profile("overlay-update");
    let profile_id = profile.id;

    store.create(&profile).expect("Failed to create profile");

    // Add overlays
    profile.repo_overlays.skills.push(SkillOverlay {
        name: "new-skill".to_string(),
        content: "New skill content".to_string(),
    });
    profile.updated_at = chrono::Utc::now();

    store.update(&profile).expect("Failed to update profile");

    // Verify
    let retrieved = store
        .get(profile_id)
        .expect("Failed to get profile")
        .expect("Profile not found");

    assert_eq!(retrieved.repo_overlays.skills.len(), 1);
    assert_eq!(retrieved.repo_overlays.skills[0].name, "new-skill");
}

#[test]
fn test_delete_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let profile = create_test_profile("delete-test");
    let profile_id = profile.id;

    store.create(&profile).expect("Failed to create profile");

    // Verify exists
    assert!(store.get(profile_id).expect("Failed to get").is_some());

    // Delete
    let deleted = store.delete(profile_id).expect("Failed to delete");
    assert!(deleted);

    // Verify deleted
    assert!(store.get(profile_id).expect("Failed to get").is_none());
}

#[test]
fn test_delete_nonexistent_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let deleted = store
        .delete(uuid::Uuid::new_v4())
        .expect("Failed to delete");
    assert!(!deleted);
}

#[test]
fn test_duplicate_name_fails() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let profile1 = create_test_profile("duplicate-name");
    let profile2 = create_test_profile("duplicate-name");

    store.create(&profile1).expect("Failed to create profile 1");

    // Second create with same name should fail
    let result = store.create(&profile2);
    assert!(result.is_err());
}

#[test]
fn test_profile_serialization_roundtrip() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let original = create_full_profile("roundtrip-test");
    let profile_id = original.id;

    store.create(&original).expect("Failed to create profile");

    let retrieved = store
        .get(profile_id)
        .expect("Failed to get profile")
        .expect("Profile not found");

    // Verify all fields match
    assert_eq!(retrieved.id, original.id);
    assert_eq!(retrieved.name, original.name);
    assert_eq!(retrieved.description, original.description);
    assert_eq!(
        retrieved.plugin_set.plugins.len(),
        original.plugin_set.plugins.len()
    );
    assert_eq!(
        retrieved.repo_overlays.skills.len(),
        original.repo_overlays.skills.len()
    );
    assert_eq!(
        retrieved.user_overlays.skills.len(),
        original.user_overlays.skills.len()
    );
    assert_eq!(
        retrieved.adapters.mcp_location,
        original.adapters.mcp_location
    );
}

#[test]
fn test_profile_timestamps() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let profile = create_test_profile("timestamp-test");
    let original_created = profile.created_at;
    let original_updated = profile.updated_at;
    let profile_id = profile.id;

    store.create(&profile).expect("Failed to create profile");

    let retrieved = store
        .get(profile_id)
        .expect("Failed to get profile")
        .expect("Profile not found");

    // Timestamps should be preserved
    assert_eq!(retrieved.created_at, original_created);
    assert_eq!(retrieved.updated_at, original_updated);
}

// ============================================================================
// ToolRef and ToolPermissions Serialization Tests
// ============================================================================

use std::path::PathBuf;
use tars_core::profile::{ToolPermissions, ToolRef, ToolType};

#[test]
fn test_tool_type_serialization() {
    // Test all tool types serialize correctly
    assert_eq!(serde_json::to_string(&ToolType::Mcp).unwrap(), "\"mcp\"");
    assert_eq!(
        serde_json::to_string(&ToolType::Skill).unwrap(),
        "\"skill\""
    );
    assert_eq!(
        serde_json::to_string(&ToolType::Agent).unwrap(),
        "\"agent\""
    );
    assert_eq!(serde_json::to_string(&ToolType::Hook).unwrap(), "\"hook\"");
}

#[test]
fn test_tool_type_deserialization() {
    // Test all tool types deserialize correctly
    assert_eq!(
        serde_json::from_str::<ToolType>("\"mcp\"").unwrap(),
        ToolType::Mcp
    );
    assert_eq!(
        serde_json::from_str::<ToolType>("\"skill\"").unwrap(),
        ToolType::Skill
    );
    assert_eq!(
        serde_json::from_str::<ToolType>("\"agent\"").unwrap(),
        ToolType::Agent
    );
    assert_eq!(
        serde_json::from_str::<ToolType>("\"hook\"").unwrap(),
        ToolType::Hook
    );
}

#[test]
fn test_tool_permissions_serialization() {
    let perms = ToolPermissions {
        allowed_directories: vec![PathBuf::from("/path/to/dir"), PathBuf::from("./relative")],
        allowed_tools: vec!["tool1".to_string(), "tool2".to_string()],
        disallowed_tools: vec!["dangerous_tool".to_string()],
    };

    let json = serde_json::to_string(&perms).unwrap();
    let deserialized: ToolPermissions = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.allowed_directories.len(), 2);
    assert_eq!(deserialized.allowed_tools, vec!["tool1", "tool2"]);
    assert_eq!(deserialized.disallowed_tools, vec!["dangerous_tool"]);
}

#[test]
fn test_tool_permissions_default() {
    let perms = ToolPermissions::default();

    assert!(perms.allowed_directories.is_empty());
    assert!(perms.allowed_tools.is_empty());
    assert!(perms.disallowed_tools.is_empty());
}

#[test]
fn test_tool_permissions_empty_fields_skipped() {
    // Empty fields should be skipped in serialization due to #[serde(default)]
    let perms = ToolPermissions {
        allowed_directories: vec![],
        allowed_tools: vec!["tool1".to_string()],
        disallowed_tools: vec![],
    };

    let json = serde_json::to_string(&perms).unwrap();

    // Deserialize back
    let deserialized: ToolPermissions = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.allowed_tools, vec!["tool1"]);
    assert!(deserialized.allowed_directories.is_empty());
    assert!(deserialized.disallowed_tools.is_empty());
}

#[test]
fn test_tool_ref_minimal() {
    let tool_ref = ToolRef {
        name: "test-server".to_string(),
        tool_type: ToolType::Mcp,
        source_scope: None,
        permissions: None,
    };

    let json = serde_json::to_string(&tool_ref).unwrap();
    let deserialized: ToolRef = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "test-server");
    assert_eq!(deserialized.tool_type, ToolType::Mcp);
    assert!(deserialized.source_scope.is_none());
    assert!(deserialized.permissions.is_none());
}

#[test]
fn test_tool_ref_with_permissions() {
    let tool_ref = ToolRef {
        name: "context7".to_string(),
        tool_type: ToolType::Mcp,
        source_scope: Some(Scope::User),
        permissions: Some(ToolPermissions {
            allowed_directories: vec![PathBuf::from("/home/user/projects")],
            allowed_tools: vec!["query-docs".to_string()],
            disallowed_tools: vec![],
        }),
    };

    let json = serde_json::to_string(&tool_ref).unwrap();
    let deserialized: ToolRef = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "context7");
    assert_eq!(deserialized.tool_type, ToolType::Mcp);
    assert_eq!(deserialized.source_scope, Some(Scope::User));

    let perms = deserialized.permissions.unwrap();
    assert_eq!(perms.allowed_directories.len(), 1);
    assert_eq!(perms.allowed_tools, vec!["query-docs"]);
}

#[test]
fn test_tool_ref_all_tool_types() {
    // Test that all tool types work correctly in a ToolRef
    let tool_types = [
        ToolType::Mcp,
        ToolType::Skill,
        ToolType::Agent,
        ToolType::Hook,
    ];

    for tool_type in tool_types {
        let tool_ref = ToolRef {
            name: format!("test-{tool_type}"),
            tool_type,
            source_scope: None,
            permissions: None,
        };

        let json = serde_json::to_string(&tool_ref).unwrap();
        let deserialized: ToolRef = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.tool_type, tool_type);
    }
}

#[test]
fn test_profile_with_tool_refs() {
    let db = Database::in_memory().expect("Failed to create database");
    let store = ProfileStore::new(db.connection());

    let mut profile = Profile::new("tools-test".to_string());
    profile.tool_refs = vec![
        ToolRef {
            name: "server1".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: Some(Scope::User),
            permissions: Some(ToolPermissions {
                allowed_directories: vec![PathBuf::from("/allowed")],
                allowed_tools: vec!["read".to_string()],
                disallowed_tools: vec!["write".to_string()],
            }),
        },
        ToolRef {
            name: "my-skill".to_string(),
            tool_type: ToolType::Skill,
            source_scope: None,
            permissions: None,
        },
    ];

    let profile_id = profile.id;
    store.create(&profile).expect("Failed to create profile");

    let retrieved = store
        .get(profile_id)
        .expect("Failed to get profile")
        .expect("Profile not found");

    // Verify tool refs are preserved
    assert_eq!(retrieved.tool_refs.len(), 2);

    // Check first tool ref (MCP with permissions)
    let mcp_ref = &retrieved.tool_refs[0];
    assert_eq!(mcp_ref.name, "server1");
    assert_eq!(mcp_ref.tool_type, ToolType::Mcp);
    assert_eq!(mcp_ref.source_scope, Some(Scope::User));

    let perms = mcp_ref.permissions.as_ref().expect("Expected permissions");
    assert_eq!(perms.allowed_directories.len(), 1);
    assert_eq!(perms.allowed_tools, vec!["read"]);
    assert_eq!(perms.disallowed_tools, vec!["write"]);

    // Check second tool ref (Skill without permissions)
    let skill_ref = &retrieved.tool_refs[1];
    assert_eq!(skill_ref.name, "my-skill");
    assert_eq!(skill_ref.tool_type, ToolType::Skill);
    assert!(skill_ref.permissions.is_none());
}

#[test]
fn test_tool_ref_display() {
    // Test ToolType Display implementation
    assert_eq!(format!("{}", ToolType::Mcp), "mcp");
    assert_eq!(format!("{}", ToolType::Skill), "skill");
    assert_eq!(format!("{}", ToolType::Agent), "agent");
    assert_eq!(format!("{}", ToolType::Hook), "hook");
}
