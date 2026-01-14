//! Profile sync integration tests
//!
//! Tests for profile-project assignment and synchronization.

use std::path::PathBuf;
use tars_core::profile::{Profile, ToolPermissions, ToolRef, ToolType};
use tars_core::project::{LocalOverrides, Project};
use tars_core::storage::db::Database;
use tars_core::storage::profiles::ProfileStore;
use tars_core::storage::projects::ProjectStore;

fn create_test_profile(name: &str) -> Profile {
    let mut profile = Profile::new(name.to_string());
    profile.tool_refs = vec![ToolRef {
        name: "profile-mcp".to_string(),
        tool_type: ToolType::Mcp,
        source_scope: None,
        permissions: Some(ToolPermissions {
            allowed_directories: vec![],
            allowed_tools: vec!["read".to_string()],
            disallowed_tools: vec![],
        }),
        source_ref: None,
    }];
    profile
}

fn create_test_project(name: &str) -> Project {
    Project::new(PathBuf::from(format!("/test/projects/{name}"))).with_name(name.to_string())
}

#[test]
fn test_assign_profile_to_project() {
    let db = Database::in_memory().expect("Failed to create database");
    let profile_store = ProfileStore::new(db.connection());
    let project_store = ProjectStore::new(db.connection());

    // Create profile and project
    let profile = create_test_profile("test-profile");
    let profile_id = profile.id;
    profile_store
        .create(&profile)
        .expect("Failed to create profile");

    let mut project = create_test_project("my-project");
    project_store
        .create(&project)
        .expect("Failed to create project");

    // Assign profile to project
    project.assigned_profile_id = Some(profile_id);
    project.updated_at = chrono::Utc::now();
    project_store
        .update(&project)
        .expect("Failed to update project");

    // Verify assignment
    let retrieved = project_store
        .get(project.id)
        .expect("Failed to get project")
        .expect("Project not found");

    assert_eq!(retrieved.assigned_profile_id, Some(profile_id));
}

#[test]
fn test_unassign_profile_from_project() {
    let db = Database::in_memory().expect("Failed to create database");
    let profile_store = ProfileStore::new(db.connection());
    let project_store = ProjectStore::new(db.connection());

    // Create profile and project
    let profile = create_test_profile("test-profile");
    let profile_id = profile.id;
    profile_store
        .create(&profile)
        .expect("Failed to create profile");

    let mut project = create_test_project("my-project");
    project.assigned_profile_id = Some(profile_id);
    project_store
        .create(&project)
        .expect("Failed to create project");

    // Unassign profile
    project.assigned_profile_id = None;
    project.updated_at = chrono::Utc::now();
    project_store
        .update(&project)
        .expect("Failed to update project");

    // Verify unassignment
    let retrieved = project_store
        .get(project.id)
        .expect("Failed to get project")
        .expect("Project not found");

    assert!(retrieved.assigned_profile_id.is_none());
}

#[test]
fn test_list_projects_by_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let profile_store = ProfileStore::new(db.connection());
    let project_store = ProjectStore::new(db.connection());

    // Create profile
    let profile = create_test_profile("shared-profile");
    let profile_id = profile.id;
    profile_store
        .create(&profile)
        .expect("Failed to create profile");

    // Create projects, some assigned to profile
    let mut project1 = create_test_project("project1");
    project1.assigned_profile_id = Some(profile_id);
    project_store
        .create(&project1)
        .expect("Failed to create project1");

    let mut project2 = create_test_project("project2");
    project2.assigned_profile_id = Some(profile_id);
    project_store
        .create(&project2)
        .expect("Failed to create project2");

    let project3 = create_test_project("project3"); // Not assigned
    project_store
        .create(&project3)
        .expect("Failed to create project3");

    // List projects by profile
    let assigned = project_store
        .list_by_profile(profile_id)
        .expect("Failed to list projects");

    assert_eq!(assigned.len(), 2);
    assert!(assigned.iter().any(|p| p.name == "project1"));
    assert!(assigned.iter().any(|p| p.name == "project2"));
    assert!(!assigned.iter().any(|p| p.name == "project3"));
}

#[test]
fn test_count_projects_by_profile() {
    let db = Database::in_memory().expect("Failed to create database");
    let profile_store = ProfileStore::new(db.connection());
    let project_store = ProjectStore::new(db.connection());

    let profile = create_test_profile("counted-profile");
    let profile_id = profile.id;
    profile_store
        .create(&profile)
        .expect("Failed to create profile");

    // Initially no projects
    let count = project_store
        .count_by_profile(profile_id)
        .expect("Failed to count");
    assert_eq!(count, 0);

    // Add 3 projects
    for i in 1..=3 {
        let mut project = create_test_project(&format!("project{i}"));
        project.assigned_profile_id = Some(profile_id);
        project_store
            .create(&project)
            .expect("Failed to create project");
    }

    let count = project_store
        .count_by_profile(profile_id)
        .expect("Failed to count");
    assert_eq!(count, 3);
}

#[test]
fn test_local_overrides_preserved() {
    let db = Database::in_memory().expect("Failed to create database");
    let project_store = ProjectStore::new(db.connection());

    let mut project = create_test_project("local-test");
    project.local_overrides = LocalOverrides {
        mcp_servers: vec![ToolRef {
            name: "local-server".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: None,
            permissions: Some(ToolPermissions {
                allowed_directories: vec![PathBuf::from("/local/path")],
                allowed_tools: vec!["local-tool".to_string()],
                disallowed_tools: vec![],
            }),
            source_ref: None,
        }],
        skills: vec![],
        agents: vec![],
        hooks: vec![],
    };

    project_store
        .create(&project)
        .expect("Failed to create project");

    let retrieved = project_store
        .get(project.id)
        .expect("Failed to get project")
        .expect("Project not found");

    // Verify local overrides are preserved
    assert_eq!(retrieved.local_overrides.mcp_servers.len(), 1);
    assert_eq!(
        retrieved.local_overrides.mcp_servers[0].name,
        "local-server"
    );

    let perms = retrieved.local_overrides.mcp_servers[0]
        .permissions
        .as_ref()
        .expect("Expected permissions");
    assert_eq!(perms.allowed_directories.len(), 1);
    assert_eq!(perms.allowed_tools, vec!["local-tool"]);
}

#[test]
fn test_local_overrides_methods() {
    let overrides = LocalOverrides::default();
    assert!(overrides.is_empty());
    assert_eq!(overrides.total_count(), 0);

    let overrides_with_tools = LocalOverrides {
        mcp_servers: vec![
            ToolRef {
                name: "server1".to_string(),
                tool_type: ToolType::Mcp,
                source_scope: None,
                permissions: None,
                source_ref: None,
            },
            ToolRef {
                name: "server2".to_string(),
                tool_type: ToolType::Mcp,
                source_scope: None,
                permissions: None,
                source_ref: None,
            },
        ],
        skills: vec![ToolRef {
            name: "skill1".to_string(),
            tool_type: ToolType::Skill,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
        agents: vec![],
        hooks: vec![],
    };

    assert!(!overrides_with_tools.is_empty());
    assert_eq!(overrides_with_tools.total_count(), 3);
}

#[test]
fn test_profile_update_does_not_clear_local_overrides() {
    let db = Database::in_memory().expect("Failed to create database");
    let profile_store = ProfileStore::new(db.connection());
    let project_store = ProjectStore::new(db.connection());

    // Create profile
    let mut profile = create_test_profile("update-profile");
    let profile_id = profile.id;
    profile_store
        .create(&profile)
        .expect("Failed to create profile");

    // Create project with profile and local overrides
    let mut project = create_test_project("override-project");
    project.assigned_profile_id = Some(profile_id);
    project.local_overrides = LocalOverrides {
        mcp_servers: vec![ToolRef {
            name: "local-mcp".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
        skills: vec![],
        agents: vec![],
        hooks: vec![],
    };
    project_store
        .create(&project)
        .expect("Failed to create project");

    // Update profile (simulate profile change)
    profile.tool_refs.push(ToolRef {
        name: "new-tool".to_string(),
        tool_type: ToolType::Skill,
        source_scope: None,
        permissions: None,
        source_ref: None,
    });
    profile.updated_at = chrono::Utc::now();
    profile_store
        .update(&profile)
        .expect("Failed to update profile");

    // Verify local overrides still exist on project
    let retrieved_project = project_store
        .get(project.id)
        .expect("Failed to get project")
        .expect("Project not found");

    // Profile assignment should still be there
    assert_eq!(retrieved_project.assigned_profile_id, Some(profile_id));

    // Local overrides should be preserved
    assert!(!retrieved_project.local_overrides.is_empty());
    assert_eq!(retrieved_project.local_overrides.mcp_servers.len(), 1);
    assert_eq!(
        retrieved_project.local_overrides.mcp_servers[0].name,
        "local-mcp"
    );
}

#[test]
fn test_delete_profile_with_assigned_project() {
    let db = Database::in_memory().expect("Failed to create database");
    let profile_store = ProfileStore::new(db.connection());
    let project_store = ProjectStore::new(db.connection());

    // Create profile
    let profile = create_test_profile("delete-profile");
    let profile_id = profile.id;
    profile_store
        .create(&profile)
        .expect("Failed to create profile");

    // Create project assigned to profile
    let mut project = create_test_project("assigned-project");
    project.assigned_profile_id = Some(profile_id);
    project_store
        .create(&project)
        .expect("Failed to create project");

    // Get projects before deletion
    let count_before = project_store
        .count_by_profile(profile_id)
        .expect("Failed to count");
    assert_eq!(count_before, 1);

    // Delete profile
    profile_store
        .delete(profile_id)
        .expect("Failed to delete profile");

    // Profile should be gone
    let profile_gone = profile_store
        .get(profile_id)
        .expect("Failed to get profile");
    assert!(profile_gone.is_none());

    // Project still exists but has orphaned reference
    // (This is expected - cleanup happens at application layer)
    let project = project_store
        .get(project.id)
        .expect("Failed to get project")
        .expect("Project should still exist");

    // The assigned_profile_id is still set (orphaned reference)
    // Application layer is responsible for converting to local overrides
    assert_eq!(project.assigned_profile_id, Some(profile_id));
}

#[test]
fn test_local_overrides_serialization_roundtrip() {
    let overrides = LocalOverrides {
        mcp_servers: vec![ToolRef {
            name: "test-mcp".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: None,
            permissions: Some(ToolPermissions {
                allowed_directories: vec![PathBuf::from("/path/one"), PathBuf::from("/path/two")],
                allowed_tools: vec!["tool1".to_string()],
                disallowed_tools: vec!["bad-tool".to_string()],
            }),
            source_ref: None,
        }],
        skills: vec![ToolRef {
            name: "test-skill".to_string(),
            tool_type: ToolType::Skill,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
        agents: vec![],
        hooks: vec![ToolRef {
            name: "test-hook".to_string(),
            tool_type: ToolType::Hook,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
    };

    let json = serde_json::to_string(&overrides).expect("Failed to serialize");
    let deserialized: LocalOverrides = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.mcp_servers.len(), 1);
    assert_eq!(deserialized.skills.len(), 1);
    assert_eq!(deserialized.agents.len(), 0);
    assert_eq!(deserialized.hooks.len(), 1);

    let perms = deserialized.mcp_servers[0]
        .permissions
        .as_ref()
        .expect("Expected permissions");
    assert_eq!(perms.allowed_directories.len(), 2);
    assert_eq!(perms.allowed_tools, vec!["tool1"]);
    assert_eq!(perms.disallowed_tools, vec!["bad-tool"]);
}

#[test]
fn test_project_with_all_override_types() {
    let db = Database::in_memory().expect("Failed to create database");
    let project_store = ProjectStore::new(db.connection());

    let mut project = create_test_project("all-overrides");
    project.local_overrides = LocalOverrides {
        mcp_servers: vec![ToolRef {
            name: "mcp-override".to_string(),
            tool_type: ToolType::Mcp,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
        skills: vec![ToolRef {
            name: "skill-override".to_string(),
            tool_type: ToolType::Skill,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
        agents: vec![ToolRef {
            name: "agent-override".to_string(),
            tool_type: ToolType::Agent,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
        hooks: vec![ToolRef {
            name: "hook-override".to_string(),
            tool_type: ToolType::Hook,
            source_scope: None,
            permissions: None,
            source_ref: None,
        }],
    };

    project_store
        .create(&project)
        .expect("Failed to create project");

    let retrieved = project_store
        .get(project.id)
        .expect("Failed to get project")
        .expect("Project not found");

    assert_eq!(retrieved.local_overrides.total_count(), 4);
    assert!(!retrieved.local_overrides.is_empty());
}
