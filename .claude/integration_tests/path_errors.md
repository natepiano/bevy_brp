# Path Disambiguation Error Tests

## Objective
Validate error handling and message quality when path disambiguation fails or is ambiguous.

## Test Steps

### 1. Check for Path Conflicts (Examples)
- Execute `mcp__brp__brp_list_bevy_examples` to check for duplicate example names
- **CRITICAL**: If NO duplicate examples exist, you MUST mark this as a FAILED test with reason: "No duplicate examples found to test disambiguation logic"
- If duplicates exist, note available paths for testing

### 2. Test Example Launch Without Path
- Execute `mcp__brp__brp_launch_bevy_example` with duplicate example name (e.g., `extras_plugin_duplicate`)
- Do NOT specify path parameter
- Verify error response lists available paths (not just workspace names)
- Check error message provides clear guidance with relative paths

### 3. Ambiguous Partial Path Test
- Test with ambiguous partial path that matches multiple paths (e.g., `"path": "test-"`)
- Verify appropriate error message about ambiguous path
- Check error lists all matching paths

### 4. Invalid Path Test
- Execute `mcp__brp__brp_launch_bevy_example` with same example name
- Use non-existent path parameter to test error handling
- Verify error message indicates the invalid path doesn't match any available paths
- Confirm `available_paths` array contains all available paths (helping users see valid options)
- Confirm error handling is robust

### 5. Validate Error Message Quality
- Check that disambiguation errors are clear and actionable
- Verify all available paths are listed (relative paths from scan root)
- Confirm guidance on using path parameter is helpful
- Ensure error format is consistent
- Verify error messages show paths, not just workspace names

## Expected Results
- Launch without path fails with clear error when conflicts exist
- Error messages list all available relative paths
- Ambiguous partial paths produce appropriate errors
- Invalid path parameters produce disambiguation error with `available_paths` array showing all available paths
- Error handling provides consistent, actionable guidance

## Special Notes
- **Current test environment**: Duplicate examples exist (`extras_plugin_duplicate` in `test-duplicate-a` and `test-duplicate-b`)
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment must provide duplicate examples
- Focus is on error handling and path disambiguation logic

## Failure Criteria
STOP if: Path errors are unclear, error messages don't list available paths, or error handling is inconsistent.
