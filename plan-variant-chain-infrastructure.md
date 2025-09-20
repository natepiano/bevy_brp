# Plan: Add variant_chain Infrastructure to RecursionContext

## Problem Statement

PathRequirement construction needs access to the full variant chain (enum ancestry) from root to current position. Currently this information is embedded in `EnumContext::Child { variant_chain }`, but it's only available when the immediate parent is an enum in Child mode.

However, we need variant chain information regardless of whether we're in Root or Child enum context, because:
- A Root enum can be nested inside another enum variant
- PathRequirements need the complete ancestry chain to show how to reach any mutation path

## Solution

Add `variant_chain: Vec<VariantPathEntry>` as a separate field on `RecursionContext`, independent of `EnumContext`. This provides clean separation of concerns:

- **`enum_context`**: Controls enum behavior (Root = multiple examples, Child = concrete example)
- **`variant_chain`**: Tracks ancestry for PathRequirement construction

## Scope Analysis

### Files Requiring Changes

1. **`recursion_context.rs`**: Add field and update constructors
2. **`builder.rs`**: Update variant chain population logic in `process_all_children`
3. **All builder files**: Update any direct RecursionContext construction (none found)

### Context Creation Points

1. **Root context creation**: `RecursionContext::new()` - starts with empty variant_chain
2. **Child context creation**: `create_recursion_context()` - inherits and potentially extends parent's variant_chain
3. **Enum context handling**: When entering enum variants, append to variant_chain

### Current Variant Chain Logic

Currently in `builder.rs` lines 286-314:
```rust
let variant_chain = match &ctx.enum_context {
    Some(super::recursion_context::EnumContext::Child {
        variant_chain: parent_chain,
    }) => {
        // Extend parent chain
        let mut new_chain = parent_chain.clone();
        new_chain.push(VariantPathEntry {
            path: ctx.mutation_path.clone(),
            variant: applicable_variants[0].clone(),
        });
        new_chain
    }
    _ => {
        // Start new chain
        vec![VariantPathEntry {
            path: ctx.mutation_path.clone(),
            variant: applicable_variants[0].clone(),
        }]
    }
};
```

This logic needs to be simplified to just work with `ctx.variant_chain`.

## Implementation Plan

### Step 1: Add variant_chain Field to RecursionContext

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

```rust
pub struct RecursionContext {
    pub path_kind: PathKind,
    pub registry: Arc<HashMap<BrpTypeName, Value>>,
    pub mutation_path: String,
    pub path_action: PathAction,
    pub enum_context: Option<EnumContext>,
    // NEW: Independent variant chain tracking
    pub variant_chain: Vec<VariantPathEntry>,
}
```

### Step 2: Update RecursionContext::new()

```rust
pub const fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
    Self {
        path_kind,
        registry,
        mutation_path: String::new(),
        path_action: PathAction::Create,
        enum_context: None,
        variant_chain: Vec::new(), // Start with empty chain
    }
}
```

### Step 3: Update create_recursion_context()

```rust
pub fn create_recursion_context(
    &self,
    path_kind: PathKind,
    child_path_action: PathAction,
) -> Self {
    // ... existing path and action logic ...

    Self {
        path_kind,
        registry: Arc::clone(&self.registry),
        mutation_path: new_path_prefix,
        path_action,
        enum_context: self.enum_context.clone(),
        variant_chain: self.variant_chain.clone(), // Inherit parent's chain
    }
}
```

### Step 4: Update Variant Chain Population in builder.rs

**Current complex logic** (lines 286-314) becomes simple:

```rust
// When we have applicable variants, extend the chain
if let Some(applicable_variants) = item.applicable_variants() {
    if !applicable_variants.is_empty() {
        let mut extended_chain = child_ctx.variant_chain.clone();
        extended_chain.push(VariantPathEntry {
            path: ctx.mutation_path.clone(),
            variant: applicable_variants[0].clone(),
        });
        child_ctx.variant_chain = extended_chain;
    }
}
```

### Step 5: Update PathRequirement Construction

**Current logic** (lines 501-508):
```rust
let path_requirement = match &ctx.enum_context {
    Some(super::recursion_context::EnumContext::Child { variant_chain })
        if !variant_chain.is_empty() => {
        // Use variant_chain
    }
    _ => None,
};
```

**Becomes**:
```rust
let path_requirement = if !ctx.variant_chain.is_empty() {
    Some(PathRequirement {
        description: Self::generate_variant_description(&ctx.variant_chain),
        example: assembled_example.clone(),
        variant_path: ctx.variant_chain.clone(),
    })
} else {
    None
};
```

### Step 6: Remove EnumContext::Child variant_chain Field

**Since variant chain is now tracked independently**, we can simplify:

```rust
pub enum EnumContext {
    Root,
    Child, // No longer needs variant_chain field
}
```

But this might be a breaking change, so initially we can leave it and mark as deprecated.

**Migration Strategy: Atomic**

## Migration Strategy

This change will be implemented atomically in a single comprehensive update:

### Single Implementation Phase
1. Add `variant_chain` field to `RecursionContext`
2. Update all constructors to initialize the new field
3. Update all variant chain population logic to use `ctx.variant_chain`
4. Update all PathRequirement construction to use `ctx.variant_chain`
5. Remove `variant_chain` from `EnumContext::Child`
6. Simplify EnumContext enum to `enum EnumContext { Root, Child }`
7. Update all pattern matching to use the simplified enum
8. Run comprehensive tests to ensure correctness

### Rationale for Atomic Approach
- This is internal refactoring with clear, well-defined scope
- All changes are within the mutation path builder module
- No external API dependencies require backward compatibility
- Atomic implementation eliminates intermediate states and reduces complexity
- Single changeset is easier to review, test, and rollback if needed

## Benefits

1. **Cleaner separation**: EnumContext for behavior, variant_chain for ancestry
2. **Always available**: PathRequirements can be built regardless of Root/Child context
3. **Simpler logic**: No complex matching on EnumContext variants
4. **Natural accumulation**: Each recursive call inherits and potentially extends the chain
5. **Independent concerns**: Enum behavior and ancestry tracking don't interfere

## Testing Strategy

1. Test with nested enum structures (enum within enum)
2. Test Root enums nested inside other enum variants
3. Test mixed structures (struct → enum → struct → enum)
4. Verify no regression in existing PathRequirement generation
5. Verify EnumContext Root/Child behavior unchanged

## Success Criteria

1. `ctx.variant_chain` contains complete ancestry for any recursive context
2. PathRequirement construction simplified to use `ctx.variant_chain` directly
3. No changes to EnumContext Root/Child behavior for example generation
4. All existing tests pass
5. New nested enum test cases pass