# Type Schema Mutations Test

## Objective
Validate that mutation paths discovered by `brp_type_schema` work correctly for actual mutation operations across all context types.

## Prerequisites
- Launch extras_plugin example on port 20115
- Tool `brp_type_schema` must be available in the MCP environment
- Tools for spawn, mutate, and get operations must be available

## Test Steps

### 1. Launch Test Application
- Execute `mcp__brp__brp_launch_bevy_example` with `extras_plugin` on port 20115

### 2. Discover Type Schemas
Execute `mcp__brp__brp_type_schema` to get mutation paths for test components (only the types we actually test):
```json
{
  "types": [
    "extras_plugin::TestArrayField",
    "extras_plugin::TestTupleField",
    "extras_plugin::TestTupleStruct",
    "extras_plugin::TestComplexComponent"
  ],
  "port": 20115
}
```
Store response to validate mutation path discovery works.

### 3. Spawn Test Entity

Spawn a single entity with ALL serializable test components to minimize API calls:

```json
{
  "components": {
    "extras_plugin::TestArrayField": {
      "vertices": [[0.0, 1.0], [2.0, 3.0], [4.0, 5.0]],
      "values": [10.0, 20.0, 30.0, 40.0]
    },
    "extras_plugin::TestTupleField": {
      "coords": [100.0, 200.0],
      "color_rgb": [255, 128, 64]
    },
    "extras_plugin::TestTupleStruct": [42.0, "test", true],
    "extras_plugin::TestComplexComponent": {
      "transform": {
        "translation": [1.0, 2.0, 3.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0]
      },
      "mode": "Active",
      "points": [[10.0, 20.0], [30.0, 40.0]],
      "range": [0.0, 100.0],
      "optional_value": 3.14
    }
  }
}
```

Store the returned entity ID for all subsequent mutation tests.

### 4. Execute Mutations and Batch Verify

Perform all mutations on the spawned entity, then verify all changes with a single `bevy_get`:

#### 4a. Execute All Mutations on Test Entity
Perform these mutations in sequence WITHOUT intermediate verification:

1. **ArrayElement Context**: Mutate `.values[1]` on TestArrayField to 999.5
2. **TupleElement Context**: Mutate `.color_rgb.2` on TestTupleField to 200  
3. **RootValue Context**: Replace entire TestTupleStruct with [99.0, "replaced", false]
4. **NestedPath Context**: Mutate `.transform.translation.y` on TestComplexComponent to 555.0
5. **StructField with Enum**: Mutate `.mode` on TestComplexComponent to "Inactive"
6. **Option Field**: Mutate `.optional_value` on TestComplexComponent to null

#### 4b. Batch Verify All Mutations
After ALL mutations complete, execute a single `bevy_get` with all components:
```json
{
  "entity": <spawned_entity_id>,
  "components": [
    "extras_plugin::TestArrayField",
    "extras_plugin::TestTupleField", 
    "extras_plugin::TestTupleStruct",
    "extras_plugin::TestComplexComponent"
  ]
}
```

Verify in the response:
- TestArrayField: values[1] is 999.5
- TestTupleField: color_rgb[2] is 200
- TestTupleStruct: entire value is [99.0, "replaced", false]
- TestComplexComponent:
  - transform.translation.y is 555.0
  - mode is "Inactive"
  - optional_value is null

### 5. Final Cleanup
- Execute `mcp__brp__brp_shutdown` to stop the test app

## Success Criteria

✅ Test passes when:
- All mutation context types work correctly (ArrayElement, TupleElement, RootValue, NestedPath, StructField)
- Enum values can be mutated successfully
- Option fields can be set to None
- Non-serializable components can still be mutated
- All mutations are verified with get operations

## Failure Investigation

If test fails:
1. Verify entity IDs are correct
2. Check mutation path syntax matches discovered paths
3. Verify value types match expected types
4. Check if component exists on entity before mutation