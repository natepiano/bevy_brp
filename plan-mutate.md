# Plan: Fix Mutation Support by Building from Paths

## Problem

The current BRP type schema system has inconsistent logic for determining mutation support:

1. **get_supported_operations()** assumes Components/Resources can mutate and adds `Mutate` upfront
2. **type_supports_mutation()** tries to validate this assumption but uses different logic
3. **Value types** (like `String`) with serialization support are incorrectly excluded from mutation
4. **Tuple structs** (like `Text`) with serializable fields get marked as non-mutatable

This causes types like `bevy_ui::widget::text::Text` to be incorrectly auto-passed when they should be testable.

## Solution Architecture

Instead of predicting mutation support, **build mutation paths first** and derive supported operations from actual results:

```
1. Build mutation paths (recursive, handles all nested types)
2. Analyze built paths → count mutatable vs NotMutatable  
3. If any mutatable paths exist → earn "Mutate" operation
4. Update supported_operations based on proof-of-work
```

## Implementation Plan

### Step 1: Remove Upfront Mutate Assignment

**File**: `mcp/src/brp_tools/brp_type_schema/type_info.rs`  
**Method**: `get_supported_operations()`

```diff
- if has_component {
-     operations.push(BrpSupportedOperation::Mutate);  // Remove assumption
-     // ...
- }
- if has_resource {
-     operations.push(BrpSupportedOperation::Mutate);  // Remove assumption  
-     // ...
- }
```

Start with minimal operations only:
- Always: `Query`
- Components: `Get` 
- Components with serialization: `Spawn`, `Insert`
- Resources with serialization: `Insert`

### Step 2: Add Post-Build Mutation Analysis

**File**: `mcp/src/brp_tools/brp_type_schema/type_info.rs`  
**Method**: `from_registry_schema()`

After mutation paths are built, add:

```rust
// After building mutation_paths, check if any are actually mutatable
let has_mutatable_paths = mutation_paths.values().any(|path| {
    !matches!(path.path_kind, MutationPathKind::NotMutatable)
});

// Earn mutation support based on actual capability
if has_mutatable_paths {
    supported_operations.push(BrpSupportedOperation::Mutate);
}
```

### Step 3: Remove type_supports_mutation Logic

**File**: `mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`  
**Method**: `type_supports_mutation()`

Current hardcoded logic:
```rust
TypeKind::Value => {
    // Complex serialization checking logic
}
```

Replace with path-building approach:
- Remove `type_supports_mutation()` method entirely
- Let mutation path builders naturally determine mutability through schema inspection
- Remove circular dependency between path building and mutation support

### Step 4: Update Value Type Handling

**File**: `mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`

For Value types, determine mutability through **actual reflection support**:

```rust
TypeKind::Value => {
    // Build mutation paths - they will be NotMutatable if type lacks reflection
    // String with Serialize/Deserialize will get mutatable paths
    // RenderTarget without serialization will get NotMutatable paths
    // Let the schema determine the outcome
}
```

## Expected Results

### Before Fix:
- **String**: `supported_operations: ["query"]` (wrong)
- **Text**: Auto-passed as non-testable (wrong)  
- **RenderTarget**: `supported_operations: ["query"]` (correct)

### After Fix:
- **String**: `supported_operations: ["query", "mutate"]` (correct - has serialization)
- **Text**: Testable with mutation paths `["", ".0"]` (correct - String field is mutatable)
- **RenderTarget**: `supported_operations: ["query"]` (correct - no serialization)

## Testing Plan

1. **String Value Type**:
   - Should get `mutate` in supported_operations (has Serialize/Deserialize)
   - Should have mutatable paths

2. **Text Tuple Struct**:
   - Should NOT be auto-passed  
   - Should have mutation paths: `["", ".0"]`
   - `.0` path should be mutatable (points to String)

3. **RenderTarget Enum**:
   - Should NOT get `mutate` in supported_operations (no serialization)
   - Should have NotMutatable paths only

4. **Regression Testing**:
   - Components with mutation should still work
   - Resources with mutation should still work
   - Complex nested structures should still work

## Benefits

1. **Single Source of Truth**: Mutation paths determine supported operations
2. **Eliminates Duplication**: No parallel logic in multiple places
3. **Self-Consistent**: What we advertise matches what actually works
4. **Extensible**: New type patterns automatically work if paths can be built
5. **Debuggable**: Clear path from schema → paths → operations