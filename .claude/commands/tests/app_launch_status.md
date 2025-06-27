# App Launch and Status Tests

## Objective
Validate application launch capabilities and BRP status monitoring functionality.

## Test Steps

### 1. Launch Example
- Use `mcp__brp__brp_launch_bevy_example` to launch the example specified in the test configuration
- Verify launch response includes PID, log file path, working directory
- Confirm example starts in background successfully

### 2. BRP Status Check
- Execute `mcp__brp__brp_status` with app name and port
- Verify status shows "running_with_brp"

### 3. BRP Connectivity Test
- Execute `mcp__brp__bevy_rpc_discover`
- Verify RPC methods are available
- Check both standard BRP and brp_extras methods are listed

### 4. Window Title Verification
- Take screenshot using `mcp__brp__brp_extras_screenshot` with path `test_screenshot_app_launch_status.png` (project root)
- Verify window title shows correct port number
- Confirm screenshot file is created and readable in project root
- **IMPORTANT**: Clean up screenshot file from project root at end of test

### 5. Clean Shutdown
- Execute `mcp__brp__brp_extras_shutdown`
- Verify clean shutdown (not process kill)
- Confirm app process terminates and BRP becomes unresponsive

## Expected Results
- ✅ App launches successfully on specified port
- ✅ BRP connectivity is established and functional
- ✅ Status monitoring works correctly
- ✅ Window title reflects correct port
- ✅ Clean shutdown is achieved

## Failure Criteria
STOP if: Launch fails, BRP unresponsive, RPC discovery incomplete, screenshot fails, or shutdown requires process termination.
