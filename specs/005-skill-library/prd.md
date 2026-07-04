# PRD: Cross-Agent Skill Library

**Status:** v1 built and shipped in the reference app (TARS, a Tauri/Rust/React
desktop tool). v2 (muting, full-tree copy drift, version-pin following) is being
built now — see §11, which is no longer a "deferred" list. This document also
ports the design to a Windows environment where the second agent is GitHub
Copilot instead of Codex. Copilot-specific mechanics are intentionally left open
(call them out where they appear); the reference mechanics are concrete enough to
copy.

> ⚠️ **Version-gated features ahead.** The muting feature (§11.1) only works on
> recent agent builds and is a silent no-op on older ones. If your fleet is
> pinned to an older Claude Code / Codex, read the **Version dependencies**
> callout in §11 *before* building the mute UI — you will need a different
> approach, and shipping the toggle blind will produce a control that lies.

---

## 1. Problem

Coding agents (Claude Code, GitHub Copilot, and similar) consume "skills":
folders containing a `SKILL.md` plus optional supporting files. A developer ends
up with the same skills scattered across agents and projects, managed by hand
(symlinks, copies, manual edits). Two delivery mechanisms exist and get
conflated:

- **Plugins**: a bundle installed from a marketplace. Its skills load as a unit
  and toggle as a unit. You cannot enable one skill inside a plugin.
- **Standalone skills**: loose `SKILL.md` folders you drop into an agent's
  skills directory. Each toggles individually.

There was no single place to see what skills exist, where each comes from, and
which agent/project has them active. Worse, a naive "skill manager" makes you
register a plugin's folder as a source, which then offers to deploy skills the
plugin already provides (double-listing, name collisions).

## 2. Goal

One surface that answers, at a glance: *which skills exist, where does each come
from (plugin vs standalone), and is it active for this agent in this scope?* With
one click to turn a skill (or a whole plugin's worth of skills) on or off for an
agent, per project or globally — and (v2) to *mute* a skill that is present but
should be hidden from the model in a given scope.

## 3. Concepts and vocabulary

- **Skill**: a directory containing `SKILL.md` (YAML frontmatter with required
  `name` and `description`) plus optional supporting files (`scripts/`,
  `references/`, `assets/`, other docs) used for progressive disclosure. The
  whole directory is the unit, not just the `.md`.
- **Source**: a directory the user registers so the Library scans it for
  standalone skills. A source can *be* a skill (SKILL.md at its root) or
  *contain* skills (`<source>/<name>/SKILL.md`).
- **Plugin**: a marketplace-installed bundle owned by an agent. The Library reads
  installed plugins but does not install them (that stays in the agent's own
  marketplace/plugin UI).
- **Agent**: a consumer of skills. Reference build has two: Claude Code
  (plugin-capable) and Codex. Your build has the primary agent (whatever plays
  Claude Code's role, plugin-capable) and GitHub Copilot.
- **Scope**: `user` (global, all projects) or `project` (one repo).
- **Deployment**: the act of making a skill available to an agent in a scope, by
  materializing the skill folder into that agent's skills directory (symlink by
  default, copy as a fallback). A deployment's *presence on disk is its on/off
  state*; nothing is written to the agent's settings files for on/off.
- **Mute (v2)**: a skill that is *deployed* (present on disk) but suppressed from
  the model in a scope via the agent's settings file. This is the one feature
  that writes settings, and it is strongly version-gated (§11).

## 4. The agent adapter (the portable abstraction)

Everything agent-specific goes behind an adapter. To add an agent (Copilot), fill
in one adapter. An adapter answers:

| Adapter capability | Claude Code (reference) | Codex (reference) | **GitHub Copilot (your build: resolve these)** |
|---|---|---|---|
| User skills dir | `~/.claude/skills/` | detected: `~/.codex/skills/` or `~/.agents/skills/` | **?** Does Copilot load `SKILL.md` folders at all? From where? |
| Project skills dir | `<repo>/.claude/skills/` | `<repo>/.agents/skills/` | **?** e.g. `.github/` conventions, prompt files, custom instructions |
| Follows directory symlinks? | yes | yes | **?** (matters for the deploy mechanism on Windows) |
| Has a plugin system? | yes (marketplaces) | discovery only | **?** (Copilot extensions? none? if none, skip plugin auto-listing for it) |
| Installed-plugin skills location | `~/.claude/plugins/.../skills/` | n/a | **?** |
| **Per-skill mute (§11): standalone** | `skillOverrides` in settings.json — **requires Claude Code ≥ 2.1.129**; silent no-op below that | **not supported** — `[[skills.config]]` in `config.toml` is ignored per-project; only the `-c` launch flag works (not a file TARS writes) | **?** (custom-instructions front-matter? none? gate the UI on it) |
| **Per-skill mute: plugin skills** | **exempt** — `skillOverrides` hard-returns `on`; use per-project `enabledPlugins: {"p@mp": false}` (whole plugin) | n/a | **?** |
| **Mute min-version / capability probe** | probe `claude --version`; expose `muteCapable(scope, kind)` | probe `codex --version`; report *file*-based mute unsupported | **?** |

**Key Copilot question to answer first:** does Copilot consume `SKILL.md`-style
skills, or does it use a different customization format (custom instructions,
prompt files, agent files)? If it uses a different format, the adapter's "deploy"
step becomes "render the skill into Copilot's format and write it to Copilot's
location," not "symlink the folder." That is the one place the whole-folder model
may not carry over. Decide this before building the Copilot column.

If Copilot has no plugin system, the plugin auto-listing (Section 7) simply does
not populate a Copilot column; standalone deploy still works.

**The adapter, not the UI, owns the mute-capability answer.** Have the adapter
expose something like `muteSupport(scope, kind) → { supported, minVersion,
mechanism, note }` and gate every mute control on it. The UI must never render a
mute toggle it cannot honor (see §11 version rules).

## 5. Data model

Two tables. (Reference used SQLite; any store works.)

**`skill_sources`** (registered standalone-source directories):

| column | type | notes |
|---|---|---|
| id | integer PK | |
| path | text, unique | absolute, canonicalized |
| label | text, nullable | friendly name; falls back to a short path |
| created_at / updated_at | text | RFC3339 |

**`skill_deployments`** (one row per materialized skill-per-target):

| column | type | notes |
|---|---|---|
| id | integer PK | |
| skill_name | text | deployed folder name |
| source_path | text | absolute path of the skill folder (symlink/copy origin) |
| agent | text | `claude` / `codex` / `copilot` — first-class in the key |
| scope | text | `user` / `project` |
| project_id | text, nullable, FK→projects, ON DELETE CASCADE | null = user scope |
| link_path | text | absolute path TARS created (needed to remove it) |
| link_kind | text | `symlink` / `copy` |
| sha256 | text, nullable | content hash at deploy — **whole bundle** (v2; was SKILL.md-only in v1) for copy drift detection |
| mute_state | text, nullable | v2: null/`on` = fully visible; `name-only` / `user-invocable-only` / `off`. Non-null implies a settings-file write; only valid where the adapter reports mute support |
| created_at / updated_at | text | |

Uniqueness: one deployment per `(agent, scope, project, skill_name)`. Because
NULL project_id (user scope) is distinct-per-NULL in SQL, use an expression
unique index that folds NULL to a sentinel:
`UNIQUE(agent, scope, IFNULL(project_id, ''), skill_name)`. On Windows/SQL Server
or Postgres, use `COALESCE(project_id, '')` in the index.

**Deployment rows survive source removal.** Removing a `skill_sources` row only
stops scanning that folder; it does not undeploy anything or delete symlinks.

## 6. Catalog composition

The catalog the UI shows is the union of two things, grouped:

1. **Installed plugins (auto-listed).** For each installed, enabled plugin on a
   plugin-capable agent, scan the plugin's own skills directory and emit a group.
   The user never registers a plugin's folder as a source. Plugin groups appear
   when installed and disappear when uninstalled/disabled.
2. **Registered standalone sources.** For each `skill_sources` row, scan the
   directory and emit a group.

## 7. Functional requirements

### 7.1 Source management
- Add a source via a folder picker. Validate it exists and is a directory.
  Canonicalize and dedupe by path.
- Remove a source (deletes the row only; deployments/symlinks untouched).

### 7.2 Scanning (both source shapes)
- If `<source>/SKILL.md` exists, the source *is* a single skill. Emit one skill;
  the skill's folder is the source itself.
- Else, each immediate subdirectory containing `SKILL.md` is a skill.
- Parse frontmatter for `name` and `description`. Skip entries that fail to parse
  (missing file, no frontmatter, missing required fields), quietly.
- **Gotcha:** do not reuse a plugin scanner's "parsed skills" if that field is
  actually derived from the plugin's *commands*. Scan the real skills directory
  for `SKILL.md` so names match the catalog's frontmatter names.

### 7.3 Plugin auto-listing
- Enumerate installed, enabled plugins for each plugin-capable agent.
- For each, resolve its skills directory and scan it (same scanner as sources).
- Build a `skill-name → plugin-id` map so any catalog skill an installed plugin
  provides is badged as plugin-owned on that agent (even a standalone skill that
  shares the name; prevents accidental duplicate deploy).
- Skip plugins with no skills directory (e.g. LSP-only plugins) so they do not
  show as empty groups.

### 7.4 Per-skill deploy / undeploy
- Deploy: materialize the skill's whole folder into the target agent's skills dir
  as `<skill_name>`, record a deployment row.
- Undeploy: remove the materialized link/copy, delete the row. **Also clears any
  mute state** (removing the skill's `skillOverrides`/`enabledPlugins` entry) so
  a settings file never keeps a stale reference to a skill that no longer exists.
- Toggling on a skill an agent already gets from a plugin is not offered (badge
  instead of a toggle).

### 7.5 Group-level bulk toggle
- A group (plugin or multi-skill folder) has a tri-state control per agent:
  none deployed / all deployed / partial (indeterminate).
- Clicking deploys all not-yet-deployed eligible skills, or removes all if
  everything is on. Rationale: plugin skills are a workflow; piecemeal breaks it,
  so the default action is all-or-nothing at the group level. Individual control
  stays available by expanding the group.

### 7.6 Adopt-by-name
- When deploying, if a symlink already exists at the target path (regardless of
  where it points), adopt it: record a deployment row for it rather than erroring
  on collision. This picks up hand-made links a user created earlier.
- Reflect adopted (untracked) links in the matrix as "adopted" (deployed, but no
  DB row until first interaction).

### 7.7 Whole-bundle materialization (hard requirement)
- Deploy moves the entire skill directory, not just `SKILL.md`. Supporting files
  (`scripts/`, `references/`, `assets/`, referenced docs) must come along for
  progressive disclosure to work.
- Symlink deploy is inherently faithful (the deployed skill is the real folder).
- Copy deploy recurses over all files and subdirs. Note it skips any symlink
  *nested inside* a skill (rare). Symlink mode has no such caveat.

### 7.8 Collision and destruction safety
- Never overwrite an existing entry at the target that is not a TARS-created
  symlink (do not clobber a hand-placed real skill directory).
- Undeploy of a symlink deployment refuses to delete anything that is not a
  symlink, so an "off" toggle can never delete a real skill folder.

### 7.9 Presence is the on/off toggle
- On/off is symlink present/absent. On/off writes nothing to agent settings
  files. This avoids editing settings/config files (which on the reference build
  held secrets) and sidesteps JSON/TOML re-serialization issues.
- **Mute is the one exception** and is layered on top of presence: a muted skill
  is still present (symlink there), plus a settings-file entry (§11). Mute never
  removes the symlink; unmuting only removes the settings entry.

## 8. Matrix response shape (UI contract)

The read endpoint returns groups, not a flat list:

```
SkillGroup {
  kind: "plugin" | "source"
  label: string
  pluginId: string | null
  sourceRoot: string | null
  singleSkill: boolean        // source IS a skill (SKILL.md at root)
  skills: SkillMatrixRow[]
}

SkillMatrixRow {
  name: string
  description: string
  sourceDir: string           // symlink/copy origin
  <agentKey>: SkillCell        // one per agent, e.g. claude, codex/copilot
}

SkillCell {
  status: "on" | "off" | "adopted" | "collision" | "plugin"
  deployed: boolean
  tracked: boolean            // has a DB row (vs adopted)
  linkKind: string | null
  deploymentId: number | null
  linkPath: string
  pluginId: string | null     // set when status == "plugin"
  drifted: boolean            // v2: copy deploy whose on-disk bundle != deploy-time hash
  muteState: string | null    // v2: null/"on" | "name-only" | "user-invocable-only" | "off"
  muteSupported: boolean      // v2: adapter says this (scope, kind) can be muted on this agent version
}
```

Cell status derivation, per (skill, agent, scope):
1. If a plugin provides the skill for that agent → `plugin` (+ pluginId), no
   toggle. (Mute for plugin skills is a per-project whole-plugin action, §11.1.)
2. Else if a tracked deployment row exists → `on` (and `muteState` reflects the
   settings-file entry, if any).
3. Else probe the target path: a symlink → `adopted`; nothing → `off`; a real
   file/dir → `collision`.

## 9. UX

Project-centric, two panes.

- **Left pane:** scope selector. "User (all projects)" plus one row per project.
- **Right pane:** a sources bar (chips: add/remove standalone sources) and the
  matrix.
- **Matrix columns:** one per agent (Claude + Copilot in your build). Header
  count "(N/M active here)".
- **Rows are grouped:**
  - Plugin group: collapsible, puzzle icon, "plugin" tag. Primary-agent column is
    a badge (plugin-owned); Copilot column is the deployable group toggle. At
    *project* scope the group header also offers "disable here" (writes
    `enabledPlugins` false for that project) when the adapter supports it.
  - Multi-skill source: collapsible, folder icon.
  - Single-skill source: flat row (no collapse), file icon.
  - Choose flat vs group by the `singleSkill` flag, not by skill count. A folder
    that happens to hold one skill still shows as a collapsible folder; a source
    that *is* a skill shows flat.
- **Cell control:** a plugin badge (click → jump to the plugin/marketplace
  surface) or a checkbox toggle. When deployed *and* the adapter reports mute
  support, the toggle carries a small mute affordance (a dropdown / cycle:
  Full → Name-only → Invocable-only → Muted). When mute is unsupported on this
  agent version, the affordance is absent (not disabled-and-lying) — optionally
  with a tooltip explaining the version requirement. Group header cell is a
  tri-state checkbox or a badge if the whole group is plugin-owned.
- **Cross-links:** Library ↔ the agent's plugin/marketplace surface, both ways,
  with plain copy ("Standalone skills live in the Library", "Plugins are managed
  in Marketplace").
- **States:** loading (scanning), error, and an empty state that prompts to add a
  source.

Layout notes learned the hard way: keep the toggle columns pinned right
(`shrink-0`) and let names/descriptions truncate (`min-w-0` + `truncate`) so the
columns never get pushed off-screen. Do not reuse a 1px divider class as a
section container (it collapses the content).

## 10. Windows specifics (your build)

- **Symlinks:** creating directory symlinks on Windows needs Developer Mode or
  elevation. Plan for it:
  - Try a directory symlink (`std::os::windows::fs::symlink_dir` in Rust, or the
    platform API).
  - Fallbacks if that fails: a **directory junction** (no privilege needed, works
    within a volume) or **copy mode**. Recommend junctions as the default Windows
    deploy kind, copy as the always-works fallback. Symlink where allowed.
  - Store the actual kind used in `link_kind` (extend the enum with `junction`).
- **Undeploy on Windows:** a directory symlink/junction is removed with
  `remove_dir`, not `remove_file`. Guard: only remove if it is a reparse point
  (symlink/junction), never a real directory.
- **Paths:** normalize separators; canonicalize; watch the `\\?\` long-path
  prefix from canonicalize and strip it for display.
- **Agent dirs:** resolve the primary agent's and Copilot's skills locations for
  Windows (they differ from the macOS `~/.claude` etc.). This is adapter work.

## 11. v2 features — muting, full-tree drift, pin-following

These were the v1 non-goals. All three are being built. The muting one carries
hard version dependencies; read the callout.

> ### ⚠️ Version dependencies (read before building the mute UI)
>
> Muting depends on undocumented-until-recently agent behavior that **changed
> across versions**. Verified on Claude Code **2.1.201** and Codex **0.142.5**
> (Jan 2026):
>
> | Agent | Mechanism | Min version | Below min | Removes from context? |
> |---|---|---|---|---|
> | Claude Code — standalone skill | `skillOverrides` in `.claude/settings.json` (project wins over user) | **2.1.129** | **silent no-op** (setting ignored outside managed policy) | ✅ yes on ≥ min (`off`; `name-only` trims the description) |
> | Claude Code — plugin skill | `enabledPlugins: {"p@mp": false}` (whole plugin, per-project) | (plugin toggle, broadly available) | — | ✅ strips all that plugin's skills |
> | Codex — any skill (file-based) | `[[skills.config]]` in `config.toml` | — | **project layer ignored at all tested versions** | file route unusable for per-project |
> | Codex — per-project | `codex -c 'skills.config=[{name="X",enabled=false}]'` launch flag | name= form on current CLI | — | ✅ but it is a runtime flag, not a file the tool writes |
>
> **If your fleet is behind, you cannot ship the file-based mute as-is:**
> - **Claude Code < 2.1.129:** `skillOverrides` is dead weight — it writes JSON
>   the agent ignores, producing a toggle that lies. Do **not** write it. Options:
>   (a) hide the mute control (adapter reports `muteSupported=false`); (b) fall
>   back to **project-scope deploys** — instead of "present-but-muted", simply
>   don't deploy the skill to the projects that shouldn't have it (opt-in per
>   project). This is the honest substitute and needs no settings writes.
> - **Codex (any tested version):** the `config.toml` route is a silent no-op
>   per-project. Do not build a Codex file-based mute. Either skip Codex muting
>   or generate a per-project launch wrapper / `direnv` snippet with `-c` and
>   document it — do not pretend a written `config.toml` mutes per project.
> - **Managed policy** (`/Library/Application Support/ClaudeCode/…` etc.) is
>   machine-global and highest-precedence — it cannot scope to one project and
>   its `skillOverrides` handling has been buggy. Not a per-project answer.
>
> **Always probe the installed version and gate the UI.** Run `claude --version`
> / `codex --version` (or read the build's own version file) and have the adapter
> return `muteSupported` accordingly. A mute control the running agent won't honor
> is worse than no control — it silently misleads. Verify end-to-end on the actual
> target build, not from docs (the docs lagged the fix by weeks).

### 11.1 Muting (present-but-hidden)
- **Claude standalone skill:** write `skillOverrides: { "<name>": <state> }` into
  the scope's `.claude/settings.json` (project file for project scope, user file
  for user scope). States: `off` (hide from model + `/`), `user-invocable-only`
  (hide from model, keep `/name`), `name-only` (keep name, drop description —
  token-trim middle gear), `on`/absent (fully visible). Removing the entry unmutes.
- **Claude plugin skill:** `skillOverrides` is ignored for plugin-provided skills.
  The only per-project lever is whole-plugin: `enabledPlugins: {"<plugin>@<mkt>":
  false}` in the project `.claude/settings.json`. Surface this at the plugin
  *group* level ("disable this plugin here"), not per skill.
- **Preserve the file.** Merge into existing settings; never clobber unrelated
  keys. On the reference build the settings/config files can hold secrets and
  hand-authored structure — read-modify-write with a format-preserving editor
  (e.g. `toml_edit` for TOML; a careful JSON merge that keeps key order where it
  matters). Back up before first write.
- **Undeploy clears mute** (§7.4) so no dangling settings entries.

### 11.2 Full-tree copy drift
- v1 hashed only `SKILL.md`, so a changed supporting file in a **copy** deploy did
  not flag as drifted — and nothing compared the hash anyway.
- v2: hash the **whole bundle** at deploy (walk the tree, sort by relative path,
  hash path + content; skip nested symlinks to match copy semantics). Store it.
- On matrix load, for each **copy** deployment, recompute the source bundle hash
  and set `drifted = stored != current`. Surface an indicator + a re-sync action
  (re-copy from source, update the hash). Symlinks/junctions never drift (they
  *are* the source).

### 11.3 Version-pin following
- Installed-plugin skills live under a version-pinned cache path
  (`…/plugins/cache/<mkt>/<plugin>/<version>/skills/<name>/`). Deploying a plugin
  group's skills to the second agent points a symlink there, so a plugin update
  (new version dir) dangles the old link until re-deploy.
- v2: on matrix load, detect deployments whose `source_path` is under the plugin
  cache and whose symlink is dangling or points to a non-current version; repoint
  to the plugin's *current* skills root and update `source_path`. Guard: only ever
  touch a symlink, never a real directory. If no installed plugin still provides
  the skill (plugin uninstalled), leave it — it will read as off/absent.

## 12. Design decisions and rationale

- **Presence = on/off (no settings writes for on/off).** Least surface, no risk to
  settings files, and it matches how these agents actually load skills (by
  directory presence). Mute is the deliberate, version-gated exception.
- **Auto-list plugins; sources are standalone-only.** Users should not register a
  plugin's folder. Plugins come from the marketplace and appear on their own,
  badged, so they cannot be double-deployed.
- **One plugin surface, install-only.** Once the Library owns cross-agent skill
  placement, the plugin/marketplace surface should stay small. The reference build
  pared its Marketplace page to **Claude-Code-only and install-only** (list
  installed marketplaces/plugins; install via Add Marketplace → Available Plugins →
  Install) and **cut two features**: the multi-target *"Add Plugin"* (install once,
  register for Claude + Codex) and *"Managed Plugins"* — a cross-agent **pin** list
  (`plugin_subscriptions`) with reapply/remove. Their only real value was
  cross-agent + pinning, both moot once Codex skills live in the Library and Claude
  plugins are managed in place. Don't build a second plugin-management surface;
  consolidate onto the agent's native one.
- **Second agent is deployable even for plugin skills.** If Copilot has no plugin
  system, mirroring a plugin's skills to Copilot as standalone deployments is the
  point. The primary agent shows a badge; the second agent shows a toggle.
- **Adopt-by-name.** Real setups already have hand-made links. Adopt them instead
  of colliding.
- **Flat vs group by source shape, not count.** "Is this a skill or a folder of
  skills" is a property of what the user pointed at, not how many happen to be
  inside right now.
- **Whole folder, always.** Skills are directories. Deploy the directory.
- **Never ship a control the agent won't honor.** Mute is gated on a live
  capability probe. A toggle that writes settings the running build ignores is a
  trust bug, not a feature. When unsupported, prefer the honest fallback
  (project-scope deploys) over a dead toggle.

## 13. Gotchas from the reference build (save your Opus the trouble)

- A plugin scanner's "parsed skills" was derived from the plugin's *command*
  files, not its skills. Scan the actual skills directory.
- A source can *be* a skill (SKILL.md at root) or *contain* skills. Handle both,
  or single-skill sources silently vanish.
- Installed-plugin skills live under a version-pinned cache path
  (`.../plugins/cache/<mkt>/<plugin>/<version>/skills/<name>/`). Resolve the real
  path from the plugin's install record; do not guess.
- Enabled state matters: a plugin can be installed but disabled. Only list
  enabled plugins.
- **`skillOverrides` was shipped broken and fixed in Claude Code 2.1.129.** Any
  guide, issue, or blog post published before ~Jan 2026 describes the *broken*
  behavior (setting ignored outside managed policy). Trust a live A/B test on the
  target build over docs. `skillOverrides` also hard-returns `on` for
  plugin-provided skills — it only affects standalone skills.
- **Codex `config.toml` per-skill disable is silently ignored per-project** at the
  versions tested; only the `-c` launch flag mutes per project. Writing the file
  looks like it works and does nothing.
- **Cutting the pin/"Managed Plugins" feature has a backend trap.** In the
  reference build the pin (`plugin_subscriptions` table + `add_plugin_to_targets`)
  was *also* the install vehicle for raw non-marketplace sources — TARS regenerated
  a hidden marketplace from the pin list and installed from it. Its helpers
  (`is_http_url`, `resolve_local_plugin_source`, `sync_managed_claude_marketplace`)
  are **shared** with the surviving install flow. So the UI was removed but the
  backend left **dormant**: deleting the "unused" subscription code without first
  untangling those shared helpers breaks normal installs. If your build never
  couples installs to a pin table, you can cut cleaner — but check first.
- On the reference desktop app, a native (installed) build and a dev build can
  both be open. Changes to the compiled backend only take effect after the dev
  harness recompiles and relaunches; frontend hot-reload does not cover backend
  changes. Make sure you are looking at the right window.

## 14. Acceptance criteria

- [ ] Registering a folder that *is* a skill shows one flat row (file icon), both
      agents deployable.
- [ ] Registering a folder that *contains* skills shows a collapsible folder
      group, even with one skill inside.
- [ ] Installed plugins appear as groups without registering their folder;
      uninstalling one removes its group.
- [ ] A skill an installed plugin provides is badged (not toggle) on that agent
      and cannot be double-deployed there.
- [ ] Toggling a group's second-agent control deploys/removes the whole group at
      once; individual control is available when expanded.
- [ ] A hand-made link at the target is adopted, not treated as a collision.
- [ ] Deploying a skill with `scripts/` and `references/` makes those files
      present/reachable at the target (verified by test).
- [ ] Undeploy never deletes a real (non-link) directory, and clears any mute
      entry for the skill.
- [ ] Removing a source hides its skills but leaves deployments and links intact.
- [ ] On Windows, deploy succeeds without elevation (junction or copy fallback),
      and undeploy removes the reparse point safely.
- [ ] **Mute (where supported):** a standalone skill set to `off` disappears from
      the model's skill list in that scope (verified live); set to `name-only` its
      description is dropped; unmute restores it. Undeploy removes the entry.
- [ ] **Mute (version gate):** on an agent build below the mute min-version, the
      mute control is absent (not a dead toggle), and no settings file is written.
- [ ] **Copy drift:** editing a supporting file in a copied deploy flags the row
      as drifted; re-sync clears it. Symlink deploys never flag.
- [ ] **Pin-following:** after a plugin version bump, a previously-deployed plugin
      skill's second-agent link is repointed to the new version and still resolves.
