# Quickstart: Profiles Feature

**Feature**: 003-profiles
**Date**: 2026-01-10

## Prerequisites

- TARS desktop app built and running
- At least one project added to TARS
- Scanner working (can discover MCP servers, skills, agents, hooks)

## Quick Start (5 minutes)

### 1. Create a Profile

```bash
# From the TARS app:
# 1. Click "Profiles" in the sidebar
# 2. Click "Create Profile" button
# 3. Enter name: "My Workflow"
# 4. Click Create
```

### 2. Add Tools to Profile

```bash
# In the Profile detail view:
# 1. Click "Add Tool" button
# 2. Select tool type tab (MCP Servers, Skills, Agents)
# 3. Check the tools you want from the inventory
# 4. For MCP servers: click the chevron to expand permissions
#    - Add allowed_tools (tools this server can use)
#    - Add disallowed_tools (tools blocked from this server)
# 5. Click "Add Tools"
```

### 3. Assign Profile to Project

```bash
# From the Projects page:
# 1. Select a project
# 2. Click "Assign Profile" button
# 3. Select your profile from dropdown
# 4. Click Assign
```

### 4. Verify Tools

```bash
# In Project overview:
# - Profile tools show "(from profile)" badge
# - Local tools show "(local)" badge
```

---

## Development Setup

### Build the App

```bash
# From repo root
cd apps/tars-desktop
bun install
bun run tauri dev
```

### Run Tests

```bash
# Rust tests
cargo test -p tars-core

# Frontend tests
cd apps/tars-desktop
bun run test
```

### File Locations

| File | Purpose |
|------|---------|
| `crates/tars-core/src/profile/types.rs` | Profile, ToolRef, ToolPermissions types |
| `crates/tars-core/src/profile/export.rs` | Export/import to .tars-profile.json |
| `crates/tars-core/src/project.rs` | Project, LocalOverrides types |
| `crates/tars-core/src/storage/profiles.rs` | Profile database operations |
| `crates/tars-core/src/storage/projects.rs` | Project database operations |
| `apps/tars-desktop/src/pages/ProfilesPage.tsx` | Profiles list & management UI |
| `apps/tars-desktop/src/pages/ProjectsPage.tsx` | Projects list & assignment UI |
| `apps/tars-desktop/src/components/ProfileDetail.tsx` | Profile detail view |
| `apps/tars-desktop/src/components/ProfileToolPicker.tsx` | Tool selection dialog with permissions |
| `apps/tars-desktop/src/components/ToolPermissionsEditor.tsx` | Permissions editor component |
| `apps/tars-desktop/src/components/ProjectOverview.tsx` | Project tools display |

### Test Files

| File | Tests |
|------|-------|
| `crates/tars-core/tests/profile_test.rs` | Profile CRUD, ToolRef serialization |
| `crates/tars-core/tests/profile_export_test.rs` | Export/import round-trip |
| `crates/tars-core/tests/profile_sync_test.rs` | Profile-project sync, LocalOverrides |

---

## Common Operations

### Export a Profile

```typescript
// Frontend
import { exportProfileJson } from '../lib/ipc';

// Export to .tars-profile.json file
const result = await exportProfileJson(profileId, '/path/to/output.tars-profile.json');
console.log(`Exported ${result.tool_count} tools`);
```

### Import a Profile

```typescript
// Frontend
import { previewProfileImport, importProfileJson } from '../lib/ipc';

// Preview first (check for collisions)
const preview = await previewProfileImport('/path/to/profile.tars-profile.json');
console.log(`Profile: ${preview.name}, Tools: ${preview.tool_count}`);

if (preview.name_exists) {
  // Ask user for new name to avoid collision
  const result = await importProfileJson(path, 'New Name');
} else {
  const result = await importProfileJson(path);
}
```

### Sync Profile Changes

Profile sync happens automatically when you save a profile. To manually trigger:

```rust
// Backend (Rust)
use tars_core::profile::sync::sync_profile_to_projects;

let result = sync_profile_to_projects(&conn, profile_id)?;
println!("Synced to {} projects", result.affected_count);
```

---

## Testing Checklist

- [x] Create profile with name and description
- [x] Add MCP server to profile with permissions
- [x] Add skill to profile
- [x] Assign profile to project
- [x] Verify project shows profile tools with "From Profile" badge
- [x] Update profile (add tool)
- [x] Verify project reflects update (auto-sync)
- [x] Add local tool to project
- [x] Update profile again
- [x] Verify local tool persists (not overwritten)
- [x] Unassign profile from project
- [x] Delete profile with assigned projects
- [x] Verify tools converted to local overrides
- [x] Export profile to .tars-profile.json file
- [x] Import profile from file
- [x] Handle import name collision with rename

---

## Key Data Types

### ToolRef (profile tool reference)
```typescript
interface ToolRef {
  name: string;           // Tool identifier
  tool_type: 'mcp' | 'skill' | 'agent' | 'hook';
  source_scope?: 'user' | 'project' | 'managed';
  permissions?: ToolPermissions;
}
```

### ToolPermissions (MCP restrictions)
```typescript
interface ToolPermissions {
  allowed_directories: string[];  // Paths tool can access
  allowed_tools: string[];        // Tools this server can use
  disallowed_tools: string[];     // Tools blocked from this server
}
```

### LocalOverrides (project-specific tools)
```typescript
interface LocalOverrides {
  mcp_servers: ToolRef[];
  skills: ToolRef[];
  agents: ToolRef[];
  hooks: ToolRef[];
}
```

---

## Troubleshooting

### Profile Not Syncing

1. Check if profile was saved (updated_at changed)
2. Verify project has `assigned_profile_id` set
3. Check console for sync errors

### Import Fails

1. Verify file is valid JSON
2. Check `version` field matches supported versions
3. Ensure file has required fields (name, tools)

### Tools Not Showing

1. Run scanner to refresh inventory
2. Check if tool exists in discovered inventory
3. Verify tool_type matches expected category
