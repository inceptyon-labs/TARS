# Multi-Runtime UI Implementation Checklist

## Product Framing

- [x] Keep TARS as the product name.
- [x] Use "Tooling, Agents, Roles, Skills" as the visible acronym.
- [ ] Update product copy from "Claude Code configuration manager" to "AI coding agent configuration manager."
- [ ] Rename Profiles to Bundles once multi-runtime export exists.
- [ ] Add runtime compatibility language: Native, Convertible, Partial, Unsupported.

## Navigation And IA

- [x] Add a top-level Runtimes page.
- [ ] Consolidate Skills, Agents, Commands, Hooks, and MCP into a unified Inventory page.
- [ ] Keep runtime support as badges and filters, not top-level vendor silos.
- [ ] Move Claude Settings under Runtimes > Claude Code.
- [ ] Add Runtimes > Codex details after Codex config editing exists.
- [ ] Broaden Plugins into Marketplace after Codex marketplace support lands.

## Runtime Model

- [x] Add a first runtime status model for Claude Code and Codex.
- [ ] Introduce canonical runtime enum: `ClaudeCode`, `Codex`, `Universal`.
- [ ] Add runtime support metadata to inventory items.
- [ ] Add scope support for Codex user, project, admin, plugin, and system locations.
- [ ] Add compatibility report model for lossy conversions.

## Codex Discovery

- [x] Detect the Codex CLI binary and installed version.
- [x] Show Codex user config, skills, agents, instructions, and marketplace paths.
- [ ] Scan `~/.codex/config.toml`.
- [ ] Scan project `.codex/config.toml`.
- [ ] Scan `.agents/skills` from CWD to repository root.
- [ ] Scan `~/.agents/skills`.
- [ ] Scan `.codex/agents` and `~/.codex/agents`.
- [ ] Scan `AGENTS.md` / `AGENTS.override.md` instruction layers.
- [ ] Scan `.agents/plugins/marketplace.json` and `~/.agents/plugins/marketplace.json`.

## Conversion And Export

- [ ] Convert Claude/Agent Skills to Codex Agent Skills by injecting required `name`.
- [ ] Convert Claude agents to Codex custom agent TOML.
- [ ] Convert Claude MCP JSON entries to Codex TOML `[mcp_servers.*]`.
- [ ] Export Bundles as Codex plugins with `.codex-plugin/plugin.json`.
- [ ] Generate Codex marketplace files at `.agents/plugins/marketplace.json`.
- [ ] Mark Claude commands as skill/prompt conversions unless Codex adds custom commands.
- [ ] Mark hooks as partial while Codex hooks remain experimental and narrower.

## First UI Slice

- [x] Commit and push current `main` changes before starting.
- [x] Create `feat/runtime-ui-foundation`.
- [x] Add implementation checklist.
- [x] Add Runtimes route and sidebar entry.
- [x] Add runtime status page with Claude Code and Codex.
- [x] Add real local status detection for binaries and key config paths.
- [ ] Add project-level runtime coverage cards to Projects.
- [ ] Add runtime badges to existing inventory rows.
