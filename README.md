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
- **Real-time YAML validation** - Inline error highlighting for frontmatter syntax issues
- **Supporting files** - View and manage progressive disclosure files (reference.md, examples.md, scripts/) alongside skills
- **Scope management** - Move skills between user and project scopes
- **Syntax highlighting** - Code blocks with language-specific highlighting

### Agents Management
- **Agent definitions** - Create and edit agent configurations
- **Enable/disable** - Toggle agents on/off without deleting
- **Scope control** - Move agents between configuration scopes
- **Markdown editing** - Rich editor for agent prompts and instructions
- **Real-time YAML validation** - Inline error highlighting for frontmatter syntax issues

### Commands Management
- **Slash commands** - Create custom `/command-name` definitions
- **Template support** - Use `$ARGUMENTS` placeholder for dynamic input
- **Scope hierarchy** - Manage commands at user or project level
- **Real-time YAML validation** - Inline error highlighting for frontmatter syntax issues

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
Profiles let you create reusable configuration bundles that can be shared across multiple projects. Think of them as "presets" for your Claude Code setup.

**Creation Wizard**
- **Guided setup** - Step-by-step wizard for creating profiles
- **Multiple source options**:
  - **Single project** - Import tools from one specific project
  - **Registered projects** - Pick tools from projects already added to TARS
  - **Development folder** - Scan an entire folder for all Claude-configured projects
  - **Start empty** - Create a blank profile and add tools later
- **Tool discovery** - Automatically finds MCP servers, skills, and agents from project `.claude/` directories and `.mcp.json` files

**Tool Management**
- **Visual tool picker** - Browse and select tools with descriptions and source info
- **Categorized view** - Filter by MCP servers, skills, or agents
- **Bulk selection** - Select all or clear selections quickly
- **Search** - Find specific tools across all discovered projects

**Profile Assignment**
- **Project binding** - Assign profiles to one or more projects
- **Auto-sync** - Profile changes automatically propagate to assigned projects
- **Local overrides** - Projects can have local tools that supplement the profile
- **Unassign** - Remove profile from a project while keeping local configurations

**Import/Export**
- **Portable format** - Export profiles as `.tars-profile.json` files
- **Share configurations** - Import profiles from teammates or community
- **Collision handling** - Detect and resolve name conflicts on import
- **Plugin export** - Convert profiles to Claude Code plugin format for distribution

**Safety Features**
- **Diff preview** - Review all changes before applying to a project
- **Automatic backups** - Every apply creates a backup for easy rollback
- **Deterministic rollback** - Restore exact previous state byte-for-byte

### Knowledge Center (CASE)
- **Documentation** - Built-in reference for all Claude Code features
- **Searchable** - Quick access to skills, agents, commands, hooks, MCP, and plugin docs
- **External links** - Direct links to official documentation

### Prompts Library
- **Personal storage** - Save prompts and notes (not loaded by Claude)
- **Rich editing** - MDXEditor with full markdown support
- **Code blocks** - Syntax highlighting for 14+ languages
- **Separate storage** - Stored in `~/.tars/prompts/`, independent of Claude config

### Updates
- **Claude Code updates** - Compare installed vs latest version with update notifications
- **Plugin updates** - Detect available updates for marketplace plugins with version comparison
- **Changelog viewer** - Browse Claude Code release notes with version highlights
- **Automatic polling** - Checks for updates on startup and every 10 minutes
- **Sidebar badge** - Visual indicator showing total update count

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

**Creating a Profile:**
1. Go to **Profiles** page and click **"Create Profile"**
2. Enter a name and optional description
3. Choose where to discover tools:
   - **Single project** - Select a specific project folder
   - **Registered projects** - Pick from projects already in TARS
   - **Development folder** - Scan a parent folder (e.g., `~/Development`)
   - **Empty** - Start blank and add tools later
4. Select the MCP servers, skills, and agents you want in the profile
5. Click **"Create Profile"** to save

**Assigning to Projects:**
1. Select a profile from the list
2. In the detail panel, click **"Assign to Project"**
3. Choose which registered project should use this profile
4. The profile's tools will be available in that project

**Sharing Profiles:**
1. Select the profile to export
2. Click **"Export"** and choose a save location
3. Share the `.tars-profile.json` file with teammates
4. They can import via **"Import Profile"** button

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
- [x] Profile sharing via Claude Code plugin format
- [ ] Diff visualization improvements
- [ ] Bulk operations
- [x] Import/export configurations
- [ ] Usage analytics dashboard
- [ ] Profile templates gallery

## Contributing

Contributions are welcome! Please read the contributing guidelines before submitting PRs.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <sub>Built with Tauri, React, and Rust</sub>
</p>
