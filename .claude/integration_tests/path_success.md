# Path Disambiguation Success Tests

## Objective
Validate that path parameter successfully resolves conflicts when multiple examples with the same name exist, testing full and partial path matching. Also validates `launched_as` metadata and `search_order` priority in launch responses.

## Test Steps

### 1. Check for Path Conflicts (Examples)
- Execute `mcp__brp__brp_list_bevy_examples` to check for duplicate example names
- **CRITICAL**: If NO duplicate examples exist, you MUST mark this as a FAILED test with reason: "No duplicate examples found to test disambiguation logic"
- If duplicates exist, note available paths for testing

### 2. Test Example Launch With Full Relative Path
- Execute `mcp__brp__brp_launch` with duplicate example name (e.g., `extras_plugin_duplicate`)
- Use FULL relative path from available paths (e.g., `"path": "test-duplicate-a"`)
- Verify successful launch from correct path
- Check response includes path information
- **Verify `launched_as` field is `"example"` in the response metadata**

### 3. Test Example Launch With Partial Path
- Execute `mcp__brp__brp_launch` with same example name
- Use PARTIAL path that uniquely identifies the example (e.g., `"path": "duplicate-a"`)
- Verify successful launch from correct path
- Confirm partial path matching works correctly
- **Verify `launched_as` field is `"example"` in the response metadata**

### 4. Test App Launch With launched_as Verification
- Execute `mcp__brp__brp_launch` with `target_name="test_app"` (no search_order, defaults to "app")
- **Verify `launched_as` field is `"app"` in the response metadata**
- This confirms the unified launch tool correctly reports the target type for apps when the default search_order finds an app first

### 5. Search Order Priority - Example First
- **Context**: The name `test_app` exists as both a binary app (in `bevy_brp_test_apps` package) and an example (in `test-app-a` package, under `test-duplicate-a/`). This cross-package name collision is intentional test infrastructure for validating `search_order`.
- Execute `mcp__brp__brp_launch` with `target_name="test_app"`, `search_order="example"`
- **Verify `launched_as` field is `"example"` in the response metadata**
- This confirms `search_order="example"` causes the example to be found first, even though an app with the same name exists

### 6. Cleanup
- Shutdown any launched apps from all test steps (steps 2-5)
- Confirm ports are available

## Expected Results
- Path conflicts are properly detected
- Full relative path parameter resolves conflicts successfully
- Partial path matching works when unambiguous
- `launched_as` is `"example"` for example targets
- `launched_as` is `"app"` for app targets
- `search_order="example"` causes example to be found before app when both exist with same name
- Default `search_order` (app) causes app to be found before example when both exist with same name

## Special Notes
- **Current test environment**: Duplicate examples exist (`extras_plugin_duplicate` in `test-duplicate-a` and `test-duplicate-b`)
- **Search order fixture**: The `test_app` example in `test-duplicate-a/examples/test_app.rs` intentionally shares its name with the `test_app` binary in `test-app/src/bin/test_app.rs`. See `test-duplicate-a/Cargo.toml` for documentation.
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment must provide duplicate examples
- The path parameter accepts: full relative paths and partial paths (if unambiguous)

## Failure Criteria
STOP if: Path specification fails to resolve conflicts, incorrect example variants are launched, `launched_as` field is missing or has wrong value, `search_order` priority is not respected, or path matching doesn't work as specified.
