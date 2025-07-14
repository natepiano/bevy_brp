# BRP Extras Methods Tests

## Objective
Validate brp_extras specific methods: discover_format, screenshot, send_keys, set_debug_mode, and shutdown.

## Test Steps

### 1. Format Discovery Method
- Execute `mcp__brp__brp_extras_discover_format` with Transform type
- Verify response includes spawn_format and mutation_info
- Check method works correctly with plugin

### 2. Screenshot Capture
- Execute `mcp__brp__brp_extras_screenshot` with absolute path (use current working directory + filename)
- Verify screenshot file is created at the specified absolute path
- Read screenshot file to confirm it's valid
- Check window content is captured correctly
- **IMPORTANT**: Clean up screenshot file at end of test

### 3. Keyboard Input Tests
- Test default duration: `mcp__brp__brp_extras_send_keys` with `["KeyA", "Space"]`
- Test custom duration: `{"keys": ["KeyH", "KeyI"], "duration_ms": 700}`
- Test modifier combinations: `{"keys": ["ControlLeft", "KeyA"], "duration_ms": 500}`
- Test boundary conditions:
  - Short duration: `{"keys": ["KeyB"], "duration_ms": 50}`
  - Zero duration: `{"keys": ["KeyC"], "duration_ms": 0}`
- Test error condition: `{"keys": ["KeyE"], "duration_ms": 70000}` (should fail)

### 4. Invalid Key Code Test
- Execute send_keys with invalid key code
- Verify appropriate error response

### 5. Debug Mode Control Tests
- **Enable debug mode**: Execute `mcp__brp__brp_extras_set_debug_mode` with `{"enabled": true}`
- **Verify enabled**: Execute `mcp__brp__brp_extras_discover_format` with Transform type
- **Check debug field**: Verify response contains `brp_extras_debug_info` field with debug details
- **Disable debug mode**: Execute `mcp__brp__brp_extras_set_debug_mode` with `{"enabled": false}`
- **Verify disabled**: Execute same discover_format operation
- **Check no debug field**: Verify response does NOT contain `brp_extras_debug_info` field

### 6. Screenshot After Key Input
- Send some keys to the app
- Take screenshot to verify UI reflects key input (use absolute path)
- Read screenshot to confirm key display updated
- Clean up this screenshot file as well

### 7. Clean Shutdown Test
- Execute `mcp__brp__brp_extras_shutdown` with app_name
- Verify clean shutdown response (shutdown_method: "clean_shutdown")
- Confirm app process terminates gracefully

## Expected Results
- ✅ Format discovery works with plugin available
- ✅ Screenshot capture succeeds and creates valid files
- ✅ Keyboard input works with various durations
- ✅ Modifier key combinations function correctly
- ✅ Duration boundaries are enforced properly
- ✅ Invalid inputs return appropriate errors
- ✅ Debug mode can be enabled/disabled independently
- ✅ Debug mode controls brp_extras_debug_info field in responses
- ✅ UI updates reflect sent keyboard input
- ✅ Clean shutdown works via brp_extras method

## Failure Criteria
STOP if: Any brp_extras method fails unexpectedly, screenshot capture fails, keyboard input doesn't work, debug mode control fails, or shutdown fails.
