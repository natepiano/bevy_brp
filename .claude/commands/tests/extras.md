# BRP Extras Methods Tests

## Objective
Validate brp_extras specific methods: discover_format, screenshot, send_keys, and set_debug_mode.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

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
- **Verify debug info enabled**: Execute `mcp__brp__brp_extras_discover_format` with Transform type and `{"enable_debug_info": true}`
- **Check debug field**: Verify response contains debug information in the response
- **Verify debug info disabled**: Execute `mcp__brp__brp_extras_discover_format` with Transform type and `{"enable_debug_info": false}` (or omit the parameter)
- **Check no debug field**: Verify response does NOT contain debug information when disabled

### 6. Screenshot After Key Input
- Send some keys to the app
- Take screenshot to verify UI reflects key input (use absolute path)
- Read screenshot to confirm key display updated
- Clean up this screenshot file as well

## Expected Results
- ✅ Format discovery works with plugin available
- ✅ Screenshot capture succeeds and creates valid files
- ✅ Keyboard input works with various durations
- ✅ Modifier key combinations function correctly
- ✅ Duration boundaries are enforced properly
- ✅ Invalid inputs return appropriate errors
- ✅ Debug info can be enabled/disabled via enable_debug_info parameter
- ✅ Debug info appears in response when enabled and is absent when disabled
- ✅ UI updates reflect sent keyboard input

## Failure Criteria
STOP if: Any brp_extras method fails unexpectedly, screenshot capture fails, keyboard input doesn't work, or debug mode control fails.
