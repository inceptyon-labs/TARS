# Research: Config Operations Layer

**Feature**: 002-config-operations | **Date**: 2026-01-09

## 1. Existing Scanner Types

### SkillInfo
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
```

### CommandInfo
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

### AgentInfo
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

### HookInfo
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
    PreToolUse, PostToolUse, PermissionRequest,
    UserPromptSubmit, SessionStart, SessionEnd,
    Notification, Stop, SubagentStop, PreCompact,
}

pub enum HookDefinition {
    Command { command: String },
    Prompt { prompt: String },
    Agent { agent: String },
}
```

### McpServer
```rust
pub struct McpServer {
    pub name: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
}

pub enum McpTransport { Stdio, Http, Sse }
```

### Scope Enum
```rust
pub enum Scope { User, Project, Local, Managed, Plugin(String) }
```

---

## 2. Existing Core Types

### Profile (tars-core)
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
```

### Backup
```rust
pub struct Backup {
    pub id: Uuid,
    pub project_id: Uuid,
    pub profile_id: Option<Uuid>,
    pub description: Option<String>,
    pub archive_path: PathBuf,
    pub files: Vec<BackupFile>,
    pub created_at: DateTime<Utc>,
}

pub struct BackupFile {
    pub path: PathBuf,
    pub original_content: Option<Vec<u8>>,
    pub sha256: Option<String>,
}
```

### DiffPlan
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
```

---

## 3. JSON Manipulation Patterns

**Library**: `serde_json 1.0`

**Parsing Pattern** (MCP example):
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMcpConfig {
    mcp_servers: HashMap<String, RawMcpServer>,
}

// Read
let content = fs::read_to_string(&path)?;
let config: RawMcpConfig = serde_json::from_str(&content)?;

// Write
let json = serde_json::to_string_pretty(&config)?;
fs::write(&path, json)?;
```

**Key Pattern for Config Ops**:
- Deserialize to HashMap (server name = key)
- Manipulate HashMap (insert, remove, update)
- Re-serialize with pretty print

---

## 4. Frontmatter Parsing

**Library**: `gray_matter 0.2` with YAML engine

**Pattern**:
```rust
use gray_matter::{Matter, engine::YAML};

let matter = Matter::<YAML>::new();
let result = matter.parse(content);

// Extract data
let frontmatter: SkillFrontmatter = result.data
    .ok_or(ScanError::NoFrontmatter)?
    .deserialize()?;

// Get body
let body = result.content;
```

**Field Naming**: Frontmatter uses kebab-case, handled by `#[serde(rename_all = "kebab-case")]`

---

## 5. File Writing Patterns

**Safe Path Join**:
```rust
pub fn safe_join(root: &Path, untrusted: &Path) -> Result<PathBuf, PathError> {
    // Rejects absolute paths, .., and symlinks
}
```

**Write with Backup**:
```rust
// 1. Read original content
let original = fs::read(&full_path).ok();

// 2. Compute hash
let sha256 = compute_sha256(&original);

// 3. Record backup
backup_files.push(BackupFile {
    path: relative_path,
    original_content: original,
    sha256: Some(sha256),
});

// 4. Write new content
fs::create_dir_all(parent)?;
fs::write(&full_path, new_content)?;
```

---

## 6. Claude Code File Formats

### .mcp.json
```json
{
  "mcpServers": {
    "server-name": {
      "type": "stdio",
      "command": "/path/to/command",
      "args": ["arg1", "arg2"],
      "env": { "KEY": "value" }
    }
  }
}
```

**Note**: Server name is the dictionary key, not a field.

### settings.json (hooks section)
```json
{
  "hooks": {
    "PreToolUse": [
      { "type": "command", "command": "./script.sh" },
      { "type": "prompt", "prompt": "Check before running" }
    ],
    "PostToolUse": [...]
  },
  "permissions": {
    "allow": ["Bash(npm:*)", "Read(*)"],
    "deny": ["Read(.env)"],
    "defaultMode": "acceptEdits"
  },
  "env": { "KEY": "value" },
  "model": "opus"
}
```

### SKILL.md
```markdown
---
name: skill-name
description: One-line description
user-invocable: true
disable-model-invocation: false
allowed-tools:
  - Read
  - Grep
model: opus-4-5
hooks:
  PreToolUse:
    - type: command
      command: "echo hello"
---

Skill body content here.
```

---

## 7. File Paths by Scope

| Item Type | User Scope | Project Scope | Local Scope |
|-----------|------------|---------------|-------------|
| MCP Servers | `~/.claude.json` | `.mcp.json` | N/A |
| Settings | `~/.claude/settings.json` | `.claude/settings.json` | `.claude/settings.local.json` |
| Skills | `~/.claude/skills/<name>/SKILL.md` | `.claude/skills/<name>/SKILL.md` | N/A |
| Commands | `~/.claude/commands/<name>.md` | `.claude/commands/<name>.md` | N/A |
| Agents | `~/.claude/agents/<name>.md` | `.claude/agents/<name>.md` | N/A |

---

## 8. Dependencies

**No new dependencies required.** Existing workspace provides:
- `serde` + `serde_json` - JSON manipulation
- `gray_matter` + `serde_yml` - Frontmatter parsing
- `thiserror` - Error handling
- `uuid`, `chrono` - IDs and timestamps
- `sha2` + `hex` - Hashing
- `similar` - Diffing
- `rusqlite` - SQLite storage

---

## 9. Design Decisions

### Decision 1: ConfigItem Enum

**Choice**: Unified enum for all config item types
```rust
pub enum ConfigItem {
    McpServer(McpServer),
    Skill(SkillInfo),
    Hook(HookInfo),
    Command(CommandInfo),
    Agent(AgentInfo),
}
```

**Rationale**: Allows generic operation handling while preserving type safety.

**Alternatives Rejected**:
- Trait-based approach: More complex, less ergonomic for CLI
- Separate APIs per type: Code duplication

### Decision 2: MCP First Priority

**Choice**: Implement MCP server operations before other types

**Rationale**:
- Most common user operation (P1 priority in spec)
- Simplest structure (JSON dict, no frontmatter)
- Tests can validate merge/preserve patterns

### Decision 3: In-Memory Manipulation

**Choice**: Load → Modify → Save entire files (not streaming)

**Rationale**:
- Config files are small (<10KB typical, <1MB max)
- Allows accurate diff preview
- Simplifies backup creation
- Matches existing profile apply pattern

**Alternative Rejected**:
- Streaming JSON manipulation: Complexity not justified for file sizes

### Decision 4: Preserve Formatting Strategy

**Choice**: Use `serde_json::to_string_pretty()` with 2-space indent

**Rationale**:
- Matches Claude Code's default format
- Human-readable for manual inspection
- Deterministic output

**Limitation**: Custom formatting (unusual indentation, trailing commas) may not be preserved.

### Decision 5: Atomic Move Operations

**Choice**: Move = Delete from source + Add to target (wrapped in backup)

**Rationale**:
- Uses existing add/remove primitives
- Backup covers both files
- Single rollback point

**Alternative Rejected**:
- Transactional file system ops: Not portable, overkill for config files

---

## 10. Test Strategy

### Unit Tests
- MCP server dict manipulation (add, remove, update)
- Frontmatter round-trip (parse → modify → serialize)
- Path validation (traversal rejection)
- Conflict detection

### Integration Tests
- Full operation flow with temp directories
- Backup creation verification
- Rollback byte-for-byte verification
- Multi-scope move operations

### Fixtures
- Sample `.mcp.json` with multiple servers
- Sample `settings.json` with hooks
- Sample `SKILL.md` files

---

## Summary

The existing TARS codebase provides all necessary infrastructure:
- Scanner types for reading current state
- Core types for backups and diff operations
- JSON and frontmatter parsing libraries
- Safe file writing patterns

The Config Operations Layer will add:
- Granular manipulation of individual config items
- Merge logic for JSON dictionaries
- Cross-scope move operations
- CLI commands for each operation type
