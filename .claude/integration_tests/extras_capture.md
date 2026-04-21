# BRP Extras Capture and Diagnostics Tests

## Objective
Validate brp_extras screenshot capture and FPS diagnostics.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Screenshot Capture
- Execute `mcp__brp__brp_extras_screenshot` with absolute path `<cwd>/test_screenshot_{{PORT}}.png` (current working directory + that exact filename)
- **IMPORTANT**: Poll for file completion using `.claude/scripts/integration_tests/extras_test_poll_screenshot.sh <absolute_path_to_screenshot>`
  - Screenshot I/O is asynchronous, this script waits up to 5 seconds for file to be ready
  - Script exits with success (0) if file exists and has non-zero size
  - Script exits with failure (1) if timeout or file not ready
- Verify screenshot file exists and has non-zero size
- **IMPORTANT**: Clean up by passing the same absolute path to: `bash .claude/scripts/integration_tests/cleanup_screenshots.sh <absolute_path_to_screenshot>`

### 3. FPS Diagnostics Test
- Execute `mcp__brp__brp_execute` with method `brp_extras/get_diagnostics` and no params
- Verify response contains `fps` object with keys: `current`, `average`, `smoothed`, `history_len`, `max_history_len`, `history_duration_secs`
- Verify response contains `frame_time_ms` object with keys: `current`, `average`, `smoothed`
- Verify response contains `frame_count` as a number
- Verify `fps.max_history_len` equals 120 (Bevy default)
- Verify `fps.current` is a positive number (app is running)

## Expected Results
- Screenshot capture succeeds and creates valid files
- FPS diagnostics returns valid fps, frame_time_ms, and frame_count data

## Failure Criteria
STOP if: Screenshot capture fails or FPS diagnostics returns invalid data.
