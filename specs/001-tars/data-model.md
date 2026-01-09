# Data Model: TARS

**Date**: 2026-01-08
**Status**: Complete

## Overview

This document defines the core data entities for TARS, derived from the inventory schema and profile design in the feature specification.

---

## Entity Relationship Diagram

```
┌──────────────────┐       ┌──────────────────┐
│     Project      │───────│     Profile      │
└──────────────────┘  1:N  └──────────────────┘
         │                         │
         │                         │
         ▼                         ▼
┌──────────────────┐       ┌──────────────────┐
│    Inventory     │       │    PluginSet     │
└──────────────────┘       └──────────────────┘
         │                         │
    ┌────┴────┐               ┌────┴────┐
    │         │               │         │
    ▼         ▼               ▼         ▼
┌────────┐ ┌────────┐   ┌────────┐ ┌────────┐
│ Skill  │ │Command │   │ Plugin │ │Overlay │
└────────┘ └────────┘   └────────┘ └────────┘
    │         │
    └────┬────┘
         ▼
┌──────────────────┐
│    Collision     │
└──────────────────┘
```

---

## Core Entities

### Project

A registered folder (typically a git repo) containing Claude Code artifacts.

```rust
pub struct Project {
    pub id: Uuid,
    pub path: PathBuf,
    pub name: String,
    pub git_info: Option<GitInfo>,
    pub last_scanned: Option<DateTime<Utc>>,
    pub assigned_profile_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct GitInfo {
    pub remote: Option<String>,
    pub branch: String,
    pub is_dirty: bool,
}
```

**Validation Rules:**
- `path` must exist and be a directory
- `name` derived from folder name if not specified
- `is_dirty` triggers warning before profile application

---

### Inventory

Complete scan result for a project or user scope.

```rust
pub struct Inventory {
    pub host: HostInfo,
    pub user_scope: UserScope,
    pub managed_scope: Option<ManagedScope>,
    pub projects: Vec<ProjectScope>,
    pub plugins: PluginInventory,
    pub collisions: CollisionReport,
    pub scanned_at: DateTime<Utc>,
}

pub struct HostInfo {
    pub os: String,
    pub username: String,
    pub home_dir: PathBuf,
}

pub struct UserScope {
    pub settings: Option<SettingsFile>,
    pub mcp: Option<McpConfig>,
    pub skills: Vec<SkillInfo>,
    pub commands: Vec<CommandInfo>,
    pub agents: Vec<AgentInfo>,
}

pub struct ManagedScope {
    pub settings: Option<SettingsFile>,
    pub mcp: Option<McpConfig>,
}

pub struct ProjectScope {
    pub path: PathBuf,
    pub name: String,
    pub git: Option<GitInfo>,
    pub claude_md: Option<FileInfo>,
    pub claude_dir: Option<PathBuf>,
    pub settings: ProjectSettings,
    pub mcp: Option<McpConfig>,
    pub skills: Vec<SkillInfo>,
    pub commands: Vec<CommandInfo>,
    pub agents: Vec<AgentInfo>,
    pub hooks: Vec<HookInfo>,
}

pub struct ProjectSettings {
    pub shared: Option<SettingsFile>,
    pub local: Option<SettingsFile>,
}
```

---

### Skill

A Claude Code skill parsed from SKILL.md.

```rust
pub struct SkillInfo {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub user_invocable: bool,
    pub disable_model_invocation: bool,
    pub allowed_tools: Vec<String>,
    pub model: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    pub hooks: HashMap<String, Vec<HookDefinition>>,
    pub sha256: String,
    pub scope: Scope,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    Project,
    Local,
    Managed,
    Plugin { plugin_id: String },
}
```

**Validation Rules:**
- `name` required, lowercase with hyphens
- `description` required, max 1024 chars
- `sha256` computed from file content for change detection

---

### Command

A Claude Code command parsed from .md file.

```rust
pub struct CommandInfo {
    pub path: PathBuf,
    pub name: String,
    pub description: Option<String>,
    pub thinking: bool,
    pub body: String,
    pub sha256: String,
    pub scope: Scope,
}
```

**Validation Rules:**
- `name` derived from filename (without .md extension)
- `body` contains template with `$ARGUMENTS`, `$1`, `$2` placeholders

---

### Agent

A Claude Code agent definition.

```rust
pub struct AgentInfo {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub permission_mode: String,
    pub skills: Vec<String>,
    pub hooks: HashMap<String, Vec<HookDefinition>>,
    pub sha256: String,
    pub scope: Scope,
}
```

---

### Hook

A hook definition from settings or embedded in skill/agent.

```rust
pub struct HookInfo {
    pub source: HookSource,
    pub trigger: HookTrigger,
    pub matcher: Option<String>,
    pub definition: HookDefinition,
}

pub enum HookSource {
    Settings { path: PathBuf },
    Skill { name: String },
    Agent { name: String },
}

pub enum HookTrigger {
    PreToolUse,
    PostToolUse,
    PermissionRequest,
    UserPromptSubmit,
    SessionStart,
    SessionEnd,
    Notification,
    Stop,
    SubagentStop,
    PreCompact,
}

pub enum HookDefinition {
    Command { command: String },
    Prompt { prompt: String },
    Agent { agent: String },
}
```

---

### Settings File

Parsed settings.json content.

```rust
pub struct SettingsFile {
    pub path: PathBuf,
    pub sha256: String,
    pub hooks_count: usize,
    pub permissions: Option<Permissions>,
    pub enabled_plugins: HashMap<String, bool>,
    pub env: HashMap<String, String>,
    pub model: Option<String>,
}

pub struct Permissions {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    pub default_mode: Option<String>,
}
```

---

### MCP Configuration

Parsed .mcp.json or ~/.claude.json content.

```rust
pub struct McpConfig {
    pub path: PathBuf,
    pub sha256: String,
    pub servers: Vec<McpServer>,
}

pub struct McpServer {
    pub name: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
}

pub enum McpTransport {
    Stdio,
    Http,
    Sse,
}
```

---

### Plugin

An installed Claude Code plugin.

```rust
pub struct PluginInventory {
    pub marketplaces: Vec<Marketplace>,
    pub installed: Vec<InstalledPlugin>,
}

pub struct Marketplace {
    pub name: String,
    pub source_type: MarketplaceSource,
    pub location: String,
    pub auto_update: bool,
}

pub enum MarketplaceSource {
    GitHub { owner: String, repo: String },
    Url { url: String },
    Local { path: PathBuf },
}

pub struct InstalledPlugin {
    pub id: String,
    pub marketplace: Option<String>,
    pub version: String,
    pub scope: Scope,
    pub enabled: bool,
    pub path: PathBuf,
    pub manifest: PluginManifest,
}

pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<Author>,
    pub commands: Vec<PathBuf>,
    pub agents: Option<PathBuf>,
    pub skills: Option<PathBuf>,
    pub hooks: Option<PathBuf>,
    pub mcp_servers: Option<PathBuf>,
}

pub struct Author {
    pub name: String,
    pub email: Option<String>,
}
```

---

### Collision

Detected name collisions across scopes.

```rust
pub struct CollisionReport {
    pub skills: Vec<Collision>,
    pub commands: Vec<Collision>,
    pub agents: Vec<Collision>,
}

pub struct Collision {
    pub name: String,
    pub winner_scope: Scope,
    pub occurrences: Vec<CollisionOccurrence>,
}

pub struct CollisionOccurrence {
    pub scope: Scope,
    pub path: PathBuf,
}
```

**Precedence Rules (highest to lowest):**
1. Managed
2. CLI args (not stored)
3. Local (project)
4. Project (shared)
5. User
6. Plugin

---

### Profile

A named configuration bundle that can be applied to projects.

```rust
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub plugin_set: PluginSet,
    pub repo_overlays: RepoOverlays,
    pub user_overlays: UserOverlays,
    pub adapters: Adapters,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PluginSet {
    pub marketplaces: Vec<MarketplaceRef>,
    pub plugins: Vec<PluginRef>,
}

pub struct MarketplaceRef {
    pub name: String,
    pub source: MarketplaceSource,
}

pub struct PluginRef {
    pub id: String,
    pub marketplace: Option<String>,
    pub scope: Scope,
    pub enabled: bool,
}

pub struct RepoOverlays {
    pub skills: Vec<SkillOverlay>,
    pub commands: Vec<CommandOverlay>,
    pub agents: Vec<AgentOverlay>,
    pub claude_md: Option<ClaudeMdOverlay>,
}

pub struct UserOverlays {
    pub skills: Vec<SkillOverlay>,
    pub commands: Vec<CommandOverlay>,
}

pub struct SkillOverlay {
    pub name: String,
    pub content: String,  // Full SKILL.md content
}

pub struct CommandOverlay {
    pub name: String,
    pub content: String,
}

pub struct AgentOverlay {
    pub name: String,
    pub content: String,
}

pub struct ClaudeMdOverlay {
    pub mode: OverlayMode,
    pub content: String,
}

pub enum OverlayMode {
    Replace,
    Prepend,
    Append,
}

pub struct Adapters {
    pub mcp_location: McpLocation,
    pub merge_strategies: HashMap<String, MergeStrategy>,
}

pub enum McpLocation {
    ProjectRoot,      // <repo>/.mcp.json
    ClaudeDir,        // <repo>/.claude/mcp.json
}

pub enum MergeStrategy {
    Replace,
    Merge,
    Skip,
}
```

**Validation Rules:**
- No secrets embedded (only env var names)
- Profile export format: `profile.json` + files in zip

---

### Backup

Rollback bundle for profile application.

```rust
pub struct Backup {
    pub id: Uuid,
    pub project_id: Uuid,
    pub profile_id: Uuid,
    pub files: Vec<BackupFile>,
    pub created_at: DateTime<Utc>,
}

pub struct BackupFile {
    pub path: PathBuf,
    pub original_content: Option<Vec<u8>>,  // None if file didn't exist
    pub sha256: Option<String>,
}
```

**Validation Rules:**
- Rollback must restore byte-for-byte original state
- Backup retained until explicit cleanup

---

## SQLite Schema

```sql
-- Projects table
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    git_remote TEXT,
    git_branch TEXT,
    git_dirty INTEGER,
    last_scanned TEXT,
    assigned_profile_id TEXT REFERENCES profiles(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Profiles table
CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    plugin_set_json TEXT NOT NULL,
    repo_overlays_json TEXT NOT NULL,
    user_overlays_json TEXT NOT NULL,
    adapters_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Backups table
CREATE TABLE backups (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    files_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Inventory cache (optional, for faster re-display)
CREATE TABLE inventory_cache (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id),
    scope TEXT NOT NULL,  -- 'user', 'managed', or project path
    inventory_json TEXT NOT NULL,
    scanned_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_projects_path ON projects(path);
CREATE INDEX idx_profiles_name ON profiles(name);
CREATE INDEX idx_backups_project ON backups(project_id);
CREATE INDEX idx_inventory_scope ON inventory_cache(scope);
```

---

## State Transitions

### Profile Application Flow

```
┌──────────────┐
│    Idle      │
└──────┬───────┘
       │ apply(profile, project)
       ▼
┌──────────────┐
│   Planning   │ ──> generates DiffPlan
└──────┬───────┘
       │ user confirms
       ▼
┌──────────────┐
│   Backup     │ ──> creates Backup
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Applying   │ ──> writes files
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Applied    │
└──────────────┘
       │ rollback()
       ▼
┌──────────────┐
│  Rolling     │ ──> restores from Backup
│    Back      │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│    Idle      │
└──────────────┘
```

### Diff Plan

```rust
pub struct DiffPlan {
    pub project_id: Uuid,
    pub profile_id: Uuid,
    pub operations: Vec<FileOperation>,
    pub warnings: Vec<Warning>,
}

pub enum FileOperation {
    Create { path: PathBuf, content: Vec<u8> },
    Modify { path: PathBuf, diff: String, new_content: Vec<u8> },
    Delete { path: PathBuf },
}

pub struct Warning {
    pub severity: WarningSeverity,
    pub message: String,
}

pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}
```
