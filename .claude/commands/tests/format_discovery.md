# Format Discovery Tests (With Plugin)

## Objective
Validate Tier 2 direct format discovery capabilities when bevy_brp_extras plugin is available.

## Test Steps

### 1. Direct Format Discovery
- Execute `mcp__brp__brp_extras_discover_format` with types:
  - `["bevy_transform::components::transform::Transform"]`
- Verify response includes spawn_format and mutation_info
- Check format shows proper structure for Transform

### 2. Test Spawn with Wrong Format (Should Auto-Correct)
- Execute `mcp__brp__bevy_spawn` with intentionally wrong Transform format:
  - Use object fields instead of arrays: `{"translation": {"x": 1.0, "y": 2.0, "z": 3.0}, "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0}, "scale": {"x": 1.0, "y": 1.0, "z": 1.0}}`
- Enable debug mode to see correction process
- Verify spawn succeeds with format correction to arrays
- Check debug info shows "Tier 2: Direct Discovery" success

### 3. Test Color Type Discovery (Verify Registration)
- Execute format discovery for `bevy_color::color::Color` (correct type path)
- Verify it IS registered and returns format info showing enum variants
- Execute format discovery for `bevy_render::camera::clear_color::ClearColor`
- Verify it IS registered but lacks Serialize/Deserialize traits
- Test spawn attempt with ClearColor to confirm it fails with trait error
- Verify error message indicates missing Serialize/Deserialize traits (not "type not registered")

### 4. Mutation Path Discovery
- Execute format discovery for Transform
- Verify mutation_info includes available paths like `.translation.x`
- Test mutation using discovered path
- Confirm mutation succeeds

### 5. Test Graceful Failure for Ambiguous Formats
- Execute `mcp__brp__bevy_spawn` with truly ambiguous Transform format:
  - Use bare array: `[1.0, 2.0, 3.0]` (unclear which field this represents)
- Verify spawn fails gracefully with helpful error message
- Check that error suggests using format discovery
- Confirm debug info shows attempted discovery but explains why correction failed

### 6. Multiple Type Discovery
- Execute format discovery with multiple types array
- Verify response includes format for all requested types
- Check response structure is organized by type

## Expected Results
- ✅ Direct format discovery returns detailed spawn formats
- ✅ Unambiguous wrong formats are auto-corrected during spawn (array fields → object fields)
- ✅ Ambiguous formats fail gracefully with helpful error messages
- ✅ Debug info shows "Tier 2: Direct Discovery" success for correctable cases
- ✅ Debug info explains why ambiguous formats cannot be auto-corrected
- ✅ Mutation paths are properly discovered
- ✅ Multi-type discovery works correctly
- ✅ Format corrections include helpful hints

## Failure Criteria
STOP if: Format discovery fails, correctable formats don't auto-correct, ambiguous formats don't fail gracefully, or debug info doesn't show appropriate Tier 2 behavior.