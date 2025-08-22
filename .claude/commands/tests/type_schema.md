# Type Schema Validation Test

## Objective
Validate that `brp_type_schema` tool correctly discovers type information and produces the expected output structure for Bevy components.

## Prerequisites
- Launch extras_plugin example on port 20114
- Tool `brp_type_schema` must be available in the MCP environment
- Expected output file at `.claude/commands/expected_sprite.json`

## Test Steps

### 1. Launch Test Application
- Execute `mcp__brp__brp_launch_bevy_example` with `extras_plugin` on port 20114
- Verify app is running with `mcp__brp__brp_status`

### 2. Run Type Schema Discovery

Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["bevy_sprite::sprite::Sprite"],
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
- `result.discovered_count` === 1
- `result.summary` object with:
  - `successful_discoveries` === 1
  - `failed_discoveries` === 0
  - `total_requested` === 1

#### 3b. Validate Type Info
Verify `result.type_info["bevy_sprite::sprite::Sprite"]` contains:
- `type_name` === "bevy_sprite::sprite::Sprite"
- `in_registry` === true
- `has_serialize` === false
- `has_deserialize` === false
- `supported_operations` array contains ["query", "get", "mutate"] (mutate is supported even without Serialize)

### 4. Validate Sprite Mutation Support (No Serialize Trait)

#### 4a. Verify Mutation Paths Exist Despite No Serialize
For Sprite (which lacks Serialize trait):
- Verify `mutation_paths` field EXISTS in the response
- Components without Serialize can still be mutated (just not spawned/inserted)
- Should contain paths for fields like `.color`, `.flip_x`, `.flip_y`, `.custom_size`, etc.

#### 4b. Verify Spawn Format Is Absent
- Verify `spawn_format` field does NOT exist in the response
- The field should be completely absent since Sprite cannot be spawned/inserted without Serialize

### 5. Validate Schema Info for Sprite

#### 5a. Check Schema Info Exists
Verify `result.type_info["bevy_sprite::sprite::Sprite"].schema_info` exists and contains:
- `type_kind` === "Struct"
- `properties` object with field definitions
- `required` array
- `module_path` === "bevy_sprite::sprite"
- `crate_name` === "bevy_sprite"

#### 5b. Validate Required Fields
Verify `schema_info.required` contains exactly:
- "image"
- "color"
- "flip_x"
- "flip_y"
- "anchor"
- "image_mode"

#### 5c. Validate Properties Structure
Verify `schema_info.properties` contains these fields:
- `anchor` with type reference
- `color` with type reference
- `custom_size` with type reference
- `flip_x` with type reference
- `flip_y` with type reference
- `image` with type reference
- `image_mode` with type reference
- `rect` with type reference
- `texture_atlas` with type reference

### 6. Multi-Type Discovery Test

Execute `mcp__brp__brp_type_schema` with multiple types:
```json
{
  "types": [
    "bevy_transform::components::transform::Transform",
    "bevy_ecs::name::Name",
    "bevy_sprite::sprite::Sprite"
  ],
  "port": 20114
}
```

Validate:
- `result.discovered_count` === 3
- All three types present in `result.type_info`
- Transform:
  - Has `mutation_paths` for translation/rotation/scale with all subfields
  - Has `spawn_format` with example values for translation, rotation, scale
  - Has `schema_info` with properties and required fields
- Sprite:
  - Does NOT have `mutation_paths` field
  - Does NOT have `spawn_format` field
  - Has `schema_info` with properties and required fields
- Name has appropriate fields for a wrapper type
- Each type has correct `type_kind` in its `schema_info`

### 7. Final Cleanup
- Execute `mcp__brp__brp_shutdown` to stop the test app
- Verify clean shutdown with status "clean_shutdown" (test_extras_plugin_app has BrpExtrasPlugin)

## Success Criteria

✅ Test passes when:
- Type schema discovery returns successfully
- All expected fields are present in the response
- Mutation paths match expected structure
- Enum variants are correctly populated
- Option types show both Some and None examples
- Multi-type discovery handles different component types correctly
- Tool provides comprehensive type information for BRP operations

## Failure Investigation

If test fails:
1. Check if app is running with BRP enabled
2. Verify type exists in registry
3. Compare actual vs expected mutation paths
4. Check if enum variants are being discovered
5. Verify Option type handling is correct

## Mutation Context Types Validation

### Objective
Validate that all MutationContext types are properly generated for different field kinds using test components from extras_plugin example

### Prerequisites
- Build and launch extras_plugin example with test components
- The example includes TestArrayField, TestTupleField, TestTupleStruct, and TestComplexComponent

### Test Components

#### 1. Array Field Component - TestArrayField
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["TestArrayField"],
  "port": 20114
}
```

Validate `mutation_paths` contains:
- `.vertices` - entire array field with StructField context
- `.vertices[0]`, `.vertices[1]`, `.vertices[2]` - array elements with ArrayElement context
- `.values` - entire array field
- `.values[0]`, `.values[1]`, `.values[2]`, `.values[3]` - array elements
- Descriptions should indicate "element [N]" for array elements

#### 2. Tuple Field Component - TestTupleField  
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["TestTupleField"],
  "port": 20114
}
```

Validate `mutation_paths` contains:
- `.coords` - entire tuple field with StructField context
- `.coords.0`, `.coords.1` - tuple elements with TupleElement context
- `.color_rgb` - entire tuple field
- `.color_rgb.0`, `.color_rgb.1`, `.color_rgb.2` - tuple elements
- Descriptions should indicate "element N" for tuple elements

#### 3. TupleStruct Component - TestTupleStruct
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["TestTupleStruct"],
  "port": 20114
}
```

Validate:
- Component itself is a TupleStruct
- `mutation_paths` contains:
  - Root path `""` with context RootValue
  - `.0` - first element (f32) with TupleElement context
  - `.1` - second element (String) with TupleElement context  
  - `.2` - third element (bool) with TupleElement context

#### 4. Complex Component - TestComplexComponent
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["TestComplexComponent"],
  "port": 20114
}
```

Validate `mutation_paths` contains:
- **StructField context**:
  - `.transform` - Transform field
  - `.mode` - Enum field with enum_variants
  - `.points` - Array field
  - `.range` - Tuple field
  - `.optional_value` - Option field
- **NestedPath context** (from transform field):
  - `.transform.translation.x`, `.transform.translation.y`, `.transform.translation.z`
  - `.transform.rotation.x`, `.transform.rotation.y`, `.transform.rotation.z`, `.transform.rotation.w`
  - `.transform.scale.x`, `.transform.scale.y`, `.transform.scale.z`
- **ArrayElement context**:
  - `.points[0]`, `.points[1]` - array elements
- **TupleElement context**:
  - `.range.0`, `.range.1` - tuple elements

#### 5. Standard Bevy Components

Also validate with standard Bevy components:

**MipBias (TupleStruct)**:
```json
{
  "types": ["bevy_render::camera::camera::MipBias"],
  "port": 20114
}
```
- Root path with RootValue context
- `.0` with TupleElement context

**Transform (Nested fields)**:
```json
{
  "types": ["bevy_transform::components::transform::Transform"],
  "port": 20114
}
```
- StructField context for `.translation`, `.rotation`, `.scale`
- NestedPath context for `.translation.x`, etc.

### Success Criteria for Mutation Contexts

✅ All MutationContext variants are properly used:
- **RootValue**: For replacing entire values (TupleStruct root)
- **StructField**: For struct fields (including array/tuple fields as wholes)
- **TupleElement**: For tuple and tuplestruct elements with `.N` syntax
- **ArrayElement**: For array elements with `[N]` syntax
- **NestedPath**: For complex paths like `.transform.rotation.x`

✅ Descriptions are context-aware and accurate:
- RootValue: "Replace the entire {type} value"
- StructField: "Mutate the {field} field of {parent_type}"
- TupleElement: "Mutate element {index} of {parent_type}"
- ArrayElement: "Mutate element [{index}] of {parent_type}"
- NestedPath: Describes the full path with final type

### Mutation Validation

After verifying the mutation paths are generated correctly, **you must perform actual mutations** following the guidance provided in each mutation path's description:

#### Test Protocol
1. Spawn test entities with each test component
2. For each component type that was validated above:
   - Use `bevy_mutate_component` to mutate **at least one path of each context type** found in the mutation_paths
   - Follow the description text as guidance for what you're mutating
   - Use the provided example values as reference (but vary them to show actual changes)
   - Verify the mutation succeeded with `bevy_get`

#### Required Mutation Tests

**IMPORTANT**: After EACH mutation, use `bevy_get` to verify the change took effect.

For **TestArrayField**:
- Mutate a StructField: `.vertices` (entire array) → Verify with `bevy_get`
- Mutate an ArrayElement: `.values[1]` (single element) → Verify with `bevy_get`

For **TestTupleField**:
- Mutate a StructField: `.coords` (entire tuple) → Verify with `bevy_get`
- Mutate a TupleElement: `.color_rgb.2` (single element) → Verify with `bevy_get`

For **TestTupleStruct**:
- Mutate RootValue: replace entire tuple struct → Verify with `bevy_get`
- Mutate a TupleElement: `.1` (string element) → Verify with `bevy_get`

For **TestComplexComponent**:
- Mutate a StructField with enum: `.mode` → Verify with `bevy_get`
- Mutate an ArrayElement: `.points[0]` → Verify with `bevy_get`
- Mutate a TupleElement: `.range.1` → Verify with `bevy_get`
- Mutate a NestedPath: `.transform.translation.y` → Verify with `bevy_get`
- Mutate an Option field: `.optional_value` to None (null) → Verify with `bevy_get`

For **Transform** (standard Bevy component):
- Mutate a NestedPath: `.scale.x` → Verify with `bevy_get`

For **Sprite** (Bevy component without Serialize):
- Verify component has mutation_paths despite lacking Serialize trait
- Mutate a StructField: `.color` → Verify with `bevy_get`
- Mutate a simple field: `.flip_x` to true → Verify with `bevy_get`
- This validates that components without Serialize can still be mutated (just not spawned/inserted)

This ensures the mutation paths not only exist but are functional and the descriptions provide accurate guidance.
