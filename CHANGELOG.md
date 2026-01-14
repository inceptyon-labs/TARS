# Changelog

All notable changes to TARS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.2.7] - 2026-01-14

### Fixed
- satisfy clippy requirements in profiles


## [0.2.6] - 2026-01-14

### Fixed
- clean up redundant closure


## [0.2.5] - 2026-01-14

### Fixed
- resolve TS and lint issues


## [0.2.4] - 2026-01-14


## [0.2.3] - 2026-01-14

### Added
- profile marketplace tooling
- add central storage and plugin install for profile assignment

### Fixed
- resolve claude CLI path on Linux and improve sidebar UI


## [0.2.2] - 2026-01-12

### Added
- automate CHANGELOG.md updates in release workflow

### Fixed
- remove unused serde_yml dependency with security vulnerability
- correctly handle project-scoped uninstall
- add direct uninstall fallback for CLI bug #14202
- prevent long plugin descriptions from breaking layout
- use marketplace.json index for available plugins
- install to multiple projects sequentially
- show actual error message when multi-project install fails
- show actual CLI error messages in plugin operations

## [0.1.3] - 2025-01-10

### Added
- Cross-platform Claude Code detection for GUI apps (macOS, Windows, Linux)
- Auto-updater with signed releases
- Windows build in CI workflow

### Fixed
- Claude Code not detected when running from .app bundle on macOS
- CI builds now wait for tests to pass before starting

## [0.1.2] - 2025-01-10

### Added
- TARS Desktop auto-update functionality
- Release script for version bumping (`scripts/release.sh`)
- Platform-specific download guide in release notes

### Fixed
- Linux builds on GitHub Actions (pkg-config glib-2.0 errors)

## [0.1.1] - 2025-01-10

### Added
- Initial release workflow with multi-platform builds
- macOS code signing and notarization support
- Pre-commit hooks for Rust and frontend formatting

## [0.1.0] - 2025-01-09

### Added
- Initial TARS Desktop application
- Project scanning and inventory management
- Profile creation, application, and rollback
- Plugin marketplace integration
- MCP server management
- SQLite database for local storage
