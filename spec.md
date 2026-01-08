# TARS — Tooling, Agents, Roles, Skills

Goal: A cross-platform desktop app that manages Claude Code configuration across projects, including plugins, skills, commands, agents, hooks, MCP servers, and reusable profiles assigned per project.

This spec is written to be used with Claude Code CLI as a build guide.

## Principles

- Plugin-first: prefer Claude Code plugin packaging for shareable bundles of skills/commands/agents/hooks/MCP.
- Safe-by-default: no silent execution of hooks; show diffs before writing files; backups + rollback.
- Profile = repeatable state: plugins + marketplaces + repo overlays + personal overlays.
- Discovery first: scan and inventory existing config before changing anything.
- Current docs first: use official Claude Code documentation and claude-code-guide for implementation guidance.

## Clarified Decisions

These decisions were clarified during spec review:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Target platforms | macOS only (MVP) | Start focused, add Windows/Linux post-MVP |
| UI framework | Tailwind + shadcn/ui | Modern, accessible component primitives |
| SQLite portability | Hybrid approach | Local DB with exportable "sync bundle" for machine migration |
| Auto-scan behavior | On app launch | Auto-scan registered projects when app opens |
| Collision handling | Report only (MVP) | Show conflicts and winner; no resolution UI for MVP |
| Marketplaces | Existing Claude Code feature | Use `claude plugin marketplace` CLI commands |

## Definitions

- Project: a folder, typically a git repo, with Claude-related assets.
- Profile: a named bundle that can be applied to a project (plugin sets, overlays, adapters).
- Overlay: repo-local or user-local files written by the app (`.claude/*`, `~/.claude/*`) in a controlled way.
- Adapter: maps logical config to actual files/locations and merge rules.

## What exists today (discovery scan)

You said “stuff everywhere.” Before building UI, implement a scanner that inventories:
- repo-level `.claude/` artifacts
- repo-level `CLAUDE.md`
- user-level `~/.claude/` artifacts
- installed plugins and marketplaces
- MCP config files and server definitions
- skills and commands folders in both scopes
- any plugin directories on disk

### Output of discovery
The scanner must produce:
1. `tars-inventory.json` (machine-readable)
2. `tars-inventory.md` (human-friendly report)
3. Optional: `tars-inventory.csv` (flat table export)

### Inventory schema (high level)
`tars-inventory.json` should include:

- `host`: os, username, home_dir, timestamp
- `user_scope`:
  - `settings`: { path, sha256, hooks_count, permissions, enabled_plugins }
  - `mcp`: { path, sha256, servers[] }
  - `skills[]`: { path, name, description, user_invocable, disable_model_invocation, sha256 }
  - `commands[]`: { path, name, description, sha256 }
  - `agents[]`: { path, name, description, tools[], sha256 }
- `managed_scope`:
  - `settings`: { path, sha256 } | null
  - `mcp`: { path, sha256, servers[] } | null
- `projects[]`:
  - `path`, `name`, `git`: { remote, branch, dirty }
  - `claude_md`: { path, sha256 } | null
  - `claude_dir`: path | null
  - `settings`:
    - `shared`: { path, sha256, hooks_count } | null
    - `local`: { path, sha256, hooks_count } | null
  - `mcp`: { path, sha256, servers[] } | null
  - `skills[]`: { path, name, description, user_invocable, disable_model_invocation, sha256 }
  - `commands[]`: { path, name, description, sha256 }
  - `agents[]`: { path, name, description, tools[], sha256 }
  - `hooks[]`: { source, trigger, matcher, type, command|prompt|agent }
- `plugins`:
  - `marketplaces[]`: { name, source_type, location, auto_update }
  - `installed[]`: { id, marketplace, version, scope, enabled, path }
- `collisions`:
  - `skills[]`: { name, winner_scope, occurrences[]: { scope, path } }
  - `commands[]`: same
  - `agents[]`: same

### Discovery rules
Scanner should look in these places:

User scope:
- `~/.claude/settings.json` — user settings with hooks, permissions, enabled plugins
- `~/.claude/skills/` — user-level skills
- `~/.claude/commands/` — user-level commands
- `~/.claude/agents/` — user-level agents (if present)
- `~/.claude.json` — user-level MCP server config (note: different from settings.json)
- Installed plugins via `claude plugin list` CLI output

Project scope:
- `<repo>/.claude/settings.json` — project settings
- `<repo>/.claude/settings.local.json` — local-only overrides (gitignored)
- `<repo>/.claude/skills/` — project skills
- `<repo>/.claude/commands/` — project commands
- `<repo>/.claude/agents/` — project agents
- `<repo>/.mcp.json` — project MCP server config (at repo root, not in .claude/)
- `<repo>/CLAUDE.md` — project instructions

Managed scope (macOS):
- `/Library/Application Support/ClaudeCode/managed-settings.json`
- `/Library/Application Support/ClaudeCode/managed-mcp.json`

Plugins:
- Parse plugin manifests and enumerate bundled skills/commands/agents/hooks/MCP
- Record plugin_id and the paths to each artifact

### Required parsing
- Skills: folder contains `SKILL.md` with frontmatter:
  - required: `name`, `description`
  - optional: `user-invocable`, `disable-model-invocation`, `allowed-tools`, `model`, `context`, `agent`, `hooks`
- Agents: `.md` files with frontmatter:
  - required: `name`, `description`
  - optional: `tools`, `model`, `permissionMode`, `skills`, `hooks`
- Commands: `.md` files with frontmatter:
  - optional: `description`, `thinking`
  - body uses `$ARGUMENTS`, `$1`, `$2`, etc. for parameter substitution
- Hooks: record trigger and command details wherever defined (settings.json files or embedded in skill/agent definitions).

### Non-destructive guarantee
Discovery must not modify files.
Any derived artifacts are written to a separate output directory chosen by the user.

## MVP App scope

### Must-have (MVP)
- Project discovery: choose directories, find repos, identify Claude artifacts
- Inventory view: show what exists in each scope and where it comes from
- Profile system:
  - Create profile from a project + user scope snapshot
  - Assign profile to a project
  - Apply profile with:
    - diff preview per file
    - atomic write
    - backup + rollback
- Skills manager:
  - view all skills across scopes
  - edit SKILL.md safely (frontmatter + body)
  - show collision resolution and precedence
- Plugin manager:
  - list marketplaces and installed plugins
  - install/enable/disable/uninstall via Claude Code CLI integration
  - generate plugin skeleton for exporting a profile

### Nice-to-have (post-MVP)
- GUI diff editor for merges
- “Apply and commit” (git commit changes)
- Cloud sync of profiles
- Plugin publishing workflow

## Profile design (recommended)

A profile contains:

1. `plugin_set`
- marketplaces to add
- plugins to install + scope + enabled state

2. `repo_overlays`
- `.claude/skills` (skill folders)
- `.claude/commands` (command files)
- `.claude/agents` (agent definitions)
- `CLAUDE.md` (template or patch rules)

3. `user_overlays`
- `~/.claude/skills` additions/updates
- `~/.claude/commands` additions/updates
- no secrets stored; only env var names

4. `adapters`
- where MCP files live and what format they are
- merge strategy per artifact type

Export format:
- `profile.json` + optional files in a zip.
- Never embed secrets.

## Safety model

- No silent hook execution from the app.
- If hooks are managed, the app generates an explicit wrapper script:
  - runs pre-hook
  - runs `claude` with args
  - runs post-hook
User chooses to run via wrapper.

- Always show a plan + diff preview before applying.
- Always make backups and store rollback bundles.
- Detect dirty git repos and warn.

## Claude Code Configuration Reference

This section documents the actual Claude Code file formats and CLI commands the scanner must understand.

### Settings Files

| Scope | Location | Purpose |
|-------|----------|---------|
| Managed | `/Library/Application Support/ClaudeCode/managed-settings.json` (macOS) | IT-deployed, highest precedence |
| User | `~/.claude/settings.json` | User-level defaults |
| Project | `.claude/settings.json` | Team-shared project config |
| Local | `.claude/settings.local.json` | Personal project overrides (gitignored) |

**Precedence (highest to lowest):** Managed → CLI args → Local → Project → User

**settings.json schema (key fields to parse):**
```json
{
  "env": { "VAR": "value" },
  "permissions": {
    "allow": ["Bash(npm:*)"],
    "deny": ["Read(.env)"],
    "defaultMode": "acceptEdits"
  },
  "hooks": { /* hook definitions */ },
  "model": "opus",
  "enabledPlugins": { "plugin@marketplace": true },
  "extraKnownMarketplaces": { /* marketplace definitions */ }
}
```

### Hooks Configuration

Hooks are defined in settings.json files (any scope) or embedded in skills/agents.

**Hook events to scan for:**
- `PreToolUse`, `PostToolUse` — tool matchers
- `PermissionRequest` — permission handling
- `UserPromptSubmit` — input validation
- `SessionStart`, `SessionEnd` — lifecycle
- `Notification`, `Stop`, `SubagentStop`, `PreCompact`

**Hook structure:**
```json
{
  "hooks": {
    "PostToolUse": [{
      "matcher": "Write|Edit",
      "hooks": [
        { "type": "command", "command": "npm run lint" },
        { "type": "prompt", "prompt": "Check for issues" },
        { "type": "agent", "agent": "agent-name" }
      ]
    }]
  }
}
```

### MCP Configuration

| Scope | File | Location |
|-------|------|----------|
| Project | `.mcp.json` | Project root |
| User | `~/.claude.json` | User home (note: different filename) |
| Managed | `managed-mcp.json` | System directory |

**.mcp.json schema:**
```json
{
  "mcpServers": {
    "server-name": {
      "type": "stdio|http|sse",
      "command": "/path/to/server",
      "args": ["--flag"],
      "env": { "KEY": "${ENV_VAR}" },
      "url": "https://..."
    }
  }
}
```

### Plugin Structure

Plugins live in `.claude-plugin/` directory with this structure:
```
my-plugin/
├── .claude-plugin/
│   └── plugin.json          # Required manifest
├── commands/                 # Default slash commands
├── agents/                   # Default subagents
├── skills/                   # Default skills
├── hooks/                    # Hook configs (optional)
├── .mcp.json                 # MCP servers (optional)
└── .lsp.json                 # LSP servers (optional)
```

**plugin.json schema:**
```json
{
  "name": "plugin-name",
  "version": "1.0.0",
  "description": "Brief description",
  "author": { "name": "Author", "email": "email@example.com" },
  "commands": ["./custom/path/"],
  "agents": "./custom/agents/",
  "skills": "./custom/skills/",
  "hooks": "./config/hooks.json",
  "mcpServers": "./mcp-config.json"
}
```

### Marketplace Structure

A marketplace is a catalog file listing available plugins:
```
my-marketplace/
└── .claude-plugin/
    └── marketplace.json
```

**marketplace.json schema:**
```json
{
  "name": "marketplace-name",
  "owner": { "name": "Team", "email": "team@example.com" },
  "metadata": { "pluginRoot": "./plugins" },
  "plugins": [
    {
      "name": "plugin-name",
      "source": "./plugins/plugin-name",
      "description": "...",
      "version": "1.0.0"
    }
  ]
}
```

### Plugin CLI Commands

Commands the CLI bridge should support:

```bash
# Plugin management
claude plugin install <plugin>[@marketplace] [--scope user|project|local]
claude plugin uninstall <plugin> [--scope ...]
claude plugin enable <plugin> [--scope ...]
claude plugin disable <plugin> [--scope ...]
claude plugin update <plugin> [--scope ...]
claude plugin list
claude plugin validate .

# Marketplace management
claude plugin marketplace add <source>    # owner/repo, URL, or local path
claude plugin marketplace list
claude plugin marketplace update <name>
claude plugin marketplace remove <name>

# MCP management
claude mcp add --transport <type> <name> <url|command>
claude mcp list
claude mcp get <name>
claude mcp remove <name>
```

### SKILL.md Format

```yaml
---
name: skill-name                    # Required, lowercase with hyphens
description: What this skill does   # Required, max 1024 chars

# Optional fields:
allowed-tools: Read, Grep, Glob
model: claude-sonnet-4-20250514
context: fork
agent: general-purpose
user-invocable: true
disable-model-invocation: false

hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./scripts/check.sh"
---

# Skill instructions in markdown
```

### Agent Definition Format

```yaml
---
name: agent-name                    # Required
description: When to use this agent # Required

# Optional fields:
tools: Read, Grep, Glob, Bash
model: sonnet
permissionMode: default
skills: skill1, skill2
hooks: { /* inline hooks */ }
---

# System prompt in markdown
```

### Command Format

```yaml
---
description: What this command does  # Optional
thinking: true                        # Optional, enables extended thinking
---

# Command instructions
Use $ARGUMENTS for all args, $1 $2 for positional.
```

## Technical approach

Suggested stack:
- Tauri (Rust backend, macOS only for MVP)
- React + TypeScript + Vite frontend
- Tailwind CSS + shadcn/ui for styling and components
- Bun for package management and running JS tooling
- SQLite embedded DB for app state (profiles, projects, history)
- OS keychain for optional secret storage (if ever needed)
- Sync bundle export for machine migration (JSON + files archive)

Rust modules:
- `scanner`: discovery implementation
- `parser`: frontmatter + basic formats
- `profiles`: snapshot/export/import/apply plan
- `diff`: file diffs
- `cli_bridge`: runs Claude Code CLI commands, captures output
- `storage`: sqlite + file bundles
- `rollback`: backups, checksums, restore

## Claude Code tasks

Implement in this order:

### Task 1: Discovery scanner CLI (no UI)
Create a standalone CLI in the repo that can:
- scan user scope + selected project folders
- output inventory JSON + MD report
- detect collisions and show the precedence winner

Acceptance criteria:
- running scan creates output files
- scan finds skills, commands, agents if present, and CLAUDE.md
- scan lists installed plugins and marketplaces (via Claude Code CLI if possible)
- scan is read-only

### Task 2: Profile snapshot + apply engine (no UI)
- Create profile from inventory snapshot
- Apply to a test project folder with:
  - plan generation
  - diff preview
  - backups
  - rollback

Acceptance criteria:
- applying and rolling back returns to original state byte-for-byte
- merge rules are deterministic

### Task 3: Minimal Tauri app
- Projects list + detail view
- Inventory display
- Profile create/assign/apply with diff preview
- Skills editor for SKILL.md

Acceptance criteria:
- app can scan, show results, apply profile safely

### Task 4: Plugin generator/export
- export a profile as a plugin skeleton:
  - `.claude-plugin/plugin.json`
  - bundled `skills/`, `commands/`, `agents/`, hooks, optional `.mcp.json`
- optionally zip it

Acceptance criteria:
- exported plugin installs cleanly and provides the skills/commands/agents as expected

## Decisions needed (make reasonable defaults)
If Claude Code supports multiple possible MCP config locations/formats, pick:
- project-local file: `<repo>/.claude/mcp.json` as default adapter output
- user-local file: `~/.claude/mcp.json` (optional)

If an artifact exists in multiple places, do not delete anything automatically.
Mark conflicts and require explicit user decision during apply.

## Deliverables
- `apps/tars-desktop/` (Tauri app)
- `crates/tars-scanner/` (scanner library)
- `crates/tars-cli/` (CLI wrapper)
- `crates/tars-core/` (profiles/apply/diff/rollback)
- `docs/` (this spec and user docs)
- `examples/` (example profile exports)

## First run experience
- User selects folders to register as projects
- App runs scanner immediately and shows an inventory report
- App suggests creating a "Baseline" profile from current state
- User can then create per-project profiles and apply safely

## Ongoing behavior
- On app launch, auto-scan all registered projects
- Show notification if any registered project has changed since last scan
- Badge projects with "dirty" git state or config drift

---

