# Contributing to TARS

Thank you for your interest in contributing to TARS! We welcome bug reports, feature requests, and pull requests.

## Getting Started

### Prerequisites

- **Rust 1.75+**
- **Bun** (for JavaScript/TypeScript tooling)
- **macOS 10.13+**, **Windows 10+**, or **Linux** (Ubuntu 20.04+)

### Local Setup

1. Clone the repository:
```bash
git clone https://github.com/anthropics/tars.git
cd tars
```

2. Enable pre-commit hooks:
```bash
git config core.hooksPath .githooks
```

3. Install dependencies:
```bash
cargo build
cd apps/tars-desktop
bun install
```

## Development Workflow

### Building & Testing

Before making changes, familiarize yourself with the project structure (see [CLAUDE.md](./CLAUDE.md)):

```bash
# Build all Rust crates
cargo build

# Run tests
cargo test

# Development mode (Tauri app)
cd apps/tars-desktop
bun run tauri dev

# Production build
bun run tauri build
```

### Code Formatting & Linting

**All commits must pass formatting and linting checks.** Run these before committing:

```bash
# Format Rust code
cargo fmt --all

# Format TypeScript/frontend code
cd apps/tars-desktop
bun run format

# Run clippy (catches common Rust mistakes)
cargo clippy --all -- -D warnings

# Type-check frontend code
bun tsc --noEmit
```

The pre-commit hooks will verify these automatically—running them first prevents rejection.

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/). Format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description | In Changelog? |
|------|-------------|---------------|
| `feat` | New feature | Yes |
| `fix` | Bug fix | Yes |
| `perf` | Performance improvement | Yes |
| `docs` | Documentation | No |
| `style` | Formatting/whitespace | No |
| `refactor` | Code restructuring | No |
| `test` | Tests | No |
| `chore` | Maintenance/deps | No |
| `ci` | CI/CD changes | No |
| `build` | Build system | No |

### Examples

```
feat(scanner): add support for discovering MCP servers

Scan project .mcp.json files and detect MCP server configurations.
Includes recursive directory traversal and validation.

fix(ui): resolve profile list not updating after deletion

Refresh the profile list query after successful deletion.

docs: update README with installation instructions for Windows
```

**Rules:**
- Type is required (must be one of the above)
- Description is required (imperative mood, lowercase, no period)
- Scope is optional but helps with clarity
- Breaking changes: add `!` after type or include `BREAKING CHANGE:` footer

## Pull Requests

1. **Create a branch** from `main` for your changes
2. **Write tests** for new functionality
3. **Run all checks** before pushing:
   ```bash
   cargo fmt --all
   cargo clippy --all -- -D warnings
   cargo test
   cd apps/tars-desktop && bun run format && bun tsc --noEmit && bun test
   ```
4. **Push and create a PR** with a clear description
5. **Address review feedback** promptly

### PR Guidelines

- Link related issues with `Closes #123` in the description
- Include a brief summary of changes
- For UI changes, describe the new behavior or include screenshots
- For backend changes, explain the approach and any trade-offs
- Keep PRs focused—one feature or fix per PR

## Architecture Overview

See [CLAUDE.md](./CLAUDE.md) for:
- Project structure (crates, apps)
- Key Rust modules (scanner, parser, profiles, config, storage)
- Configuration scopes and precedence
- Claude Code file formats

## Types of Contributions

### Bug Reports

Open an issue with:
- Clear description of the bug
- Steps to reproduce
- Expected vs actual behavior
- Environment (OS, app version)

### Feature Requests

Open an issue with:
- Clear use case and motivation
- Proposed solution (if you have one)
- Any alternatives considered

### Code Contributions

- Fix bugs
- Add features
- Improve performance
- Improve documentation or tests

## Code Style

- **Rust**: Follow standard Rust conventions (enforced by `cargo fmt` and `cargo clippy`)
- **TypeScript/React**: Follow Prettier formatting (enforced by pre-commit hooks)
- **Comments**: Add comments for non-obvious logic, not for self-evident code
- **Tests**: Include tests for new functionality

## Testing

### Rust Tests

```bash
cargo test --all
```

### Frontend Tests

```bash
cd apps/tars-desktop
bun test
```

### Manual Testing

For Tauri app changes:

```bash
cd apps/tars-desktop
bun run tauri dev  # Hot-reload development
bun run tauri build  # Full production build
```

## Licensing

By contributing, you agree that your contributions will be licensed under the project's existing license (see [LICENSE](./LICENSE) for details).

## Questions?

- Check [CLAUDE.md](./CLAUDE.md) for project details
- Open a discussion or issue for questions
- Review recent PRs for patterns and examples

---

Thanks for contributing to TARS! 🚀
