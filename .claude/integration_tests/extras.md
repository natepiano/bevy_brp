# BRP Extras Methods Tests

## Objective
Validate brp_extras specific methods: screenshot, send_keys, and type_text.

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
- **IMPORTANT**: Clean up screenshot files by running: `bash .claude/scripts/integration_tests/cleanup_screenshots.sh`

### 2. Keyboard Input Tests
- Test default duration: `mcp__brp__brp_extras_send_keys` with `["KeyA", "Space"]`
- Test custom duration: `{"keys": ["KeyH", "KeyI"], "duration_ms": 700}`
- Test modifier combinations: `{"keys": ["ControlLeft", "KeyA"], "duration_ms": 500}`
- Test boundary conditions:
  - Short duration: `{"keys": ["KeyB"], "duration_ms": 50}`
  - Zero duration: `{"keys": ["KeyC"], "duration_ms": 0}`
- Test error condition: `{"keys": ["KeyE"], "duration_ms": 70000}` (should fail)

### 3. Text Input Tests
- Clear the `TextInputContent` resource: `mcp__brp__world_insert_resources` with resource `extras_plugin::TextInputContent` and value `{"text": ""}`
- Test basic typing: `mcp__brp__brp_extras_type_text` with `{"text": "hello"}`
- Verify text appears: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello"}`
- Test sequential typing: `mcp__brp__brp_extras_type_text` with `{"text": " world"}`
- Verify concatenation: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello world"}`
- Test special characters: `mcp__brp__brp_extras_type_text` with `{"text": "!@#"}`
- Verify special chars: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello world!@#"}`
- Test unmappable characters: `mcp__brp__brp_extras_type_text` with text containing unmappable chars
- Verify skipped array is populated in response

### 4. FPS Diagnostics Test
- Execute `mcp__brp__brp_execute` with method `brp_extras/get_diagnostics` and no params
- Verify response contains `fps` object with keys: `current`, `average`, `smoothed`, `history_len`, `max_history_len`, `history_duration_secs`
- Verify response contains `frame_time_ms` object with keys: `current`, `average`, `smoothed`
- Verify response contains `frame_count` as a number
- Verify `fps.max_history_len` equals 120 (Bevy default)
- Verify `fps.current` is a positive number (app is running)

### 5. Invalid Key Code Test
- Execute send_keys with invalid key code
- Verify appropriate error response

### 6. Screenshot After Key Input
- Send some keys to the app
- Take screenshot to verify UI reflects key input (use absolute path)
- **IMPORTANT**: Poll for file completion using `.claude/scripts/extras_test_poll_screenshot.sh <absolute_path_to_screenshot>`
- Read screenshot to confirm key display updated
- Clean up is handled by the cleanup script above (it removes both screenshot files)

## Expected Results
- ✅ FPS diagnostics returns valid fps, frame_time_ms, and frame_count data
- ✅ Screenshot capture succeeds and creates valid files
- ✅ Keyboard input works with various durations
- ✅ Modifier key combinations function correctly
- ✅ Duration boundaries are enforced properly
- ✅ Text typing works sequentially and accumulates correctly
- ✅ Special characters are typed properly
- ✅ Unmappable characters are reported as skipped
- ✅ Invalid inputs return appropriate errors
- ✅ UI updates reflect sent keyboard input

## Failure Criteria
STOP if: Any brp_extras method fails unexpectedly, screenshot capture fails, keyboard input doesn't work, or text typing doesn't accumulate properly.
