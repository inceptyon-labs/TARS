# Tasks: Config Operations Layer

**Input**: Design documents from `/specs/002-config-operations/`
**Prerequisites**: plan.md, spec.md, data-model.md, contracts/cli.md, contracts/tauri-commands.md

**Tests**: Tests are NOT explicitly requested - test tasks are minimal (integration tests only).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Path Conventions

```
crates/tars-core/src/config/    # Core library operations
crates/tars-cli/src/commands/   # CLI commands
apps/tars-desktop/src-tauri/    # Tauri backend commands
apps/tars-desktop/src/          # React frontend components
```

---

## Phase 1: Setup (Shared Infrastructure) ✅ COMPLETE

**Purpose**: Create module structure and configure dependencies

- [x] T001 Create config module directory structure in crates/tars-core/src/config/
- [x] T002 [P] Add config module exports in crates/tars-core/src/lib.rs
- [x] T003 [P] Create commands directory in crates/tars-cli/src/commands/ if not exists
- [x] T004 [P] Create Tauri commands directory in apps/tars-desktop/src-tauri/src/commands/
- [x] T005 [P] Create config components directory in apps/tars-desktop/src/components/config/

---

## Phase 2: Foundational (Blocking Prerequisites) ✅ COMPLETE

**Purpose**: Core types and infrastructure that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 Implement ConfigScope enum and path resolution in crates/tars-core/src/config/scope.rs
- [x] T007 [P] Implement ConfigItem enum with all variants in crates/tars-core/src/config/item.rs
- [x] T008 [P] Implement McpServerConfig struct in crates/tars-core/src/config/mcp.rs
- [x] T009 [P] Implement SkillConfig, CommandConfig, AgentConfig structs in crates/tars-core/src/config/{skill,command,agent}.rs
- [x] T010 [P] Implement HookDefinition and HookTrigger enums in crates/tars-core/src/config/hook.rs
- [x] T011 Implement OperationPlan and FileChange structs in crates/tars-core/src/config/ops.rs
- [x] T012 Implement OperationResult struct in crates/tars-core/src/config/ops.rs
- [x] T013 Implement name validation (no separators, no .., no null bytes) in crates/tars-core/src/config/item.rs
- [x] T014 Create config module exports in crates/tars-core/src/config/mod.rs
- [x] T015 [P] Create shared UI components: ScopeSelector.tsx in apps/tars-desktop/src/components/config/
- [x] T016 [P] Create shared UI components: ConfirmDialog.tsx in apps/tars-desktop/src/components/config/
- [x] T017 [P] Create shared UI components: DiffPreview.tsx in apps/tars-desktop/src/components/config/

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1+2 - MCP Add/Remove (Priority: P1) ✅ MVP COMPLETE

**Goal**: Add and remove MCP servers surgically without affecting other servers

**Independent Test**: Add a server to existing config, verify all servers present; remove a server, verify others unchanged

### Core Library (tars-core) ✅ COMPLETE

- [x] T018 [US1] Implement mcp_list() to read MCP servers from all scopes in crates/tars-core/src/config/mcp_ops.rs
- [x] T019 [US1] Implement mcp_add() with JSON dict merge in crates/tars-core/src/config/mcp_ops.rs
- [x] T020 [US1] Implement conflict detection (server exists) in crates/tars-core/src/config/mcp_ops.rs
- [x] T021 [US1] Implement file creation for missing .mcp.json in crates/tars-core/src/config/mcp_ops.rs
- [x] T022 [US2] Implement mcp_remove() with surgical dict removal in crates/tars-core/src/config/mcp_ops.rs
- [x] T023 [US2] Implement "server not found" handling with suggestions in crates/tars-core/src/config/mcp_ops.rs
- [x] T024 [US1] Integrate backup creation before write operations in crates/tars-core/src/config/mcp_ops.rs
- [x] T025 [US1] Implement dry-run mode for add/remove in crates/tars-core/src/config/mcp_ops.rs

### CLI Commands (tars-cli) ✅ COMPLETE

- [x] T026 [US1] Implement `tars mcp add` command with all options in crates/tars-cli/src/commands/mcp.rs
- [x] T027 [US2] Implement `tars mcp remove` command in crates/tars-cli/src/commands/mcp.rs
- [x] T028 [US1] Implement `tars mcp list` command in crates/tars-cli/src/commands/mcp.rs
- [x] T029 [US1] Add mcp subcommand to main CLI in crates/tars-cli/src/main.rs
- [x] T030 [US1] Implement --json output format for mcp commands in crates/tars-cli/src/commands/mcp.rs
- [x] T031 [US1] Implement --dry-run flag for mcp commands in crates/tars-cli/src/commands/mcp.rs

### Tauri Commands (src-tauri) ✅ COMPLETE

- [x] T032 [US1] Implement mcp_list Tauri command in apps/tars-desktop/src-tauri/src/commands/config.rs
- [x] T033 [US1] Implement mcp_add Tauri command in apps/tars-desktop/src-tauri/src/commands/config.rs
- [x] T034 [US2] Implement mcp_remove Tauri command in apps/tars-desktop/src-tauri/src/commands/config.rs
- [x] T035 [US1] Register mcp commands in apps/tars-desktop/src-tauri/src/lib.rs

### React UI Components ✅ COMPLETE

- [x] T036 [US1] Implement McpPanel.tsx (list view with scope grouping) in apps/tars-desktop/src/components/config/
- [x] T037 [US1] Implement McpForm.tsx (add server form) in apps/tars-desktop/src/components/config/
- [x] T038 [US1] Implement McpListItem.tsx (single server row with actions) - integrated into McpPanel.tsx
- [x] T039 [US2] Add remove action with confirmation to McpPanel.tsx
- [x] T040 [US1] Add MCP panel to main app layout in apps/tars-desktop/src/App.tsx

**Checkpoint**: MCP add/remove fully functional via UI and CLI ✅ COMPLETE

---

## Phase 4: User Story 3+4 - MCP Move/Update (Priority: P2)

**Goal**: Move servers between scopes atomically; update server config without remove/re-add

**Independent Test**: Move server from project to user scope, verify removed from source and added to target; update one field, verify others unchanged

### Core Library (tars-core)

- [ ] T041 [US3] Implement mcp_move() as atomic remove+add in crates/tars-core/src/config/mcp.rs
- [ ] T042 [US3] Implement conflict detection for target scope in crates/tars-core/src/config/mcp.rs
- [ ] T043 [US3] Implement rollback on partial move failure in crates/tars-core/src/config/mcp.rs
- [ ] T044 [US4] Implement mcp_update() with partial field merge in crates/tars-core/src/config/mcp.rs
- [ ] T045 [US4] Implement add-env/remove-env operations in crates/tars-core/src/config/mcp.rs
- [ ] T046 [US4] Implement scope auto-detection when server exists in multiple scopes in crates/tars-core/src/config/mcp.rs

### CLI Commands (tars-cli)

- [ ] T047 [US3] Implement `tars mcp move` command in crates/tars-cli/src/commands/mcp.rs
- [ ] T048 [US4] Implement `tars mcp update` command with all options in crates/tars-cli/src/commands/mcp.rs
- [ ] T049 [US3] Add --force flag for overwrite in move in crates/tars-cli/src/commands/mcp.rs

### Tauri Commands (src-tauri)

- [ ] T050 [US3] Implement mcp_move Tauri command in apps/tars-desktop/src-tauri/src/commands/mcp.rs
- [ ] T051 [US4] Implement mcp_update Tauri command in apps/tars-desktop/src-tauri/src/commands/mcp.rs

### React UI Components

- [ ] T052 [US3] Add move action (scope picker dialog) to McpListItem.tsx
- [ ] T053 [US4] Implement McpEditForm.tsx (edit existing server) in apps/tars-desktop/src/components/config/
- [ ] T054 [US4] Add edit action to McpListItem.tsx

**Checkpoint**: Full MCP server management (add/remove/move/update) complete

---

## Phase 5: User Story 5 - Skills Management (Priority: P3)

**Goal**: Add, remove, and move SKILL.md files between scopes

**Independent Test**: Add a skill, verify SKILL.md created with correct frontmatter; remove it, verify file deleted

### Core Library (tars-core)

- [ ] T055 [P] [US5] Implement skill_list() to read skills from all scopes in crates/tars-core/src/config/skill.rs
- [ ] T056 [US5] Implement skill_add() with frontmatter generation in crates/tars-core/src/config/skill.rs
- [ ] T057 [US5] Implement skill_remove() with file deletion in crates/tars-core/src/config/skill.rs
- [ ] T058 [US5] Implement skill_move() between scopes in crates/tars-core/src/config/skill.rs
- [ ] T059 [US5] Implement SKILL.md frontmatter serialization in crates/tars-core/src/config/skill.rs

### CLI Commands (tars-cli)

- [ ] T060 [P] [US5] Implement `tars skill add` command in crates/tars-cli/src/commands/skill.rs
- [ ] T061 [P] [US5] Implement `tars skill remove` command in crates/tars-cli/src/commands/skill.rs
- [ ] T062 [P] [US5] Implement `tars skill move` command in crates/tars-cli/src/commands/skill.rs
- [ ] T063 [US5] Implement `tars skill list` command in crates/tars-cli/src/commands/skill.rs
- [ ] T064 [US5] Add skill subcommand to main CLI in crates/tars-cli/src/main.rs

### Tauri Commands (src-tauri)

- [ ] T065 [P] [US5] Implement skill Tauri commands in apps/tars-desktop/src-tauri/src/commands/skill.rs
- [ ] T066 [US5] Register skill commands in apps/tars-desktop/src-tauri/src/lib.rs

### React UI Components

- [ ] T067 [US5] Implement SkillPanel.tsx (list view) in apps/tars-desktop/src/components/config/
- [ ] T068 [US5] Implement SkillForm.tsx (add/edit skill) in apps/tars-desktop/src/components/config/
- [ ] T069 [US5] Add Skills tab/panel to app layout in apps/tars-desktop/src/App.tsx

**Checkpoint**: Skills management complete

---

## Phase 6: User Story 6 - Hooks Management (Priority: P3)

**Goal**: Add, remove hooks in settings.json without manual JSON editing

**Independent Test**: Add a PreToolUse hook, verify it appears in settings.json hooks section

### Core Library (tars-core)

- [ ] T070 [P] [US6] Implement hook_list() to read hooks from settings files in crates/tars-core/src/config/hook.rs
- [ ] T071 [US6] Implement hook_add() with settings.json modification in crates/tars-core/src/config/hook.rs
- [ ] T072 [US6] Implement hook_remove() by trigger+index in crates/tars-core/src/config/hook.rs
- [ ] T073 [US6] Implement hooks section creation if missing in crates/tars-core/src/config/hook.rs

### CLI Commands (tars-cli)

- [ ] T074 [P] [US6] Implement `tars hook add` command in crates/tars-cli/src/commands/hook.rs
- [ ] T075 [P] [US6] Implement `tars hook remove` command in crates/tars-cli/src/commands/hook.rs
- [ ] T076 [US6] Implement `tars hook list` command in crates/tars-cli/src/commands/hook.rs
- [ ] T077 [US6] Add hook subcommand to main CLI in crates/tars-cli/src/main.rs

### Tauri Commands (src-tauri)

- [ ] T078 [P] [US6] Implement hook Tauri commands in apps/tars-desktop/src-tauri/src/commands/hook.rs
- [ ] T079 [US6] Register hook commands in apps/tars-desktop/src-tauri/src/lib.rs

### React UI Components

- [ ] T080 [US6] Implement HookPanel.tsx (list view grouped by trigger) in apps/tars-desktop/src/components/config/
- [ ] T081 [US6] Implement HookForm.tsx (add hook form) in apps/tars-desktop/src/components/config/
- [ ] T082 [US6] Add Hooks tab/panel to app layout in apps/tars-desktop/src/App.tsx

**Checkpoint**: Hooks management complete

---

## Phase 7: User Story 7 - Commands and Agents (Priority: P4)

**Goal**: Manage custom commands and agents with granular operations

**Independent Test**: Add a command, verify .md file created; move agent between scopes, verify file moved

### Core Library (tars-core)

- [ ] T083 [P] [US7] Implement command_list/add/remove/move in crates/tars-core/src/config/command.rs
- [ ] T084 [P] [US7] Implement agent_list/add/remove/move in crates/tars-core/src/config/agent.rs
- [ ] T085 [US7] Implement command .md frontmatter serialization in crates/tars-core/src/config/command.rs
- [ ] T086 [US7] Implement agent .md frontmatter serialization in crates/tars-core/src/config/agent.rs

### CLI Commands (tars-cli)

- [ ] T087 [P] [US7] Implement `tars command add/remove/move/list` in crates/tars-cli/src/commands/command.rs
- [ ] T088 [P] [US7] Implement `tars agent add/remove/move/list` in crates/tars-cli/src/commands/agent.rs
- [ ] T089 [US7] Add command and agent subcommands to main CLI in crates/tars-cli/src/main.rs

### Tauri Commands (src-tauri)

- [ ] T090 [P] [US7] Implement command Tauri commands in apps/tars-desktop/src-tauri/src/commands/command.rs
- [ ] T091 [P] [US7] Implement agent Tauri commands in apps/tars-desktop/src-tauri/src/commands/agent.rs
- [ ] T092 [US7] Register command and agent commands in apps/tars-desktop/src-tauri/src/lib.rs

### React UI Components

- [ ] T093 [P] [US7] Implement CommandPanel.tsx in apps/tars-desktop/src/components/config/
- [ ] T094 [P] [US7] Implement AgentPanel.tsx in apps/tars-desktop/src/components/config/
- [ ] T095 [US7] Add Commands and Agents tabs to app layout in apps/tars-desktop/src/App.tsx

**Checkpoint**: All config types manageable

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Integration, testing, and refinements

- [ ] T096 [P] Implement config_rollback Tauri command in apps/tars-desktop/src-tauri/src/commands/mod.rs
- [ ] T097 [P] Add rollback UI (backup list, restore button) in apps/tars-desktop/src/components/config/
- [ ] T098 Integration test: MCP add/remove round-trip in crates/tars-core/tests/config/mcp_tests.rs
- [ ] T099 Integration test: Skill add/move/remove round-trip in crates/tars-core/tests/config/skill_tests.rs
- [ ] T100 Integration test: Hook add/remove round-trip in crates/tars-core/tests/config/hook_tests.rs
- [ ] T101 [P] Add error toast notifications in apps/tars-desktop/src/components/
- [ ] T102 [P] Add loading states to all panels in apps/tars-desktop/src/components/config/
- [ ] T103 Run quickstart.md validation - verify all CLI commands work as documented

---

## Dependencies & Execution Order

### Phase Dependencies

```
Phase 1 (Setup) → Phase 2 (Foundational) → [User Stories can proceed]
                                         ↓
                        ┌────────────────┼────────────────┐
                        ↓                ↓                ↓
                    Phase 3          Phase 5          Phase 7
                   (US1+US2)          (US5)            (US7)
                    MCP Add/          Skills          Cmd/Agent
                    Remove             ↓                ↓
                        ↓          Phase 6              │
                    Phase 4         (US6)               │
                   (US3+US4)        Hooks               │
                    MCP Move/          ↓                ↓
                    Update             └────────────────┘
                        ↓                      ↓
                        └──────────────────────┘
                                   ↓
                            Phase 8 (Polish)
```

### User Story Dependencies

- **US1+US2 (P1)**: Can start after Foundational - No dependencies
- **US3+US4 (P2)**: Depends on US1+US2 (uses mcp_add/mcp_remove internally)
- **US5 (P3)**: Can start after Foundational - Independent of MCP stories
- **US6 (P3)**: Can start after Foundational - Independent of MCP/Skills
- **US7 (P4)**: Can start after Foundational - Independent of others

### Parallel Opportunities

Within each phase, tasks marked [P] can run in parallel:

**Phase 2 (Foundational)**:
- T007, T008, T009, T010 (all item types)
- T015, T016, T017 (all shared UI components)

**Phase 5 (Skills)**:
- T055 (skill_list), T060-T062 (CLI commands), T065 (Tauri)

**Phase 7 (Commands/Agents)**:
- T083, T084 (core), T087, T088 (CLI), T090, T091 (Tauri), T093, T094 (UI)

---

## Parallel Example: Phase 3 (MCP Add/Remove)

```bash
# Core library tasks - sequential (shared file mcp.rs)
T018 → T019 → T020 → T021 → T022 → T023 → T024 → T025

# CLI tasks - can start after T025
T026, T027, T028 → T029 → T030, T031

# Tauri tasks - can start after T025
T032, T033, T034 → T035

# React tasks - can start after Tauri commands ready
T036 → T037 → T038 → T039 → T040
```

---

## Implementation Strategy

### MVP First (Phase 3 Only)

1. Complete Phase 1: Setup (5 tasks)
2. Complete Phase 2: Foundational (12 tasks)
3. Complete Phase 3: US1+US2 - MCP Add/Remove (23 tasks)
4. **STOP and VALIDATE**: Test MCP add/remove via UI and CLI
5. **Deploy MVP**: Users can add/remove MCP servers

### Incremental Delivery

| Milestone | Phases | Value Delivered |
|-----------|--------|-----------------|
| MVP | 1-3 | MCP add/remove |
| v0.2 | + Phase 4 | Full MCP management (move/update) |
| v0.3 | + Phase 5 | Skills management |
| v0.4 | + Phase 6 | Hooks management |
| v0.5 | + Phase 7-8 | Commands, Agents, Polish |

### Task Count Summary

| Phase | Tasks | Cumulative |
|-------|-------|------------|
| Setup | 5 | 5 |
| Foundational | 12 | 17 |
| US1+US2 (MCP Add/Remove) | 23 | 40 |
| US3+US4 (MCP Move/Update) | 14 | 54 |
| US5 (Skills) | 15 | 69 |
| US6 (Hooks) | 13 | 82 |
| US7 (Commands/Agents) | 13 | 95 |
| Polish | 8 | 103 |

**Total: 103 tasks**

---

## Notes

- All operations must create backups via existing BackupStore
- All Tauri commands must support dryRun parameter for preview
- All CLI commands must support --dry-run and --json flags
- React components should show diff preview before destructive operations
- Use existing scanner types where possible (SkillInfo, McpServer, etc.)
- Commit after completing each user story phase
