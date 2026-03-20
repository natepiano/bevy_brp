# Package Disambiguation Error Tests

## Objective
Validate error handling and message quality when package disambiguation fails or is invalid. Also validates the enriched not-found error when no target exists with the given name, and error behavior for invalid `path` search root.

## Test Steps

### 1. Check for Package Conflicts (Examples)
- Execute `mcp__brp__brp_list_bevy` to check for duplicate example names
- **CRITICAL**: If NO duplicate examples exist, you MUST mark this as a FAILED test with reason: "No duplicate examples found to test disambiguation logic"
- If duplicates exist, note available `package_name` values for testing

### 2. Test Example Launch Without package_name
- Execute `mcp__brp__brp_launch` with duplicate example name (e.g., `extras_plugin_duplicate`)
- Do NOT specify `package_name` parameter
- Verify error response includes `available_package_names` listing the packages that contain this target
- Check error message provides clear guidance to specify `package_name`

### 3. Invalid package_name Test
- Execute `mcp__brp__brp_launch` with duplicate example name (e.g., `extras_plugin_duplicate`)
- Use non-existent `package_name` parameter (e.g., `"package_name": "nonexistent-package"`)
- Verify error message indicates the target was not found in the specified package
- Confirm `available_package_names` array contains all packages with that target (helping users see valid options)
- Confirm error handling is robust

### 4. Nonexistent Target - Enriched Not-Found Error
- Execute `mcp__brp__brp_launch` with `target_name="completely_nonexistent_app_xyz"`
- Verify error message contains `"No app or example named"` text
- Verify error response includes `available_targets` array in `error_info`
- Verify `available_targets` contains entries with `name`, `kind`, and `path` fields
- Verify `available_targets` includes at least some known targets (e.g., entries with kind `"app"` and kind `"example"`)
- Verify the list is non-empty (there are real targets in the workspace)

### 5. Invalid path Search Root Test
- Execute `mcp__brp__brp_list_bevy` with `path` set to a nonexistent directory (e.g., `"/tmp/nonexistent_bevy_project_xyz"`)
- Verify the response returns 0 targets (empty list, no error crash)
- Execute `mcp__brp__brp_launch` with `target_name="extras_plugin"` and `path` set to the same nonexistent directory
- Verify an appropriate error is returned (target not found)

### 6. Validate Error Message Quality
- Check that disambiguation errors are clear and actionable
- Verify `available_package_names` are listed in disambiguation errors
- Confirm guidance on using `package_name` parameter is helpful
- Ensure error format is consistent
- Verify error messages show package names, not just paths

## Expected Results
- Launch without `package_name` fails with clear error when conflicts exist
- Error messages list all available `package_name` values
- Invalid `package_name` produces error with `available_package_names` showing valid options
- Nonexistent target produces enriched error with `available_targets` listing all apps and examples
- Invalid `path` search root returns empty results (list) or not-found error (launch)
- Error handling provides consistent, actionable guidance

## Special Notes
- **Current test environment**: Duplicate examples exist (`extras_plugin_duplicate` in `test-duplicate-a` and `test-duplicate-b`)
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment must provide duplicate examples
- Focus is on error handling and package disambiguation logic

## Failure Criteria
STOP if: Disambiguation errors are unclear, error messages don't list available package names, nonexistent target error doesn't include available_targets list, invalid path causes crashes instead of graceful handling, or error handling is inconsistent.
