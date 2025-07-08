# No Plugin

## Objective
Validate fallback behavior and error handling when bevy_brp_extras plugin is NOT available.

## Test Steps

### 1. Test BRP Extras Method Errors
- Execute `mcp__brp__brp_extras_discover_format`
- Verify helpful error about adding bevy_brp_extras with installation instructions
- Execute `mcp__brp__brp_extras_screenshot`
- Verify error includes bevy_brp_extras installation guidance
- Execute `mcp__brp__brp_extras_send_keys`
- Verify consistent error messaging

### 2. Fallback Shutdown Test
- Execute `mcp__brp__brp_extras_shutdown` with app_name
- Verify fallback to process termination
- Check response indicates method: "process_kill" with warning about clean shutdown

### 3. Tier 3 Format Discovery Fallback
- Enable debug tracing: `mcp__brp__brp_set_tracing_level` with `level: "debug"`
- Execute `mcp__brp__bevy_spawn` with intentionally incorrect Transform format:
  ```json
  {
    "components": {
      "bevy_transform::components::transform::Transform": {
        "translation": {"x": 1.0, "y": 2.0},
        "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
        "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
      }
    }
  }
  ```
  (Note: translation is missing required "z" field)
- **EXPECTED BEHAVIOR**: The format discovery engine SHOULD attempt Level 2 (calling brp_extras/discover_format) first, even though the plugin is not available. This is correct behavior.
- Verify trace log shows the engine attempting `brp_extras/discover_format` and receiving -32601 error
- Verify trace log shows "Level 2: No type information found" or similar message indicating Level 2 failed
- Verify fallback to Level 3 pattern matching succeeds
- Check trace log shows successful Level 3 completion

### 4. Basic BRP Functionality (Should Work)
- Execute `mcp__brp__bevy_list` to verify basic BRP works
- Execute `mcp__brp__bevy_query` with simple filter
- Execute `mcp__brp__bevy_get` on entity (if any exist)
- Verify core BRP methods function without plugin

### 5. Registry Discovery Without Plugin
- Test spawn with components lacking Serialize/Deserialize traits
- Try spawning with Visibility component
- Verify error includes component name and mentions missing Serialize/Deserialize traits
- Confirm error provides standard guidance for adding Serialize/Deserialize traits (this is the correct generic message regardless of whether the component is user-defined or from Bevy core)

### 6. Path Error Discovery
- Execute component mutation with wrong path (e.g., `.0.red` for ClearColor)
- Verify error suggests correct path (e.g., `.0.0.red`)
- Check error guidance is actionable

## Expected Results
- ✅ BRP extras methods return helpful installation guidance
- ✅ Error messages are consistent across extras methods
- ✅ Shutdown falls back to process termination with warning
- ✅ Format discovery correctly attempts Level 2 first (tries brp_extras/discover_format) then falls back to Level 3 pattern matching
- ✅ Trace log clearly shows tier progression: Level 2 attempt → -32601 error → Level 2 fails → Level 3 succeeds
- ✅ Basic BRP functionality works without extras
- ✅ Spawn with non-serializable components fails with helpful error mentioning missing traits
- ✅ Error messages provide standard guidance for adding Serialize/Deserialize traits (generic message applies to all components)
- ✅ Path error suggestions are accurate

## Failure Criteria
STOP if: Error messages are unclear, fallback mechanisms fail, or basic BRP doesn't work without plugin.
