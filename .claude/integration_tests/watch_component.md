# Watch Component Tests

## Objective
Validate entity component watch functionality including monitoring, logging, and watch management.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing watch functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Start Component Watch and Verify Logging
- Spawn an entity with Transform component using `mcp__brp__world_spawn_entity`
- Execute `mcp__brp__world_get_components_watch` on the spawned entity
- Specify components array: `["bevy_transform::components::transform::Transform"]`
- Verify watch returns watch_id and log_path
- Check watch starts successfully
- Execute `mcp__brp__brp_read_log` with returned log filename
- Look for COMPONENT_UPDATE entries in log
- Trigger component changes via mutation
- Verify log captures component updates

### 3. Stop Watch and Verify Clean State
- Execute `mcp__brp__brp_stop_watch` for the active watch_id
- Verify watch stops successfully
- Execute list_active_watches and confirm empty list

## Expected Results
- Component watches start and return proper identifiers
- Watch logs capture appropriate events
- Watch can be stopped and list returns empty

## Failure Criteria
STOP if: Watch creation fails, logs aren't generated, watch management doesn't work, or log files are inaccessible.
