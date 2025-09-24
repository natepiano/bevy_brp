# Release Command - cargo-release Integration

# Template Variables
VERSION = [from $ARGUMENTS]

**Migration Strategy: Phased**

Perform a coordinated release for bevy_brp workspace crates using cargo-release. This command handles the 3-crate dependency chain and ensures safe publishing to crates.io.

## Usage
- `/release X.Y.Z-rc.N` - Release all 3 crates as RC version (e.g., `0.3.0-rc.1`)
- `/release X.Y.Z` - Release all 3 crates as final version (e.g., `0.3.0`)

**Note: After initial release to crates.io**
Once all three crates are published on crates.io with proper version dependencies (not path dependencies), future releases can use the simplified `cargo publish --workspace` workflow. See "Future Releases (Post-Initial)" section below.

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

<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 0:** Execute <ArgumentValidation/>
    **STEP 1:** Execute <VersionValidationAndPreChecks/>
    **STEP 2:** Execute <ChangelogVerification/>
    **STEP 3:** Execute <ThreePhaseReleaseProcess/>
    **STEP 4:** Execute <PushAndPublishRemaining/>
    **STEP 5:** Execute <UpdateDocumentation/>
    **STEP 6:** Execute <CreateGitHubRelease/>
    **STEP 7:** Execute <PostReleaseVerification/>
    **STEP 8:** Execute <PrepareNextReleaseCycle/>
</ExecutionSteps>

<ArgumentValidation>
## Argument Validation

```bash
bash .claude/scripts/release_version_validate.sh "$ARGUMENTS"
```
→ **Auto-check**: Continue if version is valid format, stop with clear error if invalid

**Note**: This script validates the version format and sets up the VERSION variable for use throughout the release process.
</ArgumentValidation>

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

<VersionValidationAndPreChecks>
## Version Confirmation and Pre-Release Validation

Execute <ConfirmVersionFormat/>
Execute <GitStatusCheck/>
Execute <PreReleaseChecks/>
</VersionValidationAndPreChecks>

<ConfirmVersionFormat>
**Confirm version format with user:**
```
Version to release: ${VERSION}
Format should be X.Y.Z (e.g., 0.3.0) or X.Y.Z-rc.N (e.g., 0.3.0-rc.1)
```
→ **Manual confirmation**: Verify the version looks correct before proceeding
</ConfirmVersionFormat>

<GitStatusCheck>
```bash
git status
```
→ **Auto-check**: Continue if clean, stop if uncommitted changes

```bash
git fetch origin
```
</GitStatusCheck>

<PreReleaseChecks>
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
</PreReleaseChecks>

<ChangelogVerification>
## Verify CHANGELOG Entries

Execute <CheckAllChangelogs/>
</ChangelogVerification>

<CheckAllChangelogs>
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
</CheckAllChangelogs>

<ThreePhaseReleaseProcess>
## Three-Phase Release Process

Execute <Phase1ReleaseMcpMacros/>
Execute <Phase2UpdateMcpDependency/>
Execute <Phase3ReleaseExtrasAndMcp/>
</ThreePhaseReleaseProcess>

<Phase1ReleaseMcpMacros>
### Phase 1: Release mcp_macros

```bash
cargo release ${VERSION} --package bevy_brp_mcp_macros --dry-run
```
→ **Manual verification**: Review the dry run output - version bumps, CHANGELOG updates, git operations all look correct
  - Type **continue** to proceed with release execution
  - Type **stop** to halt process for manual review

```bash
cargo release ${VERSION} --package bevy_brp_mcp_macros --execute
```
→ **Manual verification**: Release completed successfully, git commit created
  - Type **continue** to proceed to next phase
  - Type **stop** to halt and investigate issues

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

# Phase 1 does not create tags

```bash
cd mcp_macros && cargo publish --dry-run
```
→ **Manual verification**: Review package contents, ensure all files included, no errors
  - Type **continue** to proceed with publishing
  - Type **stop** to halt and fix package issues

```bash
cargo publish
```
→ **Auto-check**: Continue if publish succeeds, stop if fails

```bash
cd ..
```
</Phase1ReleaseMcpMacros>

<Phase2UpdateMcpDependency>
### Phase 2: Update mcp Dependency

**Wait ~1 minute for mcp_macros to be available on crates.io**

```bash
curl -s https://crates.io/api/v1/crates/bevy_brp_mcp_macros | jq '.crate.max_version'
```
→ **Manual verification**: Shows the version you just published
  - Type **continue** to proceed with dependency update
  - Type **retry** to check again after waiting
- If null or incorrect, wait 10-20 seconds and try again

**Update mcp/Cargo.toml dependency**
Edit `mcp/Cargo.toml` to change:
```toml
# FROM:
bevy_brp_mcp_macros = { path = "../mcp_macros" }
# TO:
bevy_brp_mcp_macros = "${VERSION}"  # the version you just published
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
</Phase2UpdateMcpDependency>

<Phase3ReleaseExtrasAndMcp>
### Phase 3: Release extras and mcp

```bash
cargo release ${VERSION} --workspace --exclude bevy_brp_mcp_macros --dry-run
```
→ **Manual verification**: Review output - both extras and mcp will be released together
  - Type **continue** to proceed with workspace release
  - Type **stop** to halt and review changes

```bash
cargo release ${VERSION} --workspace --exclude bevy_brp_mcp_macros --execute
```
→ **Auto-check**: Continue if release succeeds, stop if errors
</Phase3ReleaseExtrasAndMcp>

<PushAndPublishRemaining>
## Push and Publish Remaining Crates

Execute <PushToGit/>
Execute <PublishRemainingCrates/>
</PushAndPublishRemaining>

<PushToGit>
```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

```bash
git push origin --tags
```
→ **Auto-check**: Continue if push succeeds, stop if fails
</PushToGit>

<PublishRemainingCrates>
```bash
cd extras && cargo publish --dry-run
```
→ **Manual verification**: Review package contents for extras
  - Type **continue** to proceed with extras publishing
  - Type **stop** to halt and fix package issues

```bash
cd ../mcp && cargo publish --dry-run
```
→ **Manual verification**: Review package contents for mcp
  - Type **continue** to proceed with mcp publishing
  - Type **stop** to halt and fix package issues

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
</PublishRemainingCrates>

<UpdateDocumentation>
## Update Documentation (First Release Only)

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
</UpdateDocumentation>

<CreateGitHubRelease>
## Create GitHub Release

→ **I will gather CHANGELOG entries from all three crates and create a combined release using GitHub CLI**

```bash
gh release create v${VERSION} \
  --repo natepiano/bevy_brp \
  --title "bevy_brp v${VERSION}" \
  --notes "Combined release notes from all three crates"
```
→ **Auto-check**: Continue if release created successfully, stop if fails
</CreateGitHubRelease>

<PostReleaseVerification>
## Post-Release Verification

```bash
for crate in bevy_brp_mcp_macros bevy_brp_extras bevy_brp_mcp; do
  echo -n "$crate: "
  curl -s https://crates.io/api/v1/crates/$crate | jq '.crate.max_version'
done
```
→ **Manual verification**: All three show version ${VERSION}
  - Type **continue** to proceed with installation test
  - Type **retry** to check versions again

```bash
cargo install bevy_brp_mcp --version ${VERSION}
```
→ **Manual verification**: Installation succeeds, pulling all dependencies from crates.io
  - Type **continue** to proceed to agentic testing
  - Type **stop** to halt and investigate installation issues

**Run agentic tests to verify functionality:**
→ **Manual**: Run your agentic test suite to verify RC/release functionality
</PostReleaseVerification>

<PrepareNextReleaseCycle>
## Prepare for Next Release

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
</PrepareNextReleaseCycle>

## Rollback Instructions

If something goes wrong after pushing but before publishing:

```bash
# Delete local tag
git tag -d v${VERSION}

# Delete remote tag  
git push origin :refs/tags/v${VERSION}

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

## Design Review Skip Notes

### DESIGN-4: Limited Error Recovery Mechanisms - **Verdict**: CONFIRMED
- **Status**: SKIPPED
- **Location**: Section: Rollback Instructions
- **Issue**: Rollback instructions only cover post-push scenarios. No recovery mechanisms for failures during the multi-phase release process.
- **Reasoning**: While the finding correctly identifies gaps in error recovery procedures for each phase of the release process, the current documentation is sufficient.
- **Decision**: User elected to skip this recommendation - will figure out recovery if failures happen as this isn't that complex of a process

## Future Releases (Post-Initial)

**After the first release establishes all crates on crates.io:**

Once the initial release is complete and all crates use version dependencies (not path dependencies), future releases can use the simplified Rust 1.90.0+ workflow:

### Simplified Process
1. **Version Management**: Use cargo-release for version bumps and CHANGELOG updates
   ```bash
   cargo release ${VERSION} --workspace --no-publish --execute
   ```

2. **Publishing**: Use native workspace publishing (automatically handles dependency order)
   ```bash
   cargo publish --workspace --dry-run
   cargo publish --workspace
   ```

This eliminates the complex 3-phase process and manual dependency updates. The native `cargo publish --workspace` (available since Rust 1.90.0) automatically:
- Determines correct publishing order based on dependencies
- Verifies the entire workspace builds with to-be-published versions
- Publishes all crates in the correct sequence

### Prerequisites for Simplified Workflow
- All crates must already exist on crates.io
- `mcp/Cargo.toml` must use version dependency for `mcp_macros` (not path)
- Rust toolchain version 1.90.0 or later

## Common Issues

1. **"Version already exists"**: The version is already published on crates.io
2. **"Uncommitted changes"**: Run `git status` and commit or stash changes
3. **"Not on main branch"**: Switch to main with `git checkout main`
4. **Build failures**: Fix any compilation errors before releasing
5. **Dependency chain**: Always publish mcp_macros → extras → mcp
6. **Path dependency**: Ensure mcp/Cargo.toml uses crates.io version of mcp_macros, not path

