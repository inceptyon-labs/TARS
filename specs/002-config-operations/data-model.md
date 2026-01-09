# Data Model: Config Operations Layer

**Feature**: 002-config-operations | **Date**: 2026-01-09

## Entity Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   ConfigItem    │────▶│ ConfigOperation │────▶│ OperationResult │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        ▼                       ▼                       ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   ConfigScope   │     │  OperationPlan  │     │     Backup      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

---

## Core Entities

### ConfigItem

A single configuration entry that can be managed by TARS.

| Field | Type | Description |
|-------|------|-------------|
| item_type | ConfigItemType | Discriminator (MCP, Skill, Hook, Command, Agent) |
| name | String | Unique identifier within scope and type |
| scope | ConfigScope | Where the item is stored |
| content | ItemContent | Type-specific content |

```rust
pub enum ConfigItem {
    McpServer {
        name: String,
        scope: ConfigScope,
        server: McpServerConfig,
    },
    Skill {
        name: String,
        scope: ConfigScope,
        skill: SkillConfig,
    },
    Hook {
        trigger: HookTrigger,
        scope: ConfigScope,
        matcher: Option<String>,
        definition: HookDefinition,
    },
    Command {
        name: String,
        scope: ConfigScope,
        command: CommandConfig,
    },
    Agent {
        name: String,
        scope: ConfigScope,
        agent: AgentConfig,
    },
}
```

**Validation Rules**:
- `name` must be non-empty, no path separators, no `..`
- `name` must be unique within (scope, item_type)
- `scope` must be writable (not Managed)

---

### ConfigScope

Specifies where configuration is stored.

| Variant | Path Pattern | Writable |
|---------|-------------|----------|
| User | `~/.claude/` or `~/.claude.json` | Yes |
| Project | `.claude/` or `.mcp.json` | Yes |
| Local | `.claude/settings.local.json` | Yes |
| Managed | `/Library/Application Support/ClaudeCode/` | No |

```rust
pub enum ConfigScope {
    User,
    Project { path: PathBuf },
    Local { path: PathBuf },
    Managed,
}
```

**Path Resolution**:

| Item Type | User Path | Project Path |
|-----------|-----------|--------------|
| McpServer | `~/.claude.json` | `.mcp.json` |
| Skill | `~/.claude/skills/{name}/SKILL.md` | `.claude/skills/{name}/SKILL.md` |
| Hook | `~/.claude/settings.json` | `.claude/settings.json` |
| Command | `~/.claude/commands/{name}.md` | `.claude/commands/{name}.md` |
| Agent | `~/.claude/agents/{name}.md` | `.claude/agents/{name}.md` |

---

### ConfigOperation

An action to perform on a configuration item.

| Field | Type | Description |
|-------|------|-------------|
| action | OperationAction | What to do (Add, Remove, Update, Move) |
| item | ConfigItem | The item to operate on |
| target_scope | Option<ConfigScope> | For Move: destination scope |
| dry_run | bool | Preview only, don't write |

```rust
pub struct ConfigOperation {
    pub action: OperationAction,
    pub item: ConfigItem,
    pub target_scope: Option<ConfigScope>,
    pub dry_run: bool,
}

pub enum OperationAction {
    Add,
    Remove,
    Update { fields: PartialUpdate },
    Move,
}

pub struct PartialUpdate {
    pub fields: HashMap<String, serde_json::Value>,
}
```

**State Transitions**:

```
[Not Exists] --Add--> [Exists]
[Exists] --Remove--> [Not Exists]
[Exists] --Update--> [Exists (modified)]
[Exists in Scope A] --Move--> [Exists in Scope B]
```

---

### OperationPlan

A validated plan ready for execution.

| Field | Type | Description |
|-------|------|-------------|
| operation | ConfigOperation | The operation to perform |
| file_changes | Vec<FileChange> | Files to create/modify/delete |
| warnings | Vec<Warning> | Non-blocking issues |
| conflicts | Vec<Conflict> | Blocking issues requiring resolution |

```rust
pub struct OperationPlan {
    pub operation: ConfigOperation,
    pub file_changes: Vec<FileChange>,
    pub warnings: Vec<Warning>,
    pub conflicts: Vec<Conflict>,
}

pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub diff: Option<String>,
}

pub enum FileChangeType {
    Create,
    Modify,
    Delete,
}

pub struct Warning {
    pub code: String,
    pub message: String,
}

pub struct Conflict {
    pub code: String,
    pub message: String,
    pub resolution_options: Vec<String>,
}
```

---

### OperationResult

The outcome of executing an operation.

| Field | Type | Description |
|-------|------|-------------|
| success | bool | Whether operation completed |
| backup_id | Option<Uuid> | Backup created before changes |
| changes_applied | Vec<FileChange> | What was actually modified |
| errors | Vec<String> | Error messages if failed |

```rust
pub struct OperationResult {
    pub success: bool,
    pub backup_id: Option<Uuid>,
    pub changes_applied: Vec<FileChange>,
    pub errors: Vec<String>,
}
```

---

## Type-Specific Configs

### McpServerConfig

MCP server configuration matching Claude Code schema.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| transport | McpTransport | Yes | stdio, http, or sse |
| command | Option<String> | For stdio | Executable path |
| args | Vec<String> | No | Command arguments |
| env | HashMap<String, String> | No | Environment variables |
| url | Option<String> | For http/sse | Server URL |

```rust
pub struct McpServerConfig {
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

### SkillConfig

Skill definition matching SKILL.md frontmatter schema.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| description | String | Yes | One-line description |
| user_invocable | bool | No | Can user invoke directly |
| disable_model_invocation | bool | No | Prevent model from using |
| allowed_tools | Vec<String> | No | Tool whitelist |
| model | Option<String> | No | Preferred model |
| context | Option<String> | No | Context setting |
| agent | Option<String> | No | Associated agent |
| hooks | HashMap<String, Vec<HookDefinition>> | No | Skill-specific hooks |
| body | String | Yes | Skill instructions |

```rust
pub struct SkillConfig {
    pub description: String,
    pub user_invocable: bool,
    pub disable_model_invocation: bool,
    pub allowed_tools: Vec<String>,
    pub model: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    pub hooks: HashMap<String, Vec<HookDefinition>>,
    pub body: String,
}
```

---

### HookDefinition

Hook action definition matching Claude Code schema.

```rust
pub enum HookDefinition {
    Command { command: String },
    Prompt { prompt: String },
    Agent { agent: String },
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
```

---

### CommandConfig

Command definition matching command .md schema.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| description | Option<String> | No | Command description |
| thinking | bool | No | Enable thinking mode |
| body | String | Yes | Command template |

```rust
pub struct CommandConfig {
    pub description: Option<String>,
    pub thinking: bool,
    pub body: String,
}
```

---

### AgentConfig

Agent definition matching agent .md schema.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| description | String | Yes | Agent description |
| tools | Vec<String> | No | Allowed tools |
| model | Option<String> | No | Preferred model |
| permission_mode | String | Yes | Permission handling |
| skills | Vec<String> | No | Available skills |
| hooks | HashMap<String, Vec<HookDefinition>> | No | Agent-specific hooks |
| body | String | Yes | Agent instructions |

```rust
pub struct AgentConfig {
    pub description: String,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub permission_mode: String,
    pub skills: Vec<String>,
    pub hooks: HashMap<String, Vec<HookDefinition>>,
    pub body: String,
}
```

---

## Relationships

### ConfigItem → ConfigScope

Each ConfigItem belongs to exactly one ConfigScope. The scope determines:
1. File path where the item is stored
2. Precedence when same item exists in multiple scopes
3. Whether the item can be modified (Managed = read-only)

### ConfigOperation → OperationPlan

Before execution, each operation is validated and converted to a plan:
1. Check item exists (for Remove/Update) or doesn't exist (for Add)
2. Compute file changes needed
3. Detect conflicts with existing items
4. Generate warnings for non-blocking issues

### OperationPlan → OperationResult

Execution of a plan produces a result:
1. Backup is created before any writes
2. File changes are applied atomically (all or none)
3. Result includes backup ID for rollback

### OperationResult → Backup

Each successful operation creates a Backup (existing entity from tars-core):
- Contains original file contents
- SHA256 hashes for verification
- Enables byte-for-byte rollback

---

## Index/Lookup Patterns

### By Name + Type + Scope
Primary lookup: Find a specific config item.
```
(name, item_type, scope) → ConfigItem
```

### By Type + Scope
List all items of a type in a scope.
```
(item_type, scope) → Vec<ConfigItem>
```

### By Name + Type (Cross-Scope)
Find item across all scopes (for conflict detection).
```
(name, item_type) → Vec<(ConfigScope, ConfigItem)>
```

---

## Validation Summary

| Entity | Validation |
|--------|------------|
| ConfigItem.name | Non-empty, no separators, no `..`, no null bytes |
| ConfigScope | Must be writable for Add/Update/Remove/Move |
| McpServerConfig | Either (command + stdio) or (url + http/sse) |
| SkillConfig | description non-empty, body non-empty |
| CommandConfig | body non-empty |
| AgentConfig | description non-empty, permission_mode valid |
| OperationPlan | No conflicts (or conflicts resolved) |
