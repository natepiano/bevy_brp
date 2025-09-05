# Recursion Depth Investigation

## Current State

### Two Recursive Processes
1. **Path Building** - Creating mutation paths by traversing type hierarchy
2. **Example Building** - Building example values for each path

### Current Implementation
- `RecursionDepth` passed as parameter through all `build_paths()` calls
- `TypeKind::build_paths()` increments depth for container types
- Individual builders pass depth through unchanged
- Example building inconsistently handles depth:
  - `ArrayMutationBuilder`: Resets to 0
  - `StructMutationBuilder`: Increments from current
  - `EnumMutationBuilder`: Double-increments (once in TypeKind, once for example)

## Problems

1. **Easy to misuse** - Depth passed as parameter everywhere
2. **Inconsistent example depth** - Different builders handle differently
3. **Double-counting** - Enums increment twice for same type level
4. **No shared limit** - Path and example recursion tracked separately

## Proposed Solutions

### Option 1: Depth in Context
```rust
pub struct MutationPathContext {
    // ... existing fields
    depth: RecursionDepth,  // Add depth as field
}
```
- Context manages depth automatically
- `create_child_context()` handles increment based on TypeKind

### Option 2: Builder-Owned Depth
```rust
pub trait MutationPathBuilder {
    fn depth(&self) -> RecursionDepth;
    fn child_builder(&self) -> Self;
}
```
- Each builder instance owns its depth
- Child builders created with incremented depth

### Option 3: Depth Guard Wrapper
```rust
pub struct DepthGuard<B: MutationPathBuilder> {
    builder: B,
    depth: RecursionDepth,
}
```
- Wraps builders with depth tracking
- Type system enforces proper depth management

## Key Design Question

Should `TypeKind` implement `MutationPathBuilder`? Current design mixes data (TypeKind enum) with behavior (building paths), making depth management awkward.

## Next Steps

1. Choose approach that best uses type system to prevent errors
2. Ensure path and example building share same depth counter
3. Remove all manual depth incrementing except in one controlled location