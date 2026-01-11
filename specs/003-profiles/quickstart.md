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
# 2. Select tool type (MCP, Skill, Agent, Hook)
# 3. Pick from discovered tools inventory
# 4. Optionally configure permissions
# 5. Click Add
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
| `crates/tars-core/src/profile/types.rs` | Profile data types |
| `crates/tars-core/src/profile/sync.rs` | Profile sync logic |
| `crates/tars-core/src/profile/export.rs` | Export/import |
| `apps/tars-desktop/src/pages/ProfilesPage.tsx` | Profiles UI |
| `apps/tars-desktop/src/components/ProfileDetail.tsx` | Profile detail view |

---

## Common Operations

### Export a Profile

```typescript
// Frontend
import { exportProfile } from '../lib/ipc';

const result = await exportProfile(profileId, '/path/to/output.tars-profile.json');
console.log(`Exported to ${result.path}`);
```

### Import a Profile

```typescript
// Frontend
import { previewImport, importProfile } from '../lib/ipc';

// Preview first (check for collisions)
const preview = await previewImport('/path/to/profile.tars-profile.json');

if (preview.has_name_collision) {
  // Ask user for new name
  const result = await importProfile(path, 'New Name');
} else {
  const result = await importProfile(path);
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

- [ ] Create profile with name and description
- [ ] Add MCP server to profile
- [ ] Add skill to profile
- [ ] Assign profile to project
- [ ] Verify project shows profile tools with badge
- [ ] Update profile (add tool)
- [ ] Verify project reflects update
- [ ] Add local tool to project
- [ ] Update profile again
- [ ] Verify local tool persists
- [ ] Unassign profile from project
- [ ] Delete profile with assigned projects
- [ ] Verify tools converted to local overrides
- [ ] Export profile to file
- [ ] Import profile from file
- [ ] Handle import name collision

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
