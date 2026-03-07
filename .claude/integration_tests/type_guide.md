# Type Guide Smoke Test and Error Response Validation

## Objective
Validate that `brp_type_guide` tool returns valid type information (smoke test) and that format errors include embedded type_guide for self-correction (error response validation).

**Note**: Comprehensive structural validation of type guides (mutation paths, spawn examples, array/tuple/root path syntax) is covered by the dedicated mutation test suite (`/mutation_test`), which discovers all registered types and executes every mutation path with real values. This test focuses on tool availability and error response quality.

## Prerequisites
- The app is managed externally by the integration test runner
- Use the assigned `{{PORT}}` for all BRP tool calls
- Tool `brp_type_guide` must be available in the MCP environment

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Smoke Test - Type Guide Discovery

Execute `mcp__brp__brp_type_guide` for Transform:
```json
{
  "types": ["bevy_transform::components::transform::Transform"],
  "port": {{PORT}}
}
```

Verify the response contains:
- `discovered_count`: 1
- `summary` with `successful_discoveries`: 1, `failed_discoveries`: 0
- Type guide entry for Transform with non-empty `type_info`, `mutation_paths`, `spawn_example`, and `schema_info`

### 3. Type Schema in Error Responses

Test that format errors include embedded type_guide information for self-correction.

**Note**: This section only tests error cases. Functional validation of spawn/insert/mutate operations
is covered by the dedicated mutation test suite (`/mutation_test`), which comprehensively verifies
that type_guide information produces working BRP operations.

#### 3a. Test Insert Format Error

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

#### 3b. Test Mutation Format Error

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

#### 3c. Test Enum Mutation Error

**STEP 1**: Query for entity with Visibility:
- Tool: mcp__brp__world_query
- Filter: {"with": ["bevy_camera::visibility::Visibility"]}

**STEP 2**: Attempt mutation with INCORRECT enum syntax:
```json
mcp__brp__world_mutate_components with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "component": "bevy_camera::visibility::Visibility",
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
- Smoke test retrieves Transform type guide with non-empty type_info, mutation_paths, spawn_example, and schema_info
- Format errors include embedded type_guide for failed types
- Error guidance is clear and actionable for self-correction

## Failure Investigation

If test fails:
1. Check if app is running with BRP enabled
2. Verify Transform type exists in registry
3. Verify error responses include type_guide field
