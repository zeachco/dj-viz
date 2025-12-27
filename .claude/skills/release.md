---
name: release
description: Create a new release for dj-viz with manual version bumping and GitHub release creation
---

# Release Process

## Quick Release

```bash
# 1. Determine version (check last tag)
git tag --sort=-v:refname | head -1

# 2. Set new version
NEW_VERSION="0.2.0"

# 3. Update Cargo.toml, commit, tag, and push
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
git add Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"
git push origin main --tags

# 4. Create GitHub release
gh release create "v$NEW_VERSION" --title "v$NEW_VERSION" --generate-notes
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

### 4. Custom Release Notes (Optional)

```bash
gh release create "v$NEW_VERSION" \
  --title "v$NEW_VERSION" \
  --notes "$(cat <<'EOF'
## Features
- New visualization: PlasmaWave

## Fixes
- Fixed audio device detection on Linux

## Improvements
- Better performance in release mode
EOF
)"
```

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
