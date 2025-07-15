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

### 2. Tier 3 Format Discovery Fallback
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
- **EXPECTED BEHAVIOR**: Without bevy_brp_extras plugin, format discovery should attempt fallback correction but fail due to missing required data
- Verify spawn fails with error status
- Check response contains `format_corrected: "attempted_but_failed"` indicating format correction was attempted but could not succeed
- Verify error message indicates format issue (expects "sequence of 3 f32 values" but got "map") 
- Confirm response includes helpful hint about the transformation that was attempted

### 3. Basic BRP Functionality (Should Work)
- Execute `mcp__brp__bevy_list` to verify basic BRP works
- Execute `mcp__brp__bevy_query` with simple filter
- Execute `mcp__brp__bevy_get` on entity (if any exist)
- Verify core BRP methods function without plugin

### 4. Registry Discovery Without Plugin
- Test spawn with components lacking Serialize/Deserialize traits
- Try spawning with Visibility component
- Verify error includes component name and mentions missing Serialize/Deserialize traits
- Confirm error provides standard guidance for adding Serialize/Deserialize traits (this is the correct generic message regardless of whether the component is user-defined or from Bevy core)

### 5. Path Error Discovery
- Execute component mutation with wrong path (e.g., `.red` for ClearColor)
- Verify error suggests correct path (e.g., `.0.Srgba.red`)
- Check error guidance is actionable

### 6. Fallback Shutdown Test (PERFORM LAST)
- Execute `mcp__brp__brp_extras_shutdown` with app_name
- Verify fallback to process termination
- Check response indicates shutdown_method: "process_kill" with warning about clean shutdown
- **NOTE**: This step shuts down the app, so it must be performed LAST

## Expected Results
- ✅ BRP extras methods return helpful installation guidance
- ✅ Error messages are consistent across extras methods
- ✅ Shutdown falls back to process termination with warning
- ✅ Transform spawn fails with `format_corrected: "attempted_but_failed"` (cannot recover from missing required field)
- ✅ Error message clearly indicates format mismatch (map vs sequence) with helpful transformation hint
- ✅ Basic BRP functionality works without extras
- ✅ Spawn with non-serializable components fails with helpful error mentioning missing traits
- ✅ Error messages provide standard guidance for adding Serialize/Deserialize traits (generic message applies to all components)
- ✅ Path error suggestions are accurate

## Failure Criteria
STOP if: Error messages are unclear, fallback mechanisms fail, or basic BRP doesn't work without plugin.
