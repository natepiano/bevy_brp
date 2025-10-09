# Type Schema Validation Test

## Objective
Validate that `brp_type_guide` tool correctly discovers type information and produces the expected output structure for Bevy components.

## Prerequisites
- Launch extras_plugin example on port 20114
- Tool `brp_type_guide` must be available in the MCP environment

## Test Steps

### 1. Launch Test Application
- Execute `mcp__brp__brp_launch_bevy_example` with `extras_plugin` on port 20114
- Verify app is running with `mcp__brp__brp_status`

### 2. Batch Type Schema Discovery

Execute `mcp__brp__brp_type_guide` with ALL test types in a single call:
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

**CRITICAL**: If response is saved to file due to size, you MUST use the extraction script.
**DO NOT use jq, cat, or any other bash commands directly on the file.**

Extraction script usage:
`.claude/scripts/type_guide_test_extract.sh <file_path> <operation> [type_name] [field_path]`

#### 3a. Check Top-Level Fields
Use extraction script ONLY (no direct jq/bash commands):
```bash
.claude/scripts/type_guide_test_extract.sh <file_path> discovered_count
# Expected: 8

.claude/scripts/type_guide_test_extract.sh <file_path> summary
# Expected: {"successful_discoveries": 8, "failed_discoveries": 0, "total_requested": 8}
```

### 4. Validate Sprite Component (No Serialize Trait)

#### 4a. Validate Type Info
```bash
# Get Sprite type info
.claude/scripts/type_guide_test_extract.sh <file_path> type_info "bevy_sprite::sprite::Sprite"
# Expected: type_name: "bevy_sprite::sprite::Sprite", in_registry: true, has_serialize: false, has_deserialize: false, supported_operations: ["query", "get", "mutate"]
```

#### 4b. Verify Mutation Paths Exist Despite No Serialize
```bash
# Get Sprite mutation paths
.claude/scripts/type_guide_test_extract.sh <file_path> mutation_paths "bevy_sprite::sprite::Sprite"
# Should contain paths for fields like .color, .flip_x, .flip_y, .custom_size, etc.
```

#### 4c. Verify Spawn Format Is Absent
```bash
# Check spawn format (should be null/absent)
.claude/scripts/type_guide_test_extract.sh <file_path> spawn_format "bevy_sprite::sprite::Sprite"
# Expected: null (cannot spawn without Serialize)
```

#### 4d. Validate Schema Info
```bash
# Get Sprite schema info
.claude/scripts/type_guide_test_extract.sh <file_path> schema_info "bevy_sprite::sprite::Sprite"
# Expected: type_kind: "Struct", properties with anchor/color/flip_x/etc, module_path: "bevy_sprite::sprite", crate_name: "bevy_sprite"
```

### 5. Validate Transform Component (Standard Nested)

```bash
# Get Transform mutation paths
.claude/scripts/type_guide_test_extract.sh <file_path> mutation_paths "bevy_transform::components::transform::Transform"
# Should have translation/rotation/scale with all subfields

# Get Transform spawn format
.claude/scripts/type_guide_test_extract.sh <file_path> spawn_format "bevy_transform::components::transform::Transform"
# Should have example values

# Get Transform schema info
.claude/scripts/type_guide_test_extract.sh <file_path> schema_info "bevy_transform::components::transform::Transform"
# Should have properties and required fields

# Validate specific paths exist
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "bevy_transform::components::transform::Transform" ".translation"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "bevy_transform::components::transform::Transform" ".translation.x"
```

### 6. Validate Test Components with Mutation Contexts

#### 6a. TestArrayField - Array Element Access
```bash
# Check array field paths
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestArrayField" ".vertices"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestArrayField" ".vertices[0]"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestArrayField" ".values[0]"
```

#### 6b. TestTupleField - Tuple Element Access
```bash
# Check tuple field paths
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleField" ".coords"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleField" ".coords.0"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleField" ".color_rgb.2"
```

#### 6c. TestTupleStruct - Root Value Access
```bash
# Check tuple struct paths
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleStruct" ""
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleStruct" ".0"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleStruct" ".1"
```

#### 6d. TestComplexComponent - Nested Paths
```bash
# Check complex nested paths
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestComplexComponent" ".transform.translation.y"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestComplexComponent" ".points[0]"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestComplexComponent" ".range.0"
```

### 7. Validate Name Component
Verify `result."type_guide"["bevy_ecs::name::Name"]`:
- Has appropriate fields for a wrapper type
- Has both `mutation_paths` and `spawn_format` (has Serialize/Deserialize)

### 8. Functional Mutation Testing

Spawn test entities and perform mutations to verify the discovered paths work:

#### 8a. Spawn Test Entities
Spawn entities with components that have Serialize trait:
- TestArrayField, TestTupleField, TestTupleStruct, TestComplexComponent, Transform, Name

#### 8b. Execute and Verify Mutations
For each mutation context type, perform ONE representative mutation and verify:

**ArrayElement**: Mutate `.values[1]` on TestArrayField (even though only `.values[0]` is in type_guide) → Verify with `world_get_components`

**TupleElement**: Mutate `.color_rgb.2` on TestTupleField → Verify with `world_get_components`

**RootValue**: Replace entire TestTupleStruct using path "" → Verify with `world_get_components`

**NestedPath**: Mutate `.transform.translation.y` on TestComplexComponent → Verify with `world_get_components`

**StructField with enum**: Mutate `.mode` to "Inactive" on TestComplexComponent → Verify with `world_get_components`

**Option field**: Mutate `.optional_value` to None on TestComplexComponent → Verify with `world_get_components`

**Non-Serializable**: Mutate `.flip_x` to true on Sprite → Verify with `world_get_components`

### 9. Type Schema in Error Responses

Test that format errors include embedded type_guide information for self-correction.

#### 9a. Test Format Error with Type Schema Embedding

**STEP 1**: Query for an entity with Transform:
- Tool: mcp__brp__world_query
- Use filter: {"with": ["bevy_transform::components::transform::Transform"]}

**STEP 2**: Attempt insert with INCORRECT object format:
```json
`mcp__brp__world_insert_components` with parameters:
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
- Message contains: "Format error - see 'type_guide' field for correct format"
- error_info contains:
  - original_error: The BRP error message
  - type_guide: Embedded type_guide for Transform with correct array format

**STEP 3**: Verify type_guide in error contains:
- Transform spawn_format showing correct array format for Vec3 fields
- Transform mutation_paths for reference
- Same structure as direct brp_type_guide response

#### 9b. Test Multiple Type Errors

**STEP 1**: Attempt spawn with multiple incorrect formats:
```json
mcp__brp__world_spawn_entity with parameters:
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
- Error with type_guide for both Transform and Name
- Each type shows correct format in type_guide field

#### 9c. Test Mutation Format Error with Type Schema Embedding

**STEP 1**: Query for an entity with Transform:
- Tool: mcp__brp__world_query
- Use filter: {"with": ["bevy_transform::components::transform::Transform"]}

**STEP 2**: Attempt mutation with INCORRECT object format (should be array):
```json
mcp__brp__world_mutate_components with parameters:
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
- Message contains: "Format error - see 'type_guide' field for correct format"
- error_info contains:
  - original_error: The BRP mutation error message
  - type_guide: Embedded type_guide for Transform showing correct array format
  - Should show `.translation` expects `[f32, f32, f32]` not `{x, y, z}` object

**STEP 3**: Verify mutation-specific type_guide contains:
- Transform mutation_paths including `.translation` with correct array format
- Transform spawn_format showing proper Vec3 array structure
- Clear guidance that Vec3 fields require `[x, y, z]` array format

#### 9d. Test Non-Transformable Input Error

**STEP 1**: Test completely malformed input:
```json
`mcp__brp__world_insert_components` with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "components": {
    "bevy_transform::components::transform::Transform": 123
  },
  "port": 20114
}
```

**Expected Result**:
- Error response with type_guide guidance
- Clear indication that format cannot be corrected automatically
- type_guide shows expected Transform structure

#### 9e. Test Component Without Serialize/Deserialize - Spawn Failure

**STEP 1**: Attempt to spawn Visibility (lacks Serialize/Deserialize):
```json
mcp__brp__world_spawn_entity with parameters:
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
- May include type_guide showing component is in registry but not spawnable

#### 9f. Test Enum Mutation Error Guidance

**STEP 1**: Query for entity with Visibility:
- Tool: mcp__brp__world_query
- Filter: {"with": ["bevy_render::view::visibility::Visibility"]}

**STEP 2**: Attempt mutation with INCORRECT enum syntax:
```json
mcp__brp__world_mutate_components with parameters:
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
- **Format errors include embedded type_guide for failed types**
- **Type extraction works from both parameters and error messages**
- **Mutation format errors include embedded type_guide information**
- **Error guidance is clear and actionable for self-correction**

## Failure Investigation

If test fails:
1. Check if app is running with BRP enabled
2. Verify types exist in registry
3. Compare actual vs expected mutation paths
4. Check if mutations are succeeding
5. **Verify error responses include type_guide field**
6. **Check type extraction logic in error handling**
