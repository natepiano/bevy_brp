# Release Command - Phased Workspace Release

Perform a coordinated release for bevy_brp workspace crates using a phased approach to ensure proper dependency versioning.

## Versioning Strategy

This workspace uses a **branch-first release model**:
- **main branch**: Always has `-dev` versions (e.g., `0.18.0-dev`, `0.19.0-dev`)
- **release branches**: Created BEFORE publishing, contain actual release versions
- **Publishing**: Always happens from release branches, never from main

This keeps main clean for development while isolating release work to dedicated branches.

## Usage
- `/release X.Y.Z-rc.N` - Release all 3 crates as RC version (e.g., `0.18.0-rc.1`)
- `/release X.Y.Z` - Release all 3 crates as final version (e.g., `0.18.0`)

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
- Documentation shows: `git checkout -b release-${VERSION}`
- You should run: `git checkout -b release-0.17.2`

This applies to ALL bash commands in this process.

## Prerequisites Check

Before starting the release, verify:
1. You're on the `main` branch
2. Working directory is clean (no uncommitted changes)
3. You're up to date with remote

<ProgressBehavior>
**AT START**: Dynamically generate and display the full progress list (once only):

1. Scan this document for all `## STEP N:` headers
2. Extract step number and description from each header
3. Count total steps and display as:

```
═══════════════════════════════════════════════════════════════
                 RELEASE ${VERSION} - PROGRESS
═══════════════════════════════════════════════════════════════
[ ] STEP 0:  <description from "## STEP 0: ..." header>
[ ] STEP 1:  <description from "## STEP 1: ..." header>
... (continue for all steps found)
═══════════════════════════════════════════════════════════════
```

**BEFORE EACH STEP**: Output single progress line using the total step count:
```
**[N/total] Step description...**
```
</ProgressBehavior>

<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    Display <ProgressBehavior/> full list, then proceed:

    **STEP 0:** Execute <ArgumentValidation/>
    **STEP 1:** Execute <PreReleaseChecks/>
    **STEP 2:** Execute <UpdateReadmeCompatibilityOnMain/>
    **STEP 3:** Execute <CreateReleaseBranch/>
    **STEP 4:** Execute <BumpDevToRelease/>
    **STEP 5:** Execute <ChangelogVerification/>
    **STEP 6:** Execute <FinalizeChangelogs/>
    **STEP 7:** Execute <CheckWorkspaceDependency/>
    **STEP 8:** Execute <BumpMacrosVersion/>
    **STEP 9:** Execute <Phase1PublishMacros/>
    **STEP 10:** Execute <UpdateWorkspaceDependency/>
    **STEP 11:** Execute <Phase2PublishExtrasAndMcp/>
    **STEP 12:** Execute <PushReleaseBranch/>
    **STEP 13:** Execute <CreateGitHubRelease/>
    **STEP 14:** Execute <PostReleaseVerification/>
    **STEP 15:** Execute <MergeAndPrepareNextCycle/>
</ExecutionSteps>

<ArgumentValidation>
## STEP 0: Argument Validation

**Valid version formats:**
- `X.Y.Z` - Final release (e.g., `0.18.0`)
- `X.Y.Z-rc.N` - Release candidate (e.g., `0.18.0-rc.1`)

→ **Manual check**: Verify `$ARGUMENTS` matches one of these formats before proceeding. Stop with clear error if invalid.

**Note**: Throughout this process, substitute the actual version number from `$ARGUMENTS` wherever you see `${VERSION}` or `$VERSION` in commands. Do not use shell variables.
</ArgumentValidation>

<PreReleaseChecks>
## STEP 1: Pre-Release Validation (on main)

Execute <GitStatusCheck/>
Execute <QualityChecks/>
</PreReleaseChecks>

<GitStatusCheck>
### Git Status Check

```bash
git rev-parse --abbrev-ref HEAD
```
→ **Auto-check**: Must be on `main` branch, stop if not

```bash
git status --porcelain
```
→ **Auto-check**: Continue if clean (empty output), stop if uncommitted changes

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

<UpdateReadmeCompatibilityOnMain>
## STEP 2: Update README Compatibility Tables (on main)

**IMPORTANT**: This step happens on main BEFORE creating the release branch. This ensures README updates are already on main and will be included in the release branch.

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

**Commit the README changes on main:**
```bash
git add README.md mcp/README.md extras/README.md
git commit -m "docs: update compatibility tables for v${VERSION}"
```
→ **Auto-check**: Continue if commit succeeds
</UpdateReadmeCompatibilityOnMain>

<CreateReleaseBranch>
## STEP 3: Create Release Branch

**CRITICAL**: Create the release branch AFTER README updates but BEFORE version changes. This ensures README updates are included in the release branch.

```bash
git checkout -b release-${VERSION}
```
→ **Auto-check**: Continue if branch created successfully

**Verify you're on the release branch:**
```bash
git rev-parse --abbrev-ref HEAD
```
→ **Auto-check**: Should show `release-${VERSION}`

**Note**: All subsequent steps happen on this release branch until Step 15.
</CreateReleaseBranch>

<BumpDevToRelease>
## STEP 4: Bump Versions from -dev to Release

**Check current versions (should be -dev):**
```bash
grep "^version" extras/Cargo.toml mcp/Cargo.toml
```

→ **I will update the version in these files** from `-dev` to the release version:
- `extras/Cargo.toml`: Change `version = "X.Y.Z-dev"` to `version = "${VERSION}"`
- `mcp/Cargo.toml`: Change `version = "X.Y.Z-dev"` to `version = "${VERSION}"`

**Note**: `mcp_macros` version is handled separately in Step 8.

**Commit the version bumps:**
```bash
git add extras/Cargo.toml mcp/Cargo.toml Cargo.lock
git commit -m "chore: bump extras and mcp versions to ${VERSION}"
```
→ **Auto-check**: Continue if commit succeeds
</BumpDevToRelease>

<ChangelogVerification>
## STEP 5: Verify CHANGELOG Entries

**Note**: For coordinated workspace releases where some crates have no feature changes, add placeholder entries:

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

<FinalizeChangelogs>
## STEP 6: Finalize CHANGELOG Headers

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
## STEP 7: Check Workspace Dependency

**Check current workspace dependency for mcp_macros:**

```bash
grep "bevy_brp_mcp_macros" Cargo.toml
```

→ **Manual note**: Observe current dependency (path or version). We'll update this AFTER publishing mcp_macros in Step 10.

**Note**: We deliberately DON'T update the workspace dependency yet. If we update it now to `"${VERSION}"`, cargo metadata will fail for the whole workspace until that version exists on crates.io. We'll update it after publishing mcp_macros.
</CheckWorkspaceDependency>

<BumpMacrosVersion>
## STEP 8: Bump mcp_macros Version

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
## STEP 9: Phase 1 - Publish mcp_macros Only

**Note**: The workspace dependency still points to the old version, so cargo metadata works. The mcp_macros version was bumped in Step 8.

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
## STEP 10: Update Workspace Dependency

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

**Note**: Now `cargo publish` will work for extras and mcp in Step 11 because the workspace dependency resolves correctly from crates.io.
</UpdateWorkspaceDependency>

<Phase2PublishExtrasAndMcp>
## STEP 11: Phase 2 - Publish extras and mcp

**Publish extras to crates.io:**

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

<PushReleaseBranch>
## STEP 12: Push Release Branch and Tags

**Create tag on release branch:**
```bash
git tag "v${VERSION}"
```
→ **Auto-check**: Continue if tag created

**Push the release branch:**
```bash
git push -u origin release-${VERSION}
```
→ **Auto-check**: Continue if push succeeds, stop if fails

**Push the tag:**
```bash
git push origin "v${VERSION}"
```
→ **Auto-check**: Continue if push succeeds, stop if fails
</PushReleaseBranch>

<CreateGitHubRelease>
## STEP 13: Create GitHub Release

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
## STEP 14: Post-Release Verification

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

<MergeAndPrepareNextCycle>
## STEP 15: Merge Release to Main and Prepare Next Cycle

**Return to main branch:**
```bash
git checkout main
```
→ **Auto-check**: Continue if checkout succeeds

**Merge release branch into main:**
This brings all release changes (finalized CHANGELOGs, version bumps, any fixes made during release) back to main.

```bash
git merge release-${VERSION} -m "Merge release-${VERSION} into main"
```
→ **Auto-check**: Continue if merge succeeds (usually fast-forward)

**Determine next dev version:**
- If released `0.18.0-rc.N`, next dev is `0.18.0-dev` (until final release)
- If released final `0.18.0`, next dev is `0.19.0-dev`

→ **I will ask**: What should the next dev version be?

**Update versions on main to next -dev version:**
- `extras/Cargo.toml`: `version = "${NEXT_DEV_VERSION}"`
- `mcp/Cargo.toml`: `version = "${NEXT_DEV_VERSION}"`
- `mcp_macros/Cargo.toml`: `version = "${NEXT_DEV_VERSION}"`

→ **I will add [Unreleased] sections to all three CHANGELOG.md files**

Add `## [Unreleased]` above the now-merged `## [${VERSION}]` header in each CHANGELOG.md:
```markdown
## [Unreleased]

## [${VERSION}] - YYYY-MM-DD
```

```bash
git add mcp_macros/CHANGELOG.md extras/CHANGELOG.md mcp/CHANGELOG.md mcp_macros/Cargo.toml extras/Cargo.toml mcp/Cargo.toml Cargo.lock
```
→ **Auto-check**: Continue if successful

```bash
git commit -m "chore: prepare for next release cycle (${NEXT_DEV_VERSION})"
```
→ **Auto-check**: Continue if commit succeeds, stop if fails

```bash
git push origin main
```
→ **Auto-check**: Continue if push succeeds, stop if fails

**✅ Release complete!** All crates published. Release branch merged to main. Main now at next dev version.
</MergeAndPrepareNextCycle>

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

# Delete release branch
git branch -D release-${VERSION}
git push origin :release-${VERSION}

# Return to main (which is unchanged)
git checkout main
```

If already published to crates.io, you cannot unpublish. You'll need to release a new patch version.

## Common Issues

1. **"Version already exists"**: The version is already published on crates.io
2. **"Uncommitted changes"**: Run `git status` and commit or stash changes
3. **"Not on main branch"**: Switch to main with `git checkout main`
4. **Build failures**: Fix any compilation errors before releasing
5. **Dependency chain**: Must publish mcp_macros first, then extras and mcp
6. **Path dependency during release**: If workspace has path dependency for mcp_macros, switch to version dependency before publishing

## Branch Workflow Summary

```
main (0.18.0-dev) ──[README updates]──┬────────────────────────→ (0.19.0-dev)
                                      │                              ↑
                                      └─→ release-0.18.0 (0.18.0)    │
                                              │                      │
                                              ├─→ publish            │
                                              ├─→ tag v0.18.0        │
                                              │                      │
                                              └──────── merge ───────┘
```

**Key Points:**
- Main ALWAYS has `-dev` versions
- README updates happen on main BEFORE creating release branch
- Release branch is created AFTER README updates
- Publishing happens exclusively from release branches
- After release, merge release branch into main
- Then bump main to next `-dev` version
- Each release branch can receive patches independently
