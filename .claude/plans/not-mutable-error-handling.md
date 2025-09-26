# NotMutableReason Error Handling Refactor Plan

## Overview
Refactor `NotMutableReason` to be completely internal to the mutation_path_builder module using error downcasting, removing it from the public Error enum.

## Current Problem
- `NotMutableReason` is exposed in the public `Error` enum
- Internal mutation path building details leak into the public API
- The `as_not_mutable()` method on Error exposes implementation details

## Solution: Internal Error with Downcasting

### Core Design
1. Create a private `NotMutableError` type within mutation_path_builder
2. Use error_stack's downcasting capability to detect NotMutable errors
3. Remove `Error::NotMutable` variant from public enum
4. Remove `as_not_mutable()` method from Error

### Benefits
- **Complete Encapsulation**: NotMutableReason stays entirely within mutation_path_builder
- **Clean Public API**: No mutation path building concepts in public Error enum
- **Simple Implementation**: Minimal code changes required
- **Type Safety**: Compiler ensures NotMutableReason can't leak out

## Implementation

### 1. Create Internal Error Type
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

```rust
use error_stack::Context;
use std::fmt;

/// Internal error type for NotMutable conditions
/// This is never exposed outside the mutation_path_builder module
#[derive(Debug, Clone)]
struct NotMutableError(NotMutableReason);

impl fmt::Display for NotMutableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Type cannot be mutated: {}", self.0)
    }
}

impl Context for NotMutableError {}
```

### 2. Update Builder Error Returns
**Files to update**:
- `builders/array_builder.rs`
- `builders/list_builder.rs`
- `builders/map_builder.rs`
- `builders/set_builder.rs`
- `builders/struct_builder.rs`
- `builders/tuple_builder.rs`
- `builders/value_builder.rs`
- `path_builder.rs`
- `recursion_context.rs`

**Change pattern**:
```rust
// OLD: Return Error::NotMutable
return Err(Error::NotMutable(NotMutableReason::SomeReason { ... }).into());

// NEW: Return NotMutableError
return Err(error_stack::Report::new(NotMutableError(NotMutableReason::SomeReason { ... })));
```

**Specific changes**:

In `path_builder.rs`:
```rust
// Line ~112
if element.is_complex_type() {
    return Err(error_stack::Report::new(
        NotMutableError(NotMutableReason::ComplexCollectionKey(
            ctx.type_name().clone(),
        ))
    ));
}
```

In `recursion_context.rs`:
```rust
// Line ~70
self.registry.get(self.type_name()).ok_or_else(|| {
    error_stack::Report::new(
        NotMutableError(NotMutableReason::NotInRegistry(self.type_name().clone()))
    )
})
```

In `tuple_builder.rs`:
```rust
// Line ~98
if elements.len() == 1 && elements[0].is_handle() {
    return Err(error_stack::Report::new(
        NotMutableError(NotMutableReason::NonMutableHandle {
            container_type: ctx.type_name().clone(),
            element_type: elements[0].clone(),
        })
    ));
}
```

In `value_builder.rs`:
```rust
// Line ~39
if !has_serialize {
    return Err(error_stack::Report::new(
        NotMutableError(NotMutableReason::MissingSerializationTraits(
            ctx.type_name().clone()
        ))
    ));
}

// Line ~49
Err(error_stack::Report::new(
    NotMutableError(NotMutableReason::NoExampleAvailable(
        ctx.type_name().clone()
    ))
))
```

### 3. Update Error Catching in Builder
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Replace lines 152-164:
```rust
// Handle NotMutable errors at this single choke point
result.or_else(|error| {
    // Try to downcast to our internal NotMutableError type
    error
        .downcast_ref::<NotMutableError>()
        .map(|not_mutable| {
            vec![
                MutationPathBuilder::<ValueMutationBuilder>::build_not_mutable_path(
                    ctx,
                    not_mutable.0.clone(),
                ),
            ]
        })
        .ok_or(error)
})
```

### 4. Update build_not_mutable_path Access
Since `NotMutableError` is private to the module, `build_not_mutable_path` can access it directly:

```rust
// In builder.rs, lines 453-454
Some(Ok(vec![Self::build_not_mutable_path(
    ctx,
    NotMutableReason::RecursionLimitExceeded(ctx.type_name().clone()),
)]))

// Lines 465-466
Some(Ok(vec![Self::build_not_mutable_path(
    ctx,
    NotMutableReason::NotInRegistry(ctx.type_name().clone()),
)]))
```

### 5. Clean Up Public Error Enum
**Location**: `mcp/src/error.rs`

Remove:
```rust
// Line 4: Remove import
use crate::brp_tools::NotMutableReason;

// Lines 50-51: Remove variant
#[error("Type cannot be mutated: {0}")]
NotMutable(NotMutableReason),

// Line 98: Remove from Debug impl
Self::NotMutable(reason) => f.debug_tuple("NotMutable").field(reason).finish(),

// Lines 132-138: Remove as_not_mutable method
pub const fn as_not_mutable(&self) -> Option<&NotMutableReason> {
    match self {
        Self::NotMutable(reason) => Some(reason),
        _ => None,
    }
}
```

### 6. Update Module Exports
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

Remove public export:
```rust
// Remove this line:
pub use not_mutable_reason::NotMutableReason;

// Keep it internal:
use not_mutable_reason::NotMutableReason;
```

## Migration Steps

1. **Add NotMutableError type** to mutation_path_builder/mod.rs
2. **Update all builders** to return NotMutableError instead of Error::NotMutable
3. **Update error catching** in builder.rs to use downcasting
4. **Remove NotMutable variant** from Error enum
5. **Remove as_not_mutable method** from Error
6. **Remove public export** of NotMutableReason
7. **Run tests** to ensure everything works

## Testing Strategy

1. **Existing tests should continue to pass** - the change is internal
2. **Verify NotMutable paths are still created correctly**
3. **Ensure no NotMutableReason types leak to public API**
4. **Check that error messages are still meaningful**

## Rollback Plan

If issues arise:
1. The changes are localized to mutation_path_builder module
2. Git history preserves the old approach
3. Can temporarily re-add Error::NotMutable variant if needed

## Summary

This approach achieves complete encapsulation with minimal changes:
- ~10 lines to add NotMutableError type
- ~10 error return sites to update
- 1 error catching site to modify
- Remove ~20 lines from Error enum
- No complex type conversions or boundary management needed