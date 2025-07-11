# Path Disambiguation Tests

## Objective
Validate path parameter handling when multiple apps or examples with the same name exist across different paths. Tests the full path matching and partial path matching functionality.

## Test Steps

### 1. Check for Path Conflicts
- Execute `mcp__brp__brp_list_bevy_apps`
- Look for duplicate app names across different paths
- If no conflicts found, mark tests as SKIPPED with reason
- Note available paths for testing

### 2. Test App Launch Without Path (If Conflicts Exist)
- Execute `mcp__brp__brp_launch_bevy_app` with duplicate app name
- Do NOT specify path parameter
- Verify error response lists available paths (not just workspace names)
- Check error message provides clear guidance with relative paths

### 3. Test App Launch With Different Path Matching Modes (If Conflicts Exist)

#### 3a. Full Relative Path Test
- Execute `mcp__brp__brp_launch_bevy_app` with same app name
- Use FULL relative path from error message (e.g., `"path": "bevy_brp/test-duplicate-a"`)
- Verify successful launch from correct path
- Check response includes path field

#### 3b. Partial Path Test
- Execute `mcp__brp__brp_launch_bevy_app` with same app name
- Use PARTIAL path that uniquely identifies the app (e.g., `"path": "test-duplicate-a"`)
- Verify successful launch from correct path
- Confirm partial path matching works correctly

#### 3c. Ambiguous Partial Path Test
- If multiple apps share a common path suffix, test with ambiguous partial path
- Verify appropriate error message about ambiguous path
- Check error lists all matching paths

#### 3d. Invalid Path Test
- Execute `mcp__brp__brp_launch_bevy_app` with same app name
- Use non-existent path parameter to test error handling
- Verify error message indicates the invalid path doesn't match any available paths
- Confirm `duplicate_paths` array contains all available paths (helping users see valid options)
- Confirm error handling is robust

### 4. Test Example Launch Disambiguation
- Execute `mcp__brp__brp_list_bevy_examples` to check for duplicate example names
- **CRITICAL**: If NO duplicate examples exist, you MUST mark this as a FAILED test with reason: "No duplicate examples found to test disambiguation logic"
- If duplicates exist:
  - Test launch without path (expect error with relative paths)
  - Test launch with full relative path (expect success)
  - Test launch with partial path (expect success if unambiguous)
  - Test launch with invalid path (expect disambiguation error with `duplicate_paths` array containing all available paths)

### 5. Validate Error Message Quality
- Check that disambiguation errors are clear and actionable
- Verify all available paths are listed (relative paths from scan root)
- Confirm guidance on using path parameter is helpful
- Ensure error format is consistent
- Verify error messages show paths, not just workspace names

### 6. Cleanup
- Shutdown any launched apps from path testing
- Verify clean termination
- Confirm ports are available

## Expected Results
- ✅ Path conflicts are properly detected
- ✅ Launch without path fails with clear error when conflicts exist
- ✅ Error messages list all available relative paths
- ✅ Full relative path parameter resolves conflicts successfully
- ✅ Partial path matching works when unambiguous
- ✅ Ambiguous partial paths produce appropriate errors
- ✅ Invalid path parameters produce disambiguation error with `duplicate_paths` array showing all available paths
- ✅ Launched apps include path information in responses
- ✅ Error handling is consistent between apps and examples

## Special Notes
- If no path conflicts exist for apps, app-related sub-tests will be marked as SKIPPED
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment should provide duplicate examples for comprehensive testing
- Tests adapt to available path configurations  
- Focus is on error handling and path disambiguation logic
- The path parameter accepts: full relative paths and partial paths (if unambiguous)

## Failure Criteria
STOP if: Path errors are unclear, path specification fails to resolve conflicts, incorrect app/example variants are launched, or path matching doesn't work as specified (full and partial modes).