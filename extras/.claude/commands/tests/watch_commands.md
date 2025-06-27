# Watch Commands Tests

## Objective
Validate entity watch functionality including component monitoring, list watching, and watch management.

## Test Steps

### 1. Start Component Watch
- Execute `mcp__brp__bevy_get_watch` on entity with Transform component
- Specify components array: `["bevy_transform::components::transform::Transform"]`
- Verify watch returns watch_id and log_path
- Check watch starts successfully

### 2. Verify Component Watch Logging
- Execute `mcp__brp__brp_read_log` with returned log filename
- Look for COMPONENT_UPDATE entries in log
- Trigger component changes via mutation
- Verify log captures component updates

### 3. Start List Watch
- Execute `mcp__brp__bevy_list_watch` on same entity
- Verify different watch_id and log_path returned
- Check list watch starts independently

### 4. List Active Watches
- Execute `mcp__brp__brp_list_active_watches`
- Verify both watches appear in active list
- Check watch details include entity_id, watch_type, log_path

### 5. Test Watch Differentiation
- Add/remove components to trigger list watch logging
- Read list watch log file
- Verify different watch types capture different events
- Check component watch vs list watch logs differ

### 6. Stop Individual Watch
- Execute `mcp__brp__brp_stop_watch` with first watch_id
- Verify watch stops successfully
- Check active watches list updates (one remaining)

### 7. Stop All Remaining Watches  
- Execute stop_watch for remaining watch_id
- Verify all watches are stopped
- Execute list_active_watches and confirm empty list

### 8. Log File Persistence
- Verify log files remain accessible after stopping watches
- Read final log contents to confirm watch captured events
- Check log file cleanup behavior

## Expected Results
- ✅ Component watches start and return proper identifiers
- ✅ List watches work independently from component watches
- ✅ Active watch listing shows all running watches
- ✅ Watch logs capture appropriate events
- ✅ Different watch types log different information
- ✅ Individual watches can be stopped selectively
- ✅ All watches can be stopped completely
- ✅ Log files persist and remain readable

## Failure Criteria
STOP if: Watch creation fails, logs aren't generated, watch management doesn't work, or log files are inaccessible.