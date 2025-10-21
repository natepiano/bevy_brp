# Discovery Tests

## Objective
Validate list functionality for BRP-enabled applications and examples in the workspace.

## Test Steps

### 1. List Bevy Apps
- Execute `mcp__brp__brp_list_bevy_apps`
- Verify response contains apps with name, path, build status
- Check for presence of `test_app` app
- Verify `bevy_brp_mcp` is NOT included (should be filtered out)

### 2. List Bevy Examples
- Execute `mcp__brp__brp_list_bevy_examples`
- Verify examples are organized by package
- Check for presence of `extras_plugin` and `no_extras_plugin` examples
- Check for duplicate `extras_plugin_duplicate` examples in `test-duplicate-a` and `test-duplicate-b` packages

### 3. List BRP Apps
- Execute `mcp__brp__brp_list_brp_apps`
- Verify only BRP-enabled apps are listed
- Check build status and BRP confirmation

## Expected Results
- ✅ All list methods return valid responses
- ✅ Expected app found: `test_app` (bevy_brp_mcp excluded)
- ✅ Examples `extras_plugin` and `no_extras_plugin` found in test-app package
- ✅ Duplicate examples `extras_plugin_duplicate` found in both test-duplicate-a and test-duplicate-b packages
- ✅ Response formats are consistent and complete
- ✅ Apps vs examples are properly distinguished
- ✅ Build status information is accurate

## Failure Criteria
STOP if: List methods return errors, expected apps/examples are missing, or response formats are malformed.
