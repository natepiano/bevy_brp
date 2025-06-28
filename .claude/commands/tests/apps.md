# Apps Test

## Objective
Validate app launch functionality using `mcp__brp__brp_launch_bevy_app` (not example launch), testing the distinction between apps and examples, and verifying BRP operations work with actual binary applications.

## Test Steps

### 1. Launch Application
- Use `mcp__brp__brp_launch_bevy_app` to launch the test_extras_plugin_app binary specified in the test configuration
- Verify launch response includes PID, log file path, working directory
- Confirm app starts in background successfully
- **NOTE**: This tests app launch vs example launch functionality

### 2. BRP Status Check
- Execute `mcp__brp__brp_status` with app name and port
- Verify status shows "running_with_brp"
- Confirm app process is detected and BRP is responsive

### 3. RPC Discovery
- Execute `mcp__brp__bevy_rpc_discover`
- Verify standard BRP methods are available
- Confirm bevy_brp_extras methods ARE listed (since app has extras plugin)

### 4. Basic Spawn Operation
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

### 5. Query Operation
- Execute `mcp__brp__bevy_query` to find entities with Rotator component
- Verify at least one entity is found (from app setup)
- Check Transform data is returned correctly

### 6. Mutate Component
- Use `mcp__brp__bevy_mutate_component` on spawned entity
- Change translation.x to 100.0
- Verify mutation succeeds

### 7. Get Component Data
- Execute `mcp__brp__bevy_get` on mutated entity
- Request Transform component
- Verify translation.x is now 100.0

### 8. List Operations
- Execute `mcp__brp__bevy_list` without entity parameter
- Verify comprehensive component list is returned
- Check that Transform, Sprite, and other standard components are listed

### 9. Test Extras Functionality
- Execute `mcp__brp__brp_extras_screenshot` with path `test_screenshot_apps.png`
- Verify screenshot is created successfully
- Clean up screenshot file

### 10. Clean Shutdown
- Execute `mcp__brp__brp_extras_shutdown` with app_name
- Verify clean shutdown response (method: "clean_shutdown")
- Confirm app process terminates gracefully

## Expected Results
- ✅ App launches successfully using `brp_launch_bevy_app` (not example launch)
- ✅ BRP connectivity works with binary applications
- ✅ Basic BRP operations (spawn, query, mutate, get) function correctly
- ✅ RPC discovery shows both standard BRP and extras methods
- ✅ Component listing works properly
- ✅ Extras functionality (screenshot, shutdown) works with apps
- ✅ App vs example launch distinction is validated

## Failure Criteria
STOP if: App launch fails, BRP is unresponsive, basic operations fail, or app launch behaves identically to example launch (indicating no distinction).