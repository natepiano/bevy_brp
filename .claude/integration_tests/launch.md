# Package Disambiguation and Search Root Tests

## Objective
Validate that `package_name` parameter successfully resolves conflicts when multiple targets with the same name exist across packages. Also validates `launched_as` metadata, `search_order` priority, `args` passthrough, and `path` search root override in launch responses.

## Test Steps

### 1. Check for Package Conflicts (Examples)
- Execute `mcp__brp__brp_list_bevy` to check for duplicate example names
- **CRITICAL**: If NO duplicate examples exist, you MUST mark this as a FAILED test with reason: "No duplicate examples found to test disambiguation logic"
- If duplicates exist, note available `package_name` values for testing

### 2. Test Example Launch With package_name
- Execute `mcp__brp__brp_launch` with duplicate example name (e.g., `extras_plugin_duplicate`)
- Use `package_name` to disambiguate (e.g., `"package_name": "test-app-a"`)
- Verify successful launch from correct package
- **Verify `launched_as` field is `"example"` in the response metadata**

### 3. Test Launch With path Search Root Override
- Execute `mcp__brp__brp_launch` with `target_name="extras_plugin_duplicate"`, `path` set to the absolute path of the `test-duplicate-a` directory
- Do NOT specify `package_name` — the search root should narrow results to a single match
- Verify successful launch
- **Verify `launched_as` field is `"example"` in the response metadata**

### 4. Test App Launch With launched_as Verification and Args
- Execute `mcp__brp__brp_launch` with `target_name="test_app"`, `args=["--marker", "app_args_test"]` (no search_order, defaults to "app")
- **Verify `launched_as` field is `"app"` in the response metadata**
- Wait for the app to start, then use `mcp__brp__brp_list_logs` to find the log file containing the port used
- Execute `mcp__brp__brp_read_log` with that filename and keyword `"MARKER"`
- **Verify the log contains `MARKER:app_args_test`** — this proves `args` were passed through to the app binary

### 5. Search Order Priority - Example First, With Args
- **Context**: The name `test_app` exists as both a binary app (in `bevy_brp_test_apps` package) and an example (in both `bevy_brp_test_apps` and `test-app-a` packages). Use `package_name` to select the `test-app-a` variant.
- Execute `mcp__brp__brp_launch` with `target_name="test_app"`, `search_order="example"`, `package_name="test-app-a"`, `args=["--marker", "example_args_test"]`
- **Verify `launched_as` field is `"example"` in the response metadata**
- Wait for the example to start, then use `mcp__brp__brp_list_logs` to find the log file containing the port used
- Execute `mcp__brp__brp_read_log` with that filename and keyword `"MARKER"`
- **Verify the log contains `MARKER:example_args_test`** — this proves `args` were passed through the `--` separator to the example process

### 6. Cleanup
- Shutdown any launched apps from all test steps (steps 2-5)
- Confirm ports are available

## Expected Results
- Package conflicts are properly detected via `brp_list_bevy`
- `package_name` parameter resolves conflicts successfully (exact match on package name)
- `path` search root override narrows search scope correctly
- `launched_as` is `"example"` for example targets
- `launched_as` is `"app"` for app targets
- `search_order="example"` causes example to be found before app when both exist with same name
- Default `search_order` (app) causes app to be found before example when both exist with same name
- `args` are passed through to app binaries directly
- `args` are passed through to examples via `--` separator

## Special Notes
- **Current test environment**: Duplicate examples exist (`extras_plugin_duplicate` in `test-duplicate-a` and `test-duplicate-b`)
- **Search order fixture**: The `test_app` example in `test-duplicate-a/examples/test_app.rs` intentionally shares its name with the `test_app` binary in `test-app/src/bin/test_app.rs`. See `test-duplicate-a/Cargo.toml` for documentation.
- **Args fixture**: Both `test_app` binaries (app and example) log `MARKER:<value>` at info level when launched with `--marker <value>`. This is used to verify args passthrough.
- **IMPORTANT**: Missing duplicate examples is a FAILED test, not SKIPPED - the test environment must provide duplicate examples
- The `package_name` parameter matches exactly against the `package_name` field from `brp_list_bevy`

## Failure Criteria
STOP if: `package_name` specification fails to resolve conflicts, `path` search root override doesn't narrow results, incorrect targets are launched, `launched_as` field is missing or has wrong value, `search_order` priority is not respected, args are not passed through to launched processes, or disambiguation doesn't work as specified.
