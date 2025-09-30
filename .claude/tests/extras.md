# BRP Extras Methods Tests

## Objective
Validate brp_extras specific methods: screenshot and send_keys.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

## Test Steps

### 1. Screenshot Capture
- Execute `mcp__brp__brp_extras_screenshot` with absolute path (use current working directory + filename)
- **IMPORTANT**: Poll for file completion using `.claude/scripts/extras_test_poll_screenshot.sh <absolute_path_to_screenshot>`
  - Screenshot I/O is asynchronous, this script waits up to 5 seconds for file to be ready
  - Script exits with success (0) if file exists and has non-zero size
  - Script exits with failure (1) if timeout or file not ready
- Verify screenshot file exists and has non-zero size
- Read screenshot file to confirm it's valid
- Check window content is captured correctly
- **IMPORTANT**: Clean up screenshot file at end of test

### 2. Keyboard Input Tests
- Test default duration: `mcp__brp__brp_extras_send_keys` with `["KeyA", "Space"]`
- Test custom duration: `{"keys": ["KeyH", "KeyI"], "duration_ms": 700}`
- Test modifier combinations: `{"keys": ["ControlLeft", "KeyA"], "duration_ms": 500}`
- Test boundary conditions:
  - Short duration: `{"keys": ["KeyB"], "duration_ms": 50}`
  - Zero duration: `{"keys": ["KeyC"], "duration_ms": 0}`
- Test error condition: `{"keys": ["KeyE"], "duration_ms": 70000}` (should fail)

### 3. Invalid Key Code Test
- Execute send_keys with invalid key code
- Verify appropriate error response

### 4. Screenshot After Key Input
- Send some keys to the app
- Take screenshot to verify UI reflects key input (use absolute path)
- **IMPORTANT**: Poll for file completion using `.claude/scripts/extras_test_poll_screenshot.sh <absolute_path_to_screenshot>`
- Read screenshot to confirm key display updated
- Clean up this screenshot file as well

## Expected Results
- ✅ Screenshot capture succeeds and creates valid files
- ✅ Keyboard input works with various durations
- ✅ Modifier key combinations function correctly
- ✅ Duration boundaries are enforced properly
- ✅ Invalid inputs return appropriate errors
- ✅ UI updates reflect sent keyboard input

## Failure Criteria
STOP if: Any brp_extras method fails unexpectedly, screenshot capture fails, or keyboard input doesn't work.
