# Release Command - cargo-release Integration

Perform a coordinated release for bevy_brp workspace crates using cargo-release. This command handles the 3-crate dependency chain and ensures safe publishing to crates.io.

## Usage
- `/release X.Y.Z-rc.N` - Release all 3 crates as RC version (e.g., `0.3.0-rc.1`)
- `/release X.Y.Z` - Release all 3 crates as final version (e.g., `0.3.0`)

## Crate Dependency Chain
1. `mcp_macros` (no dependencies) - must be released first
2. `extras` (depends on bevy only) - can be released with mcp
3. `mcp` (depends on mcp_macros only) - released after mcp_macros is live

## Prerequisites Check

Before starting the release, verify:
1. You're on the `main` branch
2. Working directory is clean (no uncommitted changes)
3. You're up to date with remote
4. cargo-release is installed (`cargo install cargo-release`)

## Step 0: One-Time Repository Setup (if not already done)

**Update repository URLs to use workspace inheritance:**

```bash
# Add workspace.package section to root Cargo.toml
```
→ **Manual edit**: Add this to `/Users/natemccoy/rust/bevy_brp/Cargo.toml`:
```toml
[workspace.package]
repository = "https://github.com/natepiano/bevy_brp"
```

```bash
# Update each crate to use workspace inheritance
```
→ **Manual edit**: In each crate's Cargo.toml (mcp_macros, mcp, extras), replace repository field with:
```toml
[package]
repository.workspace = true
```

## Step 1: Pre-Release Validation

```bash
git status
```
→ **Auto-check**: Continue if clean, stop if uncommitted changes

```bash
git fetch origin
```


```bash
cargo clippy --all-targets --all-features -- -D warnings
```
→ **Auto-check**: Continue if no warnings, stop to discuss if there are issues

```bash
cargo build --all
```
→ **Auto-check**: Continue if builds, stop if errors

```bash
cargo nextest run --all
```
→ **Auto-check**: Continue if tests pass, stop if any fail

```bash
cargo +nightly fmt --all
```

## Step 2: Verify CHANGELOG Entries

```bash
grep -q "## \[Unreleased\]" mcp_macros/CHANGELOG.md && grep -A 5 "## \[Unreleased\]" mcp_macros/CHANGELOG.md
```
→ **Auto-check**: Verify [Unreleased] section exists with content, stop if missing or empty

```bash
grep -q "## \[Unreleased\]" extras/CHANGELOG.md && grep -A 5 "## \[Unreleased\]" extras/CHANGELOG.md
```
→ **Auto-check**: Verify [Unreleased] section exists with content, stop if missing or empty

```bash
grep -q "## \[Unreleased\]" mcp/CHANGELOG.md && grep -A 5 "## \[Unreleased\]" mcp/CHANGELOG.md
```
→ **Auto-check**: Verify [Unreleased] section exists with content, stop if missing or empty

## Step 3: Three-Phase Release Process

### Phase 1: Release mcp_macros

```bash
cargo release <version> --package bevy_brp_mcp_macros --dry-run
```
✅ **Verify**: Review the dry run output - version bumps, CHANGELOG updates, git operations all look correct

```bash
cargo release <version> --package bevy_brp_mcp_macros --execute
```
✅ **Verify**: Release completed successfully, git commit created

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

# Phase 1 does not create tags

```bash
cd mcp_macros && cargo publish --dry-run
```
✅ **Verify**: Review package contents, ensure all files included, no errors

```bash
cargo publish
```
→ **Auto-check**: Continue if publish succeeds, stop if fails

```bash
cd ..
```

### Phase 2: Update mcp Dependency

**Wait ~1 minute for mcp_macros to be available on crates.io**

```bash
curl -s https://crates.io/api/v1/crates/bevy_brp_mcp_macros | jq '.crate.max_version'
```
✅ **Verify**: Shows the version you just published
- If null or incorrect, wait 10-20 seconds and try again

**Update mcp/Cargo.toml dependency**
Edit `mcp/Cargo.toml` to change:
```toml
# FROM:
bevy_brp_mcp_macros = { path = "../mcp_macros" }
# TO:
bevy_brp_mcp_macros = "<version>"  # the version you just published
```
→ **I will make this edit for you using the version specified**

```bash
cargo build --package bevy_brp_mcp
```
→ **Auto-check**: Continue if build succeeds, stop if fails

```bash
cargo nextest run --package bevy_brp_mcp
```
→ **Auto-check**: Continue if tests pass, stop if any fail

```bash
git add mcp/Cargo.toml && git commit -m "chore: update mcp_macros dependency to crates.io version"
```
→ **Auto-check**: Continue if commit succeeds

### Phase 3: Release extras and mcp

```bash
cargo release <version> --workspace --exclude bevy_brp_mcp_macros --dry-run
```
✅ **Verify**: Review output - both extras and mcp will be released together

```bash
cargo release <version> --workspace --exclude bevy_brp_mcp_macros --execute
```
→ **Auto-check**: Continue if release succeeds, stop if errors

## Step 4: Push and Publish Remaining Crates

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

```bash
git push origin --tags
```
→ **Auto-check**: Continue if push succeeds, stop if fails

```bash
cd extras && cargo publish --dry-run
```
✅ **Verify**: Review package contents for extras

```bash
cd ../mcp && cargo publish --dry-run
```
✅ **Verify**: Review package contents for mcp

```bash
cd ../extras && cargo publish
```
→ **Auto-check**: Continue if publish succeeds, stop if fails

```bash
cd ../mcp && cargo publish
```
→ **Auto-check**: Continue if publish succeeds, stop if fails

```bash
cd ..
```

## Step 5: Update Documentation (First Release Only)

**For first release, add migration guide to README.md:**

→ **Manual edit**: Add this section to main README.md:
```markdown
## Migrating from Split Crates

If you were using the previous split crates, update your `Cargo.toml`:

```toml
# Old (remove these)
bevy_mcp = "0.2.0"
bevy_mcp_macros = "0.2.0"
bevy_mcp_extras = "0.2.0"

# New (use these)
bevy_brp_mcp = "0.3.0"
bevy_brp_mcp_macros = "0.3.0"
bevy_brp_extras = "0.3.0"
```

No code changes required - just dependency updates.
```

```bash
git add README.md && git commit -m "docs: add migration guide for split crate users"
```
→ **Auto-check**: Continue if commit succeeds

## Step 6: Create GitHub Release

→ **I will gather CHANGELOG entries from all three crates and create a combined release using GitHub CLI**

```bash
gh release create v<version> \
  --repo natepiano/bevy_brp \
  --title "bevy_brp v<version>" \
  --notes "Combined release notes from all three crates"
```
→ **Auto-check**: Continue if release created successfully, stop if fails

## Step 7: Post-Release Verification

```bash
for crate in bevy_brp_mcp_macros bevy_brp_extras bevy_brp_mcp; do
  echo -n "$crate: "
  curl -s https://crates.io/api/v1/crates/$crate | jq '.crate.max_version'
done
```
✅ **Verify**: All three show version <version>

```bash
cargo install bevy_brp_mcp --version <version>
```
✅ **Verify**: Installation succeeds, pulling all dependencies from crates.io

**Run agentic tests to verify functionality:**
→ **Manual**: Run your agentic test suite to verify RC/release functionality

## Step 8: Prepare for Next Release

→ **I will add [Unreleased] sections to all three CHANGELOG.md files**

```bash
git add mcp_macros/CHANGELOG.md extras/CHANGELOG.md mcp/CHANGELOG.md
```
→ **Auto-check**: Continue if successful

```bash
git commit -m "chore: prepare CHANGELOGs for next release cycle"
```
→ **Auto-check**: Continue if commit succeeds, stop if fails

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

## Rollback Instructions

If something goes wrong after pushing but before publishing:

```bash
# Delete local tag
git tag -d v<version>

# Delete remote tag  
git push origin :refs/tags/v<version>

# Revert the version bump commits (may be multiple commits)
git revert HEAD~2..HEAD  # Adjust range as needed
git push origin main
```

## Configuration Notes

The workspace uses `release.toml` with:
- `shared-version = true` for synchronized releases
- Tag format: `v{{version}}`
- Pre-release hook runs `cargo build --all`
- Manual push/publish for safety
- Test apps excluded via `[package.metadata.release] release = false`

## Common Issues

1. **"Version already exists"**: The version is already published on crates.io
2. **"Uncommitted changes"**: Run `git status` and commit or stash changes
3. **"Not on main branch"**: Switch to main with `git checkout main`
4. **Build failures**: Fix any compilation errors before releasing
5. **Dependency chain**: Always publish mcp_macros → extras → mcp
6. **Path dependency**: Ensure mcp/Cargo.toml uses crates.io version of mcp_macros, not path

