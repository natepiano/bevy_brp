# Workspace Disambiguation Tests

## Objective
Validate workspace parameter handling when multiple apps or examples with the same name exist across different workspaces.

## Test Steps

### 1. Check for Workspace Conflicts
- Execute `mcp__brp__brp_list_bevy_apps`
- Look for duplicate app names across different workspaces
- If no conflicts found, mark tests as SKIPPED with reason
- Note available workspaces for testing

### 2. Test App Launch Without Workspace (If Conflicts Exist)
- Execute `mcp__brp__brp_launch_bevy_app` with duplicate app name
- Do NOT specify workspace parameter
- Verify error response lists available workspaces
- Check error message provides clear guidance

### 3. Test App Launch With Workspace Parameter (If Conflicts Exist)
- Execute `mcp__brp__brp_launch_bevy_app` with same app name but specify workspace
- Use workspace parameter from error message
- Verify successful launch from correct workspace
- Check response includes workspace field

### 4. Test Example Launch Disambiguation (If Conflicts Exist)
- Execute `mcp__brp__brp_list_bevy_examples` to check for duplicate example names
- If duplicates exist, test launch using `mcp__brp__brp_launch_bevy_example` without workspace (expect error)
- Test launch using `mcp__brp__brp_launch_bevy_example` with workspace parameter (expect success)
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
- If no workspace conflicts exist, all sub-tests will be marked as SKIPPED
- Tests adapt to available workspace configurations  
- Focus is on error handling and disambiguation logic
- Some environments may not have workspace ambiguity

## Failure Criteria
STOP if: Workspace errors are unclear, workspace specification fails to resolve conflicts, or incorrect app/example variants are launched.