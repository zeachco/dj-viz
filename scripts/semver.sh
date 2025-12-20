#!/bin/bash
set -e

DRY_RUN=false
if [[ "$1" == "--dry-run" ]]; then
    DRY_RUN=true
fi

# Get the latest tag or default to v0.0.0
LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
echo "Current version: $LATEST_TAG"

# Parse version components
VERSION=${LATEST_TAG#v}
IFS='.' read -r MAJOR MINOR PATCH <<< "$VERSION"

# Analyze commits since last tag
COMMITS=$(git log "$LATEST_TAG"..HEAD --pretty=format:"%s" 2>/dev/null || git log --pretty=format:"%s")

if [[ -z "$COMMITS" ]]; then
    echo "No new commits since $LATEST_TAG"
    exit 0
fi

# Determine bump type
BUMP="none"
while IFS= read -r msg; do
    if [[ "$msg" == *"BREAKING CHANGE"* ]] || [[ "$msg" =~ ^[a-z]+\!: ]]; then
        BUMP="major"
        break
    elif [[ "$msg" =~ ^feat(\(.+\))?: ]]; then
        if [[ "$BUMP" != "major" ]]; then
            BUMP="minor"
        fi
    elif [[ "$msg" =~ ^fix(\(.+\))?: ]]; then
        if [[ "$BUMP" == "none" ]]; then
            BUMP="patch"
        fi
    fi
done <<< "$COMMITS"

if [[ "$BUMP" == "none" ]]; then
    echo "No feat/fix commits found. No release needed."
    echo ""
    echo "Commits since $LATEST_TAG:"
    echo "$COMMITS" | head -10
    exit 0
fi

# Calculate new version
case $BUMP in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
esac

NEW_VERSION="v$MAJOR.$MINOR.$PATCH"
echo "Bump type: $BUMP"
echo "New version: $NEW_VERSION"

if [[ "$DRY_RUN" == true ]]; then
    echo ""
    echo "Commits to be included:"
    echo "$COMMITS" | grep -E "^(feat|fix)" | head -20
    exit 0
fi

# Update Cargo.toml
sed -i "s/^version = \".*\"/version = \"$MAJOR.$MINOR.$PATCH\"/" Cargo.toml
echo "Updated Cargo.toml"

# Update manifest
echo "{
  \".\": \"$MAJOR.$MINOR.$PATCH\"
}" > .release-please-manifest.json
echo "Updated .release-please-manifest.json"

# Commit and tag
git add Cargo.toml .release-please-manifest.json
git commit -m "chore: release $NEW_VERSION"
git tag "$NEW_VERSION"

echo ""
echo "Created tag $NEW_VERSION"
echo "Run 'git push && git push --tags' to trigger the release workflow"
