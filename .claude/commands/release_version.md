# Release Command - Simplified Workspace Release

Perform a coordinated release for bevy_brp workspace crates using cargo-release and workspace publishing.

## Usage
- `/release X.Y.Z-rc.N` - Release all 3 crates as RC version (e.g., `0.17.0-rc.1`)
- `/release X.Y.Z` - Release all 3 crates as final version (e.g., `0.17.0`)

## Crate Dependency Chain
1. `mcp_macros` (no dependencies)
2. `extras` (depends on bevy only)
3. `mcp` (depends on mcp_macros only)

`cargo publish --workspace` automatically handles this dependency order.

## Prerequisites Check

Before starting the release, verify:
1. You're on the `main` branch
2. Working directory is clean (no uncommitted changes)
3. You're up to date with remote
4. cargo-release is installed (`cargo install cargo-release`)

<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 0:** Execute <ArgumentValidation/>
    **STEP 1:** Execute <PreReleaseChecks/>
    **STEP 2:** Execute <ChangelogVerification/>
    **STEP 3:** Execute <UpdateMcpDependency/>
    **STEP 4:** Execute <RunCargoRelease/>
    **STEP 5:** Execute <FinalizeChangelogs/>
    **STEP 6:** Execute <PushAndPublish/>
    **STEP 7:** Execute <CreateGitHubRelease/>
    **STEP 8:** Execute <PostReleaseVerification/>
    **STEP 9:** Execute <PrepareNextReleaseCycle/>
    **STEP 10:** Execute <RestorePathDependency/>
</ExecutionSteps>

<ArgumentValidation>
## STEP 0: Argument Validation and VERSION Setup

**First, capture VERSION from arguments:**
```bash
VERSION="$ARGUMENTS"
```

**Then validate the version format:**
```bash
bash .claude/scripts/release_version_validate.sh "${VERSION}"
```
→ **Auto-check**: Continue if version is valid format, stop with clear error if invalid

**Confirm VERSION is set correctly:**
```bash
echo "Release version set to: ${VERSION}"
```

**Note**: The VERSION variable is now available for all subsequent commands in this release process.
</ArgumentValidation>

<PreReleaseChecks>
## STEP 1: Pre-Release Validation

Execute <GitStatusCheck/>
Execute <QualityChecks/>
</PreReleaseChecks>

<GitStatusCheck>
### Git Status Check

```bash
git status
```
→ **Auto-check**: Continue if clean, stop if uncommitted changes

```bash
git fetch origin
```
</GitStatusCheck>

<QualityChecks>
### Quality Checks

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
</QualityChecks>

<ChangelogVerification>
## STEP 2: Verify CHANGELOG Entries

```bash
for crate in mcp_macros extras mcp; do
  echo "Checking $crate CHANGELOG..."
  if ! grep -q "## \[Unreleased\]" $crate/CHANGELOG.md; then
    echo "ERROR: Missing [Unreleased] section in $crate/CHANGELOG.md"
    exit 1
  fi
  grep -A 5 "## \[Unreleased\]" $crate/CHANGELOG.md
done
```
→ **Auto-check**: Verify [Unreleased] section exists with content for all crates, stop if missing or empty
</ChangelogVerification>

<UpdateMcpDependency>
## STEP 3: Update mcp Dependency to crates.io Version

**Update mcp/Cargo.toml dependency from path to current crates.io version:**

**I will edit `mcp/Cargo.toml` to change:**
```toml
# FROM:
bevy_brp_mcp_macros = { path = "../mcp_macros" }

# TO (using current crates.io version):
bevy_brp_mcp_macros = "0.17.0"  # or whatever the current version is
```
→ **I will check the current version on crates.io first, then make this edit**

**After the edit, verify it builds:**
```bash
cargo build --package bevy_brp_mcp
```
→ **Auto-check**: Continue if build succeeds, stop if fails

```bash
cargo nextest run --package bevy_brp_mcp
```
→ **Auto-check**: Continue if tests pass, stop if any fail

```bash
git add mcp/Cargo.toml && git commit -m "chore: prepare for release - use crates.io version of mcp_macros"
```
→ **Auto-check**: Continue if commit succeeds
</UpdateMcpDependency>

<RunCargoRelease>
## STEP 4: Run cargo-release for Version Bumping

**Note**: cargo-release will bump versions in Cargo.toml files and create a git tag. We handle CHANGELOG updates manually due to known issues.

```bash
cargo release ${VERSION} --workspace
```
→ **Manual verification**: Review the dry run output - version bumps, git operations all look correct
  - Type **continue** to proceed with release execution
  - Type **stop** to halt process for manual review

```bash
cargo release ${VERSION} --workspace --execute
```
→ **Auto-check**: Continue if release succeeds with tag created, stop if errors
</RunCargoRelease>

<FinalizeChangelogs>
## STEP 5: Finalize CHANGELOG Headers

**Known Issue**: cargo-release doesn't automatically update `[Unreleased]` to versioned releases in our setup.

→ **I will update all three CHANGELOG.md files:**

Change `## [Unreleased]` to `## [${VERSION}] - $(date +%Y-%m-%d)` in:
- `mcp_macros/CHANGELOG.md`
- `extras/CHANGELOG.md`
- `mcp/CHANGELOG.md`

```bash
git add mcp_macros/CHANGELOG.md extras/CHANGELOG.md mcp/CHANGELOG.md
```
→ **Auto-check**: Continue if successful

```bash
git commit -m "chore: finalize CHANGELOGs for v${VERSION} release"
```
→ **Auto-check**: Continue if commit succeeds, stop if fails
</FinalizeChangelogs>

<PushAndPublish>
## STEP 6: Push and Publish All Crates

Execute <PushToGit/>
Execute <PublishAllCrates/>
</PushAndPublish>

<PushToGit>
### Push to Git

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

```bash
git push origin --tags
```
→ **Auto-check**: Continue if push succeeds, stop if fails
</PushToGit>

<PublishAllCrates>
### Publish to crates.io

**Note**: `cargo publish --workspace` (Rust 1.90.0+) automatically publishes in dependency order: mcp_macros → extras → mcp

```bash
cargo publish --workspace --dry-run
```
→ **Manual verification**: Review package contents for all three crates
  - Type **continue** to proceed with publishing
  - Type **stop** to halt and fix package issues

```bash
cargo publish --workspace
```
→ **Auto-check**: Continue if all publishes succeed, stop if any fail
</PublishAllCrates>

<CreateGitHubRelease>
## STEP 7: Create GitHub Release

→ **I will gather CHANGELOG entries from all three crates and create a combined release using GitHub CLI**

```bash
gh release create "v${VERSION}" \
  --repo natepiano/bevy_brp \
  --title "bevy_brp v${VERSION}" \
  --notes "Combined release notes from all three crates"
```
→ **Auto-check**: Continue if release created successfully, stop if fails
</CreateGitHubRelease>

<PostReleaseVerification>
## STEP 8: Post-Release Verification

```bash
for crate in bevy_brp_mcp_macros bevy_brp_extras bevy_brp_mcp; do
  echo -n "$crate: "
  curl -s "https://crates.io/api/v1/crates/$crate" | jq '.crate.max_version'
done
```
→ **Manual verification**: All three show version ${VERSION}
  - Type **continue** to proceed with installation test
  - Type **retry** to check versions again

```bash
cargo install bevy_brp_mcp --version "${VERSION}"
```
→ **Manual verification**: Installation succeeds, pulling all dependencies from crates.io
  - Type **continue** to proceed
  - Type **stop** to halt and investigate installation issues

**Run agentic tests to verify functionality:**
→ **Manual**: Run your agentic test suite to verify RC/release functionality
</PostReleaseVerification>

<PrepareNextReleaseCycle>
## STEP 9: Prepare for Next Release Cycle

→ **I will add [Unreleased] sections to all three CHANGELOG.md files**

Add this after the version header in each CHANGELOG.md:
```markdown
## [Unreleased]

```

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
</PrepareNextReleaseCycle>

<RestorePathDependency>
## STEP 10: Restore Path Dependency for Development

**Restore mcp/Cargo.toml to use path dependency:**

**I will edit `mcp/Cargo.toml` to change:**
```toml
# FROM:
bevy_brp_mcp_macros = "0.17.0"

# TO:
bevy_brp_mcp_macros = { path = "../mcp_macros" }
```
→ **I will make this edit using the actual ${VERSION} value**

**Verify it still builds:**
```bash
cargo build --package bevy_brp_mcp
```
→ **Auto-check**: Continue if build succeeds, stop if fails

```bash
cargo nextest run --package bevy_brp_mcp
```
→ **Auto-check**: Continue if tests pass, stop if any fail

```bash
git add mcp/Cargo.toml && git commit -m "chore: restore path dependency for development"
```
→ **Auto-check**: Continue if commit succeeds

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails
</RestorePathDependency>

## Why Path Dependencies for Development?

Path dependencies are essential for development because they allow you to:
- Test changes to mcp_macros immediately in the mcp crate
- Make coordinated changes across crates without publishing
- Iterate quickly without crates.io round-trips

The release process temporarily switches to version dependencies so that:
- cargo-release can properly version all crates together
- crates.io publishing works correctly with proper dependency versions
- Users installing from crates.io get the correct dependency versions

## Rollback Instructions

If something goes wrong after pushing but before publishing:

```bash
# Delete local tag
git tag -d "v${VERSION}"

# Delete remote tag
git push origin ":refs/tags/v${VERSION}"

# Revert the version bump commits (may be multiple commits)
git revert HEAD~3..HEAD  # Adjust range as needed
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
5. **Dependency chain**: `cargo publish --workspace` handles the order automatically (mcp_macros → extras → mcp)
6. **Path dependency**: Must temporarily use crates.io version during release, restore path dependency after
