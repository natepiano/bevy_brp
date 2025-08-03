# Status and Shutdown Tests

## Objective
Validate all error variants for `brp_status` and `brp_shutdown` commands, ensuring proper StructuredError responses with appropriate error_info fields.

## Test Steps

### 1. ProcessNotFoundError - Status with Non-Existent App
- Execute `mcp__brp__brp_status` with app_name: "definitely_not_running_app", port: 20113
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "definitely_not_running_app"
  - brp_responding_on_port: false
  - port: 20113
  - similar_app_name: null (or absent)
- Verify message indicates process not found

### 2. ProcessNotFoundError with BRP on Different App
- Launch extras_plugin example on port 20113
- Execute `mcp__brp__brp_status` with app_name: "wrong_app_name", port: 20113
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "wrong_app_name"
  - brp_responding_on_port: true
  - port: 20113
  - similar_app_name: possibly "extras_plugin" (if detected)
- Verify message mentions BRP is responding on the port (another process may be using it)
- Shutdown the extras_plugin app

### 3. BrpNotRespondingError - App Running Without BRP
- Launch no_extras_plugin example (which runs on port 25000, not 20113)
- Execute `mcp__brp__brp_status` with app_name: "no_extras_plugin", port: 20113
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "no_extras_plugin"
  - pid: (should be present)
  - port: 20113
- Verify message indicates process is running but not responding to BRP on specified port
- Verify message suggests adding RemotePlugin to Bevy app

### 4. ProcessNotRunningError - Shutdown Non-Existent App
- Execute `mcp__brp__brp_shutdown` with app_name: "not_running_app", port: 20113
- Verify response has status: "error"
- Verify error_info contains:
  - app_name: "not_running_app"
- Verify message indicates process is not currently running

### 5. Successful Shutdown with Process Kill (Degraded Success)
- Ensure no_extras_plugin is still running from step 3
- Execute `mcp__brp__brp_shutdown` with app_name: "no_extras_plugin", port: 25000
- Verify response has status: "success" (not error!)
- Verify metadata contains:
  - app_name: "no_extras_plugin"
  - pid: (should be present)
  - shutdown_method: "process_kill"
  - port: 25000
  - warning: "Consider adding bevy_brp_extras for clean shutdown" (or similar)
- Verify message indicates process was terminated using kill

### 6. Clean Shutdown Success (for comparison)
- Launch extras_plugin example on port 20113
- Execute `mcp__brp__brp_shutdown` with app_name: "extras_plugin", port: 20113
- Verify response has status: "success"
- Verify metadata contains:
  - app_name: "extras_plugin"
  - pid: (should be present)
  - shutdown_method: "clean_shutdown"
  - port: 20113
  - warning: null (or absent)
- Verify message indicates graceful shutdown via bevy_brp_extras

### 7. Status Success (for comparison)
- Launch extras_plugin example on port 20113
- Execute `mcp__brp__brp_status` with app_name: "extras_plugin", port: 20113
- Verify response has status: "success"
- Verify metadata contains:
  - app_name: "extras_plugin"
  - pid: (should be present)
  - port: 20113
- Verify message indicates process is running with BRP enabled
- Shutdown the app

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