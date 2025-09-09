# SetMutationBuilder Migration Plan

## Overview
Migrate SetMutationBuilder to the new protocol enforcer pattern, following the same approach as MapMutationBuilder but adapted for Set's simpler single-element structure.

## Core Concepts

### Why Sets are Terminal Mutation Points
1. **No stable addresses** - Set elements have no indices or keys
2. **Hash-based storage** - Mutating an element changes its hash, potentially breaking set invariants
3. **Unordered collection** - No meaningful way to address individual elements
4. **BRP limitation** - BRP reflection doesn't support set-specific operations

### Key Differences from MapMutationBuilder
| Aspect | MapMutationBuilder | SetMutationBuilder |
|--------|-------------------|-------------------|
| Child types | 2 (key, value) | 1 (element) |
| JSON format | Object `{"key": value}` | Array `[elem1, elem2]` |
| Schema fields | keyType, valueType | items |
| Child names | "key", "value" | "element" |

## Implementation Details

### 1. build_paths() Method
```rust
fn build_paths(&self, ctx: &RecursionContext, _depth: RecursionDepth)
    -> Result<Vec<MutationPathInternal>> {
    tracing::error!(
        "SetMutationBuilder::build_paths() called directly! Type: {}",
        ctx.type_name()
    );
    panic!(
        "SetMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
        ctx.type_name()
    );
}
```

### 2. is_migrated() Method
```rust
fn is_migrated(&self) -> bool {
    true
}
```

### 3. include_child_paths() Method - CRITICAL
```rust
fn include_child_paths(&self) -> bool {
    // Sets DON'T include child paths in the result
    //
    // Why: A HashSet<Transform> should only expose:
    //   Path: ""  ->  [{transform1}, {transform2}]
    //
    // It should NOT expose Transform's internal paths like:
    //   Path: ".rotation"     -> [0,0,0,1]  // Makes no sense for a set!
    //   Path: ".rotation.x"   -> 0.0        // These aren't valid set mutations
    //
    // Sets are terminal mutation points. Elements have no stable
    // addresses (no indices or keys) and cannot be individually mutated.
    // Only the entire set can be replaced. Mutating an element could
    // change its hash, breaking set invariants.
    //
    // The recursion still happens (we need element examples to build the set),
    // but those paths aren't included in the final mutation paths list.
    false
}
```

### 4. collect_children() Method
```rust
fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
    let Some(schema) = ctx.require_schema() else {
        tracing::warn!("No schema found for set type: {}", ctx.type_name());
        return vec![];
    };

    // Extract element type from items field using SchemaField::extract_field_type
    // This follows the same pattern as MapMutationBuilder
    let element_type = schema
        .get_field(SchemaField::Items)
        .and_then(SchemaField::extract_field_type);

    let mut children = vec![];

    if let Some(elem_t) = element_type {
        // Create context for element recursion
        let elem_path_kind = super::super::path_kind::PathKind::new_root_value(elem_t);
        let elem_ctx = ctx.create_field_context(elem_path_kind);
        children.push(("element".to_string(), elem_ctx));
    } else {
        tracing::warn!(
            "Failed to extract element type from schema for type: {}",
            ctx.type_name()
        );
    }

    children
}
```

### 5. assemble_from_children() Method
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<String, Value>,
) -> Value {
    // At this point, children contains COMPLETE examples:
    // - "element": Full example for the element type

    let Some(element_example) = children.get("element") else {
        tracing::warn!(
            "Missing element example for set type {}, using fallback",
            ctx.type_name()
        );
        return json!([]);  // Empty set as fallback
    };

    // Create array with 2 example elements
    // For Sets, these represent unique values to add
    // In real usage, these would be different unique elements
    json!([element_example.clone(), element_example.clone()])
}
```

### 6. Code to Remove
- Delete the entire `build_schema_example()` method if it exists
- Delete `build_set_example_static()` static method (currently at line 120)
- Remove ExampleBuilder import: `use crate::brp_tools::brp_type_guide::example_builder::ExampleBuilder;`

### 7. TypeKind Update
In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/type_kind.rs`, update the build_paths match arm:

```rust
// BEFORE:
Self::Set => SetMutationBuilder.build_paths(ctx, builder_depth),

// AFTER:
Self::Set => self.builder().build_paths(ctx, builder_depth),
```

## Testing Strategy

### Build Validation
1. Run `cargo build` to ensure no compilation errors
2. Run `build-check.sh` for full validation

### Runtime Testing
Test with various Set types:
- `HashSet<String>`
- `HashSet<i32>`
- `BTreeSet<String>`
- `HashSet<CustomType>` (where CustomType has complex structure)

### Expected Behavior
1. Only root path "" should be exposed
2. Example should be a JSON array with element examples
3. No child paths should appear in mutation paths
4. Protocol enforcer should handle knowledge checks

## Comparison with Current Implementation

### Current SetMutationBuilder (before migration)
- Uses `build_paths()` directly
- Calls ExampleBuilder for examples
- Has static helper method
- Doesn't implement protocol methods

### New SetMutationBuilder (after migration)
- `build_paths()` panics (never called due to ProtocolEnforcer)
- Implements protocol methods
- No static methods
- Follows depth-first traversal pattern

## Important Notes

1. **Element Uniqueness**: While we show duplicate elements in the example for simplicity, real sets would have unique elements
2. **No Indexing**: Unlike Lists/Arrays, Sets don't support `[0]` style access paths
3. **Protocol Enforcer**: Handles depth limits and knowledge checks - builder doesn't need to
4. **Schema Field Usage**: Use `SchemaField::extract_field_type` helper (like MapMutationBuilder does after recent update)

## Migration Checklist

- [ ] Add panic to `build_paths()`
- [ ] Set `is_migrated()` to true
- [ ] Add `include_child_paths()` returning false
- [ ] Implement `collect_children()`
- [ ] Implement `assemble_from_children()`
- [ ] Delete `build_schema_example()`
- [ ] Delete `build_set_example_static()`
- [ ] Remove ExampleBuilder import
- [ ] Update TypeKind match arm
- [ ] Run build-check.sh
- [ ] Test with various Set types
