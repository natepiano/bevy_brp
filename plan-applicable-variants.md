# Plan: Simplify Applicable Variants

## Problem Statement

## Proposed Solution: Add `applicable_variants` to PathKind

Add variant information directly to `PathKind::IndexedElement`. This aligns with the existing pattern of storing contextual information in PathKind.

### 1. Enhance PathKind::IndexedElement

Add `applicable_variants` field to complete the context:

```rust
pub enum PathKind {
    // ... other variants ...

    IndexedElement {
        /// The index within the parent container (0 for first element)
        index: usize,
        /// The type of this indexed element
        type_name: BrpTypeName,
        /// The parent container type (tuple or enum)
        parent_type: BrpTypeName,
        /// NEW: Which enum variants this path applies to (None for tuples)
        applicable_variants: Option<Vec<String>>,
    },

    // ... other variants ...
}
```

This change is consistent because `PathKind::IndexedElement` already stores:
- `type_name`: What type this element is
- `parent_type`: What type contains this element
- `index`: Position in parent

Adding `applicable_variants` completes the contextual information.

### 2. Remove Unnecessary Abstractions

With variants directly in PathKind, we can eliminate:

```rust
// DELETE these abstractions - no longer needed:

// The MaybeVariants trait
pub trait MaybeVariants {
    fn applicable_variants(&self) -> Option<&[String]>;
}

// The PathKindWithVariants wrapper
pub struct PathKindWithVariants {
    pub path: Option<PathKind>,
    pub applicable_variants: Vec<String>,
}
```

### 3. Update enum_builder.rs

Change enum_builder to create PathKind directly with variants:

```rust
// OLD: Creates PathKindWithVariants wrapper
children.push(PathKindWithVariants {
    path: Some(PathKind::IndexedElement {
        index,
        type_name: type_name.clone(),
        parent_type: ctx.type_name().clone(),
    }),
    applicable_variants: applicable_variants.clone(),
});

// NEW: Creates PathKind with variants directly
children.push(PathKind::IndexedElement {
    index,
    type_name: type_name.clone(),
    parent_type: ctx.type_name().clone(),
    applicable_variants: Some(applicable_variants.clone()),
});
```

### 4. Simplify Builder Interface

All builders can now return `PathKind` directly:

```rust
// OLD: Mixed return types
impl PathBuilder for EnumMutationBuilder {
    type Item = PathKindWithVariants;  // Special case
}

impl PathBuilder for TupleMutationBuilder {
    type Item = PathKind;  // Regular
}

// NEW: Unified return type
impl PathBuilder for EnumMutationBuilder {
    type Item = PathKind;  // Same as all others
}

impl PathBuilder for TupleMutationBuilder {
    type Item = PathKind;  // Same as all others
}
```


## Migration Path

1. **Phase 1**: Add `applicable_variants: Option<Vec<String>>` to `PathKind::IndexedElement`
2. **Phase 2**: Update `enum_builder.rs` to populate the new field
3. **Phase 3**: Update tuple_builder to pass `None` for the new field
4. **Phase 4**: Remove `MaybeVariants` trait and `PathKindWithVariants` struct
5. **Phase 5**: Update consumer code to extract variants directly from PathKind


## Files to Modify

### 1. `path_kind.rs` - Add `applicable_variants` field

Update the `PathKind::IndexedElement` variant:
```rust
IndexedElement {
    /// The index within the parent container (0 for first element)
    index: usize,
    /// The type of this indexed element
    type_name: BrpTypeName,
    /// The parent container type (tuple or enum)
    parent_type: BrpTypeName,
    /// NEW: Which enum variants this path applies to (None for tuples)
    applicable_variants: Option<Vec<String>>,
}
```

Also update the constructor:
```rust
// Current constructor (3 parameters):
pub const fn new_indexed_element(
    index: usize,
    type_name: BrpTypeName,
    parent_type: BrpTypeName,
) -> Self {
    Self::IndexedElement {
        index,
        type_name,
        parent_type,
    }
}

// Updated constructor (4 parameters):
pub const fn new_indexed_element(
    index: usize,
    type_name: BrpTypeName,
    parent_type: BrpTypeName,
    applicable_variants: Option<Vec<String>>,  // NEW parameter
) -> Self {
    Self::IndexedElement {
        index,
        type_name,
        parent_type,
        applicable_variants,
    }
}
```



### 2. `enum_builder.rs` - Remove PathKindWithVariants usage

Change the `PathBuilder` implementation (around line 27):
```rust
// OLD
impl PathBuilder for EnumMutationBuilder {
    type Item = PathKindWithVariants;
}

// NEW
impl PathBuilder for EnumMutationBuilder {
    type Item = PathKind;
}
```

Update `collect_children()` method (around line 345):
```rust
// OLD
children.push(PathKindWithVariants {
    path: Some(PathKind::IndexedElement {
        index,
        type_name: type_name.clone(),
        parent_type: ctx.type_name().clone(),
    }),
    applicable_variants: applicable_variants.clone(),
});

// NEW
children.push(PathKind::IndexedElement {
    index,
    type_name: type_name.clone(),
    parent_type: ctx.type_name().clone(),
    applicable_variants: Some(applicable_variants.clone()),
});
```

Also update any other places that create `PathKindWithVariants`.

### 3. `tuple_builder.rs` - Add None for applicable_variants

Update where `IndexedElement` is created (around line 85):
```rust
// OLD
PathKind::IndexedElement {
    index,
    type_name: element_type.clone(),
    parent_type: ctx.type_name().clone(),
}

// NEW
PathKind::IndexedElement {
    index,
    type_name: element_type.clone(),
    parent_type: ctx.type_name().clone(),
    applicable_variants: None,  // Tuples don't have variants
}
```

### 5. `types.rs` - Delete MaybeVariants trait and PathKindWithVariants

Remove these entirely (around lines 160-180):
```rust
// DELETE all of this:
pub trait MaybeVariants {
    fn applicable_variants(&self) -> Option<&[String]>;
}

impl MaybeVariants for PathKind {
    fn applicable_variants(&self) -> Option<&[String]> {
        None
    }
}

pub struct PathKindWithVariants {
    pub path: Option<PathKind>,
    pub applicable_variants: Vec<String>,
}

impl MaybeVariants for PathKindWithVariants {
    fn applicable_variants(&self) -> Option<&[String]> {
        Some(&self.applicable_variants)
    }
}
```

### 6. `builder.rs` - Update variant extraction

In `build_mutation_paths_recursive()` (around line 203), change how variants are extracted:
```rust
// OLD
let variant_info = item.applicable_variants().map(<[String]>::to_vec);

// NEW
let variant_info = match &item {
    PathKind::IndexedElement { applicable_variants, .. } => applicable_variants.clone(),
    _ => None,
};
```

Also update the function signature (around line 170) to accept `PathKind` directly:
```rust
// OLD
fn build_mutation_paths_recursive<I: MaybeVariants>(
    &self,
    child_items: Vec<I>,
    // ...
)

// NEW
fn build_mutation_paths_recursive(
    &self,
    child_items: Vec<PathKind>,
    // ...
)
```

### 7. Any imports cleanup

Remove imports of `MaybeVariants` and `PathKindWithVariants` from any files that used them:
- `enum_builder.rs` - Remove `use super::types::{PathKindWithVariants, MaybeVariants};`
- `builder.rs` - Remove `use super::types::MaybeVariants;`
- Any other files that imported these types
