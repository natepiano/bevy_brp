# Apps Test

## Objective
Validate that the `test_app` binary works correctly with BRP operations, testing app vs example functionality distinction and environment variable passthrough.

**NOTE**: The apps are already running on their specified ports - focus on testing functionality, not launch/shutdown.

**Known Issue**: The `value` parameter in mutation operations must be passed as a JSON number, not a string. Passing `"100.0"` (string) instead of `100.0` (number) will result in a type error. This is a known limitation in how the MCP tool handles the `value` parameter.

## App Instance Configuration
This test uses pre-launched app instances referenced by label:
- **main_app**: test_app (standard launch, used for all existing test steps)
- **env_app**: test_app with RUST_LOG=debug (used for env var verification)

## Test Steps

### 1. BRP Status Check
- Execute `mcp__brp__brp_status` with app name and port [main_app port]
- Verify response status is "success" and metadata contains app_name, pid, and port fields
- Confirm app process is detected and BRP is responsive

### 2. Environment Variable Passthrough
- Use `mcp__brp__brp_list_logs` (no app_name filter) to find the log file containing "port[env_app port]" in the filename. Pick the most recent one.
- Execute `mcp__brp__brp_read_log` with that filename and keyword "DEBUG"
- The keyword filter is case-insensitive, so it also matches header lines containing "debug" (e.g. "Profile: debug"). Ignore those header lines.
- Expect exactly 1 actual DEBUG-level log line (with ANSI-colored "DEBUG" marker from tracing) containing: "test_app starting on port"
- Also read the main_app log (find file containing "port[main_app port]") with keyword "DEBUG" — it should have 0 actual DEBUG-level log lines (only header matches)
- The difference proves `RUST_LOG=test_app=debug` was passed through to the env_app process

### 3. RPC Discovery
- Execute `mcp__brp__rpc_discover` with port [main_app port]
- Verify standard BRP methods are available
- Confirm bevy_brp_extras methods ARE listed (since app has extras plugin)

### 4. Basic Spawn Operation
- Execute `mcp__brp__world_spawn_entity` with port [main_app port] and simple Transform component:
  ```json
  {
    "components": {
      "bevy_transform::components::transform::Transform": {
        "translation": [50.0, 50.0, 0.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0]
      }
    }
  }
  ```
- Verify entity ID is returned
- Store entity ID for subsequent tests

### 5. Get Component Data
- Execute `mcp__brp__world_get_components` on spawned entity with port [main_app port]
- Request Transform component
- Verify translation matches spawned values (translation.x is 50.0, translation.y is 50.0)

### 6. Query Operation - Non-Reflected Component
- Execute `mcp__brp__world_query` with port [main_app port] for `test_app::Rotator` component (which lacks Reflect derive)
- **IMPORTANT**: The `data` parameter is required - use an empty object `{}` if you only want to filter
- With default `strict: false` and `data: {}`: Verify it returns empty results (0 entities)
- With `strict: true` and `data: {}`: Verify it returns error -23402 with message about component not being registered
- This tests proper handling of components that exist in the app but aren't reflection-enabled

### 7. Mutate Component
- Use `mcp__brp__world_mutate_components` on spawned entity with port [main_app port]
- Change translation.x to 100.0 (pass as numeric value, not string)
- Verify mutation succeeds
- **IMPORTANT**: The value must be passed as a number (100.0), not as a string ("100.0")
- If passed as string, expect error: `invalid type: string "100.0", expected f32`

## Expected Results
- BRP connectivity works with binary applications
- Basic BRP operations (spawn, get, query, mutate) function correctly
- RPC discovery shows both standard BRP and extras methods
- App vs example launch distinction is validated
- Environment variables are passed through to the launched process

## Failure Criteria
STOP if: BRP is unresponsive, basic operations fail
