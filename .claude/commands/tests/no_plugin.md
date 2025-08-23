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

**VALIDATION CRITERIA FOR SUCCESS:**
- Response MUST have `status: "error"` 
- Response error_info MUST contain `format_corrections` array with at least one correction
- Error message MUST mention format type mismatch (containing text like "map" and "sequence" or similar)
- Response error_info.format_corrections[0] SHOULD have a `hint` field (any hint text is acceptable)

**SUCCESS DETERMINATION:**
- If ALL four criteria above are met, this test step PASSES
- Only mark as FAILED if any of the MUST criteria are missing

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

## Expected Results
- ✅ BRP extras methods return helpful installation guidance
- ✅ Error messages are consistent across extras methods
- ✅ Transform spawn MUST: Have error status AND error_info contains `format_corrections` array AND mention format type issue
- ✅ Basic BRP functionality works without extras
- ✅ Spawn with non-serializable components fails with helpful error mentioning missing traits
- ✅ Error messages provide standard guidance for adding Serialize/Deserialize traits (generic message applies to all components)
- ✅ Path error suggestions are accurate

## Failure Criteria
STOP if: Error messages are unclear, fallback mechanisms fail, or basic BRP doesn't work without plugin.
