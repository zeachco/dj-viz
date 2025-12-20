---
name: release
description: Create a new release for dj-viz with automated version bumping and GitHub release creation
---

# Release Process

## Overview

The dj-viz project follows semantic versioning (MAJOR.MINOR.PATCH) with automated releases triggered by conventional commits.

## Automated Release (Preferred)

The project includes a GitHub Actions workflow that automatically creates releases when feat/fix commits are pushed to main.

### How It Works

1. Push commits to `main` branch with conventional commit messages:
   - `feat:` triggers a MINOR version bump (0.1.0 → 0.2.0)
   - `fix:` triggers a PATCH version bump (0.1.0 → 0.1.1)
   - `chore:`, `docs:`, `refactor:` do NOT trigger releases

2. The workflow automatically:
   - Calculates the new version based on commit types
   - Updates `Cargo.toml` with the new version
   - Creates a commit: `chore: bump version to X.Y.Z`
   - Creates and pushes a git tag: `vX.Y.Z`
   - Creates a GitHub release with generated release notes

### Conventional Commit Examples

```bash
# Minor version bump (new feature)
git commit -m "feat: add plasma wave visualization"

# Patch version bump (bug fix)
git commit -m "fix: correct FFT band calculation for bass frequencies"

# No release (documentation)
git commit -m "docs: update README with installation instructions"
```

## Manual Release (Fallback)

If the automated workflow fails or you need to create a release manually:

### 1. Determine Version Number

```bash
# Check current version
git tag --sort=-v:refname | head -1
grep "^version" Cargo.toml

# Review commits since last tag
LAST_TAG=$(git describe --tags --abbrev=0)
git log $LAST_TAG..HEAD --oneline
```

Version bump rules:
- **MAJOR**: Breaking API changes (0.x.x → 1.0.0)
- **MINOR**: New features, backward-compatible (0.1.x → 0.2.0)
- **PATCH**: Bug fixes, no new features (0.1.0 → 0.1.1)

### 2. Update Cargo.toml

```bash
# Edit version in Cargo.toml
NEW_VERSION="0.2.0"
sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
```

### 3. Commit and Tag

```bash
# Commit version bump
git add Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION"

# Create annotated tag
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

# Push commit and tag
git push origin main
git push origin "v$NEW_VERSION"
```

### 4. Create GitHub Release

```bash
# Generate release notes from commits
PREV_TAG=$(git describe --tags --abbrev=0 HEAD~1)
git log $PREV_TAG..v$NEW_VERSION --pretty=format:"%s" > /tmp/commits.txt

# Create GitHub release with notes
gh release create "v$NEW_VERSION" \
  --title "v$NEW_VERSION" \
  --notes-file /tmp/commits.txt
```

Or manually categorize changes:

```bash
gh release create "v$NEW_VERSION" \
  --title "v$NEW_VERSION" \
  --notes "$(cat <<'EOF'
## Features
- New plasma wave visualization
- Audio reactivity improvements

## Fixes
- Corrected FFT band calculations
- Fixed window sizing on macOS

## Improvements
- Better performance in release mode
- Reduced particle spawn overhead
EOF
)"
```

## Release Checklist

- [ ] All commits use conventional commit format
- [ ] CI/CD passes on main branch
- [ ] Version in `Cargo.toml` matches git tag
- [ ] Git tag follows `vX.Y.Z` format
- [ ] GitHub release includes categorized notes
- [ ] Release notes mention breaking changes (if any)

## Troubleshooting

### Workflow didn't trigger a release

Check that:
1. Commits are on the `main` branch
2. At least one commit since last tag starts with `feat:` or `fix:`
3. GitHub Actions has `contents: write` permissions

### Tag already exists

If you need to re-create a tag:

```bash
# Delete local and remote tag
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z

# Re-create tag
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

### Version mismatch between Cargo.toml and tag

Always ensure `Cargo.toml` version matches the git tag version (without 'v' prefix).

## Version History

Check release history:

```bash
# List all tags
git tag --sort=-v:refname

# Show tag details
git show vX.Y.Z

# View all GitHub releases
gh release list
```
