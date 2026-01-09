# Tauri IPC Commands: TARS

**Date**: 2026-01-08
**Status**: Complete

This document defines the Tauri IPC commands that form the contract between the React frontend and Rust backend.

---

## Scanner Commands

### `scan_user_scope`

Scan user-level Claude Code configuration.

```typescript
// Frontend
async function scanUserScope(): Promise<UserScope>

// Rust
#[tauri::command]
async fn scan_user_scope() -> Result<UserScope, String>
```

**Returns:** User scope inventory (settings, skills, commands, agents, MCP)

---

### `scan_managed_scope`

Scan managed (IT-deployed) Claude Code configuration.

```typescript
// Frontend
async function scanManagedScope(): Promise<ManagedScope | null>

// Rust
#[tauri::command]
async fn scan_managed_scope() -> Result<Option<ManagedScope>, String>
```

**Returns:** Managed scope inventory or null if not present

---

### `scan_project`

Scan a specific project directory.

```typescript
// Frontend
async function scanProject(path: string): Promise<ProjectScope>

// Rust
#[tauri::command]
async fn scan_project(path: String) -> Result<ProjectScope, String>
```

**Parameters:**
- `path`: Absolute path to project directory

**Returns:** Project scope inventory

**Errors:**
- Path does not exist
- Path is not a directory

---

### `scan_all`

Perform full inventory scan (user + managed + all registered projects).

```typescript
// Frontend
async function scanAll(): Promise<Inventory>

// Rust
#[tauri::command]
async fn scan_all(state: State<'_, AppState>) -> Result<Inventory, String>
```

**Returns:** Complete inventory with collision detection

---

### `detect_collisions`

Detect name collisions across scopes.

```typescript
// Frontend
async function detectCollisions(inventory: Inventory): Promise<CollisionReport>

// Rust
#[tauri::command]
fn detect_collisions(inventory: Inventory) -> CollisionReport
```

**Returns:** Collision report with precedence winners

---

## Project Commands

### `list_projects`

List all registered projects.

```typescript
// Frontend
async function listProjects(): Promise<Project[]>

// Rust
#[tauri::command]
async fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String>
```

---

### `add_project`

Register a new project.

```typescript
// Frontend
async function addProject(path: string): Promise<Project>

// Rust
#[tauri::command]
async fn add_project(
    path: String,
    state: State<'_, AppState>
) -> Result<Project, String>
```

**Parameters:**
- `path`: Absolute path to project directory

**Errors:**
- Path does not exist
- Path already registered
- Path is not a directory

---

### `remove_project`

Unregister a project (does not delete files).

```typescript
// Frontend
async function removeProject(projectId: string): Promise<void>

// Rust
#[tauri::command]
async fn remove_project(
    project_id: String,
    state: State<'_, AppState>
) -> Result<(), String>
```

---

### `get_project`

Get details for a specific project.

```typescript
// Frontend
async function getProject(projectId: string): Promise<Project>

// Rust
#[tauri::command]
async fn get_project(
    project_id: String,
    state: State<'_, AppState>
) -> Result<Project, String>
```

---

## Profile Commands

### `list_profiles`

List all profiles.

```typescript
// Frontend
async function listProfiles(): Promise<Profile[]>

// Rust
#[tauri::command]
async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, String>
```

---

### `create_profile`

Create a new profile from current state.

```typescript
// Frontend
async function createProfile(params: CreateProfileParams): Promise<Profile>

interface CreateProfileParams {
    name: string;
    description?: string;
    sourceProjectId?: string;  // If creating from project snapshot
    includeUserScope: boolean;
}

// Rust
#[tauri::command]
async fn create_profile(
    params: CreateProfileParams,
    state: State<'_, AppState>
) -> Result<Profile, String>
```

---

### `update_profile`

Update an existing profile.

```typescript
// Frontend
async function updateProfile(profileId: string, updates: ProfileUpdates): Promise<Profile>

interface ProfileUpdates {
    name?: string;
    description?: string;
    pluginSet?: PluginSet;
    repoOverlays?: RepoOverlays;
    userOverlays?: UserOverlays;
    adapters?: Adapters;
}

// Rust
#[tauri::command]
async fn update_profile(
    profile_id: String,
    updates: ProfileUpdates,
    state: State<'_, AppState>
) -> Result<Profile, String>
```

---

### `delete_profile`

Delete a profile.

```typescript
// Frontend
async function deleteProfile(profileId: string): Promise<void>

// Rust
#[tauri::command]
async fn delete_profile(
    profile_id: String,
    state: State<'_, AppState>
) -> Result<(), String>
```

**Errors:**
- Profile is assigned to projects (must unassign first)

---

### `assign_profile`

Assign a profile to a project.

```typescript
// Frontend
async function assignProfile(projectId: string, profileId: string | null): Promise<void>

// Rust
#[tauri::command]
async fn assign_profile(
    project_id: String,
    profile_id: Option<String>,
    state: State<'_, AppState>
) -> Result<(), String>
```

**Parameters:**
- `profileId`: Profile ID or null to unassign

---

### `export_profile`

Export profile as a file bundle.

```typescript
// Frontend
async function exportProfile(profileId: string, outputPath: string): Promise<string>

// Rust
#[tauri::command]
async fn export_profile(
    profile_id: String,
    output_path: String,
    state: State<'_, AppState>
) -> Result<String, String>  // Returns path to exported file
```

---

### `import_profile`

Import profile from a file bundle.

```typescript
// Frontend
async function importProfile(filePath: string): Promise<Profile>

// Rust
#[tauri::command]
async fn import_profile(
    file_path: String,
    state: State<'_, AppState>
) -> Result<Profile, String>
```

---

## Apply Commands

### `preview_apply`

Generate diff preview for profile application.

```typescript
// Frontend
async function previewApply(projectId: string, profileId: string): Promise<DiffPlan>

// Rust
#[tauri::command]
async fn preview_apply(
    project_id: String,
    profile_id: String,
    state: State<'_, AppState>
) -> Result<DiffPlan, String>
```

**Returns:** Diff plan with operations and warnings

---

### `apply_profile`

Apply profile to project (after preview confirmation).

```typescript
// Frontend
async function applyProfile(projectId: string, profileId: string): Promise<Backup>

// Rust
#[tauri::command]
async fn apply_profile(
    project_id: String,
    profile_id: String,
    state: State<'_, AppState>
) -> Result<Backup, String>
```

**Returns:** Backup for rollback

**Errors:**
- Git repo is dirty (warning, can force)
- File conflict detected

---

### `rollback`

Rollback to previous state using backup.

```typescript
// Frontend
async function rollback(backupId: string): Promise<void>

// Rust
#[tauri::command]
async fn rollback(
    backup_id: String,
    state: State<'_, AppState>
) -> Result<(), String>
```

---

### `list_backups`

List available backups for a project.

```typescript
// Frontend
async function listBackups(projectId: string): Promise<Backup[]>

// Rust
#[tauri::command]
async fn list_backups(
    project_id: String,
    state: State<'_, AppState>
) -> Result<Vec<Backup>, String>
```

---

## Plugin Commands

### `list_marketplaces`

List configured plugin marketplaces.

```typescript
// Frontend
async function listMarketplaces(): Promise<Marketplace[]>

// Rust
#[tauri::command]
async fn list_marketplaces() -> Result<Vec<Marketplace>, String>
```

**Note:** Wraps `claude plugin marketplace list`

---

### `list_plugins`

List installed plugins.

```typescript
// Frontend
async function listPlugins(): Promise<InstalledPlugin[]>

// Rust
#[tauri::command]
async fn list_plugins() -> Result<Vec<InstalledPlugin>, String>
```

**Note:** Wraps `claude plugin list`

---

### `install_plugin`

Install a plugin.

```typescript
// Frontend
async function installPlugin(params: InstallPluginParams): Promise<InstalledPlugin>

interface InstallPluginParams {
    pluginId: string;
    marketplace?: string;
    scope: 'user' | 'project' | 'local';
}

// Rust
#[tauri::command]
async fn install_plugin(params: InstallPluginParams) -> Result<InstalledPlugin, String>
```

**Note:** Wraps `claude plugin install`

---

### `uninstall_plugin`

Uninstall a plugin.

```typescript
// Frontend
async function uninstallPlugin(pluginId: string, scope: string): Promise<void>

// Rust
#[tauri::command]
async fn uninstall_plugin(
    plugin_id: String,
    scope: String
) -> Result<(), String>
```

**Note:** Wraps `claude plugin uninstall`

---

### `enable_plugin`

Enable a plugin.

```typescript
// Frontend
async function enablePlugin(pluginId: string, scope: string): Promise<void>

// Rust
#[tauri::command]
async fn enable_plugin(plugin_id: String, scope: String) -> Result<(), String>
```

---

### `disable_plugin`

Disable a plugin.

```typescript
// Frontend
async function disablePlugin(pluginId: string, scope: string): Promise<void>

// Rust
#[tauri::command]
async fn disable_plugin(plugin_id: String, scope: String) -> Result<(), String>
```

---

### `export_as_plugin`

Export a profile as a Claude Code plugin.

```typescript
// Frontend
async function exportAsPlugin(params: ExportPluginParams): Promise<string>

interface ExportPluginParams {
    profileId: string;
    outputPath: string;
    pluginName: string;
    pluginVersion: string;
    author?: Author;
}

// Rust
#[tauri::command]
async fn export_as_plugin(
    params: ExportPluginParams,
    state: State<'_, AppState>
) -> Result<String, String>  // Returns path to exported plugin
```

---

## Skill Commands

### `get_skill`

Read skill content.

```typescript
// Frontend
async function getSkill(path: string): Promise<SkillContent>

interface SkillContent {
    frontmatter: SkillFrontmatter;
    body: string;
    raw: string;
}

// Rust
#[tauri::command]
async fn get_skill(path: String) -> Result<SkillContent, String>
```

---

### `save_skill`

Save skill content.

```typescript
// Frontend
async function saveSkill(path: string, content: SkillContent): Promise<void>

// Rust
#[tauri::command]
async fn save_skill(path: String, content: SkillContent) -> Result<(), String>
```

---

### `create_skill`

Create a new skill.

```typescript
// Frontend
async function createSkill(params: CreateSkillParams): Promise<string>

interface CreateSkillParams {
    directory: string;  // Parent directory
    name: string;
    description: string;
    body?: string;
}

// Rust
#[tauri::command]
async fn create_skill(params: CreateSkillParams) -> Result<String, String>
```

**Returns:** Path to created skill directory

---

## Utility Commands

### `select_directory`

Open native directory picker.

```typescript
// Frontend
async function selectDirectory(): Promise<string | null>

// Rust
#[tauri::command]
async fn select_directory(app: AppHandle) -> Result<Option<String>, String>
```

---

### `open_in_finder`

Open path in Finder.

```typescript
// Frontend
async function openInFinder(path: string): Promise<void>

// Rust
#[tauri::command]
async fn open_in_finder(path: String) -> Result<(), String>
```

---

### `open_in_editor`

Open file in default editor.

```typescript
// Frontend
async function openInEditor(path: string): Promise<void>

// Rust
#[tauri::command]
async fn open_in_editor(path: String) -> Result<(), String>
```

---

## Error Types

All commands return `Result<T, String>` where the error string follows this format:

```typescript
interface CommandError {
    code: string;
    message: string;
    details?: Record<string, unknown>;
}
```

**Error Codes:**
- `NOT_FOUND`: Resource not found
- `ALREADY_EXISTS`: Resource already exists
- `INVALID_PATH`: Invalid file path
- `PARSE_ERROR`: Failed to parse file
- `IO_ERROR`: File system error
- `CLI_ERROR`: Claude Code CLI error
- `VALIDATION_ERROR`: Input validation failed
- `GIT_DIRTY`: Git repository has uncommitted changes
