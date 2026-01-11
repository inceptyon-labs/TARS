# Implementation Plan: Profiles

**Branch**: `003-profiles` | **Date**: 2026-01-10 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-profiles/spec.md`

## Summary

Implement a profile system where users can create reusable tool configuration bundles (MCP servers, skills, agents, hooks) and assign them to projects. Profiles are linked (not copied), so profile updates auto-sync to all assigned projects. Projects can have local overrides that persist through sync operations. Profiles can be exported/imported as `.tars-profile.json` files for sharing.

## Technical Context

**Language/Version**: Rust 1.75+ (backend), TypeScript 5.8+ (frontend)
**Primary Dependencies**: Tauri 2.x, React 19, rusqlite, serde, uuid, chrono
**Storage**: SQLite (embedded via rusqlite) - existing database with profiles/projects tables
**Testing**: cargo test (Rust), vitest (TypeScript)
**Target Platform**: macOS (MVP)
**Project Type**: Desktop app (Tauri - Rust backend + React frontend)
**Performance Goals**: Profile sync within 5 seconds, UI operations under 10 seconds
**Constraints**: Must integrate with existing Profile and Project types in tars-core
**Scale/Scope**: Single user, dozens of profiles, hundreds of projects

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Design Check (Phase 0)

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Discovery-First | PASS | Profile assignment reads existing config before applying |
| II. Safe-by-Default | PASS | Profile sync shows notification, deletion converts to local overrides |
| III. Plugin-First | PASS | Profiles align with existing plugin export architecture |
| IV. Profile Determinism | PASS | Sync is deterministic - same profile = same tools on all projects |
| V. Current Docs First | PASS | Uses existing Claude Code tool formats (MCP, skills, agents, hooks) |

**Gate Status**: PASS - proceeded to Phase 0

### Post-Design Check (Phase 1)

| Principle | Status | Design Evidence |
|-----------|--------|-----------------|
| I. Discovery-First | PASS | `get_project_tools` returns combined profile + local tools; scanner inventory used for tool picker |
| II. Safe-by-Default | PASS | `delete_profile` converts tools to local overrides (see contracts); `update_profile` returns `SyncResult` with notification |
| III. Plugin-First | PASS | `.tars-profile.json` export format compatible with plugin structure |
| IV. Profile Determinism | PASS | `ToolRef` uses references (not copies); sync overwrites profile portion, preserves local |
| V. Current Docs First | PASS | `ToolType` enum matches Claude Code artifact types (MCP, Skill, Agent, Hook) |

**Gate Status**: PASS - ready for Phase 2 (tasks)

## Project Structure

### Documentation (this feature)

```text
specs/003-profiles/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (Tauri IPC commands)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
# Existing Tauri app structure (extend, don't restructure)
apps/tars-desktop/
├── src/
│   ├── components/
│   │   ├── ProfileList.tsx          # Existing - extend
│   │   ├── ProfileDetail.tsx        # Existing - extend
│   │   ├── CreateProfileDialog.tsx  # Existing - extend
│   │   ├── AssignProfileDialog.tsx  # NEW - profile assignment UI
│   │   └── ProfileToolPicker.tsx    # NEW - tool selection UI
│   ├── pages/
│   │   ├── ProfilesPage.tsx         # Existing - extend
│   │   └── ProjectsPage.tsx         # Existing - add profile column
│   └── lib/
│       ├── ipc/index.ts             # Existing - add new commands
│       └── types/index.ts           # Existing - extend types
└── src-tauri/
    └── src/
        ├── commands/                # NEW - Tauri IPC handlers
        │   ├── mod.rs
        │   └── profiles.rs
        └── lib.rs                   # Register commands

crates/tars-core/
├── src/
│   ├── profile/
│   │   ├── mod.rs                   # Existing
│   │   ├── types.rs                 # Existing - extend with tool references
│   │   ├── snapshot.rs              # Existing
│   │   ├── sync.rs                  # NEW - profile sync logic
│   │   └── export.rs                # NEW - .tars-profile.json export/import
│   ├── storage/
│   │   ├── profiles.rs              # Existing - extend queries
│   │   └── projects.rs              # Existing - extend with profile queries
│   └── project.rs                   # Existing - has assigned_profile_id

tests/
├── integration/
│   └── profile_sync_test.rs         # NEW - sync integration tests
└── unit/
    └── profile_export_test.rs       # NEW - export/import tests
```

**Structure Decision**: Extend existing Tauri app and tars-core crate. Profile logic goes in `crates/tars-core/src/profile/`, UI components in `apps/tars-desktop/src/components/`.

## Complexity Tracking

> No constitution violations requiring justification.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | N/A | N/A |
