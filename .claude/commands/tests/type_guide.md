# Type Schema Validation Test

## Objective
Validate that `brp_type_schema` tool correctly discovers type information and produces the expected output structure for Bevy components.

## Prerequisites
- Launch extras_plugin example on port 20114
- Tool `brp_type_schema` must be available in the MCP environment

## Test Steps

### 1. Launch Test Application
- Execute `mcp__brp__brp_launch_bevy_example` with `extras_plugin` on port 20114
- Verify app is running with `mcp__brp__brp_status`

### 2. Batch Type Schema Discovery

Execute `mcp__brp__brp_type_schema` with ALL test types in a single call:
```json
{
  "types": [
    "bevy_sprite::sprite::Sprite",
    "bevy_transform::components::transform::Transform",
    "bevy_ecs::name::Name",
    "extras_plugin::TestArrayField",
    "extras_plugin::TestTupleField",
    "extras_plugin::TestTupleStruct",
    "extras_plugin::TestComplexComponent",
    "bevy_render::camera::camera::MipBias"
  ],
  "port": 20114
}
```
Store the response as `schema_response`.

### 3. Validate Response Structure

#### 3a. Check Top-Level Fields
Verify the response contains:
- `status` === "success"
- `result` object exists
- `result.type_info` object exists
- `result.discovered_count` === 8
- `result.summary` object with:
  - `successful_discoveries` === 8
  - `failed_discoveries` === 0
  - `total_requested` === 8

### 4. Validate Sprite Component (No Serialize Trait)

#### 4a. Validate Type Info
Verify `result.type_info["bevy_sprite::sprite::Sprite"]` contains:
- `type_name` === "bevy_sprite::sprite::Sprite"
- `in_registry` === true
- `has_serialize` === false
- `has_deserialize` === false
- `supported_operations` array contains ["query", "get", "mutate"]

#### 4b. Verify Mutation Paths Exist Despite No Serialize
- Verify `mutation_paths` field EXISTS (components without Serialize can still be mutated)
- Should contain paths for fields like `.color`, `.flip_x`, `.flip_y`, `.custom_size`, etc.

#### 4c. Verify Spawn Format Is Absent
- Verify `spawn_format` field does NOT exist (cannot spawn without Serialize)

#### 4d. Validate Schema Info
Verify `schema_info` exists and contains:
- `type_kind` === "Struct"
- `properties` with fields: anchor, color, custom_size, flip_x, flip_y, image, image_mode, rect, texture_atlas
- `required` array with: image, color, flip_x, flip_y, anchor, image_mode
- `module_path` === "bevy_sprite::sprite"
- `crate_name` === "bevy_sprite"

### 5. Validate Transform Component (Standard Nested)

Verify `result.type_info["bevy_transform::components::transform::Transform"]`:
- Has `mutation_paths` for translation/rotation/scale with all subfields
- Has `spawn_format` with example values
- Has `schema_info` with properties and required fields
- StructField context for `.translation`, `.rotation`, `.scale`
- NestedPath context for `.translation.x`, `.translation.y`, etc.

### 6. Validate Test Components with Mutation Contexts

#### 6a. TestArrayField - extras_plugin::TestArrayField
Validate `mutation_paths` contains:
- `.vertices` - entire array field with StructField context
- `.vertices[0]`, `.vertices[1]`, `.vertices[2]` - array elements with ArrayElement context
- `.values` - entire array field
- `.values[0]`, `.values[1]`, `.values[2]`, `.values[3]` - array elements

#### 6b. TestTupleField - extras_plugin::TestTupleField  
Validate `mutation_paths` contains:
- `.coords` - entire tuple field with StructField context
- `.coords.0`, `.coords.1` - tuple elements with TupleElement context
- `.color_rgb` - entire tuple field
- `.color_rgb.0`, `.color_rgb.1`, `.color_rgb.2` - tuple elements

#### 6c. TestTupleStruct - extras_plugin::TestTupleStruct
Validate `mutation_paths` contains:
- Root path `""` with context RootValue
- `.0` - first element (f32) with TupleElement context
- `.1` - second element (String) with TupleElement context  
- `.2` - third element (bool) with TupleElement context

#### 6d. TestComplexComponent - extras_plugin::TestComplexComponent
Validate `mutation_paths` contains:
- **StructField context**: `.transform`, `.mode`, `.points`, `.range`, `.optional_value`
- **NestedPath context**: `.transform.translation.x`, `.transform.rotation.x`, etc.
- **ArrayElement context**: `.points[0]`, `.points[1]`
- **TupleElement context**: `.range.0`, `.range.1`

#### 6e. MipBias (Standard TupleStruct) - bevy_render::camera::camera::MipBias
Validate:
- Root path with RootValue context
- `.0` with TupleElement context

### 7. Validate Name Component
Verify `result.type_info["bevy_ecs::name::Name"]`:
- Has appropriate fields for a wrapper type
- Has both `mutation_paths` and `spawn_format` (has Serialize/Deserialize)

### 8. Functional Mutation Testing

Spawn test entities and perform mutations to verify the discovered paths work:

#### 8a. Spawn Test Entities
Spawn entities with components that have Serialize trait:
- TestArrayField, TestTupleField, TestTupleStruct, TestComplexComponent, Transform, Name

#### 8b. Execute and Verify Mutations
For each mutation context type, perform ONE representative mutation and verify:

**ArrayElement**: Mutate `.values[1]` on TestArrayField → Verify with `bevy_get`

**TupleElement**: Mutate `.color_rgb.2` on TestTupleField → Verify with `bevy_get`

**RootValue**: Replace entire TestTupleStruct using path "" → Verify with `bevy_get`

**NestedPath**: Mutate `.transform.translation.y` on TestComplexComponent → Verify with `bevy_get`

**StructField with enum**: Mutate `.mode` to "Inactive" on TestComplexComponent → Verify with `bevy_get`

**Option field**: Mutate `.optional_value` to None on TestComplexComponent → Verify with `bevy_get`

**Non-Serializable**: Mutate `.flip_x` to true on Sprite → Verify with `bevy_get`

### 9. Type Schema in Error Responses

Test that format errors include embedded type_schema information for self-correction.

#### 9a. Test Format Error with Type Schema Embedding

**STEP 1**: Query for an entity with Transform:
- Tool: mcp__brp__bevy_query
- Use filter: {"with": ["bevy_transform::components::transform::Transform"]}

**STEP 2**: Attempt insert with INCORRECT object format:
```json
mcp__brp__bevy_insert with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "components": {
    "bevy_transform::components::transform::Transform": {
      "translation": {"x": 10.0, "y": 20.0, "z": 30.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
      "scale": {"x": 2.0, "y": 2.0, "z": 2.0}
    }
  },
  "port": 20114
}
```

**Expected Error Response**:
- Status: "error"
- Message contains: "Format error - see 'type_schema' field for correct format"
- error_info contains:
  - original_error: The BRP error message
  - type_schema: Embedded type_schema for Transform with correct array format

**STEP 3**: Verify type_schema in error contains:
- Transform spawn_format showing correct array format for Vec3 fields
- Transform mutation_paths for reference
- Same structure as direct brp_type_schema response

#### 9b. Test Multiple Type Errors

**STEP 1**: Attempt spawn with multiple incorrect formats:
```json
mcp__brp__bevy_spawn with parameters:
{
  "components": {
    "bevy_transform::components::transform::Transform": {
      "translation": {"x": 5.0, "y": 15.0, "z": 25.0}
    },
    "bevy_ecs::name::Name": 123
  },
  "port": 20114
}
```

**Expected Result**:
- Error with type_schema for both Transform and Name
- Each type shows correct format in type_schema field

#### 9c. Test Mutation Format Error with Type Schema Embedding

**STEP 1**: Query for an entity with Transform:
- Tool: mcp__brp__bevy_query
- Use filter: {"with": ["bevy_transform::components::transform::Transform"]}

**STEP 2**: Attempt mutation with INCORRECT object format (should be array):
```json
mcp__brp__bevy_mutate_component with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "component": "bevy_transform::components::transform::Transform",
  "path": ".translation",
  "value": {"x": 100.0, "y": 200.0, "z": 300.0},
  "port": 20114
}
```

**Expected Error Response**:
- Status: "error"
- Message contains: "Format error - see 'type_schema' field for correct format"
- error_info contains:
  - original_error: The BRP mutation error message
  - type_schema: Embedded type_schema for Transform showing correct array format
  - Should show `.translation` expects `[f32, f32, f32]` not `{x, y, z}` object

**STEP 3**: Verify mutation-specific type_schema contains:
- Transform mutation_paths including `.translation` with correct array format
- Transform spawn_format showing proper Vec3 array structure
- Clear guidance that Vec3 fields require `[x, y, z]` array format

#### 9d. Test Non-Transformable Input Error

**STEP 1**: Test completely malformed input:
```json
mcp__brp__bevy_insert with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "components": {
    "bevy_transform::components::transform::Transform": 123
  },
  "port": 20114
}
```

**Expected Result**:
- Error response with type_schema guidance
- Clear indication that format cannot be corrected automatically
- type_schema shows expected Transform structure

#### 9e. Test Component Without Serialize/Deserialize - Spawn Failure

**STEP 1**: Attempt to spawn Visibility (lacks Serialize/Deserialize):
```json
mcp__brp__bevy_spawn with parameters:
{
  "components": {
    "bevy_render::view::visibility::Visibility": "Visible"
  },
  "port": 20114
}
```

**Expected Result**:
- Error indicating component lacks required traits
- Error message mentions Serialize/Deserialize requirements
- May include type_schema showing component is in registry but not spawnable

#### 9f. Test Enum Mutation Error Guidance

**STEP 1**: Query for entity with Visibility:
- Tool: mcp__brp__bevy_query
- Filter: {"with": ["bevy_render::view::visibility::Visibility"]}

**STEP 2**: Attempt mutation with INCORRECT enum syntax:
```json
mcp__brp__bevy_mutate_component with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "component": "bevy_render::view::visibility::Visibility",
  "path": ".Visible",
  "value": {},
  "port": 20114
}
```

**Expected Result**:
- Error with helpful guidance about enum mutation
- error_info includes:
  - hint about using empty path for enums
  - valid_values array: ["Visible", "Hidden", "Inherited"]
  - examples of correct usage

## Success Criteria

✅ Test passes when:
- Single batched discovery call retrieves all 8 types successfully
- All expected fields are present for each type
- Mutation contexts are correct (RootValue, StructField, TupleElement, ArrayElement, NestedPath)
- Functional mutations work for all context types
- Components without Serialize can be mutated but not spawned
- Tool provides comprehensive type information for BRP operations
- **Format errors include embedded type_schema for failed types**
- **Type extraction works from both parameters and error messages**
- **Mutation format errors include embedded type_schema information**
- **Error guidance is clear and actionable for self-correction**

## Failure Investigation

If test fails:
1. Check if app is running with BRP enabled
2. Verify types exist in registry
3. Compare actual vs expected mutation paths
4. Check if mutations are succeeding
5. **Verify error responses include type_schema field**
6. **Check type extraction logic in error handling**
