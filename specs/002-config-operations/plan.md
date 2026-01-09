# Implementation Plan: Config Operations Layer

**Branch**: `002-config-operations` | **Date**: 2026-01-09 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-config-operations/spec.md`

## Summary

This feature adds a granular config operations layer to TARS, enabling surgical CRUD operations on individual Claude Code configuration items (MCP servers, skills, hooks, commands, agents) without overwriting entire config files. This sits between the existing scanner (read) and profile engine (write), providing merge, partial update, and cross-scope move capabilities.

**Architecture**:
```
┌─────────────────────────────────────────────────────────────┐
│                    apps/tars-desktop                        │
│                   (Tauri UI - PRIMARY)                      │
│   MCP panel │ Skills panel │ Hooks panel │ etc.            │
└─────────────────────────┬───────────────────────────────────┘
                          │ Tauri commands
┌─────────────────────────▼───────────────────────────────────┐
│                     tars-core                               │
│              config/ module (THIS FEATURE)                  │
│   add() │ remove() │ update() │ move() │ list()            │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│              tars-cli (SECONDARY)                           │
│         Power users, scripting, automation                  │
└─────────────────────────────────────────────────────────────┘
```

The **primary interface** is the Tauri desktop app. The CLI provides the same operations for power users and automation scripts. Both call into `tars-core::config` for the actual operations.

## Technical Context

**Language/Version**: Rust 1.75+ (extends existing tars-core crate)
**Primary Dependencies**: serde_json (JSON manipulation), gray_matter (frontmatter parsing), existing tars-scanner, tars-core crates
**Storage**: File-based (Claude Code config files) + SQLite (backups via existing BackupStore)
**Testing**: cargo test with integration tests against fixture files
**Target Platform**: macOS (per constitution)
**Project Type**: Library extension + CLI commands
**Performance Goals**: Operations complete in <1 second for typical configs
**Constraints**: Must preserve file formatting where possible; must integrate with existing backup/rollback system
**Scale/Scope**: Handles config files up to 1MB; typical configs are <10KB

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Discovery-First** | PASS | Operations use existing scanner inventory; add/remove/move require prior scan |
| **II. Safe-by-Default** | PASS | All operations create backups via existing BackupStore; dry-run mode required |
| **III. Plugin-First** | N/A | This feature is infrastructure; plugin export is separate concern |
| **IV. Profile Determinism** | PASS | Operations are atomic; rollback restores byte-for-byte via existing system |
| **V. Current Docs First** | PASS | Must parse/write files per Claude Code schemas |

**Additional Constraints Check**:
- Technology stack: Rust ✅, SQLite ✅, macOS ✅
- Compatibility: MCP server config ✅, SKILL.md frontmatter ✅, settings.json hooks ✅

## Project Structure

### Documentation (this feature)

```text
specs/002-config-operations/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (CLI interface specs)
└── tasks.md             # Phase 2 output
```

### Source Code (repository root)

```text
crates/tars-core/
├── src/
│   ├── config/          # NEW: Config operations module
│   │   ├── mod.rs       # Module exports
│   │   ├── item.rs      # ConfigItem enum and types
│   │   ├── scope.rs     # ConfigScope handling
│   │   ├── ops.rs       # Core operations (add, remove, update, move)
│   │   ├── mcp.rs       # MCP server-specific operations
│   │   ├── skill.rs     # Skill-specific operations
│   │   ├── hook.rs      # Hook-specific operations
│   │   ├── command.rs   # Command-specific operations
│   │   └── agent.rs     # Agent-specific operations
│   ├── apply/           # EXISTING: Extend with config-aware apply
│   └── ...
└── tests/
    ├── config/          # NEW: Config operation tests
    │   ├── mcp_tests.rs
    │   ├── skill_tests.rs
    │   └── integration_tests.rs
    └── ...

crates/tars-cli/
├── src/
│   ├── commands/
│   │   ├── mcp.rs       # NEW: tars mcp add/remove/move/update
│   │   ├── skill.rs     # NEW: tars skill add/remove/move
│   │   ├── hook.rs      # NEW: tars hook add/remove
│   │   ├── command.rs   # NEW: tars command add/remove/move
│   │   └── agent.rs     # NEW: tars agent add/remove/move
│   └── ...
└── ...

apps/tars-desktop/           # Tauri app (PRIMARY UI)
├── src-tauri/
│   ├── src/
│   │   ├── commands/        # NEW: Tauri command handlers
│   │   │   ├── mcp.rs       # MCP server operations
│   │   │   ├── skill.rs     # Skill operations
│   │   │   ├── hook.rs      # Hook operations
│   │   │   ├── command.rs   # Command operations
│   │   │   └── agent.rs     # Agent operations
│   │   └── ...
│   └── Cargo.toml           # Depends on tars-core
└── src/
    ├── components/
    │   ├── config/          # NEW: Config management UI components
    │   │   ├── McpPanel.tsx       # MCP server list + add/edit/remove
    │   │   ├── McpForm.tsx        # Add/edit MCP server form
    │   │   ├── SkillPanel.tsx     # Skills list + management
    │   │   ├── HookPanel.tsx      # Hooks list + management
    │   │   ├── ScopeSelector.tsx  # User/Project/Local scope picker
    │   │   └── ConfirmDialog.tsx  # Confirm before destructive ops
    │   └── ...
    └── ...
```

**Structure Decision**:
- `tars-core::config` provides the operations library (used by both UI and CLI)
- `tars-cli` wraps operations for command-line use
- `apps/tars-desktop` is the primary UI, with React components calling Tauri commands that invoke tars-core

## Complexity Tracking

No constitution violations. Implementation uses existing patterns:
- Backup/rollback via existing BackupStore
- File operations via existing apply/write.rs patterns
- Scanner integration via existing Scanner API
