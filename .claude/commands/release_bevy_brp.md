# Release Command - Phased Workspace Release

Perform a coordinated release for bevy_brp workspace crates using a phased approach to ensure proper dependency versioning.

## Usage
- `/release X.Y.Z-rc.N` - Release all 3 crates as RC version (e.g., `0.17.0-rc.1`)
- `/release X.Y.Z` - Release all 3 crates as final version (e.g., `0.17.0`)

## Crate Dependency Chain
1. `mcp_macros` (no dependencies) - **Published FIRST**
2. `extras` (depends on bevy only) - Published second
3. `mcp` (depends on mcp_macros via workspace) - Published second

## Phased Release Strategy

**Why phased?** The `mcp` crate depends on `mcp_macros`. To ensure correct version dependencies on crates.io:

1. **Check/Update**: Ensure workspace dependency is set to the new release version (not path)
2. **Phase 1**: Publish `mcp_macros` alone
3. **Phase 2**: Publish `extras` and `mcp` (which correctly depend on the published `mcp_macros` via workspace)

**Workspace dependency approach**: The workspace normally keeps `mcp_macros` as a version dependency pointing to the last published version. Only when actively developing macros would you temporarily use a path dependency. Before release, ensure it's set to the new version number.

## IMPORTANT: Version Handling in Commands

**Throughout this release process**, when you see `${VERSION}` in bash commands, you must substitute the actual version number directly (e.g., "0.17.2") instead of using shell variables. Shell variable assignments require user approval.

**Example:**
- Documentation shows: `cargo release ${VERSION} --package bevy_brp_mcp_macros`
- You should run: `cargo release 0.17.2 --package bevy_brp_mcp_macros`

This applies to ALL bash commands in this process.

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
    **STEP 2.5:** Execute <UpdateReadmeCompatibility/>
    **STEP 2.6:** Execute <FinalizeChangelogs/>
    **STEP 3:** Execute <CheckWorkspaceDependency/>
    **STEP 3.5:** Execute <BumpMacrosVersion/>
    **STEP 4:** Execute <Phase1PublishMacros/>
    **STEP 4.5:** Execute <UpdateWorkspaceDependency/>
    **STEP 5:** Execute <Phase2PublishExtrasAndMcp/>
    **STEP 6:** Execute <PushToGit/>
    **STEP 7:** Execute <CreateReleaseBranch/>
    **STEP 8:** Execute <CreateGitHubRelease/>
    **STEP 9:** Execute <PostReleaseVerification/>
    **STEP 10:** Execute <PrepareNextReleaseCycle/>
</ExecutionSteps>

<ArgumentValidation>
## STEP 0: Argument Validation and VERSION Setup

**Validate the version format (using the argument directly):**
```bash
bash .claude/scripts/release_version_validate.sh "$ARGUMENTS"
```
→ **Auto-check**: Continue if version is valid format, stop with clear error if invalid

**Confirm version:**
```bash
echo "Release version set to: $ARGUMENTS"
```

**Note**: Throughout this process, `$ARGUMENTS` will be used directly instead of a shell variable to avoid approval requirements. In documentation below, when you see `${VERSION}` or `$VERSION`, substitute the actual version number from `$ARGUMENTS`.
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

**Note**: For coordinated workspace releases where some crates have no feature changes, add placeholder entries (without `## [Unreleased]` header - that gets added later in Step 9):

```markdown
### Changed
- Version bump to X.Y.Z to maintain workspace version synchronization
```

```bash
for crate in mcp_macros extras mcp; do
  echo "Checking $crate CHANGELOG..."
  # Look for either [Unreleased] section or placeholder entry at top
  if ! (grep -q "## \[Unreleased\]" $crate/CHANGELOG.md || head -10 $crate/CHANGELOG.md | grep -q "### Changed"); then
    echo "ERROR: Missing changelog entry in $crate/CHANGELOG.md"
    exit 1
  fi
  head -15 $crate/CHANGELOG.md | grep -A 5 "###"
done
```
→ **Manual verification**: Verify changelog entries exist for all crates (either `[Unreleased]` section or placeholder entry)
  - Type **continue** to proceed
  - Type **stop** to add missing entries
</ChangelogVerification>

<UpdateReadmeCompatibility>
## STEP 2.5: Update README Compatibility Tables

**Update the Bevy compatibility information in README files:**

→ **Manual task**: Update compatibility tables in the following files:
  - `README.md` - Main repository README
  - `mcp/README.md` - MCP crate README
  - `extras/README.md` - Extras crate README (if exists)

**What to update:**
- Update the version range for the current Bevy series (e.g., `0.17.0-0.17.1` for patch releases)
- Update example version numbers in installation instructions
- Any version-specific notes or warnings

**Compatibility table format (use explicit ranges):**
```markdown
| bevy_brp_mcp   | Bevy  |
|----------------|-------|
| 0.17.0-0.17.1  | 0.17  |
| 0.16.0         | 0.16  |
```

→ **Manual verification**: Confirm all READMEs updated with new version range
  - Type **continue** to proceed
  - Type **stop** to update READMEs

**Commit the README changes:**
```bash
git add README.md mcp/README.md extras/README.md
git commit -m "docs: update compatibility tables for v${VERSION}"
```
→ **Auto-check**: Continue if commit succeeds
</UpdateReadmeCompatibility>

<FinalizeChangelogs>
## STEP 2.6: Finalize CHANGELOG Headers

**IMPORTANT**: This step happens BEFORE publishing so the published crates have correct version headers.

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

<CheckWorkspaceDependency>
## STEP 3: Check Workspace Dependency

**Check current workspace dependency for mcp_macros:**

```bash
grep "bevy_brp_mcp_macros" Cargo.toml
```

→ **Manual note**: Observe current dependency (path or version). We'll update this AFTER publishing mcp_macros in Step 4.5.

**Note**: We deliberately DON'T update the workspace dependency yet. If we update it now to `"${VERSION}"`, cargo metadata will fail for the whole workspace until that version exists on crates.io. We'll update it after publishing mcp_macros.
</CheckWorkspaceDependency>

<BumpMacrosVersion>
## STEP 3.5: Bump mcp_macros Version

→ **I will update only `mcp_macros/Cargo.toml` version:**

```bash
# Check current version
grep "^version" mcp_macros/Cargo.toml
```

**Update mcp_macros/Cargo.toml:**
Set `version = "${VERSION}"`

**Commit the mcp_macros version bump:**
```bash
git add mcp_macros/Cargo.toml
git commit -m "chore: bump bevy_brp_mcp_macros to ${VERSION}"
```
→ **Auto-check**: Continue if commit succeeds

**Note**: We only bump mcp_macros version here, not the workspace dependency. This allows cargo to still resolve the workspace while we publish mcp_macros.
</BumpMacrosVersion>

<Phase1PublishMacros>
## STEP 4: Phase 1 - Publish mcp_macros Only

**Note**: The workspace dependency still points to the old version, so cargo metadata works. The mcp_macros version was bumped in Step 3.5.

**Publish mcp_macros to crates.io:**

```bash
cargo publish --package bevy_brp_mcp_macros --dry-run
```
→ **Manual verification**: Review package contents
  - Type **continue** to publish
  - Type **stop** to fix issues

```bash
cargo publish --package bevy_brp_mcp_macros
```
→ **Auto-check**: Continue if publish succeeds, stop if fails

**Wait for crates.io indexing:**
```bash
echo "⏳ Waiting 30 seconds for crates.io to index bevy_brp_mcp_macros ${VERSION}..."
sleep 30
```
→ **Auto-check**: Continue after wait completes
</Phase1PublishMacros>

<UpdateWorkspaceDependency>
## STEP 4.5: Update Workspace Dependency

**Now that mcp_macros ${VERSION} exists on crates.io, update the workspace dependency:**

→ **I will update `Cargo.toml` (workspace root):**

**If it shows a path dependency**, replace with version:
```toml
bevy_brp_mcp_macros = "${VERSION}"
```

**If it shows an old version**, update to:
```toml
bevy_brp_mcp_macros = "${VERSION}"
```

**Verify the workspace builds:**
```bash
cargo build --package bevy_brp_mcp
```
→ **Auto-check**: Continue if build succeeds (now that ${VERSION} exists on crates.io)

**Commit the workspace dependency update:**
```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: update workspace dependency to bevy_brp_mcp_macros ${VERSION}"
```
→ **Auto-check**: Continue if commit succeeds

**Note**: Now cargo-release will work for extras and mcp in Step 5 because the workspace dependency resolves correctly from crates.io.
</UpdateWorkspaceDependency>

<Phase2PublishExtrasAndMcp>
## STEP 5: Phase 2 - Publish extras and mcp

**Run cargo-release for extras and mcp only:**

```bash
cargo release ${VERSION} --package bevy_brp_extras --package bevy_brp_mcp
```
→ **Manual verification**: Review the dry run output - version bumps for extras and mcp
  - Type **continue** to execute
  - Type **stop** to halt

```bash
echo "y" | cargo release ${VERSION} --package bevy_brp_extras --package bevy_brp_mcp --execute
```
→ **Auto-check**: Continue if release succeeds, stop if errors

**Verify version bumps:**
```bash
grep "^version" extras/Cargo.toml mcp/Cargo.toml
```
→ **Auto-check**: Both should show `version = "${VERSION}"`, stop if not

**Publish extras and mcp to crates.io:**

```bash
cargo publish --package bevy_brp_extras --dry-run
```
→ **Manual verification**: Review package contents
  - Type **continue** to proceed
  - Type **stop** to fix issues

```bash
cargo publish --package bevy_brp_mcp --dry-run
```
→ **Manual verification**: Review package contents
  - Type **continue** to publish both
  - Type **stop** to fix issues

**Publish both crates (extras first, then mcp):**
```bash
cargo publish --package bevy_brp_extras && cargo publish --package bevy_brp_mcp
```
→ **Auto-check**: Continue if both publishes succeed, stop if any fail
</Phase2PublishExtrasAndMcp>

<PushToGit>
## STEP 6: Push to Git

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

```bash
git push origin --tags
```
→ **Auto-check**: Continue if push succeeds, stop if fails
</PushToGit>

<CreateReleaseBranch>
## STEP 7: Create Release Branch

**CRITICAL: Every release MUST have its own release branch (e.g., `release-0.17.3`).**

This applies to:
- Initial releases from `main` (e.g., 0.17.0)
- Patch releases from existing release branches (e.g., 0.17.3 from release-0.17.2)

**Create the release branch:**

```bash
git checkout -b release-${VERSION} && git push -u origin release-${VERSION}
```
→ **Auto-check**: Continue if branch created and pushed successfully

**Return to the original branch:**

If releasing from `main`:
```bash
git checkout main
```

If releasing from a release branch (patch release):
```bash
git checkout main
```
→ **Note**: For patch releases, we return to main (not the old release branch) since the new release branch is now the canonical location for future patches.

**Why**: Release branches allow patch releases (e.g., v0.17.2 → v0.17.3) without
disturbing main development. This follows Bevy's proven workflow where:
- `main` is for active development
- `release-X.Y.Z` branches are stable points for patches
- Each release gets its own branch for potential future patches
- Both can be developed independently and fixes can be backported

</CreateReleaseBranch>

<CreateGitHubRelease>
## STEP 8: Create GitHub Release

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
## STEP 9: Post-Release Verification

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
## STEP 10: Prepare for Next Release Cycle

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

**✅ Release complete!** All crates published with correct dependencies and release branch created.
</PrepareNextReleaseCycle>

## Workspace Dependency Strategy

**Normal state (most development):**
- Workspace dependency for `mcp_macros` points to the last published version (e.g., `"0.17.1"`)
- Most development doesn't touch macros, so this keeps things simple
- Users get correct dependency versions from crates.io

**When developing macros:**
- Temporarily switch to path dependency: `{ path = "mcp_macros" }`
- Test changes to mcp_macros immediately in the mcp crate
- Iterate quickly without publishing to crates.io
- Before releasing, switch back to version dependency with the new version number

**Why this approach?**
- No artificial toggling during release process
- Workspace dependency stays stable most of the time
- Only macro developers need to temporarily use path dependencies
- Release process is simpler: just bump version and publish

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
6. **Path dependency during release**: If workspace has path dependency for mcp_macros, switch to version dependency before publishing

## Release Branch Workflow

### Philosophy

This project follows Bevy's release branch strategy:
- **main**: active development, bleeding edge
- **release-X.Y.Z**: stable release points that can receive patches
- **Tags**: `vX.Y.Z` tags on release branches mark actual releases

### When to Use Release Branches

**Every release creates a branch:**
- Patch releases (0.17.1): Create `release-0.17.1` from main
- Minor releases (0.18.0): Create `release-0.18.0` from main
- Major releases (1.0.0): Create `release-1.0.0` from main

### Patching a Released Version

If a bug is found in v0.17.1 that needs a v0.17.2 patch:

**Option A: Fix on release branch, backport to main**
```bash
# Switch to release branch
git checkout release-0.17.1

# Create fix or cherry-pick from main
git cherry-pick <fix-commit>  # or make changes directly

# Update CHANGELOG
# Edit mcp/CHANGELOG.md to add [0.17.2] section

# Commit changelog
git add mcp/CHANGELOG.md
git commit -m "chore: finalize CHANGELOG for v0.17.2"

# Tag the patch release
git tag v0.17.2
git push origin release-0.17.1 --tags

# Publish to crates.io (only mcp crate if that's what changed)
cargo publish --package bevy_brp_mcp

# Backport to main
git checkout main
git cherry-pick <fix-commit>
git push origin main
```

**Option B: Fix on main, backport to release**
```bash
# Fix on main first
git checkout main
# Make changes, commit, push
git commit -m "fix: critical bug"
git push origin main

# Backport to release branch
git checkout release-0.17.1
git cherry-pick <fix-commit-sha>

# Update CHANGELOG, tag, publish (same as Option A)
```

### Accepting PRs Against Release Branches

Contributors can target specific releases:
1. In GitHub PR UI, change target branch from `main` to `release-0.17.1`
2. Review and merge as normal
3. Cherry-pick to main: `git checkout main && git cherry-pick <merge-commit>`

### Branch Lifecycle

```
main: v0.17.0 → v0.17.1 → v0.18.0 → v0.19.0 (continuous development)
         ↓         ↓         ↓
release-0.17.0   release-0.17.1   release-0.18.0
         ↓              ↓
      v0.17.0       v0.17.1 → v0.17.2 (patches)
```

**Key Points:**
- Release branches can outlive their creation point on main
- Patches only affect the release branch + backport to main
- Main continues forward without waiting for patch releases
- Each release branch is independent and can be patched separately
