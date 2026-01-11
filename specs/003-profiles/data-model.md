# Data Model: Profiles Feature

**Feature**: 003-profiles
**Date**: 2026-01-10

## Overview

This document defines the data entities for the Profiles feature. The design extends existing `Profile` and `Project` types in `tars-core` rather than creating new tables.

---

## Entity Definitions

### Profile (Extended)

Extends the existing `Profile` struct in `crates/tars-core/src/profile/types.rs`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | Uuid | Yes | Unique identifier (existing) |
| name | String | Yes | Profile name (existing) |
| description | Option<String> | No | Optional description (existing) |
| tool_refs | Vec<ToolRef> | Yes | References to tools in this profile (NEW) |
| plugin_set | PluginSet | Yes | Plugin configuration (existing) |
| repo_overlays | RepoOverlays | Yes | Repository overlays (existing) |
| user_overlays | UserOverlays | Yes | User overlays (existing) |
| adapters | Adapters | Yes | Adapter settings (existing) |
| created_at | DateTime<Utc> | Yes | Creation timestamp (existing) |
| updated_at | DateTime<Utc> | Yes | Last update timestamp (existing) |

**Validation Rules**:
- `name` must be non-empty and unique
- `tool_refs` can be empty (profile with no tools)
- `updated_at` must be >= `created_at`

---

### ToolRef (New)

A reference to a tool (MCP server, skill, agent, or hook) with optional permissions.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | String | Yes | Tool identifier/name |
| tool_type | ToolType | Yes | Type of tool (MCP, Skill, Agent, Hook) |
| source_scope | Option<Scope> | No | Where the tool was discovered |
| permissions | Option<ToolPermissions> | No | Permission restrictions |

**Validation Rules**:
- `name` must be non-empty
- `tool_type` must be valid enum value

---

### ToolType (New Enum)

```rust
pub enum ToolType {
    Mcp,
    Skill,
    Agent,
    Hook,
}
```

---

### ToolPermissions (New)

Permission restrictions that can be applied to a tool in a profile.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| allowed_directories | Vec<PathBuf> | No | Directories the tool can access |
| allowed_tools | Vec<String> | No | Tools this agent/skill can use |
| disallowed_tools | Vec<String> | No | Tools this agent/skill cannot use |

**Validation Rules**:
- Paths should be relative (resolved against project root at apply time)
- `allowed_tools` and `disallowed_tools` should not overlap

---

### Project (Extended)

Extends the existing `Project` struct in `crates/tars-core/src/project.rs`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | Uuid | Yes | Unique identifier (existing) |
| path | PathBuf | Yes | Project path (existing) |
| name | String | Yes | Project name (existing) |
| git_info | Option<GitInfo> | No | Git repository info (existing) |
| last_scanned | Option<DateTime<Utc>> | No | Last scan timestamp (existing) |
| assigned_profile_id | Option<Uuid> | No | Linked profile ID (existing) |
| local_overrides | LocalOverrides | Yes | Project-specific tools (NEW) |
| created_at | DateTime<Utc> | Yes | Creation timestamp (existing) |
| updated_at | DateTime<Utc> | Yes | Last update timestamp (existing) |

**Validation Rules**:
- `path` must be a valid directory
- `assigned_profile_id` if set, must reference an existing profile

---

### LocalOverrides (New)

Project-specific tool additions that persist through profile sync.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| mcp_servers | Vec<ToolRef> | Yes | Local MCP server references |
| skills | Vec<ToolRef> | Yes | Local skill references |
| agents | Vec<ToolRef> | Yes | Local agent references |
| hooks | Vec<ToolRef> | Yes | Local hook references |

**Validation Rules**:
- All vectors can be empty
- Tool refs should have matching `tool_type` for their category

---

### ProfileExport (New)

Portable format for `.tars-profile.json` export/import.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| version | u32 | Yes | Schema version (currently 1) |
| name | String | Yes | Profile name |
| description | Option<String> | No | Profile description |
| tools | Vec<ExportedTool> | Yes | Tool configurations |
| exported_at | DateTime<Utc> | Yes | Export timestamp |
| exported_by | String | Yes | Exporting app version |

---

### ExportedTool (New)

Tool definition in export format.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | String | Yes | Tool identifier |
| tool_type | String | Yes | "mcp", "skill", "agent", or "hook" |
| permissions | Option<ToolPermissions> | No | Permission restrictions |

---

## Entity Relationships

```
┌─────────────┐         ┌─────────────┐
│   Profile   │◄────────│   Project   │
│             │  0..1   │             │
│ - id        │         │ - id        │
│ - name      │         │ - name      │
│ - tool_refs │         │ - assigned_ │
│             │         │   profile_id│
│             │         │ - local_    │
│             │         │   overrides │
└─────────────┘         └─────────────┘
       │
       │ 0..*
       ▼
┌─────────────┐
│   ToolRef   │
│             │
│ - name      │
│ - tool_type │
│ - permissions│
└─────────────┘
```

**Relationships**:
- Profile → ToolRef: One profile contains zero or more tool references
- Project → Profile: One project can have zero or one assigned profile
- Project → LocalOverrides: One project has exactly one local overrides container

---

## State Transitions

### Profile States

```
[Draft] ──create──▶ [Active] ──delete──▶ [Deleted]
                        │
                        │ update
                        ▼
                    [Active]
```

### Project-Profile Assignment States

```
[Unassigned] ──assign──▶ [Assigned] ──unassign──▶ [Unassigned]
                              │
                              │ profile deleted
                              ▼
                         [Unassigned]
                         (tools → local overrides)
```

---

## Database Schema Changes

### Existing Tables (No Changes Needed)

The `profiles` and `projects` tables store JSON in `data` column. Schema extensions are handled by adding fields to the Rust structs with `#[serde(default)]` for backward compatibility.

```sql
-- Existing schema (for reference)
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    data TEXT NOT NULL,  -- Full Profile JSON
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    data TEXT NOT NULL,  -- Full Project JSON
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### Migration Strategy

No SQL migrations needed. Struct changes use `#[serde(default)]`:

```rust
// Profile struct
#[serde(default)]
pub tool_refs: Vec<ToolRef>,

// Project struct
#[serde(default)]
pub local_overrides: LocalOverrides,
```

Existing data loads with empty `tool_refs`/`local_overrides`. New data includes the fields.

---

## Indexes and Query Patterns

### Common Queries

1. **List profiles**: `SELECT * FROM profiles ORDER BY name`
2. **Get profile by ID**: `SELECT data FROM profiles WHERE id = ?`
3. **List projects by profile**: Filter in Rust after loading (JSON column)
4. **Get projects with profile**: `SELECT * FROM projects` + filter `assigned_profile_id` in Rust

### Performance Notes

- Profile sync queries all projects then filters by `assigned_profile_id` in Rust
- For MVP scale (dozens of profiles, hundreds of projects), this is acceptable
- Future optimization: add `assigned_profile_id` column to `projects` table for SQL-level filtering
