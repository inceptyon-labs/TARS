# Tauri Commands Contract: Config Operations

**Feature**: 002-config-operations | **Date**: 2026-01-09

## Overview

Tauri commands that expose `tars-core::config` operations to the React frontend. All commands are async and return JSON-serializable results.

---

## MCP Server Commands

### `mcp_list`

List all MCP servers across scopes.

**Parameters**: None (or optional scope filter)

**Returns**:
```typescript
interface McpListResult {
  servers: McpServerItem[];
}

interface McpServerItem {
  name: string;
  scope: "user" | "project" | "local";
  transport: "stdio" | "http" | "sse";
  command?: string;
  args: string[];
  env: Record<string, string>;
  url?: string;
  filePath: string;  // Where this config lives
}
```

**Frontend Usage**:
```typescript
const result = await invoke<McpListResult>("mcp_list");
```

---

### `mcp_add`

Add a new MCP server.

**Parameters**:
```typescript
interface McpAddParams {
  name: string;
  scope: "user" | "project";
  transport: "stdio" | "http" | "sse";
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  dryRun?: boolean;
}
```

**Returns**:
```typescript
interface McpAddResult {
  success: boolean;
  backupId?: string;
  filePath: string;
  diff?: string;  // For dry-run preview
  error?: string;
}
```

---

### `mcp_remove`

Remove an MCP server.

**Parameters**:
```typescript
interface McpRemoveParams {
  name: string;
  scope?: "user" | "project";  // Auto-detect if not specified
  dryRun?: boolean;
}
```

**Returns**:
```typescript
interface McpRemoveResult {
  success: boolean;
  backupId?: string;
  filePath: string;
  diff?: string;
  error?: string;
}
```

---

### `mcp_update`

Update an existing MCP server.

**Parameters**:
```typescript
interface McpUpdateParams {
  name: string;
  scope?: "user" | "project";
  updates: Partial<{
    command: string;
    args: string[];
    env: Record<string, string>;
    url: string;
  }>;
  dryRun?: boolean;
}
```

**Returns**: Same as `mcp_add`

---

### `mcp_move`

Move an MCP server between scopes.

**Parameters**:
```typescript
interface McpMoveParams {
  name: string;
  fromScope?: "user" | "project";  // Auto-detect if not specified
  toScope: "user" | "project";
  force?: boolean;  // Overwrite if exists in target
  dryRun?: boolean;
}
```

**Returns**:
```typescript
interface McpMoveResult {
  success: boolean;
  backupId?: string;
  removedFrom: string;  // File path
  addedTo: string;      // File path
  diff?: string;
  error?: string;
  conflict?: {
    message: string;
    existingServer: McpServerItem;
  };
}
```

---

## Skill Commands

### `skill_list`

**Returns**:
```typescript
interface SkillListResult {
  skills: SkillItem[];
}

interface SkillItem {
  name: string;
  scope: "user" | "project";
  description: string;
  userInvocable: boolean;
  allowedTools: string[];
  model?: string;
  filePath: string;
}
```

---

### `skill_add`

**Parameters**:
```typescript
interface SkillAddParams {
  name: string;
  scope: "user" | "project";
  description: string;
  body: string;
  userInvocable?: boolean;
  allowedTools?: string[];
  model?: string;
  dryRun?: boolean;
}
```

---

### `skill_remove`, `skill_move`

Similar pattern to MCP commands.

---

## Hook Commands

### `hook_list`

**Returns**:
```typescript
interface HookListResult {
  hooks: HookItem[];
}

interface HookItem {
  trigger: HookTrigger;
  scope: "user" | "project" | "local";
  index: number;  // Position in hooks array for that trigger
  matcher?: string;
  definition: HookDefinition;
  filePath: string;
}

type HookTrigger =
  | "PreToolUse" | "PostToolUse" | "PermissionRequest"
  | "UserPromptSubmit" | "SessionStart" | "SessionEnd"
  | "Notification" | "Stop" | "SubagentStop" | "PreCompact";

type HookDefinition =
  | { type: "command"; command: string }
  | { type: "prompt"; prompt: string }
  | { type: "agent"; agent: string };
```

---

### `hook_add`

**Parameters**:
```typescript
interface HookAddParams {
  trigger: HookTrigger;
  scope: "user" | "project" | "local";
  matcher?: string;
  definition: HookDefinition;
  dryRun?: boolean;
}
```

---

### `hook_remove`

**Parameters**:
```typescript
interface HookRemoveParams {
  trigger: HookTrigger;
  scope: "user" | "project" | "local";
  index: number;  // Which hook to remove
  dryRun?: boolean;
}
```

---

## Command/Agent Commands

### `command_list`, `command_add`, `command_remove`, `command_move`

Similar pattern to skills.

### `agent_list`, `agent_add`, `agent_remove`, `agent_move`

Similar pattern to skills.

---

## Common Operations

### `config_rollback`

Rollback a previous operation.

**Parameters**:
```typescript
interface RollbackParams {
  backupId: string;
  projectPath?: string;
}
```

**Returns**:
```typescript
interface RollbackResult {
  success: boolean;
  filesRestored: string[];
  error?: string;
}
```

---

### `config_preview`

Get a preview of changes without applying.

Same as any operation with `dryRun: true`.

---

## Error Handling

All commands can return errors:

```typescript
interface CommandError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

// Error codes
type ErrorCode =
  | "ITEM_EXISTS"       // Already exists (for add)
  | "ITEM_NOT_FOUND"    // Doesn't exist (for remove/update)
  | "INVALID_SCOPE"     // Bad scope value
  | "VALIDATION_ERROR"  // Invalid item config
  | "PERMISSION_DENIED" // Can't write to managed scope
  | "CONFLICT"          // Item exists in target scope (for move)
  | "IO_ERROR"          // File system error
  | "PARSE_ERROR";      // JSON/YAML parse error
```

---

## Frontend Patterns

### Optimistic Updates
```typescript
// Show loading state
setLoading(true);

// Call Tauri command
const result = await invoke<McpAddResult>("mcp_add", { params });

if (result.success) {
  // Refresh list from source of truth
  const list = await invoke<McpListResult>("mcp_list");
  setServers(list.servers);
} else {
  // Show error
  showError(result.error);
}
```

### Dry-Run Preview
```typescript
// Get preview first
const preview = await invoke<McpAddResult>("mcp_add", {
  ...params,
  dryRun: true
});

// Show diff to user
showDiffDialog(preview.diff);

// If confirmed, apply for real
if (userConfirmed) {
  await invoke<McpAddResult>("mcp_add", params);
}
```

### Scope Selection
```typescript
// Component shows scope picker before destructive operations
<ScopeSelector
  value={scope}
  onChange={setScope}
  options={["user", "project", "local"]}
/>
```
