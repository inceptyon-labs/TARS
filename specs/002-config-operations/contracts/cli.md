# CLI Contract: Config Operations

**Feature**: 002-config-operations | **Date**: 2026-01-09

## Overview

New CLI subcommands for granular config operations. All commands support:
- `--dry-run`: Preview changes without applying
- `--json`: Output in JSON format
- `--scope`: Specify target scope (user, project, local)

---

## MCP Server Commands

### `tars mcp add <name>`

Add an MCP server to configuration.

**Arguments**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| name | string | Yes | Server name (must be unique in scope) |

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | project | user, project |
| --type | enum | stdio | stdio, http, sse |
| --command | string | - | Command for stdio transport |
| --args | string[] | [] | Command arguments |
| --env | string[] | [] | Environment vars (KEY=value) |
| --url | string | - | URL for http/sse transport |
| --dry-run | bool | false | Preview only |
| --json | bool | false | JSON output |

**Examples**:
```bash
# Add stdio MCP server
tars mcp add context7 --command "npx" --args "-y" --args "@context7/mcp"

# Add to user scope
tars mcp add context7 --scope user --command "npx" --args "-y @context7/mcp"

# Add HTTP server
tars mcp add remote-api --type http --url "https://api.example.com/mcp"

# Add with environment variables
tars mcp add neon --command "npx" --args "@neondatabase/mcp-server" \
  --env "NEON_API_KEY=$NEON_API_KEY"
```

**Output (success)**:
```
Added MCP server 'context7' to project scope.
File: .mcp.json
```

**Output (JSON)**:
```json
{
  "success": true,
  "action": "add",
  "item_type": "mcp_server",
  "name": "context7",
  "scope": "project",
  "file": ".mcp.json",
  "backup_id": "abc123"
}
```

**Errors**:
- `E001`: Server already exists in scope (suggest --update)
- `E002`: Missing required option (--command for stdio, --url for http)
- `E003`: Invalid scope

---

### `tars mcp remove <name>`

Remove an MCP server from configuration.

**Arguments**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| name | string | Yes | Server name to remove |

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | auto | user, project, or auto-detect |
| --dry-run | bool | false | Preview only |
| --json | bool | false | JSON output |

**Examples**:
```bash
# Remove from project (auto-detected)
tars mcp remove context7

# Remove from specific scope
tars mcp remove context7 --scope user
```

**Errors**:
- `E010`: Server not found in scope
- `E011`: Server exists in multiple scopes (specify --scope)

---

### `tars mcp update <name>`

Update an existing MCP server configuration.

**Arguments**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| name | string | Yes | Server name to update |

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | auto | user, project, or auto-detect |
| --command | string | - | New command |
| --args | string[] | - | Replace arguments |
| --add-arg | string[] | - | Add to arguments |
| --env | string[] | - | Replace env vars |
| --add-env | string[] | - | Add env vars |
| --remove-env | string[] | - | Remove env vars |
| --url | string | - | New URL |
| --dry-run | bool | false | Preview only |
| --json | bool | false | JSON output |

**Examples**:
```bash
# Update command
tars mcp update context7 --command "/usr/local/bin/context7"

# Add environment variable
tars mcp update neon --add-env "DEBUG=true"

# Replace all args
tars mcp update context7 --args "-y" --args "@context7/mcp@latest"
```

---

### `tars mcp move <name>`

Move an MCP server between scopes.

**Arguments**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| name | string | Yes | Server name to move |

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --from | enum | auto | Source scope |
| --to | enum | required | Target scope |
| --force | bool | false | Overwrite if exists in target |
| --dry-run | bool | false | Preview only |
| --json | bool | false | JSON output |

**Examples**:
```bash
# Move from project to user (global)
tars mcp move context7 --to user

# Move from user to project
tars mcp move context7 --from user --to project
```

**Errors**:
- `E020`: Server not found in source scope
- `E021`: Server already exists in target scope (use --force)

---

### `tars mcp list`

List all MCP servers across scopes.

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | all | Filter by scope |
| --json | bool | false | JSON output |

**Output**:
```
MCP Servers:

Project (.mcp.json):
  context7     stdio  npx -y @context7/mcp
  neon         stdio  npx @neondatabase/mcp-server

User (~/.claude.json):
  perplexity   stdio  npx @anthropic/perplexity-mcp
```

---

## Skill Commands

### `tars skill add <name>`

Add a skill from a SKILL.md file or inline definition.

**Arguments**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| name | string | Yes | Skill name |

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | project | user, project |
| --from-file | path | - | Import from existing SKILL.md |
| --description | string | - | Skill description |
| --user-invocable | bool | false | Allow direct invocation |
| --allowed-tools | string[] | [] | Tool whitelist |
| --model | string | - | Preferred model |
| --body | string | - | Skill instructions (or stdin) |
| --dry-run | bool | false | Preview only |
| --json | bool | false | JSON output |

**Examples**:
```bash
# Add from file
tars skill add code-review --from-file ./my-skill/SKILL.md

# Add inline
tars skill add quick-fix \
  --description "Quick code fixes" \
  --user-invocable \
  --allowed-tools Read --allowed-tools Edit \
  --body "Fix the bug described by the user"

# Add from stdin
cat SKILL.md | tars skill add my-skill --from-stdin
```

---

### `tars skill remove <name>`

Remove a skill.

**Examples**:
```bash
tars skill remove code-review --scope project
```

---

### `tars skill move <name>`

Move a skill between scopes.

**Examples**:
```bash
tars skill move code-review --to user
```

---

### `tars skill list`

List all skills.

**Output**:
```
Skills:

Project (.claude/skills/):
  code-review     Review code for issues        user-invocable
  quick-fix       Quick code fixes              user-invocable

User (~/.claude/skills/):
  commit-helper   Generate commit messages      user-invocable
```

---

## Hook Commands

### `tars hook add`

Add a hook to settings.

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | project | user, project, local |
| --trigger | enum | required | PreToolUse, PostToolUse, etc. |
| --matcher | string | - | Tool/event pattern to match |
| --command | string | - | Shell command to run |
| --prompt | string | - | Prompt to inject |
| --agent | string | - | Agent to invoke |
| --dry-run | bool | false | Preview only |
| --json | bool | false | JSON output |

**Examples**:
```bash
# Add command hook
tars hook add --trigger PreToolUse --matcher "Bash" --command "./lint.sh"

# Add prompt hook
tars hook add --trigger UserPromptSubmit --prompt "Think step by step"

# Add to local settings
tars hook add --scope local --trigger SessionStart --command "echo 'Starting'"
```

---

### `tars hook remove`

Remove a hook.

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | required | Scope to remove from |
| --trigger | enum | required | Hook trigger type |
| --index | int | - | Index if multiple hooks of same trigger |
| --all | bool | false | Remove all hooks of this trigger |

**Examples**:
```bash
# Remove specific hook by index
tars hook remove --trigger PreToolUse --index 0

# Remove all PreToolUse hooks
tars hook remove --trigger PreToolUse --all
```

---

### `tars hook list`

List all hooks.

**Output**:
```
Hooks:

Project (.claude/settings.json):
  PreToolUse[0]     command: ./lint.sh          matcher: Bash
  PostToolUse[0]    prompt: Verify changes

Local (.claude/settings.local.json):
  SessionStart[0]   command: echo 'Starting'
```

---

## Command Management

### `tars command add <name>`

Add a custom command.

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | project | user, project |
| --from-file | path | - | Import from .md file |
| --description | string | - | Command description |
| --thinking | bool | false | Enable thinking mode |
| --body | string | - | Command template |
| --dry-run | bool | false | Preview only |

**Examples**:
```bash
tars command add review \
  --description "Review code changes" \
  --body "Review the staged changes and suggest improvements"
```

---

### `tars command remove <name>`

Remove a custom command.

---

### `tars command move <name>`

Move a command between scopes.

---

### `tars command list`

List all commands.

---

## Agent Management

### `tars agent add <name>`

Add a custom agent.

**Options**:
| Flag | Type | Default | Description |
|------|------|---------|-------------|
| --scope | enum | project | user, project |
| --from-file | path | - | Import from .md file |
| --description | string | required | Agent description |
| --tools | string[] | [] | Allowed tools |
| --model | string | - | Preferred model |
| --permission-mode | string | ask | Permission handling |
| --skills | string[] | [] | Available skills |
| --body | string | - | Agent instructions |
| --dry-run | bool | false | Preview only |

---

### `tars agent remove <name>`

Remove a custom agent.

---

### `tars agent move <name>`

Move an agent between scopes.

---

### `tars agent list`

List all agents.

---

## Common Options

All commands support these global options:

| Flag | Description |
|------|-------------|
| --dry-run | Preview changes without applying |
| --json | Output in JSON format |
| --verbose | Show detailed output |
| --quiet | Suppress non-error output |
| --project <path> | Specify project directory |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Item not found |
| 4 | Conflict (item already exists) |
| 5 | Permission denied (managed scope) |
| 6 | Validation error |
| 10 | Rollback triggered |

---

## JSON Output Schema

All JSON output follows this structure:

```json
{
  "success": true,
  "action": "add|remove|update|move|list",
  "item_type": "mcp_server|skill|hook|command|agent",
  "name": "item-name",
  "scope": "user|project|local",
  "file": "path/to/modified/file",
  "backup_id": "uuid-if-changes-made",
  "warnings": ["optional warning messages"],
  "error": "error message if success=false"
}
```

For list commands:
```json
{
  "success": true,
  "action": "list",
  "item_type": "mcp_server",
  "items": [
    {
      "name": "context7",
      "scope": "project",
      "config": { ... }
    }
  ]
}
```
