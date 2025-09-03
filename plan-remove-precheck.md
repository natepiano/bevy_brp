# Plan: Remove Redundant Mutation Precheck

## Problem Statement

The current implementation performs redundant recursion:
1. **First pass**: `type_supports_mutation_precheck()` recursively traverses the type tree to check mutability
2. **Second pass**: Path building traverses the same tree again to build mutation paths

This is inefficient and unnecessarily complex. We're essentially asking "can we mutate this?" then immediately asking "how do we mutate this?" - when we could just try to build mutation paths and handle failures as they occur.

## Current Flow (Redundant)

```
Container Type → Precheck (recurse to inner) → Check Value type traits → Return bool
     ↓
Path Building → Recurse to inner again → Build paths or NotMutatable
```

## Proposed Flow (Efficient)

```
Container Type → Try building paths → Recurse once → Value type checks traits inline → Return paths or NotMutatable
```

## Implementation Strategy

### Phase 1: Remove Precheck Infrastructure

**File: `mutation_path_builders.rs`**

1. **Delete these methods entirely**:
   - `type_supports_mutation()` (lines 239-241)
   - `type_supports_mutation_with_depth()` (lines 244-339)
   
   These are the redundant precheck methods that duplicate work.

2. **Keep these methods** (they're still needed for detailed error messages):
   - `type_supports_mutation_detailed()` (lines 342-345)
   - `type_supports_mutation_with_depth_detailed()` (lines 347-447)

### Phase 2: Remove Precheck Calls from Path Builders

**File: `mutation_path_builders.rs` - `TypeKind::build_paths`** (lines 471-489)

Remove the entire precheck block:
```rust
// DELETE THIS ENTIRE BLOCK
match self {
    Self::List | Self::Array | Self::Map | Self::Option => {
        if !ctx.type_supports_mutation(ctx.type_name()) {
            return Ok(vec![Self::build_not_mutatable_path(ctx)]);
        }
    }
    Self::Value => {
        if !ctx.value_type_has_serialization(ctx.type_name()) {
            return Ok(vec![Self::build_not_mutatable_path(ctx)]);
        }
    }
    _ => {}
}
```

### Phase 3: Update Individual Builders to Check Inline

#### 3.1 Value Type Builder
Create a new `ValueMutationBuilder` to handle Value types properly:

```rust
pub struct ValueMutationBuilder;

impl MutationPathBuilder for ValueMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        // Check serialization traits right here, inline
        if !ctx.value_type_has_serialization(ctx.type_name()) {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        }
        
        // Has traits, build normal path
        DefaultMutationBuilder.build_paths(ctx)
    }
}
```

Update `TypeKind::build_paths` dispatch:
```rust
Self::Value => ValueMutationBuilder.build_paths(ctx),
```

#### 3.2 Container Builders (List, Array, Map, Option)
These builders should:
1. Try to determine their inner type
2. Create a context for the inner type
3. Recursively call path building on inner type
4. If inner type returns NotMutatable paths, propagate that up

Example for List/Array:
```rust
impl MutationPathBuilder for ListMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        // Extract element type
        let Some(element_type) = Self::extract_list_element_type(schema) else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        // Create context for element and try building its paths
        let elem_ctx = MutationPathContext::new(
            RootOrField::root(&element_type),
            ctx.registry,
            None,
        );
        
        // Get element type's schema and kind
        let Some(elem_schema) = ctx.get_type_schema(&element_type) else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        let elem_kind = TypeKind::from_schema(elem_schema, &element_type);
        let elem_paths = elem_kind.build_paths(&elem_ctx)?;
        
        // Check if element is mutatable based on returned paths
        let elem_mutatable = elem_paths.iter().any(|p| 
            !matches!(p.path_kind, MutationPathKind::NotMutatable)
        );
        
        if !elem_mutatable {
            // Element can't be mutated, so neither can the list
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        }
        
        // Element is mutatable, build normal list paths
        // ... existing list path building logic
    }
}
```

### Phase 4: Update Tuple Builder

The tuple builder (lines 1212-1225) currently calls `ctx.type_supports_mutation()`. Update it to check mutability inline:

```rust
fn build_tuple_element_path(
    ctx: &MutationPathContext<'_>,
    index: usize,
    element_info: &Value,
    path_prefix: &str,
    parent_type: &BrpTypeName,
) -> Option<MutationPathInternal> {
    let element_type = SchemaField::extract_field_type(element_info)?;
    
    // Instead of calling type_supports_mutation, build path and check result
    let elem_ctx = MutationPathContext::new(
        RootOrField::root(&element_type),
        ctx.registry,
        None,
    );
    
    let Some(elem_schema) = ctx.get_type_schema(&element_type) else {
        // Element type not in registry - not mutatable
        return Some(MutationPathInternal {
            path: /* ... */,
            example: json!({"NotMutatable": /* ... */}),
            path_kind: MutationPathKind::NotMutatable,
            /* ... */
        });
    };
    
    // Check element type kind
    let elem_kind = TypeKind::from_schema(elem_schema, &element_type);
    
    // For Value types, check serialization inline
    if matches!(elem_kind, TypeKind::Value) && !ctx.value_type_has_serialization(&element_type) {
        let detailed_support = ctx.type_supports_mutation_detailed(&element_type);
        return Some(MutationPathInternal {
            /* ... NotMutatable path ... */
        });
    }
    
    // Element seems mutatable, build normal path
    /* ... existing path building ... */
}
```

## Benefits of This Approach

1. **Eliminates Redundant Recursion**: We traverse the type tree only once
2. **Simpler Logic**: Mutability checking happens inline where it's needed
3. **Better Performance**: Roughly 50% reduction in recursive calls for nested types
4. **Clearer Code Flow**: Each builder is responsible for its own mutability checking
5. **Same Results**: The output remains identical - we just get there more efficiently

## Migration Strategy

1. **Keep `type_supports_mutation_detailed`**: Still needed for error messages
2. **Gradual Migration**: Can update builders one at a time
3. **Test Coverage**: Existing tests should pass without modification
4. **Fallback Safety**: If a builder doesn't check, it returns paths that may later be marked NotMutatable

## Success Criteria

1. All existing tests pass
2. No change in JSON output for any type
3. Reduced code complexity (fewer lines)
4. Performance improvement measurable for deeply nested types
5. Clearer separation of concerns in builders

## Risk Mitigation

- **Incremental approach**: Update one builder at a time
- **Preserve detailed methods**: Keep error message generation intact
- **Extensive testing**: Run against all Bevy component types
- **Fallback behavior**: Unchecked types still get NotMutatable marking during propagation

## Example: Transform Component

**Current flow**:
1. Precheck Transform → recurse to Vec3 fields → check each Vec3 has Serialize → return true
2. Build paths → recurse to Vec3 fields again → build paths

**New flow**:
1. Build Transform paths → encounter Vec3 field → Vec3 builder checks Serialize inline → returns paths

This eliminates the duplicate traversal of Transform's three Vec3 fields.