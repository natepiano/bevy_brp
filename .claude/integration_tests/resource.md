# Resource Operations Tests

## Objective
Validate BRP resource operations including insert, get, mutate, and remove.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing resource operations, not app management.

**CRITICAL** You must include the specified {{PORT}} in the call to the tool or it will default to 15702 and FAIL!

## Test Resources
This test uses two resources defined in the extras_plugin example:
- `TestConfigResource` - Standard test resource
- `RuntimeStatsResource` - Standard test resource

## Test Steps

### 1. Insert Resource Test
**STEP 1**: List available resources:
- Tool: mcp__brp__world_list_resources
- Port: {{PORT}}
- Verify response includes both test resources:
  - `extras_plugin::TestConfigResource`
  - `extras_plugin::RuntimeStatsResource`

**STEP 2**: Insert/update TestConfigResource:
```
mcp__brp__world_insert_resources with parameters:
{
  "resource": "extras_plugin::TestConfigResource",
  "value": {
    "setting_a": 3.14,
    "setting_b": "hello world",
    "enabled": true
  },
  "port": {{PORT}}
}
```
- Verify operation succeeds

**STEP 3**: Get the inserted resource to verify:
- Tool: `mcp__brp__world_get_resources`
- Resource: `extras_plugin::TestConfigResource`
- Port: {{PORT}}
- Verify the resource data matches what was inserted

**STEP 4**: Insert/update RuntimeStatsResource:
```
`mcp__brp__world_insert_resources` with parameters:
{
  "resource": "extras_plugin::RuntimeStatsResource",
  "value": {
    "frame_count": 100,
    "total_time": 5.5,
    "debug_mode": false
  },
  "port": {{PORT}}
}
```
- Verify operation succeeds

**STEP 5**: Get the inserted RuntimeStatsResource to verify:
- Tool: `mcp__brp__world_get_resources`
- Resource: `extras_plugin::RuntimeStatsResource`
- Port: {{PORT}}
- Verify the resource data matches what was inserted

### 2. Mutation Error Tests
**STEP 1**: Test mutation with invalid field path:
```
`mcp__brp__world_mutate_resources` with parameters:
{
  "resource": "extras_plugin::RuntimeStatsResource",
  "path": ".invalid_field",
  "value": 123,
  "port": {{PORT}}
}
```
- Verify error mentions: "The struct accessed doesn't have an `invalid_field` field"

**STEP 2**: Test mutation with type mismatch:
```
`mcp__brp__world_mutate_resources` with parameters:
{
  "resource": "extras_plugin::RuntimeStatsResource",
  "path": ".frame_count",
  "value": "not a number",
  "port": {{PORT}}
}
```
- Verify error mentions: "invalid type: string \"not a number\", expected u32"

**STEP 3**: Test mutation on non-existent resource type:
```
`mcp__brp__world_mutate_resources` with parameters:
{
  "resource": "my_game::config::NonExistentResource",
  "path": ".some_field",
  "value": 123,
  "port": {{PORT}}
}
```
- Verify error mentions: "Unknown resource type: `my_game::config::NonExistentResource`"

### 3. Resource Removal Test
**STEP 1**: Remove the TestConfigResource:
```
`mcp__brp__world_remove_resources` with parameters:
{
  "resource": "extras_plugin::TestConfigResource",
  "port": {{PORT}}
}
```
- Verify removal succeeds

**STEP 2**: Try to get the removed resource:
- Tool: `mcp__brp__world_get_resources`
- Resource: `extras_plugin::TestConfigResource`
- Port: {{PORT}}
- Verify it returns an error indicating resource not found

### 4. Non-Existent Resource Test
**STEP 1**: Attempt to insert a non-existent resource:
```
`mcp__brp__world_insert_resources` with parameters:
{
  "resource": "my_game::config::NonExistentResource",
  "value": {"some": "data"},
  "port": {{PORT}}
}
```

**STEP 2**: Verify error indicates resource is not registered:
- Error should indicate resource type is not registered
- Should provide clear guidance about the issue

## Expected Results
- ✅ Insert succeeds for both test resources
- ✅ Invalid field path mutations fail with clear error about missing field
- ✅ Type mismatch mutations fail with clear type error messages
- ✅ Mutations on non-existent resources fail with "Unknown resource type" error
- ✅ Resource listing shows both test resources
- ✅ Error messages are clear and actionable
- ✅ Resource removal works correctly

## Failure Criteria
STOP if: Resource errors are unclear, insert fails for valid resources, mutation fails for registered resources, or error guidance is insufficient.
