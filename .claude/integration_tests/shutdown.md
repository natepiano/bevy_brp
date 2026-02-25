# Status and Shutdown Tests

## Objective
Validate all error variants for `brp_status` and `brp_shutdown` commands, ensuring proper StructuredError responses with appropriate error_info fields.

## App Instance Configuration
This test uses pre-launched app instances referenced by label:
- **status_check_app**: extras_plugin (survives entire test)
- **shutdown_test_app**: extras_plugin (consumed by clean shutdown test)
- **no_brp_app**: no_extras_plugin on fixed port 25000 (consumed by process kill test)

**CRITICAL**: Do NOT launch or shutdown apps yourself. All instances are pre-launched and managed externally. Use the port assigned to each label as provided in the test configuration.

## Test Steps

### 1. ProcessNotFoundError - Status with Non-Existent App
- Execute `mcp__brp__brp_status` with app_name: "definitely_not_running_app", port: 29999
  - **NOTE**: Uses port 29999 where nothing is running, to get a clean "not found" with no BRP responder
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "definitely_not_running_app"
  - brp_responding_on_port: false
  - port: 29999
  - similar_app_name: null (or absent)
- Verify message indicates process not found

### 2. ProcessNotFoundError with BRP on Different App
- Execute `mcp__brp__brp_status` with app_name: "wrong_app_name", port: [status_check_app port]
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "wrong_app_name"
  - brp_responding_on_port: true
  - port: [status_check_app port]
  - similar_app_name: possibly "extras_plugin" (if detected)
- Verify message mentions BRP is responding on the port (another process may be using it)

### 3. BrpNotRespondingError - App Running Without BRP
- Execute `mcp__brp__brp_status` with app_name: "no_extras_plugin", port: 29998
  - **NOTE**: Uses port 29998 where nothing is listening. The tool finds the no_extras_plugin process by name (it's running on port 25000) but BRP doesn't respond on 29998, triggering BrpNotRespondingError.
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "no_extras_plugin"
  - pid: (should be present)
  - port: 29998
- Verify message indicates process is running but not responding to BRP on specified port
- Verify message suggests adding RemotePlugin to Bevy app

### 4. Process Kill Shutdown (Degraded Success)
- Execute `mcp__brp__brp_shutdown` with app_name: "no_extras_plugin", port: 25000
  - **IMPORTANT**: Use port 25000 (the no_brp_app's fixed port)
- Verify response has status: "success" (not error!)
- Verify metadata contains:
  - app_name: "no_extras_plugin"
  - pid: (should be present)
  - shutdown_method: "process_kill"
  - port: 25000
  - warning: "Consider adding bevy_brp_extras for clean shutdown" (or similar)
- Verify message indicates process was terminated using kill
- **NOTE**: The no_brp_app instance is now consumed

### 5. Clean Shutdown Success
- Execute `mcp__brp__brp_shutdown` with app_name: "extras_plugin", port: [shutdown_test_app port]
- Verify response has status: "success"
- Verify metadata contains:
  - app_name: "extras_plugin"
  - pid: (should be present)
  - shutdown_method: "clean_shutdown"
  - port: [shutdown_test_app port]
  - warning: null (or absent)
- Verify message indicates graceful shutdown via bevy_brp_extras
- **NOTE**: The shutdown_test_app instance is now consumed

### 6. Status Success
- Execute `mcp__brp__brp_status` with app_name: "extras_plugin", port: [status_check_app port]
- Verify response has status: "success"
- Verify metadata contains:
  - app_name: "extras_plugin"
  - pid: (should be present)
  - port: [status_check_app port]
- Verify message indicates process is running with BRP enabled

### 7. ProcessNotRunningError - Shutdown Non-Existent App
- Execute `mcp__brp__brp_shutdown` with app_name: "not_running_app", port: 29997
  - **NOTE**: Uses port 29997 where nothing is running, to get a clean "not running" error
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "not_running_app"
- Verify message indicates process is not currently running

## Expected Results
- ✅ ProcessNotFoundError includes all expected fields in error_info
- ✅ ProcessNotFoundError distinguishes between BRP responding/not responding cases
- ✅ BrpNotRespondingError includes PID and suggests adding RemotePlugin
- ✅ ProcessNotRunningError for shutdown includes app_name in error_info
- ✅ Process kill shutdown is "success" with warning, not error
- ✅ Error responses use error_info, success responses use metadata
- ✅ All structured errors provide appropriate context for debugging

## Failure Criteria
STOP if: Error responses don't include expected error_info fields, success/error status is incorrect, or structured error pattern is not followed correctly.
