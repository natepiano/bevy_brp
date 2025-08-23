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
- Verify app is running with `mcp__brp__brp_status`

### 2. Discover Type Schemas
Execute `mcp__brp__brp_type_schema` to get mutation paths for test components:
```json
{
  "types": [
    "bevy_sprite::sprite::Sprite",
    "extras_plugin::TestArrayField",
    "extras_plugin::TestTupleField",
    "extras_plugin::TestTupleStruct",
    "extras_plugin::TestComplexComponent"
  ],
  "port": 20115
}
```
Store response to use mutation paths.

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

For Sprite component (non-serializable), query for an existing entity with Sprite component.

### 4. Execute and Verify Mutations

For each mutation context type, perform mutations and verify results on the SINGLE spawned entity:

#### 4a. ArrayElement Context
Mutate `.values[1]` on TestArrayField component:
- Use `bevy_mutate_component` on spawned entity with component "extras_plugin::TestArrayField", path ".values[1]" and value 999.5
- Verify with `bevy_get` that values[1] changed from 20.0 to 999.5

#### 4b. TupleElement Context  
Mutate `.color_rgb.2` on TestTupleField component:
- Use `bevy_mutate_component` on spawned entity with component "extras_plugin::TestTupleField", path ".color_rgb.2" and value 200
- Verify with `bevy_get` that color_rgb[2] changed from 64 to 200

#### 4c. RootValue Context
Replace entire TestTupleStruct using root path:
- Use `bevy_mutate_component` on spawned entity with component "extras_plugin::TestTupleStruct", path "" and value [99.0, "replaced", false]
- Verify with `bevy_get` that entire tuple struct was replaced

#### 4d. NestedPath Context
Mutate `.transform.translation.y` on TestComplexComponent:
- Use `bevy_mutate_component` on spawned entity with component "extras_plugin::TestComplexComponent", path ".transform.translation.y" and value 555.0
- Verify with `bevy_get` that translation.y changed from 2.0 to 555.0

#### 4e. StructField with Enum
Mutate `.mode` to "Inactive" on TestComplexComponent:
- Use `bevy_mutate_component` on spawned entity with component "extras_plugin::TestComplexComponent", path ".mode" and value "Inactive"
- Verify with `bevy_get` that mode changed from "Active" to "Inactive"

#### 4f. Option Field
Mutate `.optional_value` to None on TestComplexComponent:
- Use `bevy_mutate_component` on spawned entity with component "extras_plugin::TestComplexComponent", path ".optional_value" and value null
- Verify with `bevy_get` that optional_value is now None/null

#### 4g. Non-Serializable Component
Find or create entity with Sprite, then mutate `.flip_x`:
- Query for entity with Sprite component
- Use `bevy_mutate_component` with path ".flip_x" and value true
- Verify with `bevy_get` that flip_x changed to true

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