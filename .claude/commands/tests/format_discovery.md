# Format Discovery Tests (With Plugin)

## Objective
Validate Tier 2 direct format discovery capabilities when bevy_brp_extras plugin is available, including rich response metadata.

## Prerequisites
- Launch extras_plugin example once at the beginning on the specified port
- Keep the app running throughout all test steps
- Shutdown only at the end

## Test Steps

### 1. Direct Format Discovery
- Execute `mcp__brp__brp_extras_discover_format` with types:
  - `["bevy_transform::components::transform::Transform"]`
- Verify response includes:
  - `spawn_format` with proper array-based structure for Transform
  - `mutation_info` with available paths
  - `supported_operations` array (e.g., ["spawn", "insert", "mutate"])
  - `type_category` field (e.g., "Component")
- Check format shows proper array structure: translation/rotation/scale as arrays, not objects

### 2. Test Spawn with Wrong Format (Should Auto-Correct with Rich Metadata)
- Execute `mcp__brp__bevy_spawn` with intentionally wrong Transform format:
  - Use object fields instead of arrays: `{"translation": {"x": 1.0, "y": 2.0, "z": 3.0}, "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0}, "scale": {"x": 1.0, "y": 1.0, "z": 1.0}}`
- Verify spawn succeeds with format correction to arrays
- Verify the correction response includes rich metadata:
  - `supported_operations` field populated
  - `mutation_paths` field with paths like `.translation.x`
  - `type_category` indicating "Component"

### 3. Test Color Type Discovery (Verify Registration)
- Execute format discovery for `bevy_color::color::Color` (correct type path)
- Verify it IS registered and returns:
  - Format info showing enum variants
  - `type_category` field (likely "Enum")
  - `supported_operations` array
- Execute format discovery for `bevy_render::camera::clear_color::ClearColor`
- Verify it IS registered but response includes:
  - `has_serialize: false` and/or `has_deserialize: false`
  - Educational message about missing traits
- Test spawn attempt with ClearColor to confirm it fails with trait error
- Verify error message indicates missing Serialize/Deserialize traits (not "type not registered")

### 4. Mutation Path Discovery
- Execute format discovery for Transform
- Verify response includes:
  - `mutation_paths` array with paths like:
    - `.translation.x`, `.translation.y`, `.translation.z`
    - `.rotation.x`, `.rotation.y`, `.rotation.z`, `.rotation.w`
    - `.scale.x`, `.scale.y`, `.scale.z`
- Test mutation using discovered path
- Confirm mutation succeeds

### 5. Test Educational Responses for Ambiguous Formats
- Execute `mcp__brp__bevy_spawn` with known components but ambiguous/incomplete data:
  - **Test 1 - Transform with bare array**: Use `{"bevy_transform::components::transform::Transform": [1.0, 2.0, 3.0]}`
    - Verify error includes:
      - Error message explaining the type mismatch
      - `format_corrected: "not_attempted"` in error data
      - `hint` field with correct format example
  - **Test 2 - Transform with partial object**: Use `{"bevy_transform::components::transform::Transform": {"x": 1.0, "y": 2.0, "z": 3.0}}`
    - Verify error includes:
      - Error message about missing required fields
      - `format_corrected: "not_attempted"` in error data
      - `hint` field with correct format example
  - **Test 3 - Name with wrong type**: Use `{"bevy_ecs::name::Name": 123}`
    - Verify error provides guidance about Name expecting a string value
    - Verify error includes `format_corrected: "not_attempted"` in error data
- Verify all responses demonstrate that ambiguous formats cannot be auto-corrected
- Confirm each response includes educational guidance via error messages and hints

### 6. Multiple Type Discovery
- Execute format discovery with multiple types array
- Verify response includes format for all requested types
- Check each type includes:
  - `spawn_format` example
  - `mutation_paths` where applicable
  - `supported_operations` array
  - `type_category` classification
- Verify response structure is organized by type

## Expected Results
- ✅ Direct format discovery returns detailed spawn formats with rich metadata
- ✅ Unambiguous wrong formats are auto-corrected with metadata-enriched responses
- ✅ Ambiguous formats receive educational responses with available metadata
- ✅ Educational responses explain limitations clearly
- ✅ Mutation paths are properly discovered and included in responses
- ✅ Multi-type discovery works correctly with full metadata
- ✅ Format corrections include:
  - Helpful hints
  - `supported_operations` field
  - `mutation_paths` field
  - `type_category` field

## Failure Criteria
STOP if:
- Format discovery fails to return rich metadata when bevy_brp_extras is available
- Correctable formats don't auto-correct with enriched responses
- Educational responses don't include available metadata fields
- Rich response fields (supported_operations, mutation_paths, type_category) are missing when they should be present
