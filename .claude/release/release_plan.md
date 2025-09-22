# Bevy BRP Crates Release Plan

## Overview
Coordinated release of three crates from the unified `bevy_brp` repository to crates.io:
- `bevy_brp_mcp_macros` - Procedural macros (currently v0.3.0)
- `bevy_brp_mcp` - MCP server for BRP integration (currently v0.3.0)
- `bevy_brp_extras` - Enhanced BRP methods plugin (currently v0.3.0)

## Release Strategy
Using Release Candidate (RC) process to allow early adopters to test from crates.io before final release.

## Version Numbering
- Current in repo: 0.3.0 (unpublished)
- RC1: 0.3.0-rc.1
- RC2: 0.3.0-rc.2 (if needed)
- Final: 0.3.0

## Pre-Release Checklist

### 1. Repository Preparation
- [ ] Ensure all tests pass (`cargo nextest run`)
- [ ] Update version numbers in all Cargo.toml files
- [ ] Update inter-crate dependencies to use RC versions
- [ ] Update repository URLs in all Cargo.toml files (from archived to new repo)
- [ ] Update CHANGELOG.md for each crate:
  1. Move content from "## [Unreleased]" to "## [0.3.0-rc.1] - YYYY-MM-DD"
  2. Add release date
  3. Do NOT add new Unreleased section yet (post-release step)
- [ ] Update README.md files with new installation instructions
- [ ] Add migration guide section to main README.md with simple dependency update instructions for users of previous split crates
- [ ] Tag the commit with version (e.g., `v0.3.0-rc.1`)

### 2. Dependency Order
Must release in this specific order due to dependencies:
1. `bevy_brp_mcp_macros` (no internal deps)
2. `bevy_brp_mcp` (depends on macros)
3. `bevy_brp_extras` (independent but part of suite)

## Release Process for RC1

### Phase 1: Update Versions
```bash
# Step 1: Update all crate versions to RC
# In mcp_macros/Cargo.toml:
version = "0.3.0-rc.1"

# In mcp/Cargo.toml:
version = "0.3.0-rc.1"

# In extras/Cargo.toml:
version = "0.3.0-rc.1"

# Step 2: After macros is published, update workspace dependency
# In workspace Cargo.toml, change from path to published version:
bevy_brp_mcp_macros = "0.3.0-rc.1"

# Step 3: Verify mcp/Cargo.toml uses workspace inheritance:
bevy_brp_mcp_macros.workspace = true
```

### Phase 2: Update Repository URLs
Update to use workspace inheritance following Rust ecosystem standards:

**In root Cargo.toml, add workspace.package section:**
```toml
[workspace.package]
repository = "https://github.com/natepiano/bevy_brp"
```

**In each crate's Cargo.toml, replace repository field:**
```toml
[package]
repository.workspace = true
```

This follows the pattern used by major projects like tokio, serde, bevy, and clap.

### Phase 3: Publish Sequence
```bash
# 1. Publish macros first
cd mcp_macros
cargo publish --dry-run
cargo publish

# Wait for crates.io to index (usually 1-2 minutes)

# 2. Update mcp to use published macros version
# Edit mcp/Cargo.toml to use crates.io version
cd ../mcp
cargo publish --dry-run
cargo publish

# 3. Publish extras
cd ../extras
cargo publish --dry-run
cargo publish
```

### Phase 4: Verification
- [ ] Verify all three crates are available on crates.io
- [ ] Test installation in a fresh project
- [ ] Update any example projects to use RC versions

## RC Testing Period

### Duration
- Minimum 1 week for community testing
- Maximum 2 weeks before final release

### Monitoring
- [ ] Watch GitHub issues for bug reports
- [ ] Monitor crates.io download stats
- [ ] Check for any breaking changes reported

### RC2 Criteria
Release RC2 if:
- Critical bugs found and fixed
- Breaking API changes needed
- Significant documentation updates

## Final Release (0.3.0)

### Pre-Release
- [ ] Address all feedback from RC period
- [ ] Update versions to remove `-rc.X` suffix
- [ ] Final test suite run
- [ ] Update CHANGELOG with RC feedback items

### Release
- Follow same publishing sequence as RC
- Tag with `v0.3.0`
- Create GitHub release with notes

### Post-Release
- [ ] Add new empty "## [Unreleased]" section to all three CHANGELOG.md files
- [ ] Announce on relevant channels (Discord, Reddit, etc.)
- [ ] Update any documentation sites
- [ ] Archive old repository with final notice pointing to new repo

## Rollback Plan
If critical issues found after publish:
- Yank affected versions on crates.io
- Fix issues
- Publish patch version (e.g., 0.3.1-rc.1 or 0.3.1)

## Future Releases
- Always release all three crates together
- Maintain synchronized version numbers
- Consider automating with release script

## Questions/Decisions Needed

1. **Version Strategy**: Should we use 0.3.0-rc.1 or jump straight to 0.3.0 final since this is the first publish?
2. **Breaking Changes**: Are there any breaking API changes from the split repositories that need documentation?
3. **GitHub Release**: Should we create a single release for all three crates or individual releases?
4. **Announcement Strategy**: Where should we announce the RC availability?