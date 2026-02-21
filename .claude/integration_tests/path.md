# Path Disambiguation Tests

## Objective
Validate path parameter handling when multiple examples with the same name exist across different paths. Tests the full path matching and partial path matching functionality.

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

### 3. Test Example Launch With Different Path Matching Modes

#### 3a. Full Relative Path Test
- Execute `mcp__brp__brp_launch_bevy_example` with same example name
- Use FULL relative path from error message (e.g., `"path": "test-duplicate-a"`)
- Verify successful launch from correct path
- Check response includes path information

#### 3b. Partial Path Test
- Execute `mcp__brp__brp_launch_bevy_example` with same example name
- Use PARTIAL path that uniquely identifies the example (e.g., `"path": "duplicate-a"`)
- Verify successful launch from correct path
- Confirm partial path matching works correctly

#### 3c. Ambiguous Partial Path Test
- Test with ambiguous partial path that matches multiple paths (e.g., `"path": "test-"`)
- Verify appropriate error message about ambiguous path
- Check error lists all matching paths

#### 3d. Invalid Path Test
- Execute `mcp__brp__brp_launch_bevy_example` with same example name
- Use non-existent path parameter to test error handling
- Verify error message indicates the invalid path doesn't match any available paths
- Confirm `available_paths` array contains all available paths (helping users see valid options)
- Confirm error handling is robust

### 4. Validate Error Message Quality
- Check that disambiguation errors are clear and actionable
- Verify all available paths are listed (relative paths from scan root)
- Confirm guidance on using path parameter is helpful
- Ensure error format is consistent
- Verify error messages show paths, not just workspace names

### 5. Cleanup
- Shutdown any launched apps from path testing
- Confirm ports are available

## Expected Results
- ✅ Path conflicts are properly detected
- ✅ Launch without path fails with clear error when conflicts exist
- ✅ Error messages list all available relative paths
- ✅ Full relative path parameter resolves conflicts successfully
- ✅ Partial path matching works when unambiguous
- ✅ Ambiguous partial paths produce appropriate errors
- ✅ Invalid path parameters produce disambiguation error with `available_paths` array showing all available paths
- ✅ Error handling provides consistent, actionable guidance

## Special Notes
- **Current test environment**: Duplicate examples exist (`extras_plugin_duplicate` in `test-duplicate-a` and `test-duplicate-b`)
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment must provide duplicate examples
- Focus is on error handling and path disambiguation logic
- The path parameter accepts: full relative paths and partial paths (if unambiguous)

## Failure Criteria
STOP if: Path errors are unclear, path specification fails to resolve conflicts, incorrect example variants are launched, or path matching doesn't work as specified (full and partial modes).
