# Release Branch Management Plan - Collaborative Mode

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Command examples showing what to execute
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for user confirmation ("go ahead" or similar)

3. **IMPLEMENT**: Make the changes and stop

4. **VALIDATE**: Execute verification commands:
   ```bash
   git status
   git log --oneline
   ```

5. **CONFIRM**: Wait for user to confirm the operation succeeded

6. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

7. **PROCEED**: Move to next step only after confirmation
</Instructions>

<ExecuteImplementation>
Find the next ⏳ PENDING step in the INTERACTIVE IMPLEMENTATION SEQUENCE below.

For the current step:
1. Follow the <Instructions/> above for executing the step
2. When step is complete, use Edit tool to mark it as ✅ COMPLETED
3. Continue to next PENDING step

If all steps are COMPLETED:
    Display: "✅ Implementation complete! All steps have been executed."
</ExecuteImplementation>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### STEP 0.5: Create Backup Branch ✅ COMPLETED

**Objective**: Create a safety backup of current main state before any modifications

**Change Type**: SAFE - Creates backup branch only

**Changes**:
- Create `backup-main-before-split` branch from current main
- Push to origin for safekeeping

**Files**: None (git operations only)

**Git Commands**:
```bash
git checkout main
git branch backup-main-before-split
git push -u origin backup-main-before-split
```

**Why this is important**:
This backup branch gives you a **guaranteed recovery point**. If anything goes wrong during the process, you can always:
```bash
git checkout main
git reset --hard backup-main-before-split
git push origin main --force-with-lease
```

**Verification**:
```bash
# Verify backup branch exists
git branch -a | grep backup-main-before-split
git ls-remote --heads origin | grep backup-main-before-split
```

**Success Criteria**:
- Backup branch created locally and pushed to remote
- Main branch unchanged
- Safe to proceed with modifications

---

### STEP 1: Create Preservation Branches ✅ COMPLETED

**Objective**: Create release-0.17.0 and v0.18-dev branches to preserve release states

**Change Type**: SAFE - Additive (creates branches, doesn't modify existing)

**Changes**:
- Create `release-0.17.0` branch from v0.17.0 tag
- Create `v0.18-dev` branch from dbae7815 (last breaking change commit)
- Push both branches to origin

**Files**: None (git operations only)

**Git Commands**:
```bash
# Create release-0.17.0 from tag
git checkout -b release-0.17.0 v0.17.0
git push -u origin release-0.17.0
git checkout main

# Create v0.18-dev from breaking changes
git branch v0.18-dev dbae7815
git push -u origin v0.18-dev
```

**Verification**:
```bash
# Should show 3 commits (the actual breaking changes only: 009beaa6, a47ffd59, dbae7815)
git log --oneline 1afc1bff..v0.18-dev | wc -l

# Verify branches exist on remote
git ls-remote --heads origin | grep -E '(release-0.17.0|v0.18-dev)'
```

**Success Criteria**:
- Both branches created locally and on remote
- v0.18-dev contains exactly 3 commits since 1afc1bff (the actual breaking changes)
- Current branch is main

---

### STEP 2: Fix v0.18-dev Branch ✅ COMPLETED

**Objective**: Update integration tests and CHANGELOG on v0.18-dev to match the breaking changes

**Change Type**: SAFE - Fixes on separate branch

**Dependencies**: Requires Step 1 (v0.18-dev branch must exist)

**Changes**:
1. Update `.claude/integration_tests/type_guide.md` to use `spawn_example`/`resource_example`
2. Update `.claude/scripts/type_guide_test_extract.sh` to extract new fields
3. Clean up CHANGELOG to keep only breaking changes for v0.18

**Files**:
- `.claude/integration_tests/type_guide.md`
- `.claude/scripts/type_guide_test_extract.sh`
- `mcp/CHANGELOG.md`

**Git Commands**:
```bash
# Switch to v0.18-dev
git checkout v0.18-dev

# Make edits to the files listed above
# Replace spawn_format → spawn_example for Components
# Replace spawn_format → resource_example for Resources
# Update CHANGELOG to remove non-breaking fixes

# Commit integration test updates
git add .claude/integration_tests/type_guide.md .claude/scripts/type_guide_test_extract.sh
git commit -m "test: update type_guide integration test for spawn_example/resource_example API

The breaking changes in commits c0b9a91e..dbae7815 renamed spawn_format to
spawn_example/resource_example but the integration tests were not updated.
This commit updates the test file and extraction script to use the new API."

# Commit CHANGELOG update
git add mcp/CHANGELOG.md
git commit -m "docs: update CHANGELOG for v0.18 breaking changes only

Remove non-breaking fixes that will be released in v0.17.1 on main branch.
Keep only the breaking changes for the upcoming v0.18.0 release."

# Push to remote
git push origin v0.18-dev

# Return to main
git checkout main
```

**Verification**:
```bash
# Verify test file uses new API
git show v0.18-dev:.claude/integration_tests/type_guide.md | grep spawn_example

# Verify CHANGELOG only has breaking changes
git show v0.18-dev:mcp/CHANGELOG.md | grep -A 10 "## \[Unreleased\]"
```

**Success Criteria**:
- Integration tests reference new field names
- CHANGELOG on v0.18-dev only contains breaking changes
- Commits pushed to remote v0.18-dev branch

---

### STEP 3: Reset and Cherry-Pick (ATOMIC GROUP) ✅ COMPLETED

**Objective**: Reset main to 1afc1bff (keeping non-breaking refactoring), cherry-pick commits after breaking changes, force push

**Change Type**: CRITICAL - DANGEROUS (rewrites history)

**Dependencies**: Requires Steps 0.5-2

**⚠️ WARNING**: This is an ATOMIC operation. Once you start, you MUST complete all sub-steps (3a→3b→3c) without stopping. This rewrites main history - coordinate with collaborators first!

**What's happening**:
1. Reset main to 1afc1bff (keeps 11 non-breaking refactoring commits: c0b9a91e through 1afc1bff)
2. This skips the 3 breaking change commits (009beaa6 through dbae7815)
3. Cherry-pick the 7 commits that came AFTER the breaking changes
4. Force push rewritten main

**Sub-Step 3a: Reset main to 1afc1bff**:
```bash
git checkout main
git reset --hard 1afc1bff
```

This keeps all commits up to and including 1afc1bff, which includes:
- c1488b98 (release workflow - from before)
- c0b9a91e through 1afc1bff (11 non-breaking refactoring commits)

**Sub-Step 3b: Cherry-pick commits after breaking changes**:
```bash
# Cherry-pick the 7 commits that came AFTER dbae7815 (the breaking changes)
# Build verification after each cherry-pick to catch issues early

git cherry-pick 988c15c0 && cargo build  # permission updates
# (If conflicts occur, resolve manually then: git cherry-pick --continue && cargo build)

git cherry-pick eb5813dc && cargo build  # comment cleanup
# (resolve conflicts if needed)

git cherry-pick ddf61a0a && cargo build  # file moves
# (resolve conflicts if needed)

git cherry-pick e6b1c939 && cargo build  # anyOf schema fix - CRITICAL
# (resolve conflicts if needed)

git cherry-pick 5dc7992c && cargo build  # debug_protocol
# (resolve conflicts if needed)

git cherry-pick f1d8bae9 && cargo build  # rustfmt settings
# (resolve conflicts if needed)

git cherry-pick 92f2ed0f && cargo build  # ready to implement
# (resolve conflicts if needed)

git cherry-pick 20e2edc1 && cargo build  # permission update
# (resolve conflicts if needed)
```

**Conflict Resolution Process** (if needed):
1. Run `git status` to see conflicting files
2. Manually resolve conflicts in each file
3. Run `git add <resolved-files>`
4. Run `git cherry-pick --continue`
5. Verify build: `cargo build`

**If you get stuck**:
- Run `git cherry-pick --abort` to cancel
- Review commit: `git show <commit-hash>`
- Ask for help with specific conflict
- Can restore from backup: `git reset --hard backup-main-before-split`

**Sub-Step 3c: Force push main**:
```bash
# Force push the rewritten main
git push origin main --force-with-lease
```

**Verification**:
```bash
# Should show 19 commits since v0.17.0 (1 + 11 + 7)
git log --oneline v0.17.0..HEAD | wc -l

# Verify we're at 1afc1bff + 7 cherry-picks
git log --oneline --graph HEAD~7..HEAD

# Verify brp_type_guide has non-breaking changes only
git diff v0.17.0..HEAD -- mcp/src/brp_tools/brp_type_guide/guide.rs | grep "spawn_format"
# Should show spawn_format still present (not spawn_example/resource_example)

# Verify builds successfully
cargo build

# Verify tests pass
cargo nextest run
```

**Success Criteria**:
- Main reset to 1afc1bff (keeps non-breaking refactoring)
- All 7 commits from after breaking changes cherry-picked successfully
- Total 19 commits since v0.17.0
- Build and tests succeed
- Force push completed
- TypeGuide still uses `spawn_format` (breaking change excluded)

---

### STEP 4: Merge PR and Update CHANGELOG ✅ COMPLETED

**Objective**: Merge PR #1 and prepare CHANGELOG for v0.17.1 release

**Change Type**: SAFE - Normal workflow

**Dependencies**: Requires Step 3

**Changes**:
1. Pull latest main
2. Merge PR #1 (MCP schema validation fixes)
3. Remove breaking changes from CHANGELOG
4. Convert [Unreleased] to [0.17.1]

**Sub-Step 4a: Merge PR #1**:
```bash
# Fetch latest
git pull origin main

# Check out PR
gh pr checkout 1

# Review and test
cargo build
cargo nextest run

# Merge PR
gh pr merge 1 --squash
```

**Sub-Step 4b: Update CHANGELOG**:
```bash
# Verify current state
grep -A 20 "## \[Unreleased\]" mcp/CHANGELOG.md
```

Edit `mcp/CHANGELOG.md`:
1. Delete the `### Changed (Breaking)` section entirely
2. Change `## [Unreleased]` to `## [0.17.1] - 2025-11-19` (use actual date)

**Expected result**:
```markdown
## [0.17.1] - 2025-11-19

### Changed
- Improved mutation path descriptions for non-mutable paths to clarify when examples are unavailable
- Simplified release workflow with native workspace publishing (Rust 1.90.0+)

### Fixed
- `world_query` tool description now correctly specifies `filter` parameter as object type
- Fixed "Invalid input" and "did not return structured content" errors in some MCP clients (e.g., Gemini)
- Handle custom struct references in anyOf schemas as Object type

## [0.17.0] - 2025-10-31
...
```

**Verification**:
```bash
# Verify no breaking changes in CHANGELOG
grep -A 20 "## \[0.17.1\]" mcp/CHANGELOG.md | grep -q "Breaking" && echo "ERROR: Breaking changes found!" || echo "OK"

# Verify builds and tests pass
cargo build
cargo nextest run
```

**Success Criteria**:
- PR #1 merged successfully
- CHANGELOG has [0.17.1] header with correct date
- No breaking changes in CHANGELOG
- All tests pass

---

### STEP 5: Update Release Command to Include Release Branch Creation ✅ COMPLETED

**Objective**: Update `.claude/commands/release_version.md` to incorporate release branch creation as part of standard release workflow

**Change Type**: DOCUMENTATION - Update release process

**Dependencies**: Requires Step 4

**Why**: We want release branch creation to be part of our standard release process so that every release (patch, minor, or major) creates a release branch that can be used for future patches without disturbing main development. This follows Bevy's proven workflow.

**Changes Required**:

1. Add new **Step 6.5: Create Release Branch** to `release_version.md` (after Step 6: Push and Publish, before Step 7: Create GitHub Release)
2. Update the `<ExecutionSteps>` section to include the new step
3. Add documentation section explaining the release branch workflow

**Reference**: See the "## Updating release_version.md Command" section below in this plan (starting line 569) for the exact changes to make.

**Files to Modify**:
- `.claude/commands/release_version.md`

**Verification**:
```bash
# Verify Step 6.5 was added
grep -A 5 "STEP 6.5" .claude/commands/release_version.md

# Verify ExecutionSteps updated
grep "STEP 6.5" .claude/commands/release_version.md | head -1
```

**Success Criteria**:
- Step 6.5 (Create Release Branch) added to release_version.md
- ExecutionSteps section updated to include Step 6.5
- Release branch workflow documentation added
- File committed to git

---

### STEP 6: Release v0.17.1 ⏳ PENDING

**Objective**: Execute the `/release 0.17.1` command to finalize and publish the release

**Change Type**: SAFE - Release workflow

**Dependencies**: Requires Step 5 (release_version.md must include Step 6.5)

**Command**:
```bash
/release 0.17.1
```

**What it does**: The `/release` command (per `.claude/commands/release_version.md`) will execute all release steps including:
- Version validation and pre-release checks
- CHANGELOG finalization
- Version bumping and git tagging
- Publishing to crates.io
- **Release branch creation** (via updated Step 6.5)
- GitHub release creation
- Post-release verification

**Success Criteria**:
- All steps in `/release 0.17.1` complete successfully
- `release-0.17.1` branch created and pushed (new behavior from Step 6.5)
- `v0.17.1` tag created and pushed
- All three crates published to crates.io
- CHANGELOG finalized with correct date

---

## Context

After v0.17.0 release, development continued on main with:
- Breaking changes (spawn_format → spawn_example/resource_example refactor)
- Non-breaking improvements and fixes
- Open PR #1 with important MCP schema validation fixes

**Goal**: Extract breaking changes into v0.18 development branch, keep non-breaking changes on main for v0.17.1 patch release.

## Commits Analysis

### Actual Breaking Changes (Nov 3 only - moved to v0.18-dev)
**Only 3 commits** that change the external API:
1. `009beaa6` - **[BREAKING]** refactor: colocate agent guidance with spawn/insert examples
   - Changes `spawn_format` → `spawn_example`/`resource_example`
2. `a47ffd59` - implemented and obsolete (plans/docs for above)
3. `dbae7815` - refactor: make get_default_spawn_guidance static (cleanup after above)

### Non-Breaking Refactoring (Nov 1-2 - kept on main for v0.17.1)
**11 commits** that improve code quality without changing external API:
- `c0b9a91e` - refactor: centralize type knowledge with KnowledgeAction enum (internal)
- `ce4a8626` - fix: ensure deterministic ordering of mutation paths (internal)
- `5cd7d8a6` - permission updates
- `44327e8b` - draft plan
- `361e550c` - refactor: introduce Example enum (internal type system)
- `aaae4746` - refactor: improve mutation path descriptions (text only, same structure)
- `77a88d35` - obsolete plans
- `46661934` - draft upgraded
- `9f5d5735` - refactor: differentiate default guidance (text only, same structure)
- `1afc1bff` - permission updates

**Verified**: Build and tests pass at 1afc1bff, TypeGuide still uses `spawn_format`

### Additional Commits (kept on main for v0.17.1)
**Before breaking changes:**
- `c1488b98` (Oct 31) - refactor: simplify release workflow

**After breaking changes:**
- `988c15c0` (Nov 3) - permission updates
- `eb5813dc` (Nov 4) - comment change, debug! removal
- `ddf61a0a` (Nov 4) - moved into ~/.claude/commands
- `e6b1c939` (Nov 4) - **fix: handle custom struct references in anyOf schemas** ⭐
- `5dc7992c` (Nov 5) - re-created debug_protocol
- `f1d8bae9` (Nov 5) - changed rustfmt.toml settings
- `92f2ed0f` (Nov 19) - ready to implement
- `20e2edc1` (Nov 19) - permission update

**Total for v0.17.1**: 19 commits (1 before + 11 refactoring + 7 after)

**PR #1 Analysis**:
- Author: @tobert
- Touches: `mcp/src/tool/` (handler_context, json_response, response_builder, tool_def)
- Updates: rmcp 0.8.3 → 0.9.0
- **No conflicts** with breaking changes (different modules)

## Future Workflow: Maintaining Release Branches

### Applying Fixes to Released Versions

**Scenario**: Bug found in v0.17.1 that needs patching

#### Option A: Fix on Release Branch

```bash
# Create fix on release branch
git checkout release-0.17.1

# Make changes or cherry-pick fix
git cherry-pick <fix-commit>

# Tag new patch version
git tag v0.17.2
git push origin release-0.17.1 --tags

# Publish
cargo publish --workspace

# Backport to main
git checkout main
git cherry-pick <fix-commit>
git push origin main
```

#### Option B: Fix on main, Backport to Release

```bash
# Create fix on main
git checkout main
# Make changes, commit
git push origin main

# Backport to release branch
git checkout release-0.17.1
git cherry-pick <fix-commit>
git tag v0.17.2
git push origin release-0.17.1 --tags

# Publish
cargo publish --workspace
```

### Accepting PRs Against Release Branches

Contributors can target specific release branches:

1. **Change PR target branch** in GitHub UI to `release-0.17.1`
2. Review and merge as normal
3. Cherry-pick to main: `git cherry-pick <pr-merge-commit>`

### Branch Naming Convention

- `main` - active development
- `release-0.X.Y` - stable release branches
- `v0.X.Y` - git tags on release branches
- `vN.N-dev` - temporary development branches for major feature work

## Updating release_version.md Command

The `.claude/commands/release_version.md` slash command needs to be updated to incorporate the release branch workflow into the standard release process.

### When to Create Release Branches

**Create a release branch for EVERY release** (patch, minor, or major):
- Enables future patch releases (v0.17.1 → v0.17.2)
- Maintains release history separate from main development
- Allows accepting PRs against specific versions
- Matches Bevy's proven workflow

### Required Changes to release_version.md

Add a new step after **STEP 6: Push and Publish** and before **STEP 7: Create GitHub Release**:

#### New Step 6.5: Create Release Branch

```markdown
<CreateReleaseBranch>
## STEP 6.5: Create Release Branch

**Create a release branch to enable future patches:**

```bash
# Check if branch already exists (prevents accidental overwrites)
if git show-ref --verify --quiet refs/heads/release-${VERSION}; then
  echo "ERROR: Branch release-${VERSION} already exists"
  echo "If you need to recreate it, delete it first with:"
  echo "  git branch -D release-${VERSION}"
  echo "  git push origin --delete release-${VERSION}"
  exit 1
fi

# Create release branch from current commit
git checkout -b release-${VERSION}

# Verify we're on the new branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "release-${VERSION}" ]; then
  echo "ERROR: Failed to create branch release-${VERSION}"
  exit 1
fi

# Push to remote and set up tracking
git push -u origin release-${VERSION}

# Verify branch exists on remote
if ! git ls-remote --heads origin release-${VERSION} | grep -q release-${VERSION}; then
  echo "ERROR: Failed to push branch release-${VERSION} to remote"
  exit 1
fi

echo "✅ Successfully created and pushed release-${VERSION} branch"

# Return to main for continued development
git checkout main
```
→ **Auto-check**: Continue if branch created and pushed successfully, stop if fails

**Why**: Release branches allow patch releases (e.g., v0.17.1 → v0.17.2) without
disturbing main development. This follows Bevy's proven workflow where:
- `main` is for active development
- `release-X.Y.Z` branches are stable points for patches
- Both can be developed independently and fixes can be backported

</CreateReleaseBranch>
```

Update the ExecutionSteps section:
```markdown
<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 0:** Execute <ArgumentValidation/>
    **STEP 1:** Execute <PreReleaseChecks/>
    **STEP 2:** Execute <ChangelogVerification/>
    **STEP 3:** Execute <UpdateMcpDependency/>
    **STEP 4:** Execute <RunCargoRelease/>
    **STEP 5:** Execute <FinalizeChangelogs/>
    **STEP 6:** Execute <PushAndPublish/>
    **STEP 6.5:** Execute <CreateReleaseBranch/>  <!-- NEW -->
    **STEP 7:** Execute <CreateGitHubRelease/>
    **STEP 8:** Execute <PostReleaseVerification/>
    **STEP 9:** Execute <PrepareNextReleaseCycle/>
    **STEP 10:** Execute <RestorePathDependency/>
</ExecutionSteps>
```

### Additional Documentation Section

Add a new section at the end of `release_version.md`:

```markdown
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
```

### Integration with Existing Process

The updated workflow ensures:
1. **Minimal disruption**: Only adds one step after publishing
2. **Automatic safety**: Release branch created from tested, published code
3. **Future flexibility**: Can patch any release without affecting main
4. **Clear history**: Each release has a named branch, not just a tag

### Complete Updated Step Sequence

```
STEP 0: Validate version argument
STEP 1: Pre-release validation (git status, build, test)
STEP 2: Verify CHANGELOG entries
STEP 3: Update mcp dependency to crates.io version
STEP 4: Run cargo-release (version bump + tag)
STEP 5: Finalize CHANGELOG headers
STEP 6: Push to git + publish to crates.io
STEP 6.5: Create release branch ← NEW
STEP 7: Create GitHub release
STEP 8: Post-release verification
STEP 9: Prepare next release cycle (add [Unreleased])
STEP 10: Restore path dependency
```

### Example Scenarios

**Scenario 1: Normal v0.17.1 release**
- Follow all 11 steps (including new Step 6.5)
- Creates `release-0.17.1` branch
- Main continues development toward v0.18.0

**Scenario 2: Critical bug found in v0.17.1**
- Checkout `release-0.17.1` branch
- Apply fix + update CHANGELOG
- Tag as v0.17.2
- Publish only changed crates
- Cherry-pick fix to main

**Scenario 3: Community PR for v0.17.1 patch**
- Contributor opens PR against `release-0.17.1` branch
- Review, merge into release branch
- Tag as v0.17.2
- Publish to crates.io
- Cherry-pick to main

## Verification Checklist

After executing the plan:

- [ ] `release-0.17.0` branch exists and points to v0.17.0 tag
- [ ] `v0.18-dev` branch exists with 7 breaking change commits
- [ ] `main` has 6 independent commits since v0.17.0
- [ ] `main` does not contain breaking changes (no `spawn_example`/`resource_example` in code)
- [ ] PR #1 merged successfully into main
- [ ] `v0.17.1` tag created and pushed
- [ ] All tests pass: `cargo nextest run`
- [ ] Published to crates.io successfully

## Files Modified by Each Commit Series

### Breaking Changes Touch:
- `mcp/src/brp_tools/brp_type_guide/**` (29 files)
- `.claude/scripts/mutation_test/` (prepare.py, config.py)
- `.claude/plans/` (various plan files)

### Independent Commits Touch:
- `.claude/commands/release_version.md`
- `mcp/src/tool/parameters.rs`
- Various cleanup/config files

### PR #1 Touches:
- `mcp/src/tool/` (handler_context, json_response, response_builder, tool_def)
- `Cargo.toml`, `Cargo.lock`
- `mcp/CHANGELOG.md`

**No overlap = clean separation!**

## Rollback Plan

If something goes wrong:

### Before Force Push (Step 3c)
```bash
# Just reset main back
git checkout main
git reset --hard origin/main
```

### After Force Push (Step 3c)
```bash
# Find the old main commit (check git reflog or GitHub)
git checkout main
git reset --hard <old-main-commit-sha>
git push origin main --force-with-lease
```

### Nuclear Option
```bash
# If v0.18-dev branch preserved breaking changes, you can always:
git checkout main
git reset --hard v0.18-dev
git push origin main --force-with-lease
# Then restart the process
```

## Notes

- The breaking change is very clean - all 7 commits are cohesive refactoring work
- No risky cherry-picks - all independent commits are truly independent
- PR #1 is important (fixes Gemini compatibility) and belongs in v0.17.1
- The only "loss" is `ce4a8626` (deterministic ordering) waits for v0.18, but it's minor
- This workflow matches Bevy's approach: main for development, release branches for stability

## References

- Original analysis conversation: 2025-11-19
- v0.17.0 tag: `cf73ba6f` (chore: finalize CHANGELOGs for v0.17.0 release)
- Breaking change range: `c0b9a91e..dbae7815`
- PR #1: https://github.com/natepiano/bevy_brp/pull/1
