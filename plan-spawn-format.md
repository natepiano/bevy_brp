# Plan: Unified Spawn Format Generation

## Overview
Currently, spawn format generation and mutation path generation use separate code paths despite producing essentially the same output for root-level values. This plan outlines how to unify these systems by reusing the mutation path infrastructure for spawn format generation.

## Current State

### Problems
1. **Duplicate Logic**: `build_spawn_format()` and `build_mutation_paths()` have separate implementations for building type examples
2. **Missing Enum Support**: Enums don't get spawn formats even when they're spawnable components
3. **Inconsistent Examples**: Spawn format examples may differ from root mutation path examples for the same type
4. **Code Duplication**: `build_struct_spawn_format()`, `build_tuple_spawn_format()` duplicate logic that exists in mutation builders

### Current Code Structure
- `TypeInfo::build_spawn_format()` - Orchestrator that dispatches to specific builders
- `TypeInfo::build_struct_spawn_format()` - Builds spawn format for structs
- `TypeInfo::build_tuple_spawn_format()` - Builds spawn format for tuples/tuple structs
- No enum spawn format builder exists
- Mutation path builders generate field/element paths but not root replacement paths for structs

## Proposed Solution

### Key Insight
The root-level mutation path (empty string path `""`) for a type should contain the exact same example that would be used as the spawn format. Instead of maintaining separate code paths, we can:
1. Generate complete mutation paths including root replacement
2. Extract the spawn format from the root mutation path

### Implementation Strategy

#### Phase 1: Add Root Mutation Paths
Modify mutation path builders to always generate a root replacement path:
- `StructMutationBuilder`: Add root path with complete struct example
- `TupleMutationBuilder`: Already generates root path
- `EnumMutationBuilder`: Already generates root path
- Other builders: Verify they generate appropriate root paths

#### Phase 2: Extract Spawn Format from Mutation Paths
Replace `build_spawn_format()` logic:
```rust
// Pseudocode - actual implementation depends on refactoring
let mutation_paths = build_mutation_paths(...);
let spawn_format = mutation_paths
    .iter()
    .find(|p| p.path.is_empty())
    .map(|p| p.example.clone());
```

#### Phase 3: Remove Duplicate Code
Delete obsolete methods:
- `TypeInfo::build_spawn_format()`
- `TypeInfo::build_struct_spawn_format()` 
- `TypeInfo::build_tuple_spawn_format()`
- Related helper methods that are no longer needed

#### Phase 4: Ensure Consistency
- Verify that root mutation examples match expected spawn formats
- Add tests to ensure spawn format extraction works correctly
- Validate that enum components now get spawn formats

## Benefits

1. **Single Source of Truth**: One code path for generating type examples
2. **Automatic Enum Support**: Enums get spawn formats "for free" from their root mutation paths
3. **Consistency**: Spawn format always matches root mutation example
4. **Less Code**: Eliminates duplicate builder methods
5. **Easier Maintenance**: Changes to example generation affect both spawn and mutation uniformly

## Dependencies

This plan depends on completion of:
- Current mutation path builder refactoring (plan-wrapper-removal.md)
- Type system cleanup that's in progress

## Testing Strategy

1. **Compatibility Tests**: Ensure spawn formats remain backward compatible
2. **Enum Tests**: Verify enum components get proper spawn formats
3. **Example Consistency**: Test that root mutation path examples match spawn formats
4. **Coverage**: Test all type kinds (struct, tuple, enum, array, etc.)

### A/B Comparison Testing

Since this change affects the MCP tool's behavior, we need to verify compatibility through real-world testing:

1. **Two-Session Comparison**:
   - Session A: Install and run current MCP tool version (before changes)
   - Session B: Install and run new MCP tool version (after changes)
   - Run identical agentic tests in both sessions
   - Compare outputs to ensure:
     - All existing spawn formats remain identical
     - New enum spawn formats appear only in Session B
     - No regressions in other type kinds

2. **Test Coverage Requirements**:
   - **Struct spawn formats**: Verify complex structs like `Transform`, `Sprite`
   - **Tuple struct spawn formats**: Test single-field (unwrapped) and multi-field variants
   - **Tuple spawn formats**: Ensure proper array generation
   - **Enum spawn formats**: NEW - verify all variant types (unit, tuple, struct)
   - **Nested types**: Test spawn formats with nested structs, Options, arrays

3. **Agentic Test Execution**:
   ```bash
   # Session A (baseline)
   cargo install --path mcp  # Install current version
   # Exit and restart Claude to pick up current tool
   # Run: .claude/commands/test.md with spawn format tests
   
   # Session B (new implementation)  
   cargo install --path mcp  # Install new version
   # Exit and restart Claude to pick up new tool
   # Run: .claude/commands/test.md with spawn format tests
   
   # Compare outputs to verify backward compatibility
   ```

4. **Specific Test Cases**:
   - `extras_plugin::TestEnumWithSerDe` - Should gain spawn format in new version
   - `bevy_transform::components::transform::Transform` - Should remain identical
   - `extras_plugin::TestTupleStruct` - Should remain unwrapped for single field
   - `extras_plugin::TestComplexComponent` - Should handle nested types correctly

## Migration Path

1. Implement alongside existing code initially
2. Add feature flag or configuration to switch between old/new implementations
3. Migrate tests incrementally
4. Remove old implementation once new one is proven stable

## Open Questions

1. Should we preserve any spawn-format-specific logic for special cases?
2. How do we handle types that support spawn but not mutation (if any exist)?
3. Should the root mutation path be generated for types that can't be spawned as components?

## Success Criteria

- [ ] All existing spawn formats continue to work
- [ ] Enum components get spawn formats
- [ ] Code duplication is eliminated
- [ ] Root mutation path examples match spawn formats exactly
- [ ] Test coverage for all type kinds