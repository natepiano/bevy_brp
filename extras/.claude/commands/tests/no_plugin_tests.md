# No Plugin Tests (Consolidated)

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

### 3. Tier 3/4 Format Discovery Fallback
- Enable debug mode: `mcp__brp__brp_set_debug_mode` with `enabled: true`
- Execute `mcp__brp__bevy_spawn` with wrong Transform format
- Verify debug info shows "FAILED Tier 2: Direct Discovery" 
- Verify fallback to pattern matching succeeds
- Check debug info shows "SUCCESS Tier 3/4" messages

### 4. Basic BRP Functionality (Should Work)
- Execute `mcp__brp__bevy_list` to verify basic BRP works
- Execute `mcp__brp__bevy_query` with simple filter
- Execute `mcp__brp__bevy_get` on entity (if any exist)
- Verify core BRP methods function without plugin

### 5. Registry Discovery Without Plugin
- Test spawn with components lacking Serialize/Deserialize traits
- Try spawning with Visibility component
- Verify appropriate registry diagnostic errors
- Confirm error messages are helpful

### 6. Path Error Discovery
- Execute component mutation with wrong path (e.g., `.0.red` for ClearColor)
- Verify error suggests correct path (e.g., `.0.0.red`)
- Check error guidance is actionable

## Expected Results
- ✅ BRP extras methods return helpful installation guidance
- ✅ Error messages are consistent across extras methods
- ✅ Shutdown falls back to process termination with warning
- ✅ Format discovery falls back to Tier 3/4 pattern matching
- ✅ Debug info clearly shows tier progression (failed Tier 2 → success Tier 3/4)
- ✅ Basic BRP functionality works without extras
- ✅ Registry errors provide helpful diagnostics
- ✅ Path error suggestions are accurate

## Failure Criteria
STOP if: Error messages are unclear, fallback mechanisms fail, or basic BRP doesn't work without plugin.