# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TARS (Tooling, Agents, Roles, Skills) is a cross-platform desktop app for managing Claude Code configuration across projects. It handles plugins, skills, commands, agents, hooks, MCP servers, and reusable profiles.

**Platforms**: Windows, macOS, Linux

## Build Commands

```bash
# Rust crates
cargo build                           # Build all crates
cargo test                            # Run all tests
cargo run -p tars-cli -- scan         # Run scanner CLI

# Tauri app
cd apps/tars-desktop
bun install
bun run tauri dev                     # Development mode
bun run tauri build                   # Production build
```

## Linting

**IMPORTANT**: After making significant changes to Rust code, always run clippy before committing to catch issues that would fail CI:

```bash
cargo clippy --all -- -D warnings    # Run clippy on all crates (must pass with no warnings)
```

For TypeScript/frontend changes:

```bash
cd apps/tars-desktop
bun tsc --noEmit                     # Type-check frontend code
```

## Setup

After cloning, enable the pre-commit hooks:

```bash
git config core.hooksPath .githooks
```

This runs `cargo fmt` checks before each commit.

## Commit Message Convention

This project uses [Conventional Commits](https://www.conventionalcommits.org/). **All commits MUST follow this format:**

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description | In Changelog? |
|------|-------------|---------------|
| `feat` | New feature for users | Yes |
| `fix` | Bug fix for users | Yes |
| `perf` | Performance improvement | Yes |
| `docs` | Documentation only | No |
| `style` | Formatting, whitespace | No |
| `refactor` | Code restructuring (no behavior change) | No |
| `test` | Adding/updating tests | No |
| `chore` | Maintenance, deps, config | No |
| `ci` | CI/CD changes | No |
| `build` | Build system changes | No |

### Rules

1. **Type is required** - Must be one of the types above
2. **Description is required** - Imperative mood, lowercase, no period at end
3. **Scope is optional** - Use for clarity (e.g., `scanner`, `ui`, `cli`, `core`)
4. **Breaking changes** - Add `!` after type or `BREAKING CHANGE:` in footer
5. **Only `feat`, `fix`, `perf`** appear in the auto-generated changelog

## Architecture

### Structure

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
- `config`: MCP operations, config file management
- `storage`: SQLite database for projects and profiles

### Configuration Scopes (precedence high to low)

**macOS/Linux:**
1. **Managed**: `/Library/Application Support/ClaudeCode/managed-*.json` (macOS) or `/etc/claude-code/` (Linux)
2. **Local**: `<repo>/.claude/settings.local.json`
3. **Project**: `<repo>/.claude/settings.json`, `<repo>/.mcp.json`
4. **User**: `~/.claude/settings.json`, `~/.claude.json`

**Windows:**
1. **Managed**: `%ProgramData%\ClaudeCode\managed-*.json`
2. **Local**: `<repo>\.claude\settings.local.json`
3. **Project**: `<repo>\.claude\settings.json`, `<repo>\.mcp.json`
4. **User**: `%USERPROFILE%\.claude\settings.json`, `%USERPROFILE%\.claude.json`

## Key Principles

1. **Discovery-First**: Always scan before modifying
2. **Safe-by-Default**: Diff preview, backups, rollback for all changes
3. **Plugin-First**: Export as Claude Code plugin format
4. **Profile Determinism**: Apply+rollback = byte-for-byte original
5. **Cross-Platform**: All paths and operations work on Windows, macOS, and Linux
6. Use Bun for JS tooling
7. If anything is unclear, ask via AskUserQuestionTool before proceeding

## Claude Code File Formats

The scanner parses:

- **SKILL.md**: Frontmatter with `name`, `description`, optional `allowed-tools`, `model`, `hooks`
- **Agent definitions**: Frontmatter with `name`, `description`, optional `tools`, `model`, `skills`
- **Commands**: Frontmatter with optional `description`, `thinking`; body uses `$ARGUMENTS`
- **plugin.json**: Manifest in `.claude-plugin/` directories
- **settings.json**: Hooks, permissions, enabled plugins
- **.mcp.json**: MCP server configurations (stdio/http/sse types)

## Tech Stack

- **Backend**: Rust 1.75+, Tauri 2.x, SQLite (rusqlite), Tokio, Serde
- **Frontend**: TypeScript 5.x, React 19, Vite, TanStack Query, Zustand, Tailwind CSS, shadcn/ui
- **Tooling**: Bun, Cargo
