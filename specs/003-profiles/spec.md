# Feature Specification: Profiles

**Feature Branch**: `003-profiles`
**Created**: 2026-01-10
**Status**: Draft
**Input**: User description: "Profiles feature - A profile is a curated template of tools (MCP servers, skills, agents, hooks, permissions) for a specific workflow like 'Rust Development' or 'iOS Mobile'. Users can create profiles in their profile library, then assign profiles to projects. Profiles are linked (not copied) so when a profile updates, all assigned projects auto-sync. Projects can have local additions on top of their assigned profile."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Create a Profile (Priority: P1)

A user wants to create a reusable profile for their Rust development workflow. They open TARS, navigate to the Profiles section, and create a new profile named "Rust Development". They add the rust-analyzer MCP server, a `/cargo-test` skill, and a pre-commit hook for `cargo fmt`. They save the profile to their library.

**Why this priority**: Creating profiles is the foundational capability. Without the ability to create profiles, no other profile features can function.

**Independent Test**: Can be fully tested by creating a profile with tools and verifying it persists in `~/.tars/profiles/`. Delivers value as a personal tool organization system even before project assignment works.

**Acceptance Scenarios**:

1. **Given** a user is on the Profiles view, **When** they click "Create Profile" and enter a name, **Then** a new empty profile is created and displayed in the profile list
2. **Given** a user is editing a profile, **When** they add MCP servers, skills, agents, or hooks from the discovered inventory, **Then** those tools are saved to the profile
3. **Given** a user has created a profile, **When** they close and reopen TARS, **Then** the profile persists with all configured tools

---

### User Story 2 - Assign Profile to Project (Priority: P1)

A user imports a new Rust project into TARS. During or after import, they assign their "Rust Development" profile to the project. The project now has access to all tools defined in that profile.

**Why this priority**: Assigning profiles to projects is the core value proposition - applying curated toolsets to projects quickly.

**Independent Test**: Can be tested by assigning a profile to a project and verifying the project's effective configuration includes the profile's tools.

**Acceptance Scenarios**:

1. **Given** a project exists in TARS and a profile exists, **When** the user assigns the profile to the project, **Then** the project shows the profile name and displays all profile tools as available
2. **Given** a project has an assigned profile, **When** the user views the project's tool inventory, **Then** tools from the profile are visually distinguished (e.g., badge showing "from profile")
3. **Given** a project has an assigned profile, **When** the user unassigns the profile, **Then** the profile tools are removed from the project's effective configuration

---

### User Story 3 - Profile Auto-Sync (Priority: P2)

A user updates their "Rust Development" profile by adding a new skill. All projects using that profile automatically receive the new skill. The user sees a notification about the sync.

**Why this priority**: Auto-sync differentiates linked profiles from one-time templates. It's important but the core assign/create flows must work first.

**Independent Test**: Can be tested by modifying a profile and verifying assigned projects reflect the change, with a notification shown.

**Acceptance Scenarios**:

1. **Given** a profile is assigned to multiple projects, **When** the user adds a tool to the profile, **Then** all assigned projects include the new tool in their effective configuration
2. **Given** a profile is modified, **When** the sync occurs, **Then** a notification appears showing "Profile 'X' updated - N projects affected"
3. **Given** a profile has a tool removed, **When** the sync occurs, **Then** assigned projects no longer include that tool

---

### User Story 4 - Local Overrides (Priority: P2)

A user has the "Rust Development" profile assigned to a project, but this specific project also needs a custom MCP server for database access. They add the database MCP as a local override. When the profile updates, their local addition is preserved.

**Why this priority**: Local overrides allow customization without breaking the profile link, making profiles flexible enough for real-world use.

**Independent Test**: Can be tested by adding a local tool to a project with an assigned profile, then modifying the profile and verifying the local tool persists.

**Acceptance Scenarios**:

1. **Given** a project has an assigned profile, **When** the user adds a tool directly to the project, **Then** it is saved as a local override separate from profile tools
2. **Given** a project has local overrides, **When** the profile syncs, **Then** local overrides are preserved and merged with profile tools
3. **Given** a project has a local override, **When** viewing the project inventory, **Then** local overrides are visually distinguished from profile tools (e.g., "local" badge)

---

### User Story 5 - Export/Import Profiles (Priority: P3)

A user wants to share their "Rust Development" profile with a teammate. They export it as a `.tars-profile.json` file. The teammate imports the file into their TARS instance, creating a copy in their profile library.

**Why this priority**: Sharing is valuable but not essential for individual productivity. Core create/assign/sync must work first.

**Independent Test**: Can be tested by exporting a profile to a file and importing it on a fresh TARS instance, verifying all tools are present.

**Acceptance Scenarios**:

1. **Given** a profile exists, **When** the user clicks "Export", **Then** a `.tars-profile.json` file is saved containing all profile data
2. **Given** a user has a `.tars-profile.json` file, **When** they import it into TARS, **Then** a new profile is created in their library with all tools from the file
3. **Given** a profile with the same name exists, **When** importing a profile file, **Then** the user is prompted to rename, replace, or cancel

---

### User Story 6 - Configure Tool Permissions in Profile (Priority: P3)

A user wants their "Rust Development" profile to include the rust-analyzer MCP with restricted directory access (only `./src` and `./tests`). They configure these permissions as part of the profile. When assigned to a project, the MCP runs with those restrictions.

**Why this priority**: Permissions add security value but profiles work without them. This can be added after core flows are stable.

**Independent Test**: Can be tested by creating a profile with permission-restricted tools and verifying the permissions are applied when the profile is assigned.

**Acceptance Scenarios**:

1. **Given** a user is editing a profile, **When** they select an MCP server, **Then** they can configure directory access restrictions
2. **Given** a profile has permission-restricted tools, **When** assigned to a project, **Then** the generated configuration includes those restrictions
3. **Given** a profile with permissions is exported, **When** imported elsewhere, **Then** the permissions are preserved

---

### Edge Cases

- What happens when a profile is deleted that is assigned to projects? Projects lose the profile link and keep a snapshot of tools as local overrides.
- What happens when a tool in a profile references an MCP server that doesn't exist on the target machine? Show warning, skip that tool during sync.
- What happens when two profiles are assigned to the same project? Not supported in v1 - one profile per project.
- What happens when a project's local override conflicts with a profile tool? Local override takes precedence.
- What happens when importing a profile with tools that reference unavailable skills? Import succeeds, unavailable tools shown with warning badge.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST allow users to create, read, update, and delete profiles in `~/.tars/profiles/`
- **FR-002**: System MUST store each profile as a separate JSON file with a unique identifier
- **FR-003**: Profiles MUST support including MCP servers, skills, agents, and hooks
- **FR-004**: System MUST allow assigning exactly one profile to a project
- **FR-005**: System MUST allow unassigning a profile from a project
- **FR-006**: System MUST track which tools came from a profile vs. local additions per project
- **FR-007**: System MUST automatically sync profile changes to all assigned projects
- **FR-008**: System MUST display a notification when profile sync affects projects
- **FR-009**: System MUST preserve local overrides when syncing profile changes
- **FR-010**: System MUST allow exporting a profile as a `.tars-profile.json` file
- **FR-011**: System MUST allow importing a `.tars-profile.json` file to create a new profile
- **FR-012**: System MUST prompt for resolution when importing a profile with a duplicate name
- **FR-013**: Profiles MUST support configuring tool permissions (e.g., directory restrictions for MCP servers)
- **FR-014**: System MUST visually distinguish profile tools from local tools in the project view
- **FR-015**: When a profile is deleted, assigned projects MUST convert profile tools to local overrides
- **FR-016**: System MUST warn when a profile references tools unavailable on the current machine

### Key Entities

- **Profile**: A named, reusable collection of tool configurations. Contains id, name, description, lists of MCP servers, skills, agents, hooks, and optional permission settings. Stored in `~/.tars/profiles/{id}.json`.
- **ProfileAssignment**: Links a project to a profile. Contains project ID, profile ID, and timestamp. Stored in project metadata.
- **LocalOverride**: Project-specific tool additions that exist alongside profile tools. Stored separately from profile-derived tools in project configuration.
- **ToolReference**: A pointer to a specific tool (MCP, skill, agent, hook) with optional permission configuration. Used within profiles.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can create a profile and add tools in under 2 minutes
- **SC-002**: Users can assign a profile to a project in under 10 seconds
- **SC-003**: Profile changes sync to all assigned projects within 5 seconds of modification
- **SC-004**: Users can export and import a profile in under 30 seconds total
- **SC-005**: 100% of profile tools are correctly applied to assigned projects (no tool loss during sync)
- **SC-006**: Local overrides persist through at least 10 consecutive profile sync operations
- **SC-007**: Users can distinguish profile tools from local tools at a glance (visual differentiation)
- **SC-008**: System correctly handles profile deletion by preserving tools as local overrides in 100% of cases

## Assumptions

- Users have TARS installed with scanner functionality working (can discover available tools)
- The project database exists and can store profile assignments
- File system access to `~/.tars/profiles/` is available
- Only one profile per project is needed for v1 (no stacking/inheritance)
- Profile sync is triggered on profile save, not on a background schedule
- Notifications appear within the TARS app (not system notifications)

## Out of Scope

- Profile inheritance (e.g., "Rust + WASM" extending "Rust")
- Multiple profiles per project
- Cloud sync of profiles between machines
- Profile versioning/history
- Community profile marketplace
- Automatic profile suggestions based on project type
