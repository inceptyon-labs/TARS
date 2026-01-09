# Research: TARS Implementation

**Date**: 2026-01-08
**Status**: Complete

## Summary

This document consolidates research findings for the TARS implementation across four key areas: Tauri 2 patterns, Rust workspace organization, YAML frontmatter parsing, and shadcn/ui component architecture.

---

## 1. Tauri 2.x Architecture

### Decision: Domain-based organization with Rust-side state management

**Project Structure:**
```
apps/tars-desktop/
├── src/                      # React frontend
│   ├── components/
│   ├── hooks/
│   │   └── useTauri.ts       # Thin wrappers for Tauri APIs
│   ├── lib/
│   │   ├── ipc/              # TypeScript IPC wrappers by domain
│   │   └── types/shared/     # Generated from Rust via ts-rs
│   └── pages/
├── src-tauri/
│   ├── capabilities/
│   │   └── default.json      # Security permissions
│   ├── src/
│   │   ├── lib.rs            # Main wiring (commands, state, plugins)
│   │   ├── main.rs           # Minimal desktop entrypoint
│   │   ├── commands/         # Domain-grouped Tauri commands
│   │   ├── state/            # App state structs
│   │   └── db/               # Database layer
│   └── Cargo.toml
└── tauri.conf.json
```

**Rationale:**
- Keep `main.rs` minimal; all logic in `lib.rs` (Tauri 2 recommendation)
- Group commands by domain, not by language
- Generate TypeScript types from Rust using `ts-rs` to avoid manual sync

### IPC Command Pattern

```rust
// Async commands with proper error handling
#[tauri::command]
pub async fn scan_project(
    path: String,
    state: State<'_, AppState>
) -> Result<Inventory, String> {
    tars_scanner::scan(&path)
        .await
        .map_err(|e| e.to_string())
}
```

```typescript
// Typed wrapper - never call invoke directly in components
export async function scanProject(path: string): Promise<Inventory> {
  return invoke("scan_project", { path });
}
```

### State Management

**Decision:** Heavy state in Rust, UI-only state in React

- Use `Mutex<Connection>` for SQLite in Rust
- Access via `State<'_, T>` injection in commands
- Use Zustand for UI state (selected project, sidebar open)
- Use TanStack Query for async data caching

### SQLite Integration

```rust
pub fn open_app_db(app: &AppHandle) -> Result<Connection> {
    let db_path = app.path().app_data_dir()?.join("tars.sqlite");
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    run_migrations(&conn)?;
    Ok(conn)
}
```

**Rationale:**
- Use `bundled` feature for cross-platform compatibility
- Single `Mutex<Connection>` sufficient for desktop app
- Use `PRAGMA user_version` for migration tracking

### File System Security

```json
// capabilities/default.json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "fs:default",
    {
      "identifier": "fs:allow-read-text-file",
      "allow": [{ "path": "$HOME/.claude/**/*" }]
    }
  ]
}
```

**Rationale:**
- Tauri 2 uses capability-based security
- Scope permissions to specific paths
- Use `$HOME`, `$APPDATA` path variables

---

## 2. Rust Workspace Organization

### Decision: Virtual manifest with workspace dependencies

**Root Cargo.toml:**
```toml
[workspace]
members = ["crates/*", "apps/tars-desktop/src-tauri"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tokio = { version = "1.35", features = ["rt-multi-thread", "macros"] }
tars-scanner = { path = "crates/tars-scanner" }
tars-core = { path = "crates/tars-core" }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
```

### Dependency Graph

```
tars-cli ──────┬──> tars-core ──> tars-scanner
               │
tars-app ──────┘
```

**Rationale:**
- `tars-core` owns shared types (Profile, Diff, RollbackState)
- `tars-scanner` is focused library, depended on by core
- Both CLI and Tauri app depend on `tars-core`
- Avoids circular dependencies

### Build Optimization

```toml
[profile.dev]
incremental = true
opt-level = 0
debug = "line-tables-only"

[profile.dev.package."*"]
debug = false  # No debug info for dependencies
```

**Rationale:**
- `debug = "line-tables-only"` provides backtraces without full debug overhead
- Disabling debug for dependencies speeds up builds 2-3x

---

## 3. YAML Frontmatter Parsing

### Decision: `gray_matter` + `serde_yml`

**Cargo.toml:**
```toml
[dependencies]
gray_matter = { version = "0.2", features = ["yaml"] }
serde = { version = "1.0", features = ["derive"] }
serde_yml = "0.0.12"
thiserror = "1.0"
rayon = "1.10"
walkdir = "2.5"
```

**Rationale:**
- `gray_matter` is a fast Rust port of battle-tested JavaScript library
- Supports YAML, TOML, JSON with pluggable engines
- `serde_yml` is the active fork of deprecated `serde_yaml`

### Type-Safe Parsing

```rust
#[derive(Deserialize, Debug)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub user_invocable: bool,
    #[serde(default)]
    pub disable_model_invocation: bool,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    pub model: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    #[serde(default)]
    pub hooks: HashMap<String, Vec<HookDefinition>>,
}

#[derive(Deserialize, Debug)]
pub struct AgentFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub tools: Vec<String>,
    pub model: Option<String>,
    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,
    #[serde(default)]
    pub skills: Vec<String>,
}

fn default_permission_mode() -> String {
    "default".to_string()
}
```

### Error Handling

```rust
#[derive(Error, Debug)]
pub enum FrontmatterError {
    #[error("Failed to parse frontmatter: {0}")]
    ParseError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("No frontmatter found in file")]
    NoFrontmatter,
}
```

### Performance for Many Files

```rust
use rayon::prelude::*;

pub fn scan_directory(dir: &Path) -> Vec<(PathBuf, SkillFrontmatter)> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        .collect::<Vec<_>>()
        .par_iter()
        .filter_map(|entry| {
            let content = fs::read_to_string(entry.path()).ok()?;
            let (fm, _) = parse_skill_file(&content).ok()?;
            Some((entry.path().to_owned(), fm))
        })
        .collect()
}
```

**Rationale:**
- Use `rayon` for parallel file processing
- Early rejection by checking `---` prefix before parsing
- `walkdir` for recursive directory traversal

---

## 4. shadcn/ui Component Architecture

### Decision: Zustand for client state + TanStack Query for server state

**State Management:**
```typescript
// Zustand for UI state
const useUIStore = create((set) => ({
  selectedProjectId: null,
  sidebarOpen: true,
  setSelectedProject: (id) => set({ selectedProjectId: id }),
}));

// React Query for server data
const { data: projects } = useQuery({
  queryKey: ['projects'],
  queryFn: () => invoke('list_projects'),
});
```

**Rationale:**
- Zustand for UI state (selected project, sidebar, tabs)
- TanStack Query for async data with caching/invalidation
- Avoid duplicating server state in Zustand

### Component Libraries

| Component | Solution |
|-----------|----------|
| Tree views | `@mrlightful/shadcn-tree-view` |
| Diff viewer | Monaco DiffEditor via `@monaco-editor/react` |
| Forms | React Hook Form + Zod + shadcn Form |
| Theme | CSS variables + ThemeProvider |

### Tree View

```tsx
import { TreeView, TreeDataItem } from '@mrlightful/tree-view';

const data: TreeDataItem[] = [
  {
    id: '1',
    name: '.claude',
    icon: <Folder />,
    children: [
      { id: '2', name: 'skills', icon: <Folder />, children: [...] },
      { id: '3', name: 'settings.json', icon: <FileJson /> },
    ],
  },
];

<TreeView
  data={data}
  onSelectChange={(item) => setSelectedFile(item)}
  expandAll
/>
```

### Diff Viewer

```tsx
import { DiffEditor } from '@monaco-editor/react';

<DiffEditor
  original={originalContent}
  modified={modifiedContent}
  language="yaml"
  theme={theme === 'dark' ? 'vs-dark' : 'light'}
  options={{ renderSideBySide: true, readOnly: false }}
/>
```

### Form Validation

```typescript
const profileSchema = z.object({
  name: z.string().min(1),
  plugin_set: z.object({
    marketplaces: z.array(z.string()),
    plugins: z.array(z.object({
      id: z.string(),
      scope: z.enum(['user', 'project', 'local']),
      enabled: z.boolean(),
    })),
  }),
});
```

### Dark Mode

```tsx
<ThemeProvider defaultTheme="system" storageKey="tars-theme">
  <App />
</ThemeProvider>
```

**Rationale:**
- shadcn uses HSL CSS variables
- ThemeProvider respects system preference
- Tauri apps should follow OS theme by default

---

## Technology Stack Summary

| Layer | Technology | Version |
|-------|------------|---------|
| Desktop runtime | Tauri | 2.x |
| Backend language | Rust | 1.75+ |
| Frontend framework | React | 18.x |
| Frontend build | Vite | 5.x |
| UI components | shadcn/ui | latest |
| Styling | Tailwind CSS | 3.x |
| Package manager | Bun | latest |
| Database | SQLite (rusqlite) | bundled |
| YAML parsing | gray_matter + serde_yml | 0.2.x |
| Client state | Zustand | 4.x |
| Server state | TanStack Query | 5.x |
| Form validation | React Hook Form + Zod | latest |
| Diff viewer | Monaco Editor | latest |

---

## Alternatives Considered

| Decision | Alternative | Why Rejected |
|----------|-------------|--------------|
| Tauri 2 | Electron | Larger bundle size, higher memory usage |
| rusqlite | Tauri SQL plugin | Direct control preferred, workspace integration |
| gray_matter | Custom parser | Battle-tested library preferred |
| Zustand | Redux | Simpler API, less boilerplate for this scale |
| Monaco | react-diff-view | Monaco provides better UX for config editing |
