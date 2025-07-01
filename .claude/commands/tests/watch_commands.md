# Watch Commands Tests

## Objective
Validate entity watch functionality including component monitoring, list watching, timeout configuration, debug logging, and watch management.

## Test Steps

### 1. Start Component Watch with Default Timeout
- Execute `mcp__brp__bevy_get_watch` on entity with Transform component
- Specify components array: `["bevy_transform::components::transform::Transform"]`
- Do NOT specify timeout_seconds parameter
- Verify watch returns watch_id and log_path
- Check watch starts successfully with default 30-second timeout

### 2. Test Configurable Timeout
- Execute `mcp__brp__bevy_get_watch` with `timeout_seconds: 2`
- Wait 3 seconds without triggering changes
- Read log file and verify WATCH_TIMEOUT entry appears
- Verify timeout shows `elapsed_seconds: 2` and `configured_timeout_seconds: 2`
- Check watch is removed from active watches list

### 3. Test Never Timeout (timeout=0)
- Execute `mcp__brp__bevy_list_watch` with `timeout_seconds: 0`
- Wait 5 seconds (longer than configurable timeout test)
- Verify watch remains active (no timeout occurs)

### 4. Test Debug Mode Integration
- Execute `mcp__brp__brp_set_debug_mode` with `enabled: false`
- Start a watch and verify NO DEBUG_* entries in log
- Execute `mcp__brp__brp_set_debug_mode` with `enabled: true`
- Start another watch and verify DEBUG_* entries appear (DEBUG_HTTP_RESPONSE, DEBUG_STREAM_STARTED, etc.)

### 5. Verify Component Watch Logging
- Execute `mcp__brp__brp_read_log` with returned log filename
- Look for COMPONENT_UPDATE entries in log
- Trigger component changes via mutation
- Verify log captures component updates

### 6. Start List Watch
- Execute `mcp__brp__bevy_list_watch` on same entity
- Verify different watch_id and log_path returned
- Check list watch starts independently

### 7. List Active Watches
- Execute `mcp__brp__brp_list_active_watches`
- Verify both watches appear in active list
- Check watch details include entity_id, watch_type, log_path, timeout_reason (null for active)

### 8. Test Watch Differentiation
- Add/remove components to trigger list watch logging
- Read list watch log file
- Verify different watch types capture different events
- Check component watch vs list watch logs differ

### 9. Stop Individual Watch
- Execute `mcp__brp__brp_stop_watch` with first watch_id
- Verify watch stops successfully
- Check active watches list updates (one remaining)

### 10. Stop All Remaining Watches  
- Execute stop_watch for remaining watch_id
- Verify all watches are stopped
- Execute list_active_watches and confirm empty list

### 11. Log File Persistence
- Verify log files remain accessible after stopping watches
- Read final log contents to confirm watch captured events
- Check log file cleanup behavior

## Expected Results
- ✅ Component watches start and return proper identifiers
- ✅ Default timeout is 30 seconds when not specified
- ✅ Configurable timeouts work as specified (2-second timeout actually times out, 0 = never timeout)
- ✅ WATCH_TIMEOUT logs show clear timeout information (not generic errors)
- ✅ Debug logging only appears when debug mode is enabled
- ✅ List watches work independently from component watches
- ✅ Active watch listing shows all running watches with timeout_reason field
- ✅ Watch logs capture appropriate events
- ✅ Different watch types log different information
- ✅ Individual watches can be stopped selectively
- ✅ All watches can be stopped completely
- ✅ Log files persist and remain readable

## Failure Criteria
STOP if: Watch creation fails, timeout configuration doesn't work as specified, debug logs appear when disabled, WATCH_TIMEOUT shows generic errors, watches with timeout=0 still timeout, logs aren't generated, watch management doesn't work, or log files are inaccessible.