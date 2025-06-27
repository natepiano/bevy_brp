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
  - Use simple array: `[1.0, 2.0, 3.0]` instead of full structure
- Enable debug mode to see correction process
- Verify spawn succeeds with format correction
- Check debug info shows "Tier 2: Direct Discovery" success

### 3. Test ClearColor Discovery  
- Execute format discovery for `bevy_render::color::Color`
- Test spawn with wrong LinearRgba format: `[0.8, 0.2, 0.1, 1.0]`
- Verify auto-correction to proper object format
- Confirm entity spawns successfully

### 4. Mutation Path Discovery
- Execute format discovery for Transform
- Verify mutation_info includes available paths like `.translation.x`
- Test mutation using discovered path
- Confirm mutation succeeds

### 5. Multiple Type Discovery
- Execute format discovery with multiple types array
- Verify response includes format for all requested types
- Check response structure is organized by type

## Expected Results
- ✅ Direct format discovery returns detailed spawn formats
- ✅ Wrong formats are auto-corrected during spawn
- ✅ Debug info shows "Tier 2: Direct Discovery" success
- ✅ Mutation paths are properly discovered
- ✅ Multi-type discovery works correctly
- ✅ Format corrections include helpful hints

## Failure Criteria
STOP if: Format discovery fails, auto-correction doesn't work, or debug info doesn't show Tier 2 success.