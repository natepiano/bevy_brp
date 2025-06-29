# Registry Discovery Tests

## Objective
Validate BRP behavior with components that lack Serialize/Deserialize traits but are still reflection-registered.

## Test Steps

### 1. Component Without Serialize/Deserialize - Spawn Test
- Execute `mcp__brp__bevy_spawn` with Visibility component
- Verify spawn fails with registry diagnostic
- Check error mentions "lacks Serialize and Deserialize traits"
- Confirm error includes BRP registration requirements guidance

### 2. Component Without Serialize/Deserialize - Insert Test  
- Spawn entity with basic Transform
- Execute `mcp__brp__bevy_insert` with Aabb component
- Verify insert fails with appropriate registry error
- Check error message is helpful and actionable

### 3. Mutation Should Work (Even Without Serialize/Deserialize)
- Execute `mcp__brp__bevy_mutate_component` on Visibility component
  - Entity: Use an entity with Visibility component
  - Component: "bevy_render::view::visibility::Visibility"
  - Path: "" (empty string)
  - Value: "Visible"
- Verify mutation succeeds
- Test other variants with empty path: "Hidden", "Inherited"

### 4. Registry Requirements Validation
- Execute `mcp__brp__bevy_list` to see registered components
- Verify all reflection-registered components appear
- Check that Transform, Name appear (have Serialize/Deserialize traits)
- Confirm Visibility, Aabb appear in list (registered but missing Serialize/Deserialize traits)

### 5. Error Message Quality Check
- Verify all registry errors include:
  - Clear problem description
  - Specific missing traits (Serialize, Deserialize)
  - Guidance on BRP registration requirements
  - Helpful suggestions for resolution

### 6. Enum Mutation Error Message Test
- Execute `mcp__brp__bevy_mutate_component` with INCORRECT syntax:
  - Path: ".Visible"
  - Value: {}
- Verify error response includes:
  - Error message mentioning variant access issue
  - Format correction with:
    - "usage" field explaining empty path requirement
    - "valid_values" array listing all variants
    - "examples" showing correct usage
  - Hint text clearly stating: "Enum 'Visibility' requires empty path..."
- This ensures users get helpful guidance when making this common mistake

## Expected Results
- ✅ Spawn fails appropriately for components lacking Serialize/Deserialize
- ✅ Insert fails appropriately for components lacking Serialize/Deserialize  
- ✅ Mutation works for reflection-registered components (even without Serialize/Deserialize)
- ✅ Component listing shows all reflection-registered types (regardless of Serialize/Deserialize)
- ✅ Error messages are clear and actionable
- ✅ Registration requirements are well explained

## Failure Criteria
STOP if: Registry errors are unclear, mutation fails for registered components, or error guidance is insufficient.