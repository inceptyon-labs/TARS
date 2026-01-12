# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

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

## Linting & Formatting

**Run before committing** to ensure CI passes:

```bash
cargo fmt --all                       # Format Rust code
cargo clippy --all -- -D warnings     # Lint Rust (must pass with no warnings)

cd apps/tars-desktop
bun run format                        # Format TypeScript/frontend
bun tsc --noEmit                      # Type-check frontend
```

## Setup

After cloning, enable pre-commit hooks:

```bash
git config core.hooksPath .githooks
```

## Architecture

```
apps/tars-desktop/     # Tauri app (React + TypeScript + Vite)
crates/tars-scanner/   # Discovery - reads Claude Code config from all scopes
crates/tars-cli/       # CLI wrapper for scanner
crates/tars-core/      # Profiles, diff, apply, rollback engine
```

### Key Modules

- `scanner`: Non-destructive discovery of skills, commands, agents, hooks, MCP, plugins
- `parser`: Frontmatter parsing for SKILL.md, agent definitions, commands
- `profiles`: Snapshot/export/import/apply operations
- `config`: MCP operations, config file management
- `storage`: SQLite database for projects and profiles

### Configuration Scopes (precedence high to low)

**macOS:**
1. **Managed**: `/Library/Application Support/ClaudeCode/managed-*.json`
2. **Local**: `<repo>/.claude/settings.local.json` (gitignored)
3. **Project**: `<repo>/.claude/settings.json`, `<repo>/.mcp.json`
4. **User**: `~/.claude/settings.json`, `~/.claude.json`

**Linux:**
1. **Managed**: `/etc/claude-code/`
2. **Local**: `<repo>/.claude/settings.local.json` (gitignored)
3. **Project**: `<repo>/.claude/settings.json`, `<repo>/.mcp.json`
4. **User**: `~/.claude/settings.json`, `~/.claude.json`

**Windows:**
1. **Managed**: `%ProgramData%\ClaudeCode\managed-*.json`
2. **Local**: `<repo>\.claude\settings.local.json` (gitignored)
3. **Project**: `<repo>\.claude\settings.json`, `<repo>\.mcp.json`
4. **User**: `%USERPROFILE%\.claude\settings.json`, `%USERPROFILE%\.claude.json`

## Code Style

### Rust

- Prefer `?` operator over `.unwrap()` - propagate errors up
- Use `thiserror` for library error types, `anyhow` for CLI/commands
- Async with Tokio - use `async fn`, avoid `block_on`
- No `unsafe` code (enforced by `#![forbid(unsafe_code)]`)

### TypeScript / React

- Functional components only, no classes
- TanStack Query for server state (fetching, caching)
- Zustand for UI state (sidebar, dialogs)
- Tailwind for styling - no separate CSS files
- shadcn/ui components as base

## Error Handling

- **Tauri commands**: Return `Result<T, String>` - errors become frontend exceptions
- **Frontend**: Use `toast.error()` from Sonner for user-facing errors
- **Never panic** in library code - always return `Result`
- **Log errors** with context before returning

## Testing

- Run `cargo test` before committing
- New features should include tests when practical
- Frontend tests: Vitest + React Testing Library
- Test files: `*.test.ts` / `*.test.tsx`

## Security

This app manages configuration files - security is critical:

- **Never execute** code from scanned configs
- **Validate paths** to prevent directory traversal
- **Sanitize input** before any shell commands
- **No secrets** in config files - warn users if detected

## Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): description`

Types: `feat`, `fix`, `perf` (appear in changelog) | `docs`, `style`, `refactor`, `test`, `chore`, `ci`

## Key Principles

1. **Discovery-First**: Always scan before modifying
2. **Safe-by-Default**: Diff preview, backups, rollback for all changes
3. **Cross-Platform**: All paths/operations work on Windows, macOS, Linux
4. **Ask if unclear**: Use AskUserQuestionTool before making assumptions

## Tech Stack

- **Backend**: Rust 1.75+, Tauri 2.x, SQLite (rusqlite), Tokio, Serde
- **Frontend**: TypeScript 5.x, React 19, Vite, TanStack Query, Zustand, Tailwind CSS, shadcn/ui
- **Tooling**: Bun, Cargo
