<!--
SYNC IMPACT REPORT
==================
Version change: 0.0.0 → 1.0.0
Bump type: MAJOR (initial constitution ratification)

Modified principles: N/A (initial version)

Added sections:
- Core Principles (5 principles)
- Additional Constraints (Technology & Compatibility)
- Development Workflow
- Governance

Removed sections: N/A (initial version)

Templates requiring updates:
- .specify/templates/plan-template.md ✅ (Constitution Check section compatible)
- .specify/templates/spec-template.md ✅ (Requirements align with principles)
- .specify/templates/tasks-template.md ✅ (Phase structure compatible)

Follow-up TODOs: None
-->

# TARS Constitution

## Core Principles

### I. Discovery-First

All operations MUST begin with a complete scan and inventory of existing Claude Code configuration before any modifications are made.

- Scanner MUST be read-only and non-destructive
- Discovery output MUST be generated before any apply/write operation
- System MUST inventory: skills, commands, agents, hooks, MCP servers, plugins, and marketplaces across all scopes (user, project, managed)
- Collisions and precedence winners MUST be detected and reported

**Rationale**: Users have Claude Code configuration distributed across multiple locations. Any tool that modifies configuration without first understanding the current state risks data loss or unexpected behavior.

### II. Safe-by-Default

No operation MUST silently modify user configuration. All changes require explicit preview and confirmation.

- All file writes MUST show diff preview before execution
- Backups MUST be created before any modification
- Rollback MUST be available for all applied changes
- No hook execution from the app without explicit user-initiated wrapper scripts
- Dirty git state MUST trigger warnings before profile application

**Rationale**: Configuration management tools that make silent changes undermine user trust. Users MUST maintain full control and visibility over what changes are made to their systems.

### III. Plugin-First Architecture

Shareable configuration bundles MUST prefer the Claude Code plugin packaging format.

- Profiles SHOULD be exportable as valid Claude Code plugins
- Plugin structure MUST follow `.claude-plugin/` conventions with `plugin.json` manifest
- Bundled skills, commands, agents, hooks, and MCP servers MUST use standard Claude Code formats
- No secrets MUST be embedded in plugins or exported profiles

**Rationale**: Aligning with Claude Code's native plugin system ensures maximum interoperability and allows users to share configurations through established distribution channels (marketplaces).

### IV. Profile Determinism

Profile application MUST produce identical results given identical inputs.

- Applying and rolling back a profile MUST return to original state byte-for-byte
- Merge rules MUST be deterministic and documented per artifact type
- Conflicts MUST be explicitly reported, never silently resolved
- MVP scope: conflicts are reported only; no automatic resolution

**Rationale**: Non-deterministic configuration changes make debugging impossible. Users MUST be able to predict exactly what a profile application will do.

### V. Current Docs First

Implementation MUST reference official Claude Code documentation and maintain compatibility with the current Claude Code CLI.

- File formats (SKILL.md, agent definitions, plugin.json) MUST match current Claude Code specifications
- CLI bridge MUST use `claude plugin` and `claude mcp` commands as documented
- Scanner MUST parse files according to current frontmatter schemas
- Version incompatibilities MUST be detected and reported

**Rationale**: Claude Code evolves rapidly. Relying on assumptions rather than current documentation leads to compatibility failures.

## Additional Constraints

### Technology Stack (MVP)

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Platform | macOS only | Focused MVP; Windows/Linux post-MVP |
| Backend | Tauri (Rust) | Native performance, single binary |
| Frontend | React + TypeScript + Vite | Modern, type-safe UI |
| Styling | Tailwind + shadcn/ui | Accessible component primitives |
| Package Manager | Bun | Fast JS tooling |
| Storage | SQLite (embedded) | Local-first, exportable sync bundles |

### Compatibility Requirements

- MUST support Claude Code plugin manifest schema
- MUST support Claude Code marketplace schema
- MUST parse SKILL.md frontmatter (name, description, allowed-tools, model, context, agent, user-invocable, disable-model-invocation, hooks)
- MUST parse agent definition frontmatter (name, description, tools, model, permissionMode, skills, hooks)
- MUST parse command frontmatter (description, thinking)
- MUST support MCP server configuration (stdio, http, sse types)

## Development Workflow

### Code Quality Gates

- All PRs MUST verify compliance with this constitution
- Scanner changes MUST include non-destructive guarantee tests
- Profile apply/rollback MUST include byte-for-byte verification tests
- CLI bridge MUST be tested against actual Claude Code CLI output

### Task Structure

Implementation follows the phased approach defined in `.specify/templates/tasks-template.md`:

1. **Task 1**: Discovery scanner CLI (no UI) - read-only inventory
2. **Task 2**: Profile snapshot + apply engine (no UI) - deterministic apply/rollback
3. **Task 3**: Minimal Tauri app - UI for scan, view, apply
4. **Task 4**: Plugin generator/export - profile-to-plugin conversion

### Deliverables Structure

```
apps/tars-desktop/     # Tauri app
crates/tars-scanner/   # Scanner library
crates/tars-cli/       # CLI wrapper
crates/tars-core/      # Profiles/apply/diff/rollback
docs/                  # Documentation
examples/              # Example profile exports
```

## Governance

### Amendment Process

1. Proposed amendments MUST be documented with rationale
2. Amendments MUST include migration plan for existing implementations
3. Version number MUST be updated according to semantic versioning:
   - MAJOR: Principle removal or redefinition
   - MINOR: New principle or section added
   - PATCH: Clarifications or wording fixes

### Compliance Review

- This constitution supersedes conflicting practices
- Implementation complexity MUST be justified against these principles
- Violations MUST be documented in the Complexity Tracking section of implementation plans

### Runtime Guidance

For development guidance beyond this constitution, consult:
- Official Claude Code documentation
- `.specify/specs/001-tars/spec.md` for feature details
- `claude-code-guide` agent for Claude Code implementation questions

**Version**: 1.0.0 | **Ratified**: 2026-01-08 | **Last Amended**: 2026-01-08
