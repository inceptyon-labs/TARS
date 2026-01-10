#!/bin/bash
#
# TARS Release Script
# Usage: ./scripts/release.sh [major|minor|patch]
#
# Examples:
#   ./scripts/release.sh patch   # 0.1.0 -> 0.1.1
#   ./scripts/release.sh minor   # 0.1.0 -> 0.2.0
#   ./scripts/release.sh major   # 0.1.0 -> 1.0.0
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Config file path
TAURI_CONF="apps/tars-desktop/src-tauri/tauri.conf.json"

# Get script directory and change to repo root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Check if we're in the right directory
if [[ ! -f "$TAURI_CONF" ]]; then
    echo -e "${RED}Error: Cannot find $TAURI_CONF${NC}"
    echo "Make sure you're running this from the repo root"
    exit 1
fi

# Check for uncommitted changes
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${RED}Error: You have uncommitted changes.${NC}"
    echo "Please commit or stash your changes before releasing."
    git status --short
    exit 1
fi

# Check we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    echo -e "${YELLOW}Warning: You're on branch '$CURRENT_BRANCH', not 'main'.${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Get current version from tauri.conf.json
CURRENT_VERSION=$(grep '"version"' "$TAURI_CONF" | head -1 | sed 's/.*"version": "\([^"]*\)".*/\1/')

if [[ -z "$CURRENT_VERSION" ]]; then
    echo -e "${RED}Error: Could not read current version from $TAURI_CONF${NC}"
    exit 1
fi

echo -e "${BLUE}Current version: ${GREEN}v$CURRENT_VERSION${NC}"

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Determine bump type
BUMP_TYPE="${1:-}"

if [[ -z "$BUMP_TYPE" ]]; then
    echo ""
    echo "Select version bump type:"
    echo -e "  ${GREEN}1)${NC} patch  ($CURRENT_VERSION -> $MAJOR.$MINOR.$((PATCH + 1)))"
    echo -e "  ${GREEN}2)${NC} minor  ($CURRENT_VERSION -> $MAJOR.$((MINOR + 1)).0)"
    echo -e "  ${GREEN}3)${NC} major  ($CURRENT_VERSION -> $((MAJOR + 1)).0.0)"
    echo -e "  ${GREEN}4)${NC} custom (enter version manually)"
    echo ""
    read -p "Choice [1-4]: " -n 1 -r CHOICE
    echo ""

    case $CHOICE in
        1) BUMP_TYPE="patch" ;;
        2) BUMP_TYPE="minor" ;;
        3) BUMP_TYPE="major" ;;
        4) BUMP_TYPE="custom" ;;
        *)
            echo -e "${RED}Invalid choice${NC}"
            exit 1
            ;;
    esac
fi

# Calculate new version
case $BUMP_TYPE in
    patch)
        NEW_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
        ;;
    minor)
        NEW_VERSION="$MAJOR.$((MINOR + 1)).0"
        ;;
    major)
        NEW_VERSION="$((MAJOR + 1)).0.0"
        ;;
    custom)
        read -p "Enter new version (without 'v' prefix): " NEW_VERSION
        # Validate version format
        if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo -e "${RED}Invalid version format. Use X.Y.Z (e.g., 1.2.3)${NC}"
            exit 1
        fi
        ;;
    *)
        echo -e "${RED}Invalid bump type: $BUMP_TYPE${NC}"
        echo "Usage: $0 [major|minor|patch|custom]"
        exit 1
        ;;
esac

echo ""
echo -e "${BLUE}Version change: ${YELLOW}v$CURRENT_VERSION${NC} -> ${GREEN}v$NEW_VERSION${NC}"
echo ""

# Confirm
read -p "Proceed with release? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

echo ""
echo -e "${BLUE}Updating version in $TAURI_CONF...${NC}"

# Update version in tauri.conf.json using sed (macOS compatible)
if [[ "$(uname)" == "Darwin" ]]; then
    sed -i '' "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$TAURI_CONF"
else
    sed -i "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$TAURI_CONF"
fi

# Verify the change
UPDATED_VERSION=$(grep '"version"' "$TAURI_CONF" | head -1 | sed 's/.*"version": "\([^"]*\)".*/\1/')
if [[ "$UPDATED_VERSION" != "$NEW_VERSION" ]]; then
    echo -e "${RED}Error: Version update failed${NC}"
    git checkout "$TAURI_CONF"
    exit 1
fi

echo -e "${GREEN}✓ Version updated to $NEW_VERSION${NC}"

# Commit
echo -e "${BLUE}Creating commit...${NC}"
git add "$TAURI_CONF"
git commit -m "chore: bump version to $NEW_VERSION"
echo -e "${GREEN}✓ Commit created${NC}"

# Create tag
TAG="v$NEW_VERSION"
echo -e "${BLUE}Creating tag $TAG...${NC}"
git tag -a "$TAG" -m "Release $TAG"
echo -e "${GREEN}✓ Tag created${NC}"

# Push
echo -e "${BLUE}Pushing to remote...${NC}"
git push origin "$CURRENT_BRANCH"
git push origin "$TAG"
echo -e "${GREEN}✓ Pushed to remote${NC}"

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Release v$NEW_VERSION initiated successfully!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "The GitHub Actions workflow is now building the release."
echo -e "Check progress at:"
echo -e "${BLUE}  https://github.com/inceptyon-labs/TARS/actions${NC}"
echo ""
echo -e "Once complete, the release will be at:"
echo -e "${BLUE}  https://github.com/inceptyon-labs/TARS/releases/tag/$TAG${NC}"
echo ""
