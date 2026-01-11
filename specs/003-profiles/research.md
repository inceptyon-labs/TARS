# Research: Profiles Feature

**Feature**: 003-profiles
**Date**: 2026-01-10

## Overview

This document captures research findings and design decisions for the Profiles feature. The existing codebase already has foundational Profile and Project types - this feature extends them with linking, sync, and export capabilities.

---

## Research Topics

### 1. Profile-Project Linking Strategy

**Decision**: Store `assigned_profile_id` on Project (already exists), add `local_overrides` field

**Rationale**:
- The `Project` struct already has `assigned_profile_id: Option<Uuid>` field
- Adding a `local_overrides` field keeps profile tools and local tools cleanly separated
- On sync, profile tools are resolved from the Profile, local overrides are preserved

**Alternatives Considered**:
- Store link in separate `profile_assignments` table → Rejected: adds complexity, Project already has the field
- Store resolved tools on Project → Rejected: violates "linked not copied" requirement

---

### 2. Profile Sync Mechanism

**Decision**: On-demand sync triggered by profile save, not background polling

**Rationale**:
- Simpler implementation - no background task management
- User already expects save to complete the operation
- Constitution requires notifications on sync - easier to show at save time
- Performance: sync on save affects only changed profile's projects

**Implementation**:
1. When profile is updated/saved, query all projects with `assigned_profile_id = profile.id`
2. For each project, update the effective configuration (profile tools + local overrides)
3. Show notification: "Profile 'X' updated - N projects affected"
4. No file system writes unless user applies to project (Safe-by-Default)

**Alternatives Considered**:
- Background polling → Rejected: unnecessary complexity for desktop app
- File watcher on `~/.tars/profiles/` → Rejected: profiles stored in SQLite, not files

---

### 3. Local Overrides Storage

**Decision**: Store `local_overrides: LocalOverrides` on Project struct

**Rationale**:
- Keep profile tools and local additions clearly separated
- On sync, only profile portion changes, local overrides untouched
- UI can show "(from profile)" vs "(local)" badges easily

**Data Structure**:
```rust
pub struct LocalOverrides {
    pub mcp_servers: Vec<McpServerRef>,
    pub skills: Vec<SkillRef>,
    pub agents: Vec<AgentRef>,
    pub hooks: Vec<HookRef>,
}
```

**Alternatives Considered**:
- Merge all tools into single list with `source` field → Rejected: harder to preserve on sync
- Store in separate table → Rejected: adds query complexity, Project already serialized as JSON

---

### 4. Tool Reference Format

**Decision**: Use references (name/id + optional permissions), not full content

**Rationale**:
- Profile references tools from the discovered inventory
- Avoids duplicating tool definitions in profiles
- Permissions are profile-specific overrides

**Data Structure**:
```rust
pub struct ToolRef {
    pub name: String,           // Tool identifier
    pub tool_type: ToolType,    // MCP, Skill, Agent, Hook
    pub permissions: Option<ToolPermissions>,
}

pub struct ToolPermissions {
    pub allowed_directories: Vec<PathBuf>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
}
```

**Alternatives Considered**:
- Store full tool content in profile → Rejected: duplicates data, harder to sync
- Use inline permissions per tool type → Rejected: different types have different permission models

---

### 5. Export/Import Format

**Decision**: Use `.tars-profile.json` with version field for forward compatibility

**Rationale**:
- JSON is human-readable and easy to edit
- Version field allows schema evolution
- Consistent with existing Claude Code JSON formats

**Format**:
```json
{
  "version": 1,
  "name": "Rust Development",
  "description": "Tools for Rust projects",
  "tools": [
    { "name": "rust-analyzer", "type": "mcp", "permissions": {...} },
    { "name": "cargo-test", "type": "skill" }
  ],
  "exported_at": "2026-01-10T12:00:00Z",
  "exported_by": "tars-desktop/0.1.8"
}
```

**Alternatives Considered**:
- YAML format → Rejected: JSON more common for Claude Code ecosystem
- Binary format → Rejected: not human-readable, harder to debug

---

### 6. Profile Deletion Behavior

**Decision**: Convert profile tools to local overrides on deletion

**Rationale**:
- Prevents projects from losing their tool configuration
- Users explicitly chose those tools - don't remove silently
- Follows Safe-by-Default principle

**Implementation**:
1. Find all projects with `assigned_profile_id = deleted_profile.id`
2. For each project:
   - Move profile tool references to `local_overrides`
   - Set `assigned_profile_id = None`
3. Delete the profile
4. Show notification: "Profile deleted - N projects converted to local config"

**Alternatives Considered**:
- Remove tools from projects → Rejected: violates Safe-by-Default
- Block deletion if projects assigned → Rejected: too restrictive

---

### 7. Existing Profile Type Extension

**Decision**: Extend existing `Profile` struct, don't create new type

**Rationale**:
- `Profile` struct in `crates/tars-core/src/profile/types.rs` already exists
- Has `PluginSet`, `RepoOverlays`, `UserOverlays` - similar to our tool refs
- Add new fields for tool references alongside existing structure

**Changes Needed**:
```rust
// Add to existing Profile struct
pub struct Profile {
    // ... existing fields ...

    /// Tool references (NEW)
    #[serde(default)]
    pub tool_refs: Vec<ToolRef>,
}
```

**Alternatives Considered**:
- Create `ProfileTemplate` separate type → Rejected: duplicates existing Profile infrastructure
- Repurpose overlays as tool refs → Rejected: overlays are content, not references

---

### 8. Notification System

**Decision**: Use existing Sonner toast library for in-app notifications

**Rationale**:
- `sonner` already in package.json dependencies
- Consistent with existing app patterns
- No system notification permissions needed

**Implementation**:
- Import `toast` from `sonner`
- Show success/info toasts on profile sync
- Show error toasts on failures

---

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| sonner | ^2.0.7 | Toast notifications (existing) |
| rusqlite | existing | SQLite storage (existing) |
| serde | existing | JSON serialization (existing) |
| uuid | existing | Profile/project IDs (existing) |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Sync performance with many projects | Low | Medium | Query only affected projects by profile_id |
| Tool reference breaking (tool deleted) | Medium | Low | Show warning badge, don't crash |
| Import collisions | Medium | Low | Prompt user for rename/replace/cancel |
| SQLite migration needed | Low | Low | Add columns with defaults, no breaking changes |

---

## Open Questions (Resolved)

1. ~~How to handle profile sync?~~ → On-save, not background
2. ~~Where to store local overrides?~~ → On Project struct
3. ~~Export format?~~ → JSON with version field
4. ~~Deletion behavior?~~ → Convert to local overrides
