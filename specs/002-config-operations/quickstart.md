# Quickstart: Config Operations Layer

**Feature**: 002-config-operations | **Date**: 2026-01-09

## Prerequisites

- TARS CLI installed (`cargo install --path crates/tars-cli`)
- Claude Code installed and configured
- A project directory with `.claude/` or `.mcp.json`

## Basic Usage

### 1. Add an MCP Server

```bash
# Add to current project
tars mcp add context7 --command "npx" --args "-y @context7/mcp"

# Preview changes first
tars mcp add context7 --command "npx" --args "-y @context7/mcp" --dry-run
```

### 2. List MCP Servers

```bash
# See all servers across scopes
tars mcp list

# JSON output for scripting
tars mcp list --json
```

### 3. Remove an MCP Server

```bash
# Remove from project
tars mcp remove context7

# Remove from user (global) scope
tars mcp remove context7 --scope user
```

### 4. Move Between Scopes

```bash
# Make a project server global
tars mcp move context7 --to user

# Move global server to project only
tars mcp move context7 --from user --to project
```

### 5. Update Configuration

```bash
# Add an environment variable
tars mcp update neon --add-env "DEBUG=true"

# Change command path
tars mcp update context7 --command "/usr/local/bin/npx"
```

## Skills Management

```bash
# Add skill from file
tars skill add code-review --from-file ./skills/code-review/SKILL.md

# Add inline skill
tars skill add quick-fix \
  --description "Quick code fixes" \
  --user-invocable \
  --body "Fix the bug described by the user"

# Move skill to global
tars skill move code-review --to user

# List all skills
tars skill list
```

## Hooks Management

```bash
# Add a pre-tool hook
tars hook add --trigger PreToolUse --matcher "Bash" --command "./lint.sh"

# Add a prompt injection hook
tars hook add --trigger UserPromptSubmit --prompt "Think step by step"

# List hooks
tars hook list

# Remove a hook
tars hook remove --trigger PreToolUse --index 0
```

## Commands and Agents

```bash
# Add custom command
tars command add review \
  --description "Review code changes" \
  --body "Review the staged changes and suggest improvements"

# Add custom agent
tars agent add security-reviewer \
  --description "Security-focused code reviewer" \
  --tools Read Grep \
  --body "Review code for security vulnerabilities"

# List all
tars command list
tars agent list
```

## Safety Features

### Dry Run
Preview any change before applying:
```bash
tars mcp add context7 --command "npx" --args "-y @context7/mcp" --dry-run
```

Output shows exactly what will change:
```
DRY RUN - No changes made

Would modify: .mcp.json
+ "context7": {
+   "type": "stdio",
+   "command": "npx",
+   "args": ["-y", "@context7/mcp"]
+ }
```

### Backups & Rollback
Every operation creates a backup:
```bash
# List recent backups
tars profile backups

# Rollback last operation
tars profile rollback <backup-id> .
```

### Conflict Detection
```bash
$ tars mcp add context7 --command "npx" --args "-y @context7/mcp"
Error: MCP server 'context7' already exists in project scope.
Hint: Use 'tars mcp update context7' to modify, or add --scope user for global.
```

## JSON Output for Scripting

All commands support `--json` for machine-readable output:

```bash
tars mcp list --json | jq '.items[] | select(.scope == "project") | .name'
```

## Common Workflows

### Share a server globally
```bash
# Move from project to user scope
tars mcp move my-server --to user
```

### Copy a skill to multiple projects
```bash
# Export skill, then add to each project
tars skill add shared-skill --from-file ~/.claude/skills/shared-skill/SKILL.md
```

### Clean up unused servers
```bash
# List all servers
tars mcp list

# Remove unused ones
tars mcp remove old-server
tars mcp remove deprecated-api
```

### Add development-only hooks
```bash
# Add to local settings (not committed)
tars hook add --scope local --trigger SessionStart --command "echo 'Dev mode'"
```

## Next Steps

- Run `tars --help` for full command reference
- Check `tars mcp --help` for MCP-specific options
- Use `--verbose` for detailed operation logs
