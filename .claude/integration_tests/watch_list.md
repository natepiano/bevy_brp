# Watch List Tests

## Objective
Validate entity list watch functionality including detecting component additions and removals.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing watch functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Test List Watch Component Changes
- Spawn an entity with Transform component using `mcp__brp__world_spawn_entity`
- Execute `mcp__brp__world_list_components_watch` on the spawned entity
- Remove Transform component using `mcp__brp__world_remove_components` with components array `["bevy_transform::components::transform::Transform"]`
- Read list watch log file and verify COMPONENT_UPDATE shows Transform in removed array
- Add Transform component back using `mcp__brp__world_insert_components` with Transform data
- Read list watch log file again and verify COMPONENT_UPDATE shows Transform in added array

### 3. Stop Watch and Verify Clean State
- Execute `mcp__brp__brp_stop_watch` for the active watch_id
- Verify watch stops successfully
- Execute list_active_watches and confirm empty list

## Expected Results
- List watches start and return proper identifiers
- Watch logs capture component additions and removals
- Watch can be stopped and list returns empty

## Failure Criteria
STOP if: Watch creation fails, logs aren't generated, watch management doesn't work, or log files are inaccessible.
