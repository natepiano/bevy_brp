# Resource Serialization Tests

## Objective
Validate BRP behavior with resources that lack Serialize/Deserialize traits and ensure proper error handling for insert_resource operations.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing resource operations, not app management.

**CRITICAL** You must include the specified {{PORT}} in the call to the tool or it will default to 15702 and FAIL!

## Test Resources
This test uses two resources defined in the extras_plugin example:
- `TestConfigResource` - Has Serialize, Deserialize, and Reflect traits
- `RuntimeStatsResource` - Has only Reflect trait (no Serialize/Deserialize)

For resources, apparently it doesn't matter if they have Serialize/Deserialize traits - they can be inserted and updated without them.

## Test Steps

### 1. Insert Resource Test
**STEP 1**: List available resources:
- Tool: mcp__brp__bevy_list_resources
- Port: {{PORT}}
- Verify response includes both test resources:
  - `extras_plugin::TestConfigResource`
  - `extras_plugin::RuntimeStatsResource`

**STEP 2**: Insert/update TestConfigResource (has traits):
```
mcp__brp__bevy_insert_resource with parameters:
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
- Confirm no error messages about missing traits

**STEP 3**: Get the inserted resource to verify:
- Tool: mcp__brp__bevy_get_resource
- Resource: `extras_plugin::TestConfigResource`
- Port: {{PORT}}
- Verify the resource data matches what was inserted

**STEP 4**: Insert/update RuntimeStatsResource (no Serialize/Deserialize traits):
```
mcp__brp__bevy_insert_resource with parameters:
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
- Verify operation succeeds despite lacking Serialize/Deserialize traits
- Confirm no error messages about missing traits

**STEP 5**: Get the inserted RuntimeStatsResource to verify:
- Tool: mcp__brp__bevy_get_resource
- Resource: `extras_plugin::RuntimeStatsResource`
- Port: {{PORT}}
- Verify the resource data matches what was inserted

### 2. Resource Mutation Test (Should Work Without Serialize/Deserialize)
**STEP 1**: Mutate the RuntimeStatsResource (which lacks Serialize/Deserialize):
```
mcp__brp__bevy_mutate_resource with parameters:
{
  "resource": "extras_plugin::RuntimeStatsResource",
  "path": ".debug_mode",
  "value": true,
  "port": {{PORT}}
}
```
- Verify mutation SUCCEEDS (mutation doesn't require Serialize/Deserialize)

**STEP 2**: Mutate another field:
```
mcp__brp__bevy_mutate_resource with parameters:
{
  "resource": "extras_plugin::RuntimeStatsResource",
  "path": ".frame_count",
  "value": 42,
  "port": {{PORT}}
}
```
- Verify this also succeeds

**STEP 3**: Get resource to verify mutations:
- Tool: mcp__brp__bevy_get_resource
- Resource: `extras_plugin::RuntimeStatsResource`
- Port: {{PORT}}
- Verify frame_count is 42 and debug_mode is true

### 3. Mutation Error Tests
**STEP 1**: Test mutation with invalid field path:
```
mcp__brp__bevy_mutate_resource with parameters:
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
mcp__brp__bevy_mutate_resource with parameters:
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
mcp__brp__bevy_mutate_resource with parameters:
{
  "resource": "my_game::config::NonExistentResource",
  "path": ".some_field",
  "value": 123,
  "port": {{PORT}}
}
```
- Verify error mentions: "Unknown resource type: `my_game::config::NonExistentResource`"

### 4. Resource Removal Test
**STEP 1**: Remove the TestConfigResource:
```
mcp__brp__bevy_remove_resource with parameters:
{
  "resource": "extras_plugin::TestConfigResource",
  "port": {{PORT}}
}
```
- Verify removal succeeds

**STEP 2**: Try to get the removed resource:
- Tool: mcp__brp__bevy_get_resource
- Resource: `extras_plugin::TestConfigResource`
- Port: {{PORT}}
- Verify it returns an error indicating resource not found

### 5. Error Message Quality Check
All resource insertion errors should include:
- Clear problem description mentioning the specific resource type
- Guidance that this is a BRP requirement for insert_resource
- Helpful suggestion to add the traits to the resource definition

### 6. Non-Existent Resource Test
**STEP 1**: Attempt to insert a non-existent resource:
```
mcp__brp__bevy_insert_resource with parameters:
{
  "resource": "my_game::config::NonExistentResource",
  "value": {"some": "data"},
  "port": {{PORT}}
}
```

**STEP 2**: Verify error is different from missing traits:
- Error should indicate resource type is not registered
- Should NOT mention Serialize/Deserialize traits
- This confirms the system can distinguish between:
  - Resources that exist but lack traits
  - Resources that don't exist at all

## Expected Results
- ✅ Insert succeeds for TestConfigResource
- ✅ Mutation works for RuntimeStatsResource
- ✅ Correct mutations update resource fields properly
- ✅ Invalid field path mutations fail with clear error about missing field
- ✅ Type mismatch mutations fail with clear type error messages
- ✅ Mutations on non-existent resources fail with "Unknown resource type" error
- ✅ Resource listing shows both test resources
- ✅ Error messages clearly distinguish between missing traits vs non-existent resources
- ✅ Educational error messages guide users to the solution
- ✅ Resource removal works correctly

## Failure Criteria
STOP if: Resource errors are unclear, insert succeeds when it shouldn't, mutation fails for registered resources, or error guidance is insufficient.
