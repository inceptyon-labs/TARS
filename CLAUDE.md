# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TARS (Tooling, Agents, Roles, Skills) is a macOS desktop app for managing Claude Code configuration across projects. It handles plugins, skills, commands, agents, hooks, MCP servers, and reusable profiles.

**Status**: Greenfield project - specs complete, implementation not started.

## Build Commands

Once implementation begins:

```bash
# Rust crates
cargo build                           # Build all crates
cargo test                            # Run all tests
cargo run -p tars-cli -- scan         # Run scanner CLI

# Tauri app (after Task 3)
cd apps/tars-desktop
bun install
bun run tauri dev                     # Development mode
bun run tauri build                   # Production build
```

## Architecture

### Planned Structure

```
apps/tars-desktop/     # Tauri app (React + TypeScript + Vite)
crates/tars-scanner/   # Discovery - reads Claude Code config from all scopes
crates/tars-cli/       # CLI wrapper for scanner
crates/tars-core/      # Profiles, diff, apply, rollback engine
```

### Key Rust Modules

- `scanner`: Non-destructive discovery of skills, commands, agents, hooks, MCP, plugins
- `parser`: Frontmatter parsing for SKILL.md, agent definitions, commands
- `profiles`: Snapshot/export/import/apply operations
- `cli_bridge`: Wraps `claude plugin` and `claude mcp` CLI commands
- `rollback`: Backups, checksums, byte-for-byte restore

### Configuration Scopes (precedence high→low)

1. **Managed**: `/Library/Application Support/ClaudeCode/managed-*.json`
2. **Local**: `<repo>/.claude/settings.local.json`
3. **Project**: `<repo>/.claude/settings.json`, `<repo>/.mcp.json`
4. **User**: `~/.claude/settings.json`, `~/.claude.json`

## Constitution Principles

See `.specify/memory/constitution.md` for full details. Key rules:

1. **Discovery-First**: Always scan before modifying
2. **Safe-by-Default**: Diff preview, backups, rollback for all changes
3. **Plugin-First**: Export as Claude Code plugin format
4. **Profile Determinism**: Apply+rollback = byte-for-byte original
5. **Current Docs First**: Match Claude Code CLI and file format specs
6. Use Bun for JS tooling.
7. Use Context7 plugin for current docs and dependency guidance and latest versions
8. If anything is unclear, ask via AskUserQuestionTool before proceeding.
9. Use the skills available to you for specialized tasks.
10. Use the code review subagent after each phase to review the code.

## Implementation Order

Follow these tasks sequentially (from spec):

1. **Scanner CLI** - Read-only inventory, JSON + MD output, collision detection
2. **Profile Engine** - Snapshot, apply with diff preview, deterministic rollback
3. **Tauri App** - Projects list, inventory view, profile management, skills editor
4. **Plugin Export** - Convert profiles to `.claude-plugin/` format

## Claude Code File Formats

The scanner must parse:

- **SKILL.md**: Frontmatter with `name`, `description`, optional `allowed-tools`, `model`, `hooks`
- **Agent definitions**: Frontmatter with `name`, `description`, optional `tools`, `model`, `skills`
- **Commands**: Frontmatter with optional `description`, `thinking`; body uses `$ARGUMENTS`
- **plugin.json**: Manifest in `.claude-plugin/` directories
- **settings.json**: Hooks, permissions, enabled plugins
- **.mcp.json**: MCP server configurations (stdio/http/sse types)

## Speckit Commands

This project uses the Speckit workflow. Available commands:

- `/speckit.specify` - Create/update feature specification
- `/speckit.plan` - Generate implementation plan from spec
- `/speckit.tasks` - Generate task list from plan
- `/speckit.implement` - Execute tasks from tasks.md
- `/speckit.constitution` - Update project constitution

## Active Technologies
- Rust 1.75+ (backend/CLI), TypeScript 5.x (frontend) + Tauri 2.x, React 18, Vite 5, shadcn/ui, Tailwind CSS (001-tars)
- SQLite (embedded via rusqlite), file-based backups (001-tars)

## Recent Changes
- 001-tars: Added Rust 1.75+ (backend/CLI), TypeScript 5.x (frontend) + Tauri 2.x, React 18, Vite 5, shadcn/ui, Tailwind CSS
