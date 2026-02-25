# Type Schema Validation Test

## Objective
Validate that `brp_type_guide` tool correctly discovers type information and produces the expected output structure for Bevy components.

## Prerequisites
- The app is managed externally by the integration test runner
- Use the assigned `{{PORT}}` for all BRP tool calls
- Tool `brp_type_guide` must be available in the MCP environment

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Batch Type Schema Discovery

Execute `mcp__brp__brp_type_guide` with test types in a single call:
```json
{
  "types": [
    "bevy_transform::components::transform::Transform",
    "extras_plugin::TestArrayField",
    "extras_plugin::TestTupleField",
    "extras_plugin::TestTupleStruct"
  ],
  "port": {{PORT}}
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
# Expected: 4

.claude/scripts/type_guide_test_extract.sh <file_path> summary
# Expected: {"successful_discoveries": 4, "failed_discoveries": 0, "total_requested": 4}
```

### 4. Validate Transform Component

Transform serves as the reference component for standard nested struct validation.

```bash
# Get Transform type info
.claude/scripts/type_guide_test_extract.sh <file_path> type_info "bevy_transform::components::transform::Transform"
# Expected: type_name present, in_registry: true
# Check schema_info.reflect_types contains "Component"

# Get Transform mutation paths
.claude/scripts/type_guide_test_extract.sh <file_path> mutation_paths "bevy_transform::components::transform::Transform"
# Should have translation/rotation/scale with all subfields

# Get Transform spawn format
.claude/scripts/type_guide_test_extract.sh <file_path> spawn_example "bevy_transform::components::transform::Transform"
# Should have example values

# Get Transform schema info
.claude/scripts/type_guide_test_extract.sh <file_path> schema_info "bevy_transform::components::transform::Transform"
# Should have properties, required fields, type_kind: "Struct"

# Validate specific nested paths exist
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "bevy_transform::components::transform::Transform" ".translation"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "bevy_transform::components::transform::Transform" ".translation.x"
```

### 5. Validate Unique Mutation Contexts

These test components validate mutation path syntax that differs from standard struct fields.

#### 5a. TestArrayField - Array Element Access
```bash
# Check array field paths
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestArrayField" ".vertices"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestArrayField" ".vertices[0]"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestArrayField" ".values[0]"
```

#### 5b. TestTupleField - Tuple Element Access
```bash
# Check tuple field paths
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleField" ".coords"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleField" ".coords.0"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleField" ".color_rgb.2"
```

#### 5c. TestTupleStruct - Root Value Access
```bash
# Check tuple struct paths
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleStruct" ""
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleStruct" ".0"
.claude/scripts/type_guide_test_extract.sh <file_path> validate_field "extras_plugin::TestTupleStruct" ".1"
```

### 6. Type Schema in Error Responses

Test that format errors include embedded type_guide information for self-correction.

**Note**: This section only tests error cases. Functional validation of spawn/insert/mutate operations
is covered by the dedicated mutation test suite (`/mutation_test`), which comprehensively verifies
that type_guide information produces working BRP operations.

#### 6a. Test Insert Format Error

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
  "port": {{PORT}}
}
```

**Expected Error Response**:
- Status: "error"
- Message contains: "Format error - see 'type_guide' field for correct format"
- error_info contains:
  - original_error: The BRP error message
  - type_guide: Embedded type_guide for Transform with correct array format

**STEP 3**: Verify type_guide in error contains:
- Transform spawn_example showing correct array format for Vec3 fields
- Transform mutation_paths for reference
- Same structure as direct brp_type_guide response

#### 6b. Test Mutation Format Error

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
  "port": {{PORT}}
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
- Transform spawn_example showing proper Vec3 array structure
- Clear guidance that Vec3 fields require `[x, y, z]` array format

#### 6c. Test Enum Mutation Error

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
  "port": {{PORT}}
}
```

**Expected Result**:
- Error with format error message: "Format error - see 'type_guide' field for correct format"
- error_info includes:
  - original_error: The BRP error message about the mutation failure
  - type_guide: Full type_guide for the component including:
    - mutation_paths array with entries for the enum field showing all variants
    - Each variant entry includes examples showing the correct structure
    - path_info with enum-specific metadata (applicable_variants, enum_instructions)

## Success Criteria

✅ Test passes when:
- Single batched discovery call retrieves all 4 types successfully
- Transform has: type_info, mutation_paths, spawn_example, schema_info with reflect_types
- Unique mutation contexts validated: array `[0]`, tuple `.0`, root `""`
- Format errors include embedded type_guide for failed types
- Error guidance is clear and actionable for self-correction

## Failure Investigation

If test fails:
1. Check if app is running with BRP enabled
2. Verify types exist in registry
3. Compare actual vs expected mutation paths
4. Verify error responses include type_guide field
