# BRP Extras Methods Tests

## Objective
Validate brp_extras specific methods: discover_format, screenshot, send_keys, and shutdown.

## Test Steps

### 1. Format Discovery Method
- Execute `mcp__brp__brp_extras_discover_format` with Transform type
- Verify response includes spawn_format and mutation_info
- Check method works correctly with plugin

### 2. Screenshot Capture
- Execute `mcp__brp__brp_extras_screenshot` with path `test_screenshot_brp_extras_methods.png` (project root)
- Verify screenshot file is created in project root
- Read screenshot file to confirm it's valid
- Check window content is captured correctly
- **IMPORTANT**: Clean up screenshot file from project root at end of test

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

### 5. Screenshot After Key Input
- Send some keys to the app
- Take screenshot to verify UI reflects key input (save to project root)
- Read screenshot to confirm key display updated
- Clean up this screenshot file as well

### 6. Clean Shutdown Test
- Execute `mcp__brp__brp_extras_shutdown` with app_name
- Verify clean shutdown response (method: "clean_shutdown")
- Confirm app process terminates gracefully

## Expected Results
- ✅ Format discovery works with plugin available
- ✅ Screenshot capture succeeds and creates valid files
- ✅ Keyboard input works with various durations
- ✅ Modifier key combinations function correctly
- ✅ Duration boundaries are enforced properly
- ✅ Invalid inputs return appropriate errors
- ✅ UI updates reflect sent keyboard input
- ✅ Clean shutdown works via brp_extras method

## Failure Criteria
STOP if: Any brp_extras method fails unexpectedly, screenshot capture fails, keyboard input doesn't work, or shutdown fails.
