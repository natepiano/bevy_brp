# Watch Commands Tests

## Objective
Validate entity watch functionality including component monitoring, list watching, timeout configuration, and watch management.

## Test Steps

### 1. Start Component Watch and Verify Logging
- Spawn an entity with Transform component using `mcp__brp__bevy_spawn`
- Execute `mcp__brp__bevy_get_watch` on the spawned entity
- Specify components array: `["bevy_transform::components::transform::Transform"]`
- Specify `timeout_seconds: 10` (reduced from default 30)
- Verify watch returns watch_id and log_path
- Check watch starts successfully
- Execute `mcp__brp__brp_read_log` with returned log filename
- Look for COMPONENT_UPDATE entries in log
- Trigger component changes via mutation
- Verify log captures component updates

### 2. Test Configurable Timeout
- Execute `mcp__brp__bevy_get_watch` with `timeout_seconds: 2`
- Wait 3 seconds without triggering changes
- Read log file and verify WATCH_TIMEOUT entry appears
- Verify timeout shows `elapsed_seconds: 2` and `configured_timeout_seconds: 2`
- Check watch is removed from active watches list

### 3. Test List Watch Component Changes
- Execute `mcp__brp__bevy_list_watch` on the same entity from step 1 with `timeout_seconds: 30`
- Remove Transform component using `mcp__brp__bevy_remove` with components array `["bevy_transform::components::transform::Transform"]`
- Wait 1 second
- Read list watch log file and verify COMPONENT_UPDATE shows Transform in removed array
- Add Transform component back using `mcp__brp__bevy_insert` with Transform data
- Read list watch log file again and verify COMPONENT_UPDATE shows Transform in added array
- Stop this watch using `mcp__brp__brp_stop_watch`

### 4. Test Never Timeout Watch
- Execute `mcp__brp__bevy_get_watch` on the same entity with `timeout_seconds: 0`
- Wait 3 seconds to verify no timeout occurs
- Execute `mcp__brp__brp_list_active_watches` and verify watch is still active
- Stop this watch using `mcp__brp__brp_stop_watch`

### 5. Stop All Watches and Verify Persistence (Combined Tests 9, 10, 11)
- Execute `mcp__brp__brp_stop_watch` for all active watch_ids
- Verify all watches stop successfully
- Execute list_active_watches and confirm empty list
- Verify log files remain accessible after stopping watches
- Read final log contents to confirm watches captured events
- Check log file cleanup behavior

## Expected Results
- ✅ Component watches start with reduced timeout and return proper identifiers
- ✅ Configurable timeouts work as specified (2-second timeout actually times out in 2.5s, 0 = never timeout)
- ✅ WATCH_TIMEOUT logs show clear timeout information (not generic errors)
- ✅ List watches work independently from component watches
- ✅ Active watch listing shows all running watches with timeout_reason field
- ✅ Watch logs capture appropriate events
- ✅ Different watch types log different information
- ✅ All watches can be stopped in batch
- ✅ Log files persist and remain readable

## Failure Criteria
STOP if: Watch creation fails, timeout configuration doesn't work as specified, WATCH_TIMEOUT shows generic errors, watches with timeout=0 still timeout, logs aren't generated, watch management doesn't work, or log files are inaccessible.
