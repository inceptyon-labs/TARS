# Feature Specification: Config Operations Layer

**Feature Branch**: `002-config-operations`
**Created**: 2026-01-09
**Status**: Draft
**Input**: User description: "Config Operations Layer - Granular CRUD operations for MCP servers, skills, hooks, and other Claude Code configuration items. Enables surgical add/remove/move operations across scopes without overwriting entire config files."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Add MCP Server to Project (Priority: P1)

A developer wants to add a new MCP server to their project configuration without affecting other servers already configured. They specify the server name, command, arguments, and environment variables, and TARS adds it to the appropriate scope's configuration file while preserving all existing entries.

**Why this priority**: This is the most common operation - installing new tooling. Without surgical add, users must manually edit JSON files or risk losing existing configuration.

**Independent Test**: Can be tested by adding a single MCP server to a project with existing servers and verifying both old and new servers appear in the merged config.

**Acceptance Scenarios**:

1. **Given** a project with 3 existing MCP servers in `.mcp.json`, **When** user adds a 4th server via TARS, **Then** the `.mcp.json` contains all 4 servers with correct configuration.
2. **Given** a project with no `.mcp.json` file, **When** user adds an MCP server, **Then** TARS creates the file with proper structure containing just that server.
3. **Given** user adds a server that already exists by name, **When** the operation is attempted, **Then** TARS warns about the conflict and offers to update instead of add.

---

### User Story 2 - Remove MCP Server from Project (Priority: P1)

A developer wants to remove an MCP server that is no longer needed. TARS surgically removes only that server's entry from the configuration file, leaving all other servers intact.

**Why this priority**: Equally important as add - users need to uninstall tools cleanly without manual JSON editing.

**Independent Test**: Can be tested by removing one server from a multi-server config and verifying other servers remain unchanged.

**Acceptance Scenarios**:

1. **Given** a project with 3 MCP servers, **When** user removes one server by name, **Then** the remaining 2 servers are preserved exactly as configured.
2. **Given** user attempts to remove a server that doesn't exist, **When** the operation runs, **Then** TARS reports the server was not found and lists available servers.
3. **Given** removing the last server from a scope, **When** the operation completes, **Then** the config file either becomes empty or is optionally deleted (user choice).

---

### User Story 3 - Move MCP Server Between Scopes (Priority: P2)

A developer has an MCP server configured at project level but wants it available globally (user scope), or vice versa. TARS removes it from the source scope and adds it to the target scope atomically.

**Why this priority**: Common workflow when developers realize a tool should be shared across projects or isolated to one project.

**Independent Test**: Can be tested by moving a server from project to user scope and verifying it's removed from project config and added to user config.

**Acceptance Scenarios**:

1. **Given** an MCP server in project scope, **When** user moves it to user scope, **Then** the server is removed from `.mcp.json` and added to `~/.claude/settings.json` with identical configuration.
2. **Given** the target scope already has a server with the same name, **When** move is attempted, **Then** TARS warns about the conflict and asks whether to overwrite, rename, or cancel.
3. **Given** a move operation fails midway, **When** the error occurs, **Then** TARS rolls back to the original state in both scopes.

---

### User Story 4 - Update MCP Server Configuration (Priority: P2)

A developer needs to change an MCP server's settings - perhaps updating the command path, adding environment variables, or changing arguments. TARS modifies only the specified fields while preserving all other settings.

**Why this priority**: Configuration drift and updates are common; users shouldn't have to remove and re-add to make changes.

**Independent Test**: Can be tested by updating one field of a server config and verifying other fields remain unchanged.

**Acceptance Scenarios**:

1. **Given** an MCP server with 5 configured fields, **When** user updates 1 field, **Then** all other 4 fields remain exactly as they were.
2. **Given** user adds a new environment variable to a server, **When** the operation completes, **Then** existing environment variables are preserved and the new one is added.
3. **Given** user updates a server that exists in multiple scopes, **When** no scope is specified, **Then** TARS asks which scope to modify or provides a list to choose from.

---

### User Story 5 - Add/Remove/Move Skills (Priority: P3)

A developer wants to manage skills (SKILL.md files) with the same granular control as MCP servers - adding new skills, removing unused ones, or moving them between user and project scopes.

**Why this priority**: Skills are the second most commonly managed config type after MCP servers.

**Independent Test**: Can be tested by adding a skill file to a scope and verifying it appears in the scanner inventory.

**Acceptance Scenarios**:

1. **Given** a skill definition (name, description, content), **When** user adds it to project scope, **Then** a properly formatted SKILL.md file is created in `.claude/skills/`.
2. **Given** a skill exists in user scope, **When** user moves it to project scope, **Then** the file is moved from `~/.claude/skills/` to `.claude/skills/`.
3. **Given** user removes a skill, **When** the operation completes, **Then** the SKILL.md file is deleted and no longer appears in scanner inventory.

---

### User Story 6 - Manage Hooks Configuration (Priority: P3)

A developer wants to add, remove, or modify hooks (PreToolUse, PostToolUse, etc.) in their configuration without editing JSON manually.

**Why this priority**: Hooks are powerful but error-prone to configure manually; granular operations reduce mistakes.

**Independent Test**: Can be tested by adding a hook to settings.json and verifying it's triggered appropriately.

**Acceptance Scenarios**:

1. **Given** no hooks configured, **When** user adds a PreToolUse hook, **Then** the hooks section is created in settings.json with the hook properly configured.
2. **Given** existing hooks, **When** user adds another hook of the same type, **Then** both hooks are preserved (hooks can have multiple entries).
3. **Given** user removes a hook, **When** the operation completes, **Then** only that specific hook is removed, others remain intact.

---

### User Story 7 - Manage Commands and Agents (Priority: P4)

A developer wants to manage custom commands (command .md files) and agents (agent .md files) with granular operations.

**Why this priority**: Less frequently modified than MCP servers and skills, but still benefits from granular control.

**Independent Test**: Can be tested by adding a command file and verifying it appears in the commands inventory.

**Acceptance Scenarios**:

1. **Given** a command definition, **When** user adds it to project scope, **Then** a properly formatted .md file is created in `.claude/commands/`.
2. **Given** an agent definition, **When** user adds it to project scope, **Then** a properly formatted .md file is created in `.claude/agents/`.
3. **Given** user moves a command from project to user scope, **When** the operation completes, **Then** the file moves from `.claude/commands/` to `~/.claude/commands/`.

---

### Edge Cases

- What happens when the target config file has invalid JSON? TARS reports the parse error and refuses to modify until fixed.
- How does TARS handle concurrent modifications? TARS detects if the file changed since last read and warns before overwriting.
- What happens when a config item references another that doesn't exist? TARS warns about broken references but allows the operation.
- How are permissions handled for global scope on systems with restricted access? TARS reports permission errors clearly and suggests alternatives.
- What happens when moving a skill that has custom hooks defined? The hooks configuration is optionally moved along with the skill (user choice).

## Requirements *(mandatory)*

### Functional Requirements

**Core Operations**

- **FR-001**: System MUST support adding a single config item to a specified scope without modifying other items in that scope.
- **FR-002**: System MUST support removing a single config item from a specified scope without modifying other items.
- **FR-003**: System MUST support moving a config item from one scope to another as an atomic operation (remove from source + add to target).
- **FR-004**: System MUST support updating specific fields of a config item without affecting other fields.
- **FR-005**: System MUST create parent directories and config files as needed when adding items.

**Config Item Types**

- **FR-006**: System MUST support operations on MCP servers (in `.mcp.json` and `settings.json`).
- **FR-007**: System MUST support operations on skills (SKILL.md files in skills directories).
- **FR-008**: System MUST support operations on hooks (in settings.json hooks sections).
- **FR-009**: System MUST support operations on commands (.md files in commands directories).
- **FR-010**: System MUST support operations on agents (.md files in agents directories).

**Scope Support**

- **FR-011**: System MUST support operations on user scope (`~/.claude/`).
- **FR-012**: System MUST support operations on project scope (`.claude/` and `.mcp.json`).
- **FR-013**: System MUST support operations on local scope (`.claude/settings.local.json`).
- **FR-014**: System MUST handle scope precedence when items exist in multiple scopes.

**Safety & Integrity**

- **FR-015**: System MUST create a backup before any write operation.
- **FR-016**: System MUST validate config item structure before writing.
- **FR-017**: System MUST preserve file formatting (indentation, key order) where possible.
- **FR-018**: System MUST detect and warn about naming conflicts before operations.
- **FR-019**: System MUST support dry-run mode to preview changes without applying them.
- **FR-020**: System MUST support rollback to restore previous state if operation fails or user requests.

**CLI Integration**

- **FR-021**: System MUST expose all operations through CLI commands.
- **FR-022**: System MUST support both interactive and non-interactive (scripted) modes.
- **FR-023**: System MUST output operation results in both human-readable and JSON formats.

### Key Entities

- **ConfigItem**: A single configuration entry (MCP server, skill, hook, command, or agent) with its type, name, scope, and content.
- **ConfigScope**: The location where config is stored - user (`~/.claude/`), project (`.claude/`), local (`.claude/settings.local.json`), or managed (`/Library/Application Support/ClaudeCode/`).
- **ConfigOperation**: An action to perform - add, remove, update, or move - with source item, target scope, and optional parameters.
- **OperationResult**: The outcome of an operation including success/failure status, backup ID, and any warnings or conflicts detected.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can add a new MCP server to a project in under 10 seconds via CLI command, compared to manual JSON editing.
- **SC-002**: Users can move config items between scopes without data loss - 100% of item properties are preserved during move operations.
- **SC-003**: All write operations create backups that enable byte-for-byte restoration within 5 seconds.
- **SC-004**: Dry-run mode accurately predicts 100% of changes that will be made by the actual operation.
- **SC-005**: Users can perform any config operation without directly editing JSON or Markdown files.
- **SC-006**: Conflicting item names are detected and reported before any changes are written in 100% of cases.
- **SC-007**: Failed operations leave the system in its original state (no partial writes).

## Assumptions

- Users have appropriate file system permissions for the scopes they're modifying.
- Config files follow Claude Code's documented JSON schema and frontmatter formats.
- The existing scanner module accurately identifies all config items and their locations.
- Backup storage has sufficient space for config file backups (typically small files).
- Users accept that managed scope (`/Library/Application Support/ClaudeCode/`) is read-only for TARS operations.
