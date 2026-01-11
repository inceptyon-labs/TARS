# Tauri IPC Contract: Profiles Feature

**Feature**: 003-profiles
**Date**: 2026-01-10

## Overview

This document defines the Tauri IPC commands for the Profiles feature. Commands are invoked from the React frontend via `@tauri-apps/api/core` and handled by Rust command handlers.

---

## Command Definitions

### Profile CRUD

#### `create_profile`

Create a new empty profile.

**Request**:
```typescript
interface CreateProfileRequest {
  name: string;
  description?: string;
}
```

**Response**:
```typescript
interface ProfileSummary {
  id: string;           // UUID
  name: string;
  description: string | null;
  tool_count: number;
  project_count: number; // Number of projects using this profile
  created_at: string;   // ISO 8601
  updated_at: string;   // ISO 8601
}
```

**Errors**:
- `ProfileNameExists`: Profile with this name already exists
- `InvalidName`: Name is empty or invalid
- `DatabaseError`: Storage operation failed

---

#### `get_profile`

Get full profile details by ID.

**Request**:
```typescript
interface GetProfileRequest {
  id: string; // UUID
}
```

**Response**:
```typescript
interface ProfileDetails {
  id: string;
  name: string;
  description: string | null;
  tool_refs: ToolRef[];
  assigned_projects: ProjectRef[];
  created_at: string;
  updated_at: string;
}

interface ToolRef {
  name: string;
  tool_type: "mcp" | "skill" | "agent" | "hook";
  source_scope: "user" | "project" | "managed" | null;
  permissions: ToolPermissions | null;
}

interface ToolPermissions {
  allowed_directories: string[];
  allowed_tools: string[];
  disallowed_tools: string[];
}

interface ProjectRef {
  id: string;
  name: string;
  path: string;
}
```

**Errors**:
- `ProfileNotFound`: Profile with this ID does not exist

---

#### `list_profiles`

List all profiles with summary info.

**Request**: None

**Response**:
```typescript
ProfileSummary[]
```

---

#### `update_profile`

Update profile name, description, or tools.

**Request**:
```typescript
interface UpdateProfileRequest {
  id: string;
  name?: string;
  description?: string | null;
  tool_refs?: ToolRef[];
}
```

**Response**:
```typescript
interface UpdateProfileResponse {
  profile: ProfileSummary;
  sync_result: SyncResult;
}

interface SyncResult {
  affected_projects: number;
  synced_at: string;
}
```

**Errors**:
- `ProfileNotFound`: Profile with this ID does not exist
- `ProfileNameExists`: New name conflicts with existing profile
- `InvalidName`: Name is empty or invalid

---

#### `delete_profile`

Delete a profile and convert assigned project tools to local overrides.

**Request**:
```typescript
interface DeleteProfileRequest {
  id: string;
}
```

**Response**:
```typescript
interface DeleteProfileResponse {
  deleted: boolean;
  converted_projects: number; // Projects that had tools moved to local overrides
}
```

**Errors**:
- `ProfileNotFound`: Profile with this ID does not exist

---

### Profile Assignment

#### `assign_profile`

Assign a profile to a project.

**Request**:
```typescript
interface AssignProfileRequest {
  project_id: string;
  profile_id: string;
}
```

**Response**:
```typescript
interface AssignProfileResponse {
  project_id: string;
  profile_id: string;
  assigned_at: string;
}
```

**Errors**:
- `ProjectNotFound`: Project with this ID does not exist
- `ProfileNotFound`: Profile with this ID does not exist
- `ProjectAlreadyAssigned`: Project already has a profile (must unassign first)

---

#### `unassign_profile`

Remove profile assignment from a project.

**Request**:
```typescript
interface UnassignProfileRequest {
  project_id: string;
}
```

**Response**:
```typescript
interface UnassignProfileResponse {
  project_id: string;
  unassigned_at: string;
}
```

**Errors**:
- `ProjectNotFound`: Project with this ID does not exist
- `NoProfileAssigned`: Project does not have an assigned profile

---

### Local Overrides

#### `add_local_tool`

Add a tool as a local override to a project.

**Request**:
```typescript
interface AddLocalToolRequest {
  project_id: string;
  tool_ref: ToolRef;
}
```

**Response**:
```typescript
interface AddLocalToolResponse {
  project_id: string;
  tool_name: string;
  added_at: string;
}
```

**Errors**:
- `ProjectNotFound`: Project does not exist
- `ToolAlreadyExists`: Tool with this name already in local overrides

---

#### `remove_local_tool`

Remove a tool from project's local overrides.

**Request**:
```typescript
interface RemoveLocalToolRequest {
  project_id: string;
  tool_name: string;
  tool_type: "mcp" | "skill" | "agent" | "hook";
}
```

**Response**:
```typescript
interface RemoveLocalToolResponse {
  project_id: string;
  removed: boolean;
}
```

**Errors**:
- `ProjectNotFound`: Project does not exist
- `ToolNotFound`: Tool not in local overrides

---

#### `get_project_tools`

Get combined tools for a project (profile + local overrides).

**Request**:
```typescript
interface GetProjectToolsRequest {
  project_id: string;
}
```

**Response**:
```typescript
interface ProjectToolsResponse {
  project_id: string;
  profile: ProfileRef | null;
  profile_tools: ToolRefWithSource[];
  local_tools: ToolRefWithSource[];
}

interface ProfileRef {
  id: string;
  name: string;
}

interface ToolRefWithSource {
  name: string;
  tool_type: "mcp" | "skill" | "agent" | "hook";
  source: "profile" | "local";
  permissions: ToolPermissions | null;
}
```

**Errors**:
- `ProjectNotFound`: Project does not exist

---

### Export/Import

#### `export_profile`

Export a profile to `.tars-profile.json` file.

**Request**:
```typescript
interface ExportProfileRequest {
  profile_id: string;
  output_path: string; // Where to save the file
}
```

**Response**:
```typescript
interface ExportProfileResponse {
  path: string;
  size_bytes: number;
  exported_at: string;
}
```

**Errors**:
- `ProfileNotFound`: Profile does not exist
- `IoError`: Cannot write to output path

---

#### `import_profile`

Import a profile from `.tars-profile.json` file.

**Request**:
```typescript
interface ImportProfileRequest {
  input_path: string;
  rename_to?: string; // Optional new name if collision
}
```

**Response**:
```typescript
interface ImportProfileResponse {
  profile: ProfileSummary;
  imported_from: string;
  collision_resolved: boolean; // True if rename was needed
}
```

**Errors**:
- `IoError`: Cannot read input file
- `InvalidFormat`: File is not valid profile JSON
- `VersionUnsupported`: Profile version not supported
- `NameCollision`: Profile name exists and no rename provided

---

#### `preview_import`

Preview what a profile import would do (for collision detection).

**Request**:
```typescript
interface PreviewImportRequest {
  input_path: string;
}
```

**Response**:
```typescript
interface PreviewImportResponse {
  name: string;
  description: string | null;
  tool_count: number;
  has_name_collision: boolean;
  existing_profile_id: string | null; // ID of colliding profile
  version: number;
}
```

**Errors**:
- `IoError`: Cannot read input file
- `InvalidFormat`: File is not valid profile JSON

---

## Error Response Format

All errors follow this format:

```typescript
interface TauriError {
  code: string;      // Error code (e.g., "ProfileNotFound")
  message: string;   // Human-readable message
  details?: unknown; // Optional additional context
}
```

---

## Frontend Integration

### TypeScript Types File

Add to `apps/tars-desktop/src/lib/types/index.ts`:

```typescript
// Profile types
export interface ProfileSummary { ... }
export interface ProfileDetails { ... }
export interface ToolRef { ... }
export interface ToolPermissions { ... }

// Request/Response types
export interface CreateProfileRequest { ... }
export interface UpdateProfileRequest { ... }
// ... etc
```

### IPC Wrapper Functions

Add to `apps/tars-desktop/src/lib/ipc/index.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

// Profile CRUD
export const createProfile = (req: CreateProfileRequest) =>
  invoke<ProfileSummary>('create_profile', req);

export const getProfile = (id: string) =>
  invoke<ProfileDetails>('get_profile', { id });

export const listProfiles = () =>
  invoke<ProfileSummary[]>('list_profiles');

export const updateProfile = (req: UpdateProfileRequest) =>
  invoke<UpdateProfileResponse>('update_profile', req);

export const deleteProfile = (id: string) =>
  invoke<DeleteProfileResponse>('delete_profile', { id });

// Profile assignment
export const assignProfile = (projectId: string, profileId: string) =>
  invoke<AssignProfileResponse>('assign_profile', { project_id: projectId, profile_id: profileId });

export const unassignProfile = (projectId: string) =>
  invoke<UnassignProfileResponse>('unassign_profile', { project_id: projectId });

// Local overrides
export const addLocalTool = (projectId: string, toolRef: ToolRef) =>
  invoke<AddLocalToolResponse>('add_local_tool', { project_id: projectId, tool_ref: toolRef });

export const removeLocalTool = (projectId: string, toolName: string, toolType: string) =>
  invoke<RemoveLocalToolResponse>('remove_local_tool', { project_id: projectId, tool_name: toolName, tool_type: toolType });

export const getProjectTools = (projectId: string) =>
  invoke<ProjectToolsResponse>('get_project_tools', { project_id: projectId });

// Export/Import
export const exportProfile = (profileId: string, outputPath: string) =>
  invoke<ExportProfileResponse>('export_profile', { profile_id: profileId, output_path: outputPath });

export const importProfile = (inputPath: string, renameTo?: string) =>
  invoke<ImportProfileResponse>('import_profile', { input_path: inputPath, rename_to: renameTo });

export const previewImport = (inputPath: string) =>
  invoke<PreviewImportResponse>('preview_import', { input_path: inputPath });
```
