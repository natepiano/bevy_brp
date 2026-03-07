# Path Disambiguation Success Tests

## Objective
Validate that path parameter successfully resolves conflicts when multiple examples with the same name exist, testing full and partial path matching.

## Test Steps

### 1. Check for Path Conflicts (Examples)
- Execute `mcp__brp__brp_list_bevy_examples` to check for duplicate example names
- **CRITICAL**: If NO duplicate examples exist, you MUST mark this as a FAILED test with reason: "No duplicate examples found to test disambiguation logic"
- If duplicates exist, note available paths for testing

### 2. Test Example Launch With Full Relative Path
- Execute `mcp__brp__brp_launch_bevy_example` with duplicate example name (e.g., `extras_plugin_duplicate`)
- Use FULL relative path from available paths (e.g., `"path": "test-duplicate-a"`)
- Verify successful launch from correct path
- Check response includes path information

### 3. Test Example Launch With Partial Path
- Execute `mcp__brp__brp_launch_bevy_example` with same example name
- Use PARTIAL path that uniquely identifies the example (e.g., `"path": "duplicate-a"`)
- Verify successful launch from correct path
- Confirm partial path matching works correctly

### 4. Cleanup
- Shutdown any launched apps from path testing
- Confirm ports are available

## Expected Results
- Path conflicts are properly detected
- Full relative path parameter resolves conflicts successfully
- Partial path matching works when unambiguous

## Special Notes
- **Current test environment**: Duplicate examples exist (`extras_plugin_duplicate` in `test-duplicate-a` and `test-duplicate-b`)
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment must provide duplicate examples
- The path parameter accepts: full relative paths and partial paths (if unambiguous)

## Failure Criteria
STOP if: Path specification fails to resolve conflicts, incorrect example variants are launched, or path matching doesn't work as specified.
