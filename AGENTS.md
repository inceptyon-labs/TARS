# TARS — agent notes

Desktop app for managing Claude Code configuration (skills, agents, commands,
hooks, MCP servers, plugins, profiles). See README for features and usage.

## Build & test

- Frontend lives in `apps/tars-desktop`: `bun install`, then
  `bun run tauri dev` / `bun run tauri build`.
- Rust workspace: `cargo build`, `cargo test`, `cargo run -p tars-cli -- scan`.
- Frontend tests: `bun run test:run` (Vitest + React Testing Library,
  `*.test.ts(x)`), e2e `bun run test:e2e` (Playwright).
- Coverage: `bun run test:coverage` (v8). There is no cargo tarpaulin setup.
- Lint/typecheck before commit: `bun run lint`, `bun run typecheck`,
  `cargo fmt --all`, `cargo clippy --all -- -D warnings`.
- There is no `format` script — Prettier runs via the pre-commit hook.
  Enable hooks after clone: `git config core.hooksPath .githooks`.

## Modules

- `crates/tars-scanner` — non-destructive discovery of config across scopes,
  collision detection.
- `crates/tars-core` — profiles, diff/apply/rollback engine, SQLite storage.
- `crates/tars-providers` — AI provider integrations (key validation, model
  discovery, pricing); consumed by `src-tauri/src/commands/api_keys.rs`.
- `crates/tars-cli` — CLI wrapper for the scanner.
- `apps/tars-desktop/src-tauri` — Tauri IPC backend over the crates.

## Invariants

- Tauri commands return `Result<T, String>`; frontend shows failures with
  `toast.error()` (Sonner). Never panic in library code.
- `unsafe_code = "forbid"` workspace-wide via `[workspace.lints]`; clippy
  must pass with `-D warnings`.
- IPC types live in `apps/tars-desktop/src/lib/types/` — keep Rust and TS
  shapes in sync when changing commands.
- Never execute code from scanned configs; validate paths against traversal
  and sanitize anything passed to a shell.
- Discovery-first, safe-by-default: scan before modifying; changes get diff
  preview, backup, and rollback.

## Configuration scopes

Precedence high → low: **Managed > Local > Project > User**. The full per-OS
path table lives in README ("Configuration Scopes") — single source, don't
duplicate it here.

## Architecture diagram

`docs/architecture.json` is the Archify spec for the system overview
(webview UI, Tauri backend, scanner, core engine, providers, SQLite, the
apply-a-profile path, the webview and Rust-process trust boundaries, and the
external services), with file:line sources pinned to a commit. If a change
adds, removes, or rewires a top-level component, external service, trust
boundary, or the deploy shape, update the spec after committing it: re-pin
`meta.repository.revision` to that commit, fix the affected nodes, edges, and
sources, re-render with the `archify` skill (`finalize ... --repo-root .`), and
commit the spec. Internal refactors and shifted line numbers don't need it.
Rendered HTML goes in `.archify/` (gitignored).
