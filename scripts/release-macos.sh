#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Get current version
CURRENT_VERSION=$(grep '"version"' apps/tars-desktop/package.json | head -1 | sed 's/.*"version": "\([^"]*\)".*/\1/')

# Parse current version
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Calculate version options
PATCH_VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))"
MINOR_VERSION="${MAJOR}.$((MINOR + 1)).0"
MAJOR_VERSION="$((MAJOR + 1)).0.0"

# Display menu
clear
echo -e "${BOLD}${GREEN}"
echo "  ╔═══════════════════════════════════════════╗"
echo "  ║       TARS macOS Release Script           ║"
echo "  ╚═══════════════════════════════════════════╝"
echo -e "${NC}"
echo -e "  Current version: ${CYAN}${CURRENT_VERSION}${NC}"
echo ""
echo -e "${BOLD}  Select version bump:${NC}"
echo ""
echo -e "    ${YELLOW}1)${NC} Patch  ${GREEN}→ ${PATCH_VERSION}${NC}  (bug fixes)"
echo -e "    ${YELLOW}2)${NC} Minor  ${GREEN}→ ${MINOR_VERSION}${NC}  (new features)"
echo -e "    ${YELLOW}3)${NC} Major  ${GREEN}→ ${MAJOR_VERSION}${NC}  (breaking changes)"
echo -e "    ${YELLOW}4)${NC} Custom version"
echo -e "    ${YELLOW}5)${NC} Exit"
echo ""
read -p "  Enter choice [1-5]: " choice

case $choice in
    1)
        NEW_VERSION="$PATCH_VERSION"
        ;;
    2)
        NEW_VERSION="$MINOR_VERSION"
        ;;
    3)
        NEW_VERSION="$MAJOR_VERSION"
        ;;
    4)
        echo ""
        read -p "  Enter custom version (e.g., 1.0.0): " NEW_VERSION
        if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo -e "${RED}  Invalid version format. Use x.y.z${NC}"
            exit 1
        fi
        ;;
    5)
        echo "  Bye!"
        exit 0
        ;;
    *)
        echo -e "${RED}  Invalid choice${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "  ${BOLD}Version: ${CYAN}${CURRENT_VERSION}${NC} → ${GREEN}${NEW_VERSION}${NC}"
echo ""
read -p "  Proceed with release? (y/n): " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "  Aborted."
    exit 1
fi

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}  Starting release process...${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Load signing key
SIGNING_KEY_FILE="$HOME/.tauri/tars.key"
if [[ -f "$SIGNING_KEY_FILE" ]]; then
    export TAURI_SIGNING_PRIVATE_KEY=$(cat "$SIGNING_KEY_FILE")
    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="!Ufgat0rs1993"
    echo -e "${BLUE}[0/7]${NC} Loaded signing key from ~/.tauri/tars.key"
    echo ""
else
    echo -e "${RED}  Signing key not found at ~/.tauri/tars.key${NC}"
    echo "  Generate one with: bunx tauri signer generate -w ~/.tauri/tars.key"
    exit 1
fi

# Update version in all files
echo -e "${BLUE}[1/7]${NC} Updating version in files..."

# 1. Cargo.toml (workspace)
sed -i '' "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml

# 2. package.json
sed -i '' "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" apps/tars-desktop/package.json

# 3. tauri.conf.json
sed -i '' "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" apps/tars-desktop/src-tauri/tauri.conf.json

echo "       ✓ Cargo.toml"
echo "       ✓ package.json"
echo "       ✓ tauri.conf.json"

# Verify versions match
CARGO_VER=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\([^"]*\)"/\1/')
PKG_VER=$(grep '"version"' apps/tars-desktop/package.json | head -1 | sed 's/.*"version": "\([^"]*\)".*/\1/')
TAURI_VER=$(grep '"version"' apps/tars-desktop/src-tauri/tauri.conf.json | head -1 | sed 's/.*"version": "\([^"]*\)".*/\1/')

if [[ "$CARGO_VER" != "$NEW_VERSION" || "$PKG_VER" != "$NEW_VERSION" || "$TAURI_VER" != "$NEW_VERSION" ]]; then
    echo -e "${RED}  Version mismatch detected!${NC}"
    echo "    Cargo.toml: $CARGO_VER"
    echo "    package.json: $PKG_VER"
    echo "    tauri.conf.json: $TAURI_VER"
    exit 1
fi

# Run checks
echo ""
echo -e "${BLUE}[2/7]${NC} Running cargo fmt..."
cargo fmt --all

echo ""
echo -e "${BLUE}[3/7]${NC} Running cargo clippy..."
cargo clippy --all -- -D warnings

echo ""
echo -e "${BLUE}[4/7]${NC} Running TypeScript check..."
cd apps/tars-desktop
bun run typecheck
cd "$PROJECT_ROOT"

# Build the app
echo ""
echo -e "${BLUE}[5/7]${NC} Building Tauri app for macOS..."
echo "       This may take a few minutes..."
cd apps/tars-desktop
CI=true bun run tauri build

# Find the built artifacts (workspace target dir)
cd "$PROJECT_ROOT"
BUNDLE_DIR="target/release/bundle"
DMG_FILE=$(find "$BUNDLE_DIR/dmg" -name "*.dmg" 2>/dev/null | head -1)
UPDATER_FILE=$(find "$BUNDLE_DIR/macos" -name "*.tar.gz" 2>/dev/null | head -1)
UPDATER_SIG=$(find "$BUNDLE_DIR/macos" -name "*.tar.gz.sig" 2>/dev/null | head -1)

echo ""
echo "       Built artifacts:"
[[ -n "$DMG_FILE" ]] && echo "       ✓ $(basename "$DMG_FILE")"
[[ -n "$UPDATER_FILE" ]] && echo "       ✓ $(basename "$UPDATER_FILE")"
[[ -n "$UPDATER_SIG" ]] && echo "       ✓ $(basename "$UPDATER_SIG")"

# Format before commit
echo ""
echo -e "${BLUE}[6/7]${NC} Formatting and committing..."
cd apps/tars-desktop
bun run prettier --write "src/**/*.{ts,tsx,js,jsx,json,css}"
cd "$PROJECT_ROOT"

git add Cargo.toml apps/tars-desktop/package.json apps/tars-desktop/src-tauri/tauri.conf.json
git add Cargo.lock 2>/dev/null || true
git add apps/tars-desktop/ 2>/dev/null || true
git commit -m "chore: bump version to $NEW_VERSION"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

echo "       ✓ Committed version bump"
echo "       ✓ Created tag v$NEW_VERSION"

# Push and release
echo ""
echo -e "${BLUE}[7/7]${NC} Pushing to GitHub and creating release..."
git pull --rebase origin main
git push origin main
git push origin "v$NEW_VERSION"

RELEASE_NOTES="## Downloads

| Platform | File | Notes |
|----------|------|-------|
| **macOS (Apple Silicon)** | \`TARS_*_aarch64.dmg\` | M1/M2/M3/M4 Macs |
| **macOS (Intel)** | \`TARS_*_x64.dmg\` | Intel-based Macs |
| **Linux (Debian/Ubuntu)** | \`TARS_*_amd64.deb\` | \`sudo dpkg -i <file>\` |
| **Linux (Universal)** | \`TARS_*_amd64.AppImage\` | Make executable, then run |
| **Linux (Fedora/RHEL)** | \`TARS_*_x86_64.rpm\` | \`sudo rpm -i <file>\` |
| **Windows** | \`TARS_*_x64.exe\` | Windows 10/11 (64-bit) |

## Installation

**macOS**: Download DMG, open it, drag TARS to Applications

**Linux**: Download your preferred format and install:
- \`.deb\`: \`sudo dpkg -i TARS_*.deb\`
- \`.rpm\`: \`sudo rpm -i TARS_*.rpm\`
- \`.AppImage\`: \`chmod +x TARS_*.AppImage && ./TARS_*.AppImage\`

**Windows**: Download and run the .exe installer

---
*Built on $(date '+%Y-%m-%d')*"

# Collect artifacts to upload
ARTIFACTS=""
[[ -n "$DMG_FILE" && -f "$DMG_FILE" ]] && ARTIFACTS="$ARTIFACTS $DMG_FILE"
[[ -n "$UPDATER_FILE" && -f "$UPDATER_FILE" ]] && ARTIFACTS="$ARTIFACTS $UPDATER_FILE"
[[ -n "$UPDATER_SIG" && -f "$UPDATER_SIG" ]] && ARTIFACTS="$ARTIFACTS $UPDATER_SIG"

# Generate latest.json for auto-updater
if [[ -n "$UPDATER_FILE" && -n "$UPDATER_SIG" ]]; then
    SIGNATURE=$(cat "$UPDATER_SIG")
    UPDATER_FILENAME=$(basename "$UPDATER_FILE")

    # Determine architecture from system
    if [[ "$(uname -m)" == "arm64" ]]; then
        PLATFORM_KEY="darwin-aarch64"
    else
        PLATFORM_KEY="darwin-x86_64"
    fi

    DOWNLOAD_URL="https://github.com/inceptyon-labs/TARS/releases/download/v${NEW_VERSION}/${UPDATER_FILENAME}"
    PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    cat > /tmp/latest.json << EOF
{
  "version": "${NEW_VERSION}",
  "notes": "Release v${NEW_VERSION}",
  "pub_date": "${PUB_DATE}",
  "platforms": {
    "${PLATFORM_KEY}": {
      "signature": "${SIGNATURE}",
      "url": "${DOWNLOAD_URL}"
    }
  }
}
EOF

    ARTIFACTS="$ARTIFACTS /tmp/latest.json"
    echo "       ✓ Generated latest.json"
fi

if [[ -n "$ARTIFACTS" ]]; then
    gh release create "v$NEW_VERSION" \
        --title "TARS v$NEW_VERSION" \
        --notes "$RELEASE_NOTES" \
        $ARTIFACTS
else
    gh release create "v$NEW_VERSION" \
        --title "TARS v$NEW_VERSION" \
        --notes "$RELEASE_NOTES"
fi

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}${GREEN}  ✓ Release v$NEW_VERSION published!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${CYAN}https://github.com/inceptyon-labs/TARS/releases/tag/v$NEW_VERSION${NC}"
echo ""
