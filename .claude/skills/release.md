---
name: release
description: Create a new release for dj-viz with version bumping, GitHub release, and user-friendly release notes
---

# Release Process

## Overview

Releases are automated via GitHub Actions. When you push a tag, the workflow:
1. Creates a draft release
2. Builds binaries for Linux (x86_64, aarch64, armv7) and Windows
3. Uploads binaries to the release
4. Publishes the release

**Your job**: bump the version, push the tag, then update the release notes once binaries are uploaded.

## Quick Release

```bash
# 1. Determine version (check last tag)
git tag --sort=-v:refname | head -1

# 2. Set new version based on commits (see Version Bump Rules below)
NEW_VERSION="X.Y.Z"

# 3. Update Cargo.toml, commit, tag, and push
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
git add Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"
git push origin main --tags

# 4. Wait for GitHub Actions to build and upload binaries
gh run list --limit 1  # Check workflow status

# 5. Update release with user-friendly notes (see Release Notes section)
gh release edit "v$NEW_VERSION" --notes "$(cat <<'EOF'
[Release notes here - see Release Notes section below]
EOF
)"
```

## Version Bump Rules

Based on commits since last tag:
- **MINOR** (0.1.0 → 0.2.0): Any `feat:` commits (new features)
- **PATCH** (0.1.0 → 0.1.1): Only `fix:` commits (bug fixes)
- **No release**: Only `chore:`, `docs:`, `refactor:` commits

## Detailed Steps

### 1. Check Current State

```bash
# Current version
grep "^version" Cargo.toml

# Last tag
git tag --sort=-v:refname | head -1

# Commits since last tag
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null)
git log ${LAST_TAG}..HEAD --oneline
```

### 2. Determine New Version

```bash
# Count feat/fix commits
git log ${LAST_TAG}..HEAD --oneline | grep -c "^[a-f0-9]* feat:" || echo 0
git log ${LAST_TAG}..HEAD --oneline | grep -c "^[a-f0-9]* fix:" || echo 0
```

### 3. Update and Release

```bash
NEW_VERSION="X.Y.Z"

# Update Cargo.toml
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Commit and tag
git add Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

# Push everything
git push origin main --tags

# Create release with auto-generated notes
gh release create "v$NEW_VERSION" \
  --title "v$NEW_VERSION" \
  --generate-notes
```

### 4. Update Release Notes

After binaries are uploaded, update the release with user-friendly notes:

```bash
gh release edit "v$NEW_VERSION" --notes "$(cat <<'EOF'
[Your release notes - see format below]
EOF
)"
```

## Release Notes Format

Write release notes for **non-technical users** (DJs, VJs, event organizers). Avoid jargon.

### Structure

```markdown
## What's New

Brief 1-2 sentence summary of the most exciting changes.

### New Features
- **Feature Name** - What it does in plain English

### Improvements
- **Area improved** - What's better now

### Bug Fixes
- Fixed [issue description in user terms]

---

**Downloads**: Choose your platform below
- **Windows**: `dj-viz-windows-x86_64.exe`
- **Linux**: `dj-viz-linux-x86_64`
- **Raspberry Pi 4/5**: `dj-viz-linux-aarch64`
- **Raspberry Pi 3**: `dj-viz-linux-armv7`
```

### Writing Guidelines

1. **Describe benefits, not implementation**:
   - Bad: "Added rhai scripting engine"
   - Good: "Create custom visualizations with simple scripts"

2. **Focus on what users see/experience**:
   - Bad: "Fixed memory leak in FFT buffer"
   - Good: "Fixed occasional stuttering during long sessions"

3. **Group related changes** under meaningful headings

4. **Include download instructions** at the bottom

## Troubleshooting

### Tag Already Exists

```bash
# Delete and recreate
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

### Undo Last Release

```bash
# Delete release, tag, and reset commit
gh release delete vX.Y.Z --yes
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
git reset --hard HEAD~1
git push origin main --force
```

## Conventional Commits Reference

```bash
feat: add plasma wave visualization    # → MINOR bump
fix: correct FFT band calculation      # → PATCH bump
docs: update README                    # → no release
chore: update dependencies             # → no release
refactor: simplify renderer            # → no release
```
