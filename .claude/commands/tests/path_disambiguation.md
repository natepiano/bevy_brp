# Path Disambiguation Tests

## Objective
Validate path parameter handling when multiple apps or examples with the same name exist across different paths.

## Test Steps

### 1. Check for Path Conflicts
- Execute `mcp__brp__brp_list_bevy_apps`
- Look for duplicate app names across different paths
- If no conflicts found, mark tests as SKIPPED with reason
- Note available paths for testing

### 2. Test App Launch Without Path (If Conflicts Exist)
- Execute `mcp__brp__brp_launch_bevy_app` with duplicate app name
- Do NOT specify path parameter
- Verify error response lists available paths
- Check error message provides clear guidance

### 3. Test App Launch With Path Parameter (If Conflicts Exist)
- Execute `mcp__brp__brp_launch_bevy_app` with same app name but specify path
- Use path parameter from error message
- Verify successful launch from correct path
- Check response includes path field

### 4. Test Example Launch Disambiguation
- Execute `mcp__brp__brp_list_bevy_examples` to check for duplicate example names
- **CRITICAL**: If NO duplicate examples exist, you MUST mark this as a FAILED test with reason: "No duplicate examples found to test disambiguation logic"
- If duplicates exist, test launch using `mcp__brp__brp_launch_bevy_example` without path (expect error)
- Test launch using `mcp__brp__brp_launch_bevy_example` with path parameter (expect success)
- Verify correct example variant is launched

### 5. Validate Error Message Quality
- Check that disambiguation errors are clear and actionable
- Verify all available workspaces are listed
- Confirm guidance on using workspace parameter is helpful
- Ensure error format is consistent

### 6. Cleanup
- Shutdown any launched apps from workspace testing
- Verify clean termination
- Confirm ports are available

## Expected Results
- ✅ Workspace conflicts are properly detected
- ✅ Launch without workspace fails with clear error when conflicts exist
- ✅ Error messages list all available workspaces
- ✅ Workspace parameter resolves conflicts successfully
- ✅ Launched apps include workspace information in responses
- ✅ Error handling is consistent between apps and examples

## Special Notes
- If no workspace conflicts exist for apps, app-related sub-tests will be marked as SKIPPED
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment should provide duplicate examples for comprehensive testing
- Tests adapt to available workspace configurations  
- Focus is on error handling and disambiguation logic
- Some environments may not have workspace ambiguity

## Failure Criteria
STOP if: Workspace errors are unclear, workspace specification fails to resolve conflicts, or incorrect app/example variants are launched.