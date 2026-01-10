# TARS - Tooling, Agents, Roles, Skills

<p align="center">
  <img src="apps/tars-desktop/src-tauri/icons/tars-base.png" alt="TARS Logo" width="128" height="128">
</p>

<p align="center">
  <strong>A macOS desktop application for managing Claude Code configuration across projects</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#development">Development</a>
</p>

---

## Overview

TARS is a centralized hub for discovering, creating, editing, and managing Claude Code resources. It provides a visual interface for managing skills, agents, commands, hooks, MCP servers, and plugins across multiple projects with safe apply/rollback operations and profile-based configuration sharing.

Inspired by the robot from Interstellar, TARS brings order to your Claude Code configuration chaos.

## Features

### Project Management
- **Multi-project tracking** - Add and manage multiple projects from a single interface
- **Configuration scanning** - Automatically discover skills, commands, agents, hooks, and MCP servers
- **Collision detection** - Identify configuration conflicts across scopes
- **Context stats** - View token and character usage for project context

### Skills Management
- **Browse & search** - Filter skills by scope (User, Project, Plugin, Managed)
- **Create & edit** - Full markdown editor with YAML frontmatter support
- **Scope management** - Move skills between user and project scopes
- **Syntax highlighting** - Code blocks with language-specific highlighting

### Agents Management
- **Agent definitions** - Create and edit agent configurations
- **Enable/disable** - Toggle agents on/off without deleting
- **Scope control** - Move agents between configuration scopes
- **Markdown editing** - Rich editor for agent prompts and instructions

### Commands Management
- **Slash commands** - Create custom `/command-name` definitions
- **Template support** - Use `$ARGUMENTS` placeholder for dynamic input
- **Scope hierarchy** - Manage commands at user or project level

### Hooks Configuration
- **Event-driven hooks** - Configure actions for Claude Code events
- **Event types supported**:
  - `PreToolUse` / `PostToolUse` - Before/after tool execution
  - `Stop` - When Claude stops
  - `SessionStart` / `SessionEnd` - Session lifecycle
  - `UserPromptSubmit` - User input handling
  - `PreCompact` - Before context compaction
  - `Notification` - System notifications
  - `SubagentStop` - Subagent completion
- **Hook types** - Command execution or prompt injection
- **Matcher patterns** - Filter which tools/events trigger hooks

### MCP Servers
- **Server management** - Add, remove, and configure MCP servers
- **Transport types** - Support for stdio, HTTP, and SSE
- **Scope configuration** - User-level or project-specific servers
- **Environment variables** - Configure server environment

### Plugin Management
- **Marketplace support** - Add GitHub, URL, or local marketplace sources
- **Install/uninstall** - Manage plugin lifecycle
- **Scope control** - Install at user, project, or local scope
- **Enable/disable** - Toggle plugins without uninstalling
- **Auto-update** - Configure automatic marketplace updates
- **Cache management** - View and clean plugin cache

### Profiles
- **Snapshot projects** - Save complete project configuration as a profile
- **Apply profiles** - Apply saved configurations to other projects
- **Diff preview** - Review changes before applying
- **Rollback support** - Restore from automatic backups
- **Plugin export** - Convert profiles to shareable Claude Code plugin format

### Knowledge Center (CASE)
- **Documentation** - Built-in reference for all Claude Code features
- **Searchable** - Quick access to skills, agents, commands, hooks, MCP, and plugin docs
- **External links** - Direct links to official documentation

### Prompts Library
- **Personal storage** - Save prompts and notes (not loaded by Claude)
- **Rich editing** - MDXEditor with full markdown support
- **Code blocks** - Syntax highlighting for 14+ languages
- **Separate storage** - Stored in `~/.tars/prompts/`, independent of Claude config

### UI Features
- **Theme support** - System, light, and dark modes
- **Collapsible sidebar** - Maximize workspace when needed
- **TARS design system** - Brushed metal aesthetic inspired by Interstellar

## Installation

### Prerequisites

- **macOS** (Apple Silicon or Intel)
- **Rust** 1.75+ with Cargo
- **Bun** (or npm/yarn/pnpm)
- **Claude Code** CLI installed

### Build from Source

```bash
# Clone the repository
git clone https://github.com/jasongoodwin/tars.git
cd tars

# Install frontend dependencies
cd apps/tars-desktop
bun install

# Build the application
bun run tauri build

# The built app will be at:
# src-tauri/target/release/bundle/macos/TARS - Tooling, Agents, Roles, Skills.app
```

### Development Mode

```bash
cd apps/tars-desktop
bun install
bun run tauri dev
```

## Usage

### Adding Projects

1. Click **"Add Project"** in the Projects page
2. Select or enter a project directory path
3. TARS will scan for Claude Code configuration files
4. View discovered skills, commands, agents, and hooks in the inventory panel

### Creating Skills

1. Navigate to **Skills** in the sidebar
2. Click **"New Skill"**
3. Choose scope (User or Project)
4. Edit the skill template with your prompt and configuration
5. Save with `Cmd+S` or click Save

### Managing Profiles

1. Go to **Profiles** page
2. Click **"New Profile"** to snapshot a project
3. Select source project and provide a name/description
4. To apply: select profile, choose target, preview diff, apply
5. Use rollback if needed to restore previous state

### Configuring Hooks

1. Open **Hooks** page
2. Choose User or Project scope
3. Add hooks for desired events (PreToolUse, Stop, etc.)
4. Configure:
   - **Type**: `command` (shell) or `prompt` (inject text)
   - **Matcher**: Tool/event pattern to match
   - **Timeout**: Max execution time (ms)
5. Save configuration

### Installing Plugins

1. Navigate to **Plugins** page
2. Add marketplace source (if needed):
   - GitHub: `github:owner/repo`
   - URL: `https://example.com/marketplace.json`
   - Local: `/path/to/local/marketplace`
3. Browse available plugins
4. Click **Install** and select scope
5. Enable/disable plugins as needed

## Architecture

```
tars/
├── apps/
│   └── tars-desktop/           # Tauri desktop application
│       ├── src/                # React frontend
│       │   ├── pages/          # Route pages
│       │   ├── components/     # UI components
│       │   ├── lib/            # IPC & utilities
│       │   └── stores/         # Zustand state
│       └── src-tauri/          # Rust backend
│           └── src/commands/   # Tauri commands
│
├── crates/
│   ├── tars-scanner/           # Configuration discovery
│   ├── tars-core/              # Profile engine & operations
│   └── tars-cli/               # CLI wrapper
│
└── Cargo.toml                  # Rust workspace
```

### Configuration Scopes

TARS respects Claude Code's scope hierarchy (highest to lowest precedence):

1. **Managed** - `/Library/Application Support/ClaudeCode/managed-*.json`
2. **Local** - `<repo>/.claude/settings.local.json`
3. **Project** - `<repo>/.claude/settings.json`, `<repo>/.mcp.json`
4. **User** - `~/.claude/settings.json`, `~/.claude.json`

### Tech Stack

**Frontend:**
- React 19 + TypeScript 5
- Vite 7
- TanStack Query (server state)
- Zustand (UI state)
- Tailwind CSS + shadcn/ui
- MDXEditor (rich markdown)
- Lucide icons

**Backend:**
- Rust + Tauri 2
- SQLite (embedded database)
- Tokio (async runtime)
- Serde (serialization)
- Gray Matter (frontmatter parsing)

## Development

### Build Commands

```bash
# Rust crates
cargo build                    # Build all crates
cargo test                     # Run tests
cargo run -p tars-cli -- scan  # Run scanner CLI

# Frontend
cd apps/tars-desktop
bun run dev                    # Vite dev server only
bun run build                  # Build frontend
bun run tauri dev              # Full Tauri dev mode
bun run tauri build            # Production build
```

### Project Structure

| Directory | Purpose |
|-----------|---------|
| `apps/tars-desktop/src/pages/` | React route pages |
| `apps/tars-desktop/src/components/` | Reusable UI components |
| `apps/tars-desktop/src/lib/` | IPC wrapper, types, utilities |
| `apps/tars-desktop/src-tauri/src/commands/` | Tauri command handlers |
| `crates/tars-scanner/` | Non-destructive config discovery |
| `crates/tars-core/` | Profile apply, diff, rollback engine |

### File Formats

**Skills/Commands/Agents** use markdown with YAML frontmatter:

```markdown
---
name: my-skill
description: A helpful skill
allowed-tools:
  - Read
  - Write
model: sonnet
---

Your skill prompt here with $ARGUMENTS placeholder.
```

**Settings** use JSON:
- `settings.json` - Permissions, hooks, enabled plugins
- `.mcp.json` - MCP server configurations

## Roadmap

- [ ] Windows and Linux support
- [ ] Profile sharing via Claude Code plugin format
- [ ] Diff visualization improvements
- [ ] Bulk operations
- [ ] Import/export configurations

## Contributing

Contributions are welcome! Please read the contributing guidelines before submitting PRs.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <sub>Built with Tauri, React, and Rust</sub>
</p>
