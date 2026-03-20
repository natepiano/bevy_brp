# Discovery Tests

## Objective
Validate the unified `brp_list_bevy` tool for discovering Bevy apps and examples in the workspace.

## Test Steps

### 1. List All Bevy Targets
- Execute `mcp__brp__brp_list_bevy`
- Verify response contains targets with `name`, `kind`, `brp_level`, `builds`, and `relative_path` fields
- Check for presence of `test_app` with `kind: "app"` in the `bevy_brp_test_apps` package
- Verify `bevy_brp_mcp` is NOT included (should be filtered out)
- Check for presence of `extras_plugin` and `no_extras_plugin` examples with `kind: "example"`
- Check for duplicate `extras_plugin_duplicate` examples in `test-duplicate-a` and `test-duplicate-b` packages

### 2. Same-Name Deduplication (bin + example)
- **CRITICAL**: The `bevy_brp_test_apps` package has BOTH a `[[bin]]` and `[[example]]` named `test_app`
- Verify that BOTH appear in the response — one with `kind: "app"` and one with `kind: "example"`
- If only one `test_app` entry appears for `bevy_brp_test_apps`, mark this as FAILED: "Dedup key collision — same-name bin+example deduplicated incorrectly"

### 3. Verify BRP Level
- **Bins**: Verify `test_app` (kind: "app") has `brp_level: "extras"` (package `src/` tree uses `BrpExtrasPlugin`)
- **Examples in app packages**: Verify `extras_plugin` has `brp_level: "extras"` (source file imports `BrpExtrasPlugin`)
- **Examples in app packages**: Verify `no_extras_plugin` has `brp_level: "brp_only"` (source file imports `RemotePlugin` but not `BrpExtrasPlugin`)
- **Examples in example-only packages**: Verify `extras_plugin_duplicate` from `test-duplicate-a` has `brp_level: "extras"` (source file imports `BrpExtrasPlugin`)
- **Examples in example-only packages**: Verify `extras_plugin_duplicate` from `test-duplicate-b` has `brp_level: "extras"` (source file imports `BrpExtrasPlugin`)
- **CRITICAL**: If any target has `brp_level: "none"` when its source imports BRP plugins, mark as FAILED: "Per-file BRP detection not working"

### 4. Verify Kind Field
- Confirm all items have a `kind` field with value `"app"` or `"example"`
- Verify apps and examples are properly distinguished

### 5. Test path Search Root Override
- Execute `mcp__brp__brp_list_bevy` with `path` set to the absolute path of `test-duplicate-a` directory (under the workspace root)
- Verify only targets from that package are returned (should find `extras_plugin_duplicate` and `test_app` examples from `test-app-a`)
- Verify the result count is smaller than the full list from step 1

## Expected Results
- ✅ `brp_list_bevy` returns valid response with all targets
- ✅ Expected app found: `test_app` with `kind: "app"` (bevy_brp_mcp excluded)
- ✅ Same-name `test_app` also found with `kind: "example"` in `bevy_brp_test_apps` (no dedup collision)
- ✅ Examples `extras_plugin` and `no_extras_plugin` found with `kind: "example"`
- ✅ Duplicate examples `extras_plugin_duplicate` found in both test-duplicate-a and test-duplicate-b packages
- ✅ `brp_level` correctly distinguishes "extras", "brp_only", and "none" for each target
- ✅ `no_extras_plugin` has `brp_level: "brp_only"` (not "none")
- ✅ All items include `kind`, `brp_level`, `builds`, and `relative_path` fields
- ✅ Build status information is accurate
- ✅ `path` search root override returns only targets from the specified directory

## Failure Criteria
STOP if: `brp_list_bevy` returns errors, expected apps/examples are missing, same-name bin+example deduplicated into one entry, `brp_level` is "none" for BRP-using targets, `kind` or `brp_level` fields are missing, `path` override doesn't scope results correctly, or response format is malformed.
