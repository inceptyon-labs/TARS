# Quickstart: TARS

**Date**: 2026-01-08
**Status**: Complete

This document provides setup instructions and verification steps for TARS development.

---

## Prerequisites

### Required Tools

| Tool | Version | Installation |
|------|---------|--------------|
| Rust | 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Bun | latest | `curl -fsSL https://bun.sh/install \| bash` |
| Xcode CLT | latest | `xcode-select --install` |
| Claude Code | latest | Required for CLI bridge testing |

### Verify Prerequisites

```bash
# Check Rust
rustc --version    # Should show 1.75.0 or higher
cargo --version

# Check Bun
bun --version

# Check Xcode tools
xcode-select -p    # Should show /Library/Developer/CommandLineTools

# Check Claude Code CLI
claude --version
```

---

## Project Setup

### 1. Clone and Initialize

```bash
cd /path/to/tars

# Initialize Rust workspace
cat > Cargo.toml << 'EOF'
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
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
gray_matter = { version = "0.2", features = ["yaml"] }
serde_yml = "0.0.12"
rusqlite = { version = "0.31", features = ["bundled"] }
walkdir = "2.5"
rayon = "1.10"
sha2 = "0.10"
hex = "0.4"

tars-scanner = { path = "crates/tars-scanner" }
tars-core = { path = "crates/tars-core" }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
EOF
```

### 2. Create Crate Structure

```bash
# Create crate directories
mkdir -p crates/tars-scanner/src
mkdir -p crates/tars-core/src
mkdir -p crates/tars-cli/src
mkdir -p apps/tars-desktop

# Initialize tars-scanner
cat > crates/tars-scanner/Cargo.toml << 'EOF'
[package]
name = "tars-scanner"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
gray_matter = { workspace = true }
serde_yml = { workspace = true }
walkdir = { workspace = true }
rayon = { workspace = true }
sha2 = { workspace = true }
hex = { workspace = true }

[lints]
workspace = true
EOF

echo 'pub fn scan() { println!("scanner"); }' > crates/tars-scanner/src/lib.rs

# Initialize tars-core
cat > crates/tars-core/Cargo.toml << 'EOF'
[package]
name = "tars-core"
version.workspace = true
edition.workspace = true

[dependencies]
tars-scanner = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
rusqlite = { workspace = true }

[lints]
workspace = true
EOF

cat > crates/tars-core/src/lib.rs << 'EOF'
pub use tars_scanner;

pub fn core() { println!("core"); }
EOF

# Initialize tars-cli
cat > crates/tars-cli/Cargo.toml << 'EOF'
[package]
name = "tars-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "tars"
path = "src/main.rs"

[dependencies]
tars-core = { workspace = true }
tars-scanner = { workspace = true }
clap = { version = "4.4", features = ["derive"] }

[lints]
workspace = true
EOF

cat > crates/tars-cli/src/main.rs << 'EOF'
fn main() {
    println!("TARS CLI");
    tars_scanner::scan();
    tars_core::core();
}
EOF
```

### 3. Initialize Tauri App

```bash
cd apps/tars-desktop

# Create Tauri app with Bun + React + TypeScript
bun create tauri-app . --template react-ts --manager bun --yes

# Install additional dependencies
bun add @tanstack/react-query zustand @monaco-editor/react
bun add -d @types/node

# Add shadcn/ui
bunx shadcn@latest init -y
bunx shadcn@latest add button card dialog form input label table tabs

cd ../..
```

### 4. Configure Tauri Backend

```bash
# Update Tauri Cargo.toml to use workspace
cat > apps/tars-desktop/src-tauri/Cargo.toml << 'EOF'
[package]
name = "tars-app"
version.workspace = true
edition.workspace = true

[lib]
name = "tars_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tars-core = { workspace = true }
tars-scanner = { workspace = true }
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { workspace = true }
serde_json = { workspace = true }

[lints]
workspace = true
EOF
```

---

## Build Verification

### Build All Crates

```bash
# From repo root
cargo build

# Expected output: Compiling tars-scanner, tars-core, tars-cli, tars-app
```

### Run CLI

```bash
cargo run -p tars-cli

# Expected output: "TARS CLI" followed by crate messages
```

### Run Tests

```bash
cargo test --workspace

# Should pass (no tests yet, but verifies compilation)
```

### Run Tauri Dev

```bash
cd apps/tars-desktop
bun run tauri dev

# Expected: Tauri window opens with React app
```

---

## Development Workflow

### Daily Commands

```bash
# Build all
cargo build

# Build specific crate
cargo build -p tars-scanner

# Run CLI
cargo run -p tars-cli -- scan ~/Development

# Run tests
cargo test --workspace

# Run specific test
cargo test -p tars-scanner test_name

# Run Tauri app
cd apps/tars-desktop && bun run tauri dev

# Check lints
cargo clippy --workspace

# Format code
cargo fmt --all
```

### Task 1 Verification (Scanner CLI)

```bash
# After implementing scanner:
cargo run -p tars-cli -- scan ~

# Should produce:
# - tars-inventory.json
# - tars-inventory.md

# Verify read-only (no files modified)
```

### Task 2 Verification (Profile Engine)

```bash
# After implementing profile engine:
# 1. Create profile from scan
# 2. Apply to test project
# 3. Verify diff preview shown
# 4. Verify backup created
# 5. Rollback
# 6. Verify byte-for-byte match with original
```

### Task 3 Verification (Tauri App)

```bash
cd apps/tars-desktop
bun run tauri dev

# Verify:
# - Projects list displays
# - Scan shows inventory
# - Profile create/assign works
# - Apply shows diff preview
# - Skills editor works
```

### Task 4 Verification (Plugin Export)

```bash
# After implementing export:
# 1. Export profile as plugin
# 2. Install with: claude plugin install ./exported-plugin
# 3. Verify skills/commands/agents available
```

---

## Directory Structure After Setup

```
tars/
├── Cargo.toml                    # Workspace manifest
├── CLAUDE.md                     # Claude Code guidance
├── apps/
│   └── tars-desktop/
│       ├── src/                  # React frontend
│       ├── src-tauri/            # Rust backend
│       ├── package.json
│       └── tauri.conf.json
├── crates/
│   ├── tars-scanner/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── tars-core/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── tars-cli/
│       ├── Cargo.toml
│       └── src/main.rs
├── specs/
│   └── 001-tars/
│       ├── plan.md
│       ├── research.md
│       ├── data-model.md
│       ├── quickstart.md
│       ├── contracts/
│       └── tasks.md              # Generated by /speckit.tasks
└── .specify/
    ├── memory/constitution.md
    ├── specs/001-tars/spec.md
    └── templates/
```

---

## Troubleshooting

### Rust Build Errors

```bash
# Clear build cache
cargo clean

# Update dependencies
cargo update

# Check for missing system deps
xcode-select --install
```

### Tauri Build Errors

```bash
# Ensure Xcode CLT installed
xcode-select -p

# Clear Bun cache
rm -rf node_modules bun.lockb
bun install

# Check Tauri CLI version
bunx tauri --version
```

### SQLite Errors

```bash
# bundled feature should handle this, but if issues:
brew install sqlite3
```

### Claude Code CLI Not Found

```bash
# Ensure Claude Code is installed and in PATH
which claude

# If not found, add to PATH or reinstall Claude Code
```

---

## Next Steps

After setup is complete:

1. Run `/speckit.tasks` to generate the task list
2. Start with Task 1: Discovery Scanner CLI
3. Follow the phased implementation order in the spec
