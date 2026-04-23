# Multi-Runtime UI Implementation Checklist

## Product Framing

- [x] Keep TARS as the product name.
- [x] Use "Tooling, Agents, Roles, Skills" as the visible acronym.
- [x] Update product copy from "Claude Code configuration manager" to "AI coding agent configuration manager."
- [x] Rename Profiles to Bundles once multi-runtime export exists.
- [x] Add runtime compatibility language: Native, Convertible, Partial, Unsupported.

## Navigation And IA

- [x] Add a top-level Runtimes page.
- [x] Consolidate Skills, Agents, Commands, Hooks, and MCP into a unified Inventory page.
- [x] Keep runtime support as badges and filters, not top-level vendor silos.
- [x] Move Claude Settings under Runtimes > Claude Code.
- [x] Add Runtimes > Codex details after Codex config editing exists.
- [x] Broaden Plugins into Marketplace after Codex marketplace support lands.

## Runtime Model

- [x] Add a first runtime status model for Claude Code and Codex.
- [x] Introduce canonical runtime enum: `ClaudeCode`, `Codex`, `Universal`.
- [x] Add runtime support metadata to inventory items.
- [x] Add scope support for Codex user, project, admin, plugin, and system locations.
- [x] Add compatibility report model for lossy conversions.

## Codex Discovery

- [x] Detect the Codex CLI binary and installed version.
- [x] Show Codex user config, skills, agents, instructions, and marketplace paths.
- [x] Scan `~/.codex/config.toml`.
- [x] Scan project `.codex/config.toml`.
- [x] Scan `.agents/skills` from CWD to repository root.
- [x] Scan `~/.agents/skills`.
- [x] Scan `.codex/agents` and `~/.codex/agents`.
- [x] Scan `AGENTS.md` / `AGENTS.override.md` instruction layers.
- [x] Scan `.agents/plugins/marketplace.json` and `~/.agents/plugins/marketplace.json`.

## Conversion And Export

- [x] Convert Claude/Agent Skills to Codex Agent Skills by injecting required `name`.
- [x] Convert Claude agents to Codex custom agent TOML.
- [x] Convert Claude MCP JSON entries to Codex TOML `[mcp_servers.*]`.
- [x] Export Bundles as Codex plugins with `.codex-plugin/plugin.json`.
- [x] Generate Codex marketplace files at `.agents/plugins/marketplace.json`.
- [x] Mark Claude commands as skill/prompt conversions unless Codex adds custom commands.
- [x] Mark hooks as partial while Codex hooks remain experimental and narrower.

## First UI Slice

- [x] Commit and push current `main` changes before starting.
- [x] Create `feat/runtime-ui-foundation`.
- [x] Add implementation checklist.
- [x] Add Runtimes route and sidebar entry.
- [x] Add runtime status page with Claude Code and Codex.
- [x] Add real local status detection for binaries and key config paths.
- [x] Add Marketplace preview for Claude Code and Codex plugin surfaces.
- [x] Add project-level runtime coverage cards to Projects.
- [x] Add runtime badges to existing inventory rows.
