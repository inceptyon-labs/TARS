# Tasks: TARS

**Input**: Design documents from `/specs/001-tars/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests included per constitution requirement (non-destructive guarantee tests for scanner, byte-for-byte rollback tests for profiles).

**Organization**: Tasks grouped by user story (spec Tasks 1-4) to enable independent implementation.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1=Scanner CLI, US2=Profile Engine, US3=Tauri App, US4=Plugin Export
- Include exact file paths in descriptions

## Path Conventions

- **Rust crates**: `crates/tars-scanner/src/`, `crates/tars-core/src/`, `crates/tars-cli/src/`
- **Tauri app**: `apps/tars-desktop/src-tauri/src/`, `apps/tars-desktop/src/`
- **Tests**: `crates/*/tests/`

---

## Phase 1: Setup (Shared Infrastructure) ✅

**Purpose**: Initialize Rust workspace and project structure

- [X] T001 Create workspace Cargo.toml at repository root with workspace.dependencies
- [X] T002 [P] Create crates/tars-scanner/Cargo.toml with scanner dependencies
- [X] T003 [P] Create crates/tars-core/Cargo.toml with core dependencies
- [X] T004 [P] Create crates/tars-cli/Cargo.toml with CLI dependencies
- [X] T005 Initialize Tauri app in apps/tars-desktop/ with bun create tauri-app
- [X] T006 [P] Configure rustfmt.toml and clippy.toml at repository root
- [X] T007 [P] Add shadcn/ui components to apps/tars-desktop/ (button, card, dialog, form, table, tabs)
- [X] T008 Configure Tauri capabilities in apps/tars-desktop/src-tauri/capabilities/default.json

---

## Phase 2: Foundational (Blocking Prerequisites) ✅

**Purpose**: Core types and infrastructure shared by all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T009 Create shared types module in crates/tars-scanner/src/types.rs (Scope, FileInfo, HostInfo)
- [X] T010 [P] Create inventory types in crates/tars-scanner/src/inventory.rs (Inventory, UserScope, ProjectScope, ManagedScope)
- [X] T011 [P] Create artifact types in crates/tars-scanner/src/artifacts.rs (SkillInfo, CommandInfo, AgentInfo, HookInfo)
- [X] T012 [P] Create settings types in crates/tars-scanner/src/settings.rs (SettingsFile, Permissions, McpConfig, McpServer)
- [X] T013 [P] Create plugin types in crates/tars-scanner/src/plugins.rs (PluginInventory, Marketplace, InstalledPlugin, PluginManifest)
- [X] T014 [P] Create collision types in crates/tars-scanner/src/collision.rs (CollisionReport, Collision, CollisionOccurrence)
- [X] T015 Create error types with thiserror in crates/tars-scanner/src/error.rs
- [X] T016 Create crates/tars-scanner/src/lib.rs exporting all public types
- [X] T017 Create profile types in crates/tars-core/src/profile.rs (Profile, PluginSet, RepoOverlays, UserOverlays)
- [X] T018 [P] Create diff types in crates/tars-core/src/diff.rs (DiffPlan, FileOperation, Warning)
- [X] T019 [P] Create backup types in crates/tars-core/src/backup.rs (Backup, BackupFile)
- [X] T020 [P] Create project types in crates/tars-core/src/project.rs (Project, GitInfo)
- [X] T021 Create storage module skeleton in crates/tars-core/src/storage/mod.rs
- [X] T022 Create SQLite schema and migrations in crates/tars-core/src/storage/migrations.rs
- [X] T023 Implement database connection in crates/tars-core/src/storage/db.rs
- [X] T024 Create crates/tars-core/src/lib.rs exporting all public types and re-exporting tars_scanner

**Checkpoint**: Foundation ready - all types defined, database initialized ✅

---

## Phase 3: User Story 1 - Discovery Scanner CLI (Priority: P1) 🎯 MVP ✅

**Goal**: Standalone CLI that scans Claude Code configuration and outputs inventory JSON/MD

**Independent Test**: Run `cargo run -p tars-cli -- scan ~` and verify tars-inventory.json is created with correct structure

### Tests for User Story 1

- [X] T025 [P] [US1] Create scanner integration test in crates/tars-scanner/tests/scan_test.rs
- [X] T026 [P] [US1] Create non-destructive guarantee test in crates/tars-scanner/tests/readonly_test.rs
- [X] T027 [P] [US1] Create frontmatter parser tests in crates/tars-scanner/tests/parser_test.rs

### Implementation for User Story 1

- [X] T028 [P] [US1] Implement YAML frontmatter parser in crates/tars-scanner/src/parser/frontmatter.rs
- [X] T029 [P] [US1] Implement JSON settings parser in crates/tars-scanner/src/parser/settings.rs
- [X] T030 [P] [US1] Implement MCP config parser in crates/tars-scanner/src/parser/mcp.rs
- [X] T031 [US1] Create parser module in crates/tars-scanner/src/parser/mod.rs
- [X] T032 [US1] Implement user scope scanner in crates/tars-scanner/src/scope/user.rs
- [X] T033 [P] [US1] Implement managed scope scanner in crates/tars-scanner/src/scope/managed.rs
- [X] T034 [US1] Implement project scope scanner in crates/tars-scanner/src/scope/project.rs
- [X] T035 [US1] Create scope module in crates/tars-scanner/src/scope/mod.rs
- [X] T036 [US1] Implement collision detection in crates/tars-scanner/src/collision.rs (in scan.rs)
- [X] T037 [US1] Implement plugin inventory via CLI bridge in crates/tars-scanner/src/plugins.rs
- [X] T038 [US1] Implement full inventory scan in crates/tars-scanner/src/scan.rs
- [X] T039 [US1] Implement JSON output formatter in crates/tars-scanner/src/output/json.rs
- [X] T040 [P] [US1] Implement Markdown output formatter in crates/tars-scanner/src/output/markdown.rs
- [X] T041 [US1] Create output module in crates/tars-scanner/src/output/mod.rs
- [X] T042 [US1] Implement CLI with clap in crates/tars-cli/src/main.rs (scan subcommand)
- [X] T043 [US1] Add scan command help and argument validation in crates/tars-cli/src/main.rs

**Checkpoint**: Scanner CLI fully functional - `tars scan` produces inventory files

---

## Phase 4: User Story 2 - Profile Engine (Priority: P2) ✅

**Goal**: Create profiles from snapshots, apply with diff preview, backup and rollback

**Independent Test**: Apply profile to test directory, rollback, verify byte-for-byte match with original

### Tests for User Story 2

- [X] T044 [P] [US2] Create profile CRUD tests in crates/tars-core/tests/profile_test.rs
- [X] T045 [P] [US2] Create diff generation tests in crates/tars-core/tests/diff_test.rs
- [X] T046 [P] [US2] Create byte-for-byte rollback test in crates/tars-core/tests/rollback_test.rs

### Implementation for User Story 2

- [X] T047 [US2] Implement profile storage (CRUD) in crates/tars-core/src/storage/profiles.rs
- [X] T048 [P] [US2] Implement project storage (CRUD) in crates/tars-core/src/storage/projects.rs
- [X] T049 [P] [US2] Implement backup storage in crates/tars-core/src/storage/backups.rs
- [X] T050 [US2] Implement profile snapshot creation in crates/tars-core/src/profile/snapshot.rs
- [X] T051 [US2] Implement diff plan generation in crates/tars-core/src/diff/plan.rs
- [X] T052 [US2] Implement text diff display in crates/tars-core/src/diff/display.rs
- [X] T053 [US2] Implement backup creation in crates/tars-core/src/backup/create.rs
- [X] T054 [US2] Implement atomic file writes in crates/tars-core/src/apply/write.rs
- [X] T055 [US2] Implement profile application in crates/tars-core/src/apply/mod.rs
- [X] T056 [US2] Implement rollback restore in crates/tars-core/src/backup/restore.rs
- [X] T057 [US2] Implement git dirty check in crates/tars-core/src/diff/plan.rs (check_git_dirty)
- [X] T058 [US2] Add profile subcommands to CLI in crates/tars-cli/src/main.rs (create, apply, rollback, show)

**Checkpoint**: Profile engine fully functional - apply + rollback works byte-for-byte ✅

---

## Phase 5: User Story 3 - Tauri Desktop App (Priority: P3)

**Goal**: Desktop app with project list, inventory view, profile management, skills editor

**Independent Test**: Launch app, add project, scan, create profile, apply with diff preview

### Implementation for User Story 3 - Backend (Tauri Commands)

- [X] T059 [US3] Create Tauri app state in apps/tars-desktop/src-tauri/src/state.rs
- [X] T060 [US3] Implement scanner commands in apps/tars-desktop/src-tauri/src/commands/scanner.rs
- [X] T061 [P] [US3] Implement project commands in apps/tars-desktop/src-tauri/src/commands/projects.rs
- [X] T062 [P] [US3] Implement profile commands in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [X] T063 [P] [US3] Implement apply commands in apps/tars-desktop/src-tauri/src/commands/apply.rs
- [X] T064 [P] [US3] Implement skill commands in apps/tars-desktop/src-tauri/src/commands/skills.rs
- [X] T065 [P] [US3] Implement utility commands in apps/tars-desktop/src-tauri/src/commands/utils.rs
- [X] T066 [US3] Create commands module in apps/tars-desktop/src-tauri/src/commands/mod.rs
- [X] T067 [US3] Wire up Tauri app in apps/tars-desktop/src-tauri/src/lib.rs
- [X] T068 [US3] Update Tauri main.rs entry point in apps/tars-desktop/src-tauri/src/main.rs

### Implementation for User Story 3 - Frontend (React)

- [X] T069 [US3] Create TypeScript types from Rust in apps/tars-desktop/src/lib/types/index.ts
- [X] T070 [US3] Create IPC wrapper functions in apps/tars-desktop/src/lib/ipc/index.ts
- [X] T071 [US3] Setup Zustand UI store in apps/tars-desktop/src/stores/ui-store.ts
- [X] T072 [US3] Setup TanStack Query provider in apps/tars-desktop/src/App.tsx
- [X] T073 [US3] Create ProjectList component in apps/tars-desktop/src/components/ProjectList.tsx
- [X] T074 [P] [US3] Create ProjectDetail component in apps/tars-desktop/src/components/ProjectDetail.tsx
- [X] T075 [P] [US3] Create InventoryView component in apps/tars-desktop/src/components/InventoryView.tsx
- [X] T076 [P] [US3] Create InventoryTree component in apps/tars-desktop/src/components/InventoryTree.tsx
- [X] T077 [US3] Create ProfileList component in apps/tars-desktop/src/components/ProfileList.tsx
- [X] T078 [P] [US3] Create ProfileForm component in apps/tars-desktop/src/components/ProfileForm.tsx
- [X] T079 [US3] Create DiffPreview component with Monaco in apps/tars-desktop/src/components/DiffPreview.tsx
- [X] T080 [US3] Create ApplyDialog component in apps/tars-desktop/src/components/ApplyDialog.tsx
- [X] T081 [US3] Create SkillEditor component in apps/tars-desktop/src/components/SkillEditor.tsx
- [X] T082 [US3] Create CollisionBadge component in apps/tars-desktop/src/components/CollisionBadge.tsx
- [X] T083 [US3] Create main layout in apps/tars-desktop/src/pages/MainLayout.tsx
- [X] T084 [US3] Create ProjectsPage in apps/tars-desktop/src/pages/ProjectsPage.tsx
- [X] T085 [P] [US3] Create ProfilesPage in apps/tars-desktop/src/pages/ProfilesPage.tsx
- [X] T086 [P] [US3] Create SkillsPage in apps/tars-desktop/src/pages/SkillsPage.tsx
- [X] T087 [US3] Setup routing in apps/tars-desktop/src/App.tsx
- [X] T088 [US3] Add dark mode support via ThemeProvider in apps/tars-desktop/src/App.tsx

**Checkpoint**: Desktop app fully functional - can scan, create profiles, apply with preview

---

## Phase 6: User Story 4 - Plugin Export (Priority: P4)

**Goal**: Export profiles as Claude Code plugins with proper structure

**Independent Test**: Export profile as plugin, install with `claude plugin install`, verify artifacts available

### Tests for User Story 4

- [X] T089 [P] [US4] Create plugin export tests in crates/tars-core/tests/plugin_export_test.rs

### Implementation for User Story 4

- [X] T090 [US4] Implement plugin manifest generation in crates/tars-core/src/export/manifest.rs
- [X] T091 [US4] Implement plugin directory structure in crates/tars-core/src/export/structure.rs
- [X] T092 [US4] Implement profile-to-plugin conversion in crates/tars-core/src/export/convert.rs
- [X] T093 [US4] Implement zip archive creation in crates/tars-core/src/export/archive.rs
- [X] T094 [US4] Create export module in crates/tars-core/src/export/mod.rs
- [X] T095 [US4] Add export_as_plugin Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [X] T096 [US4] Create ExportPluginDialog component in apps/tars-desktop/src/components/ExportPluginDialog.tsx
- [X] T097 [US4] Add export button to ProfilesPage in apps/tars-desktop/src/pages/ProfilesPage.tsx

**Checkpoint**: Plugin export fully functional - exported plugins install cleanly

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, cleanup, and final validation

- [ ] T098 [P] Create user documentation in docs/user-guide.md
- [ ] T099 [P] Create example profile exports in examples/
- [X] T100 Run all tests with cargo test --workspace
- [X] T101 Run clippy and fix warnings with cargo clippy --workspace
- [ ] T102 Validate quickstart.md setup instructions
- [X] T103 Build release binary with cargo build --release
- [ ] T104 Test release build on clean macOS system

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 - BLOCKS all user stories
- **US1 Scanner (Phase 3)**: Depends on Phase 2
- **US2 Profile Engine (Phase 4)**: Depends on Phase 2, uses US1 scanner
- **US3 Tauri App (Phase 5)**: Depends on Phase 2, uses US1 and US2
- **US4 Plugin Export (Phase 6)**: Depends on Phase 2, uses US2 profile types
- **Polish (Phase 7)**: Depends on all user stories

### User Story Dependencies

- **US1 (Scanner CLI)**: Independent after Phase 2 - **MVP deliverable**
- **US2 (Profile Engine)**: Uses scanner from US1 but can be tested independently
- **US3 (Tauri App)**: Uses both US1 and US2 but provides independent value
- **US4 (Plugin Export)**: Uses US2 profiles, adds export capability

### Within Each User Story

- Tests written FIRST (TDD per constitution)
- Types/models before services
- Services before commands/UI
- Core implementation before integration

### Parallel Opportunities

```bash
# Phase 1 parallel tasks:
T002, T003, T004  # Create all crate Cargo.tomls in parallel
T006, T007        # Config files in parallel

# Phase 2 parallel tasks:
T010, T011, T012, T013, T014  # All type definitions in parallel
T018, T019, T020              # Core types in parallel

# Phase 3 (US1) parallel tasks:
T025, T026, T027  # All scanner tests in parallel
T028, T029, T030  # All parsers in parallel
T039, T040        # Output formatters in parallel

# Phase 5 (US3) parallel tasks:
T061, T062, T063, T064, T065  # Tauri commands in parallel
T074, T075, T076              # View components in parallel
T085, T086                    # Additional pages in parallel
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (Scanner CLI)
4. **STOP and VALIDATE**: Run `tars scan ~` and verify output
5. Deliver CLI scanner as first milestone

### Incremental Delivery

1. Setup + Foundational → Types and infrastructure ready
2. US1 Scanner CLI → `tars scan` works → **Milestone 1**
3. US2 Profile Engine → `tars apply` works → **Milestone 2**
4. US3 Tauri App → Desktop app works → **Milestone 3**
5. US4 Plugin Export → Full feature set → **Milestone 4**

### Sequential Single-Developer Strategy

1. Complete all phases in order (1 → 2 → 3 → 4 → 5 → 6 → 7)
2. Commit after each task or logical group
3. Validate at each checkpoint before proceeding
4. Each milestone is independently releasable

---

## Notes

- [P] tasks = different files, no dependencies within phase
- [USn] label maps task to specific user story
- Constitution requires: non-destructive scanner tests, byte-for-byte rollback tests
- Commit after each task or logical group
- Run `cargo test` after each phase completion
- Stop at checkpoints to validate independently
