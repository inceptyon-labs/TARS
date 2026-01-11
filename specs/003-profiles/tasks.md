# Tasks: Profiles

**Input**: Design documents from `/specs/003-profiles/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/tauri-ipc.md

**Tests**: Not explicitly requested - tests will be added in Polish phase for key functionality.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md:
- **Backend (Rust)**: `crates/tars-core/src/`, `apps/tars-desktop/src-tauri/src/`
- **Frontend (React)**: `apps/tars-desktop/src/`
- **Tests**: `crates/tars-core/tests/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Extend existing types and create foundational data structures

- [x] T001 [P] Add ToolType enum to crates/tars-core/src/profile/types.rs
- [x] T002 [P] Add ToolRef struct to crates/tars-core/src/profile/types.rs
- [x] T003 [P] Add ToolPermissions struct to crates/tars-core/src/profile/types.rs
- [x] T004 [P] Add LocalOverrides struct to crates/tars-core/src/project.rs
- [x] T005 Add tool_refs field to Profile struct in crates/tars-core/src/profile/types.rs
- [x] T006 Add local_overrides field to Project struct in crates/tars-core/src/project.rs
- [x] T007 [P] Add TypeScript types for ToolRef, ToolPermissions, LocalOverrides in apps/tars-desktop/src/lib/types/index.ts

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T008 Create profile sync module skeleton in crates/tars-core/src/profile/sync.rs
- [ ] T009 Add sync module to crates/tars-core/src/profile/mod.rs exports
- [ ] T010 [P] Create Tauri commands module structure in apps/tars-desktop/src-tauri/src/commands/mod.rs
- [ ] T011 [P] Create profile commands file skeleton in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T012 Register commands module in apps/tars-desktop/src-tauri/src/lib.rs
- [ ] T013 Add query for projects by profile_id in crates/tars-core/src/storage/projects.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Create a Profile (Priority: P1) 🎯 MVP

**Goal**: Users can create profiles with name/description and add tools from discovered inventory

**Independent Test**: Create a profile, add tools, close app, reopen - profile persists with all tools

### Implementation for User Story 1

- [ ] T014 [US1] Implement create_profile Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T015 [US1] Implement list_profiles Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T016 [US1] Implement get_profile Tauri command (with tool_refs) in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T017 [US1] Implement update_profile Tauri command (update name, description, tool_refs) in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T018 [US1] Implement delete_profile Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T019 [P] [US1] Add IPC wrapper functions for profile CRUD in apps/tars-desktop/src/lib/ipc/index.ts
- [ ] T020 [US1] Create ProfileToolPicker component for selecting tools from inventory in apps/tars-desktop/src/components/ProfileToolPicker.tsx
- [ ] T021 [US1] Extend ProfileDetail component to display tool_refs list in apps/tars-desktop/src/components/ProfileDetail.tsx
- [ ] T022 [US1] Add "Add Tool" button to ProfileDetail that opens ProfileToolPicker in apps/tars-desktop/src/components/ProfileDetail.tsx
- [ ] T023 [US1] Extend CreateProfileDialog to support description field in apps/tars-desktop/src/components/CreateProfileDialog.tsx
- [ ] T024 [US1] Update ProfileList to show tool count per profile in apps/tars-desktop/src/components/ProfileList.tsx

**Checkpoint**: User Story 1 complete - profiles can be created, tools added, and persist across restarts

---

## Phase 4: User Story 2 - Assign Profile to Project (Priority: P1)

**Goal**: Users can assign a profile to a project, see profile tools with "(from profile)" badge, and unassign

**Independent Test**: Assign profile to project, verify tools show with badge, unassign and verify tools removed

### Implementation for User Story 2

- [ ] T025 [US2] Implement assign_profile Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T026 [US2] Implement unassign_profile Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T027 [US2] Implement get_project_tools Tauri command (combines profile + local) in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T028 [P] [US2] Add IPC wrapper functions for profile assignment in apps/tars-desktop/src/lib/ipc/index.ts
- [ ] T029 [US2] Create AssignProfileDialog component for profile selection in apps/tars-desktop/src/components/AssignProfileDialog.tsx
- [ ] T030 [US2] Add profile column/badge to project list in apps/tars-desktop/src/pages/ProjectsPage.tsx
- [ ] T031 [US2] Add "Assign Profile" button to project view that opens AssignProfileDialog in apps/tars-desktop/src/pages/ProjectsPage.tsx
- [ ] T032 [US2] Update ProjectOverview to show tools with source badges ("from profile" vs "local") in apps/tars-desktop/src/components/ProjectOverview.tsx
- [ ] T033 [US2] Show assigned projects list in ProfileDetail component in apps/tars-desktop/src/components/ProfileDetail.tsx

**Checkpoint**: User Story 2 complete - profiles can be assigned to projects with visual distinction

---

## Phase 5: User Story 3 - Profile Auto-Sync (Priority: P2)

**Goal**: Profile changes automatically sync to all assigned projects with notification

**Independent Test**: Assign profile to 2 projects, update profile, verify both projects reflect change, see notification

### Implementation for User Story 3

- [ ] T034 [US3] Implement sync_profile_to_projects function in crates/tars-core/src/profile/sync.rs
- [ ] T035 [US3] Add sync call to update_profile command (after profile save) in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T036 [US3] Return SyncResult from update_profile command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T037 [US3] Add toast notification on profile sync in apps/tars-desktop/src/pages/ProfilesPage.tsx
- [ ] T038 [US3] Invalidate project queries after profile update in apps/tars-desktop/src/pages/ProfilesPage.tsx

**Checkpoint**: User Story 3 complete - profile changes auto-sync to projects with notifications

---

## Phase 6: User Story 4 - Local Overrides (Priority: P2)

**Goal**: Projects can have local tool additions that persist through profile sync

**Independent Test**: Assign profile, add local tool, update profile, verify local tool persists alongside profile tools

### Implementation for User Story 4

- [ ] T039 [US4] Implement add_local_tool Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T040 [US4] Implement remove_local_tool Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T041 [US4] Update sync logic to preserve local_overrides in crates/tars-core/src/profile/sync.rs
- [ ] T042 [P] [US4] Add IPC wrapper functions for local overrides in apps/tars-desktop/src/lib/ipc/index.ts
- [ ] T043 [US4] Add "Add Local Tool" button to ProjectOverview in apps/tars-desktop/src/components/ProjectOverview.tsx
- [ ] T044 [US4] Update tool list to show "(local)" badge for local overrides in apps/tars-desktop/src/components/ProjectOverview.tsx
- [ ] T045 [US4] Add remove button for local tools in ProjectOverview in apps/tars-desktop/src/components/ProjectOverview.tsx

**Checkpoint**: User Story 4 complete - local overrides work alongside profile tools

---

## Phase 7: User Story 5 - Export/Import Profiles (Priority: P3)

**Goal**: Users can export profiles to .tars-profile.json and import from file

**Independent Test**: Export profile, delete profile, import from file, verify restored with all tools

### Implementation for User Story 5

- [ ] T046 [P] [US5] Create ProfileExport struct in crates/tars-core/src/profile/export.rs
- [ ] T047 [P] [US5] Create ExportedTool struct in crates/tars-core/src/profile/export.rs
- [ ] T048 [US5] Implement export_profile function in crates/tars-core/src/profile/export.rs
- [ ] T049 [US5] Implement import_profile function in crates/tars-core/src/profile/export.rs
- [ ] T050 [US5] Implement preview_import function in crates/tars-core/src/profile/export.rs
- [ ] T051 [US5] Add export module to crates/tars-core/src/profile/mod.rs exports
- [ ] T052 [US5] Implement export_profile Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T053 [US5] Implement import_profile Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T054 [US5] Implement preview_import Tauri command in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T055 [P] [US5] Add IPC wrapper functions for export/import in apps/tars-desktop/src/lib/ipc/index.ts
- [ ] T056 [US5] Add "Export" button to ProfileDetail in apps/tars-desktop/src/components/ProfileDetail.tsx
- [ ] T057 [US5] Add "Import Profile" button to ProfilesPage header in apps/tars-desktop/src/pages/ProfilesPage.tsx
- [ ] T058 [US5] Create ImportProfileDialog with collision handling in apps/tars-desktop/src/components/ImportProfileDialog.tsx

**Checkpoint**: User Story 5 complete - profiles can be shared via file export/import

---

## Phase 8: User Story 6 - Configure Tool Permissions (Priority: P3)

**Goal**: Users can configure directory restrictions and allowed/disallowed tools for profile items

**Independent Test**: Add tool with directory restriction, assign to project, verify permission appears in tool config

### Implementation for User Story 6

- [ ] T059 [US6] Create ToolPermissionsEditor component for editing permissions in apps/tars-desktop/src/components/ToolPermissionsEditor.tsx
- [ ] T060 [US6] Integrate ToolPermissionsEditor into ProfileToolPicker in apps/tars-desktop/src/components/ProfileToolPicker.tsx
- [ ] T061 [US6] Display permissions in tool list items in ProfileDetail in apps/tars-desktop/src/components/ProfileDetail.tsx
- [ ] T062 [US6] Display permissions in tool list items in ProjectOverview in apps/tars-desktop/src/components/ProjectOverview.tsx
- [ ] T063 [US6] Ensure permissions are preserved in export/import in crates/tars-core/src/profile/export.rs

**Checkpoint**: User Story 6 complete - tool permissions can be configured per-profile

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Handle edge cases, improve UX, add tests for critical paths

### Edge Case Handling

- [ ] T064 Implement profile deletion with local override conversion (FR-015) in apps/tars-desktop/src-tauri/src/commands/profiles.rs
- [ ] T065 Add warning badge for unavailable tools (FR-016) in apps/tars-desktop/src/components/ProfileDetail.tsx
- [ ] T066 Handle name collision on import with rename prompt in apps/tars-desktop/src/components/ImportProfileDialog.tsx

### Tests (Critical Paths)

- [ ] T067 [P] Add unit tests for ToolRef and LocalOverrides serialization in crates/tars-core/src/profile/types.rs
- [ ] T068 [P] Add unit tests for export/import round-trip in crates/tars-core/tests/profile_export_test.rs
- [ ] T069 Add integration test for profile sync in crates/tars-core/tests/profile_sync_test.rs

### Documentation

- [ ] T070 Update quickstart.md with actual screenshots/commands in specs/003-profiles/quickstart.md
- [ ] T071 Run build verification: cargo build && bun run build

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-8)**: All depend on Foundational phase completion
  - US1 and US2 are both P1 - complete US1 first (MVP)
  - US3 depends on US2 (need assignment before sync matters)
  - US4 depends on US2 (need assignment before local overrides matter)
  - US5 and US6 are independent P3 features
- **Polish (Phase 9)**: Depends on US1-US4 being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational - No dependencies on other stories (but recommended after US1)
- **User Story 3 (P2)**: Depends on US2 (assignment must work for sync to matter)
- **User Story 4 (P2)**: Depends on US2 (assignment must work for local overrides to make sense)
- **User Story 5 (P3)**: Can start after Foundational - Independent
- **User Story 6 (P3)**: Can start after US1 (need tool picker first)

### Within Each User Story

- Backend commands before frontend components
- IPC wrappers in parallel with backend
- Core components before integration

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T001-T004, T007)
- Foundational [P] tasks can run in parallel (T010-T011)
- IPC wrapper tasks are parallelizable with backend commands
- Export structs (T046-T047) can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch T019 (IPC wrappers) in parallel with T014-T018 (backend commands)
# Once both complete, T020-T024 (frontend) can proceed

# Backend (can run in parallel):
T014: create_profile command
T015: list_profiles command
T016: get_profile command
T017: update_profile command
T018: delete_profile command

# Parallel with backend:
T019: IPC wrappers in apps/tars-desktop/src/lib/ipc/index.ts

# After both complete:
T020-T024: Frontend components
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete Phase 1: Setup (types and structs)
2. Complete Phase 2: Foundational (command scaffolding)
3. Complete Phase 3: User Story 1 (profile CRUD with tools)
4. **CHECKPOINT**: Test profile creation and tool addition
5. Complete Phase 4: User Story 2 (profile assignment)
6. **CHECKPOINT**: Test full assign/unassign flow
7. Deploy/demo as MVP

### Incremental Delivery

1. **MVP**: Setup → Foundational → US1 → US2 ✅
2. **v1.1**: US3 (auto-sync) + US4 (local overrides)
3. **v1.2**: US5 (export/import) + US6 (permissions)
4. **Polish**: Edge cases, tests, documentation

---

## Summary

| Metric | Count |
|--------|-------|
| Total Tasks | 71 |
| Setup Tasks | 7 |
| Foundational Tasks | 6 |
| US1 Tasks | 11 |
| US2 Tasks | 9 |
| US3 Tasks | 5 |
| US4 Tasks | 7 |
| US5 Tasks | 13 |
| US6 Tasks | 5 |
| Polish Tasks | 8 |
| Parallel Opportunities | 15 tasks marked [P] |

**Suggested MVP Scope**: Phase 1-4 (Setup + Foundational + US1 + US2) = 33 tasks
