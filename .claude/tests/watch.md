# Watch Commands Tests

## Objective
Validate entity watch functionality including component monitoring, list watching, timeout configuration, and watch management.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing watch functionality, not app management.

## Test Steps

### 1. Start Component Watch and Verify Logging
- Spawn an entity with Transform component using `mcp__brp__world_spawn_entity`
- Execute `mcp__brp__world_get_components_watch` on the spawned entity
- Specify components array: `["bevy_transform::components::transform::Transform"]`
- Verify watch returns watch_id and log_path
- Check watch starts successfully
- Execute `mcp__brp__brp_read_log` with returned log filename
- Look for COMPONENT_UPDATE entries in log
- Trigger component changes via mutation
- Verify log captures component updates

### 2. Test List Watch Component Changes
- Execute `mcp__brp__world_list_components_watch` on the same entity from step 1
- Remove Transform component using `mcp__brp__bevy_remove` with components array `["bevy_transform::components::transform::Transform"]`
- Read list watch log file and verify COMPONENT_UPDATE shows Transform in removed arrayevy
- Add Transform component back using `mcp__brp__bevy_insert` with Transform data
- Read list watch log file again and verify COMPONENT_UPDATE shows Transform in added array
- Stop this watch using `mcp__brp__brp_stop_watch`

### 3. Stop All Watches and Verify Persistence
- Execute `mcp__brp__brp_stop_watch` for all active watch_ids
- Verify all watches stop successfully
- Execute list_active_watches and confirm empty list
- Verify log files remain accessible after stopping watches
- Read final log contents to confirm watches captured events
- Check log file cleanup behavior

## Expected Results
- ✅ Component watches start and return proper identifiers
- ✅ List watches work independently from component watches
- ✅ Active watch listing shows all running
- ✅ Watch logs capture appropriate events
- ✅ Different watch types log different information
- ✅ All watches can be stopped in batch
- ✅ Log files persist and remain readable

## Failure Criteria
STOP if: Watch creation fails, logs aren't generated, watch management doesn't work, or log files are inaccessible.
