# Repository Guidelines

## Project Overview
TARS is a cross-platform desktop app for managing Claude Code configuration (skills, agents, commands, hooks, MCP servers, plugins, profiles). Platforms: Windows, macOS, Linux.

## Project Structure & Module Organization
- `apps/tars-desktop/` Tauri app.
  - `apps/tars-desktop/src/` React UI and shared components.
  - `apps/tars-desktop/src-tauri/` Rust IPC backend.
- `crates/tars-core/` profiles, apply/sync, storage.
- `crates/tars-scanner/` discovery + collision detection.
- `crates/tars-cli/` CLI wrapper.
- Assets: `apps/tars-desktop/src-tauri/icons/`.

## Build, Test, and Development Commands
- `cd apps/tars-desktop && bun install` install frontend deps.
- `cd apps/tars-desktop && bun run tauri dev` run the full app.
- `cd apps/tars-desktop && bun run tauri build` build production app.
- `cargo build` build Rust workspace.
- `cargo test` run Rust tests.
- `cargo run -p tars-cli -- scan` run scanner CLI.
- `bun tsc --noEmit` type-check frontend.

## Linting & Formatting (run before commit)
- `cargo fmt --all` format Rust.
- `cargo clippy --all -- -D warnings` lint Rust (no warnings).
- `cd apps/tars-desktop && bun run format` format frontend.
- `cd apps/tars-desktop && bun tsc --noEmit` type-check frontend.

## Coding Style & Naming Conventions
- TypeScript: functional components only, 2-space indentation, Tailwind + shadcn/ui.
- Rust: prefer `?` over `unwrap`, no `unsafe`, use `thiserror` for libraries and `anyhow` for CLI.
- File naming: `PascalCase.tsx` for components, `camelCase.ts` for helpers.
- IPC types live in `apps/tars-desktop/src/lib/types/`.

## Error Handling
- Tauri commands return `Result<T, String>`; frontend uses `toast.error()` for user-facing errors.
- Do not panic in library code; return `Result` with context.

## Testing Guidelines
- Rust: `cargo test` (unit tests alongside modules).
- Frontend: Vitest + React Testing Library (`*.test.ts`/`*.test.tsx`).
- Add tests when behavior changes (especially profile sync or IPC).

## Configuration Scopes (precedence high to low)
- Managed, Local, Project, User.
- macOS example: `/Library/Application Support/ClaudeCode/managed-*.json` then `<repo>/.claude/settings.local.json`, `<repo>/.claude/settings.json`, `~/.claude/settings.json`.

## Commit & Pull Request Guidelines
- Conventional Commits: `feat`, `fix`, `perf`, `docs`, `refactor`, `test`, `chore`, `ci`.
- PRs: brief summary, linked issue (if any), screenshots for UI changes, note data migrations.

## Security & Principles
- Never execute scanned config code; validate paths and sanitize shell input.
- No secrets in configs or tests.
- Key principles: discovery-first, safe-by-default, cross-platform, ask if unclear.

## Setup
- Enable hooks: `git config core.hooksPath .githooks`.
