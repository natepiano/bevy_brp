# Format Discovery Tests (With Plugin)

## Objective
Validate Tier 2 direct format discovery capabilities when bevy_brp_extras plugin is available, including rich response metadata.

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
- Enable debug mode to see correction process
- Verify spawn succeeds with format correction to arrays
- Check debug info shows "Tier 2: Direct Discovery" success
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
- Execute `mcp__brp__bevy_spawn` with ambiguous inputs:
  - Use bare array: `[1.0, 2.0, 3.0]` 
  - Use string: `"hello world"`
- Verify spawn provides educational response with:
  - Clear explanation of why format cannot be auto-corrected
  - Rich metadata if available (supported_operations, type_category)
  - Suggestion to use format discovery tools
- Confirm debug info shows attempted discovery and educational response generation

### 6. Multiple Type Discovery
- Execute format discovery with multiple types array
- Verify response includes format for all requested types
- Check each type includes:
  - `spawn_format` example
  - `mutation_paths` where applicable
  - `supported_operations` array
  - `type_category` classification
- Verify response structure is organized by type

### 7. Test Without bevy_brp_extras (Fallback Behavior)
- When bevy_brp_extras is not available:
  - Verify pattern-based corrections still work
  - Check that responses don't include rich metadata
  - Confirm basic hints are provided without `supported_operations`, `mutation_paths`, etc.

## Expected Results
- ✅ Direct format discovery returns detailed spawn formats with rich metadata
- ✅ Unambiguous wrong formats are auto-corrected with metadata-enriched responses
- ✅ Ambiguous formats receive educational responses with available metadata
- ✅ Debug info shows "Tier 2: Direct Discovery" success for correctable cases
- ✅ Educational responses explain limitations clearly
- ✅ Mutation paths are properly discovered and included in responses
- ✅ Multi-type discovery works correctly with full metadata
- ✅ Format corrections include:
  - Helpful hints
  - `supported_operations` field
  - `mutation_paths` field
  - `type_category` field
- ✅ Graceful fallback when bevy_brp_extras unavailable

## Failure Criteria
STOP if:
- Format discovery fails to return rich metadata when bevy_brp_extras is available
- Correctable formats don't auto-correct with enriched responses
- Educational responses don't include available metadata fields
- Debug info doesn't show appropriate Tier 2 behavior
- Rich response fields (supported_operations, mutation_paths, type_category) are missing when they should be present