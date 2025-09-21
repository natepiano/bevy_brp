# Plan: Simplify Applicable Variants

## Problem Statement

Currently, the enum builder needs to return additional variant information along with PathKind, which led to creating PathKindWithVariants wrapper and the MaybeVariants trait. This creates unnecessary complexity with generic types throughout the codebase.

## Key Insight

Since `PathKind::IndexedElement` is the only PathKind variant used by enums that needs variant information, we can embed the `applicable_variants` directly into it. This eliminates the need for:
- The wrapper type `PathKindWithVariants`
- The trait `MaybeVariants`
- Generic type parameters in PathBuilder trait
- Complex variant extraction logic in builder.rs

## Proposed Solution: Add `applicable_variants` to PathKind

Add variant information directly to `PathKind::IndexedElement`. This aligns with the existing pattern of storing contextual information in PathKind.

### 1. Enhance PathKind::IndexedElement

Add `context` field with an `ElementContext` enum to properly model the distinction between tuple and enum contexts:

```rust
#[derive(Debug, Clone)]
pub enum ElementContext {
    Tuple,
    Enum { applicable_variants: Vec<String> },
}

pub enum PathKind {
    // ... other variants ...

    IndexedElement {
        /// The index within the parent container (0 for first element)
        index: usize,
        /// The type of this indexed element
        type_name: BrpTypeName,
        /// The parent container type (tuple or enum)
        parent_type: BrpTypeName,
        /// NEW: Context-specific information (tuple vs enum with variants)
        context: ElementContext,
    },

    // ... other variants ...
}
```

This change uses proper type-driven design:
- `ElementContext::Tuple`: For tuple elements (no variant information needed)
- `ElementContext::Enum`: For enum elements (always includes variant information)
- Makes invalid states unrepresentable (can't have variants on tuples or missing variants on enums)

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

### 4. Simplify PathBuilder Trait

Since all builders now return `PathKind` directly, we can remove the generic Item type entirely:

```rust
// OLD: PathBuilder trait with generic Item type
pub trait PathBuilder {
    type Item: MaybeVariants;  // Requires trait bound
    type Iter<'a>: Iterator<Item = Self::Item>
    where
        Self: 'a;
    // ...
}

// NEW: PathBuilder trait without Item type
pub trait PathBuilder {
    // No Item type at all - just specify PathKind directly in Iterator
    type Iter<'a>: Iterator<Item = PathKind>
    where
        Self: 'a;
    // ...
}
```

This simplifies all builder implementations:

```rust
impl PathBuilder for EnumMutationBuilder {
    // No Item type needed anymore
    type Iter<'a> = std::vec::IntoIter<PathKind> where Self: 'a;
}

impl PathBuilder for TupleMutationBuilder {
    // No Item type needed anymore
    type Iter<'a> = std::vec::IntoIter<PathKind> where Self: 'a;
}
```


## Migration Path

1. **Phase 1**: Add `applicable_variants: Option<Vec<String>>` to `PathKind::IndexedElement`
2. **Phase 2**: Update `enum_builder.rs` to populate the new field
3. **Phase 3**: Update tuple_builder to pass `None` for the new field
4. **Phase 4**: Remove `MaybeVariants` trait and `PathKindWithVariants` struct
5. **Phase 5**: Update consumer code to extract variants directly from PathKind


## Files to Modify

### 1. `path_kind.rs` - Add `context` field to both IndexedElement and StructField

Update both `PathKind::IndexedElement` and `PathKind::StructField` variants to include context:
```rust
StructField {
    /// The field name
    field_name: String,
    /// The type of this field
    type_name: BrpTypeName,
    /// The parent struct type
    parent_type: BrpTypeName,
    /// NEW: Context-specific information (regular struct vs enum struct variant)
    context: ElementContext,
}

IndexedElement {
    /// The index within the parent container (0 for first element)
    index: usize,
    /// The type of this indexed element
    type_name: BrpTypeName,
    /// The parent container type (tuple or enum)
    parent_type: BrpTypeName,
    /// NEW: Context-specific information (tuple vs enum with variants)
    context: ElementContext,
}
```

Note: StructField also needs the context field because enum struct variants require variant information, just like indexed elements in enums.

Also update constructors to be context-specific:
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

// NEW: Separate constructors for each context
// For tuple elements (can remain const):
pub const fn new_indexed_element_tuple(
    index: usize,
    type_name: BrpTypeName,
    parent_type: BrpTypeName,
) -> Self {
    Self::IndexedElement {
        index,
        type_name,
        parent_type,
        context: ElementContext::Tuple,
    }
}

// For enum elements (cannot be const due to Vec):
pub fn new_indexed_element_enum(
    index: usize,
    type_name: BrpTypeName,
    parent_type: BrpTypeName,
    applicable_variants: Vec<String>,
) -> Self {
    debug_assert!(!applicable_variants.is_empty(), "Enum variants should not be empty");
    Self::IndexedElement {
        index,
        type_name,
        parent_type,
        context: ElementContext::Enum { applicable_variants },
    }
}

// For regular struct fields (can be const):
pub const fn new_struct_field(
    field_name: String,
    type_name: BrpTypeName,
    parent_type: BrpTypeName,
) -> Self {
    Self::StructField {
        field_name,
        type_name,
        parent_type,
        context: ElementContext::Tuple, // Regular structs use Tuple context
    }
}

// For enum struct variant fields (cannot be const due to Vec):
pub fn new_struct_field_enum(
    field_name: String,
    type_name: BrpTypeName,
    parent_type: BrpTypeName,
    applicable_variants: Vec<String>,
) -> Self {
    debug_assert!(!applicable_variants.is_empty(), "Enum variants should not be empty");
    Self::StructField {
        field_name,
        type_name,
        parent_type,
        context: ElementContext::Enum { applicable_variants },
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

// NEW - Use the enum-specific constructor
children.push(PathKind::new_indexed_element_enum(
    index,
    type_name.clone(),
    ctx.type_name().clone(),
    applicable_variants.clone(),
));
```

Also update struct field creation in enum_builder.rs (around line 352-364) for enum struct variants:
```rust
// OLD - for enum struct variants
children.push(PathKindWithVariants {
    path: Some(PathKind::StructField {
        field_name: field_name.clone(),
        type_name: field_type.clone(),
        parent_type: ctx.type_name().clone(),
    }),
    applicable_variants: applicable_variants.clone(),
});

// NEW - Use the enum struct field constructor
children.push(PathKind::new_struct_field_enum(
    field_name.clone(),
    field_type.clone(),
    ctx.type_name().clone(),
    applicable_variants.clone(),
));
```

### 3. `struct_builder.rs` - Use regular struct field constructor

Update where `StructField` is created for regular structs:
```rust
// OLD
PathKind::StructField {
    field_name: field_name.clone(),
    type_name: field_type.clone(),
    parent_type: ctx.type_name().clone(),
}

// NEW - Use the regular struct field constructor
PathKind::new_struct_field(
    field_name.clone(),
    field_type.clone(),
    ctx.type_name().clone(),
)
```

### 4. `tuple_builder.rs` - Use tuple-specific constructor

Update where `IndexedElement` is created (around line 85):
```rust
// OLD
PathKind::IndexedElement {
    index,
    type_name: element_type.clone(),
    parent_type: ctx.type_name().clone(),
}

// NEW - Use the tuple-specific constructor
PathKind::new_indexed_element_tuple(
    index,
    element_type.clone(),
    ctx.type_name().clone(),
)
```

### 5. `path_builder.rs` - Remove MaybeVariants trait and simplify PathBuilder

Remove the MaybeVariants trait entirely:
```rust
// DELETE all of this:
pub trait MaybeVariants {
    fn applicable_variants(&self) -> Option<&[String]> {
        None
    }
    fn into_path_kind(self) -> Option<PathKind>;
}
```

Update the PathBuilder trait to remove the Item type entirely and directly specify PathKind:
```rust
// OLD
pub trait PathBuilder {
    type Item: MaybeVariants;
    type Iter<'a>: Iterator<Item = Self::Item>
    where
        Self: 'a;
    // ...
}

// NEW - Remove Item type, specify PathKind directly
pub trait PathBuilder {
    type Iter<'a>: Iterator<Item = PathKind>  // Directly returns PathKind
    where
        Self: 'a;
    // ...
}
```

Note: Do NOT use `type Item = PathKind;` as that's invalid Rust syntax - associated types cannot have default values.

### 6. `path_kind.rs` - Remove MaybeVariants implementation

Remove the MaybeVariants implementation for PathKind:
```rust
// DELETE this implementation:
impl MaybeVariants for PathKind {
    fn applicable_variants(&self) -> Option<&[String]> {
        None
    }
    fn into_path_kind(self) -> Option<PathKind> {
        Some(self)
    }
}
```

### 7. `enum_builder.rs` - Remove PathKindWithVariants

Delete the PathKindWithVariants struct and its MaybeVariants implementation:
```rust
// DELETE all of this:
pub struct PathKindWithVariants {
    pub path: Option<PathKind>,
    pub applicable_variants: Vec<String>,
}

impl MaybeVariants for PathKindWithVariants {
    fn applicable_variants(&self) -> Option<&[String]> {
        Some(&self.applicable_variants)
    }
    fn into_path_kind(self) -> Option<PathKind> {
        self.path
    }
}
```

### 8. `builder.rs` - Update variant extraction

In `process_all_children()` (around line 195-210), the code currently uses MaybeVariants methods. Update to work directly with PathKind:

```rust
// OLD (around line 205-210)
for item in child_items {
    let variant_info = item.applicable_variants().map(<[String]>::to_vec);

    if let Some(path_kind) = item.into_path_kind() {
        let mut child_ctx = ctx.create_recursion_context(path_kind.clone(), ...);
        // ...
    }
}

// NEW
for path_kind in child_items {
    // Extract variant information from PathKind directly using the new context field
    let variant_info = match &path_kind {
        PathKind::IndexedElement { context: ElementContext::Enum { applicable_variants }, .. } => {
            Some(applicable_variants.clone())
        },
        PathKind::StructField { context: ElementContext::Enum { applicable_variants }, .. } => {
            Some(applicable_variants.clone())
        },
        _ => None,
    };

    let mut child_ctx = ctx.create_recursion_context(path_kind.clone(), ...);
    // ...
}
```

Note: Since all PathBuilder implementations now return PathKind directly, we no longer need to check if `into_path_kind()` returns None - all items are valid PathKinds.

### 9. Cleanup imports

Remove imports of `MaybeVariants` and `PathKindWithVariants` from any files that used them:
- `enum_builder.rs` - Remove `use super::types::{PathKindWithVariants, MaybeVariants};`
- `builder.rs` - Remove `use super::types::MaybeVariants;`
- Any other files that imported these types

Add `ElementContext` import where needed:
- `path_kind.rs` - Define `ElementContext` enum
- `builder.rs` - Add `use super::path_kind::ElementContext;` (needs it for pattern matching when extracting variant info)
- Note: The builder files (`tuple_builder.rs`, `struct_builder.rs`, `enum_builder.rs`) don't need to import `ElementContext` since they only call constructors on `PathKind` that handle the context internally
