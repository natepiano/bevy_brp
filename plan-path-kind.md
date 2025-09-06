# Plan: Consolidate PathKind and PathLocation

## Problem
`PathKind` and `PathLocation` have significant conceptual overlap and duplication:
- Both track parent/child relationships
- Both distinguish between root and field-level operations
- `PathKind` already generates path segments via `to_path_segment()`
- Manual string parsing in `create_field_context()` duplicates logic that `PathKind` constructors handle
- This duplication has led to code mistakes and maintenance burden

## Solution
Replace `PathLocation` with `PathKind` in `RecursionContext`, leveraging `PathKind`'s richer metadata and existing path generation logic.

## Specific Changes

### 1. Update `RecursionContext` struct
**File:** `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/recursion_context.rs`

**Before:**
```rust
pub struct RecursionContext {
    pub location: PathLocation,
    pub registry: Arc<HashMap<BrpTypeName, Value>>,
    pub mutation_path: String,
    pub parent_knowledge: Option<&'static MutationKnowledge>,
}
```

**After:**
```rust
pub struct RecursionContext {
    pub path_kind: PathKind,
    pub registry: Arc<HashMap<BrpTypeName, Value>>,
    pub mutation_path: String,
    pub parent_knowledge: Option<&'static MutationKnowledge>,
}
```

### 2. Update `RecursionContext::new()`
**Before:**
```rust
pub const fn new(location: PathLocation, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
    Self {
        location,
        registry,
        mutation_path: String::new(),
        parent_knowledge: None,
    }
}
```

**After:**
```rust
pub const fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
    Self {
        path_kind,
        registry,
        mutation_path: String::new(),
        parent_knowledge: None,
    }
}
```

### 3. Update `RecursionContext::type_name()`
**Before:**
```rust
pub const fn type_name(&self) -> &BrpTypeName {
    self.location.type_name()
}
```

**After:**
```rust
pub const fn type_name(&self) -> &BrpTypeName {
    self.path_kind.type_name()
}
```

### 4. Add `type_name()` method to `PathKind`
**File:** `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/path_kind.rs`

**Add new method:**
```rust
impl PathKind {
    /// Get the type name being processed (matches PathLocation::type_name() behavior)
    pub const fn type_name(&self) -> &BrpTypeName {
        match self {
            Self::RootValue { type_name } => type_name,
            Self::StructField { type_name, .. } => type_name,
            Self::IndexedElement { type_name, .. } => type_name,
            Self::ArrayElement { type_name, .. } => type_name,
        }
    }
}
```

### 5. Simplify `RecursionContext::create_field_context()`
**Before:**
```rust
pub fn create_field_context(&self, accessor: &str, field_type: &BrpTypeName) -> Self {
    let parent_type = self.type_name();
    let new_path_prefix = format!("{}{}", self.mutation_path, accessor);
    
    // Manual string parsing to extract field name
    let mutation_path = accessor
        .trim_start_matches('.')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();

    let field_knowledge = BRP_MUTATION_KNOWLEDGE
        .get(&KnowledgeKey::struct_field(parent_type, &mutation_path))
        .or_else(|| BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(field_type)));

    Self {
        location: PathLocation::element(&mutation_path, field_type, parent_type),
        registry: Arc::clone(&self.registry),
        mutation_path: new_path_prefix,
        parent_knowledge: field_knowledge,
    }
}
```

**After:**
```rust
pub fn create_field_context(&self, path_kind: PathKind) -> Self {
    let parent_type = self.type_name();
    let new_path_prefix = format!("{}{}", self.mutation_path, path_kind.to_path_segment());
    
    // Extract field name for knowledge lookup based on path kind
    let field_name = match &path_kind {
        PathKind::StructField { field_name, .. } => field_name.clone(),
        PathKind::IndexedElement { index, .. } => index.to_string(),
        PathKind::ArrayElement { index, .. } => index.to_string(),
        PathKind::RootValue { .. } => String::new(),
    };

    let field_knowledge = if !field_name.is_empty() {
        BRP_MUTATION_KNOWLEDGE
            .get(&KnowledgeKey::struct_field(parent_type, &field_name))
            .or_else(|| BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(path_kind.type_name())))
    } else {
        None
    };

    Self {
        path_kind,
        registry: Arc::clone(&self.registry),
        mutation_path: new_path_prefix,
        parent_knowledge: field_knowledge,
    }
}
```

### 6. Remove `PathLocation` enum and impl
**File:** `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/recursion_context.rs`

**Remove entirely:**
```rust
#[derive(Debug, Clone)]
pub enum PathLocation {
    Root { type_name: BrpTypeName },
    Element {
        field_name: String,
        element_type: BrpTypeName,
        parent_type: BrpTypeName,
    },
}

impl PathLocation {
    // ... all methods
}
```

## Call Sites to Update

### 1. Builder constructors that create `RecursionContext`
**Files:** All builder files in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/`

**Pattern Before:**
```rust
let context = RecursionContext::new(PathLocation::root(type_name), registry);
```

**Pattern After:**
```rust
let context = RecursionContext::new(PathKind::new_root_value(type_name.clone()), registry);
```

### 2. Builder methods that call `create_field_context()`
**Files:** `struct_builder.rs`, `tuple_builder.rs`, `array_builder.rs`, etc.

**Pattern Before:**
```rust
let field_context = context.create_field_context(&format!(".{}", field_name), field_type);
```

**Pattern After:**
```rust
let field_path_kind = PathKind::new_struct_field(field_name.to_string(), field_type.clone(), context.type_name().clone());
let field_context = context.create_field_context(field_path_kind);
```

**Array elements before:**
```rust
let element_context = context.create_field_context(&format!("[{}]", i), element_type);
```

**Array elements after:**
```rust
let element_path_kind = PathKind::new_array_element(i, element_type.clone(), context.type_name().clone());
let element_context = context.create_field_context(element_path_kind);
```

**Tuple elements before:**
```rust
let element_context = context.create_field_context(&format!(".{}", i), element_type);
```

**Tuple elements after:**
```rust
let element_path_kind = PathKind::new_indexed_element(i, element_type.clone(), context.type_name().clone());
let element_context = context.create_field_context(element_path_kind);
```

### 3. Add missing `type_name` field to `PathKind` variants
**File:** `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/path_kind.rs`

Since `PathLocation::Element` tracks `type_name` (the type at that point in the hierarchy), we need to add this to `PathKind` variants:

**Update variants:**
```rust
#[derive(Debug, Clone, Deserialize)]
pub enum PathKind {
    RootValue { type_name: BrpTypeName },
    StructField {
        field_name: String,
        type_name: BrpTypeName,   // Add this - the type at this field
        parent_type: BrpTypeName,
    },
    IndexedElement {
        index: usize,
        type_name: BrpTypeName,   // Add this - the type at this element
        parent_type: BrpTypeName,
    },
    ArrayElement {
        index: usize,
        type_name: BrpTypeName,   // Add this - the type at this element
        parent_type: BrpTypeName,
    },
}
```

**Update constructors:**
```rust
pub const fn new_struct_field(field_name: String, type_name: BrpTypeName, parent_type: BrpTypeName) -> Self {
    Self::StructField { field_name, type_name, parent_type }
}

pub const fn new_indexed_element(index: usize, type_name: BrpTypeName, parent_type: BrpTypeName) -> Self {
    Self::IndexedElement { index, type_name, parent_type }
}

pub const fn new_array_element(index: usize, type_name: BrpTypeName, parent_type: BrpTypeName) -> Self {
    Self::ArrayElement { index, type_name, parent_type }
}
```

**Note:** The `type_name()` method implementation from Section 4 already correctly returns `type_name` from all variants.

## Benefits
1. **Single source of truth** for path building logic
2. **Eliminates duplication** between `PathKind.to_path_segment()` and manual accessor building
3. **Reduces string parsing** - field names/indices come directly from `PathKind` constructors
4. **Better type safety** - more specific variants than generic `Element`
5. **Cleaner API** - `create_field_context()` takes structured data instead of string accessors

## Testing
All existing tests should continue to pass as the public API remains the same - only internal representation changes from `PathLocation` to `PathKind`.

## Design Review Skip Notes

### TYPE-SYSTEM-1: Constructor signatures cannot be const with String parameters
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: Section 6: Add missing element_type field to PathKind variants
- **Issue**: Plan shows constructor signatures as 'const fn' but they take String parameters which cannot be const in Rust
- **Proposed Change**: Remove const from constructor signatures
- **Verdict**: MODIFIED
- **Reasoning**: The issue is confirmed - const functions cannot take String parameters or BrpTypeName (which wraps String). However, the original suggested fix incorrectly adds field_type and element_type parameters that don't exist in the current enum variants. The correct fix is simply to remove 'const' from all three constructor functions since BrpTypeName contains a String and cannot be used in const context.
- **Decision**: Current code compiles successfully with const fn and BrpTypeName(String), indicating Rust has become more permissive with const fn in recent versions. No changes needed.

### TYPE-SYSTEM-2: Parameter count mismatch between constructor signatures and call sites
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: Section 2: Builder methods that call create_field_context()
- **Issue**: Plan shows call sites with 2 parameters but proposed constructor signatures require 3 parameters (field_name, field_type, parent_type)
- **Proposed Change**: Update call sites to pass 3 parameters to PathKind constructors
- **Verdict**: REJECTED
- **Reasoning**: This finding is a false positive. After examining the actual code, there is no parameter count mismatch. The create_field_context method correctly takes 2 parameters (accessor: &str, field_type: &BrpTypeName), and all call sites provide exactly 2 parameters. The PathKind::new_struct_field constructor takes 2 parameters (field_name: String, parent_type: BrpTypeName), and all usage sites correctly provide 2 parameters. The existing implementation is working correctly and consistently across all modules. The suggested change would actually break the working code by attempting to pass 3 parameters to a 2-parameter constructor and changing the API design without justification.
- **Decision**: User elected to skip this recommendation