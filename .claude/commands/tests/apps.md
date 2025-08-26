# Apps Test

## Objective
Validate that the `test_extras_plugin_app` binary works correctly with BRP operations, testing app vs example functionality distinction.

**NOTE**: The app is already running on the specified port - focus on testing functionality, not launch/shutdown.

**Known Issue**: The `value` parameter in mutation operations must be passed as a JSON number, not a string. Passing `"100.0"` (string) instead of `100.0` (number) will result in a type error. This is a known limitation in how the MCP tool handles the `value` parameter.

## Test Steps

### 1. BRP Status Check
- Execute `mcp__brp__brp_status` with app name and port
- Verify response status is "success" and metadata contains app_name, pid, and port fields
- Confirm app process is detected and BRP is responsive

### 2. RPC Discovery
- Execute `mcp__brp__bevy_rpc_discover`
- Verify standard BRP methods are available
- Confirm bevy_brp_extras methods ARE listed (since app has extras plugin)

### 3. Basic Spawn Operation
- Execute `mcp__brp__bevy_spawn` with simple Transform component:
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

### 4. Query Operation - Non-Reflected Component
- Execute `mcp__brp__bevy_query` for `test_extras_plugin_app::Rotator` component (which lacks Reflect derive)
- **IMPORTANT**: The `data` parameter is required - use an empty object `{}` if you only want to filter
- With default `strict: false` and `data: {}`: Verify it returns empty results (0 entities)
- With `strict: true` and `data: {}`: Verify it returns error -23402 with message about component not being registered
- This tests proper handling of components that exist in the app but aren't reflection-enabled

### 5. Mutate Component
- Use `mcp__brp__bevy_mutate_component` on spawned entity
- Change translation.x to 100.0 (pass as numeric value, not string)
- Verify mutation succeeds
- **IMPORTANT**: The value must be passed as a number (100.0), not as a string ("100.0")
- If passed as string, expect error: `invalid type: string "100.0", expected f32`

### 6. Get Component Data
- Execute `mcp__brp__bevy_get` on mutated entity
- Request Transform component
- Verify translation.x is now 100.0

### 7. List Operations
- Execute `mcp__brp__bevy_list` without entity parameter
- Verify comprehensive component list is returned
- Check that Transform, Sprite, and other standard components are listed

## Expected Results
- ✅ BRP connectivity works with binary applications
- ✅ Basic BRP operations (spawn, query, mutate, get) function correctly
- ✅ RPC discovery shows both standard BRP and extras methods
- ✅ Component listing works properly
- ✅ App vs example launch distinction is validated

## Failure Criteria
STOP if: BRP is unresponsive, basic operations fail
