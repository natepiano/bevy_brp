# Registry Discovery Tests

## Objective
Validate BRP behavior with components that lack proper registry traits (Serialize/Deserialize).

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
- Verify mutation succeeds despite spawn/insert limitations
- Test mutation on Aabb component if entity has one
- Confirm mutation works for registered components

### 4. Registry Requirements Validation
- Execute `mcp__brp__bevy_list` to see registered components
- Verify only properly registered components appear
- Check that Transform, Name appear (have proper traits)
- Confirm Visibility, Aabb don't appear in list (missing traits)

### 5. Error Message Quality Check
- Verify all registry errors include:
  - Clear problem description
  - Specific missing traits (Serialize, Deserialize)
  - Guidance on BRP registration requirements
  - Helpful suggestions for resolution

## Expected Results
- ✅ Spawn fails appropriately for unregistered components
- ✅ Insert fails appropriately for unregistered components  
- ✅ Mutation works even for components with missing traits
- ✅ Component listing shows only properly registered types
- ✅ Error messages are clear and actionable
- ✅ Registration requirements are well explained

## Failure Criteria
STOP if: Registry errors are unclear, mutation fails for registered components, or error guidance is insufficient.