# Type Schema Discovery Test

## Objective
Validate that `brp_type_schema` tool correctly discovers type information and produces the expected output structure for Bevy components.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing type schema discovery, not app management.

## Test Steps

### 1. Batch Type Schema Discovery

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


## Success Criteria

✅ Test passes when:
- Single batched discovery call retrieves all 8 types successfully
- All expected fields are present for each type
- Mutation contexts are correct (RootValue, StructField, TupleElement, ArrayElement, NestedPath)
- Components without Serialize can be mutated but not spawned
- Tool provides comprehensive type metadata and structure information

## Failure Investigation

If test fails:
1. Check if app is running with BRP enabled
2. Verify types exist in registry
3. Compare actual vs expected mutation paths and contexts
4. Check schema structure matches expected format