# Plan 0: Type-Safe Path Signatures

## Goal
Add type-safe signatures for path grouping to replace string-based signatures proposed in Plan 1. This is a prerequisite improvement that provides the foundation for Plan 1's deferred grouping.

## Motivation
Plan 1 proposes using string-based signatures like `format!("field:{}", type_name)` for grouping paths. This violates Rust's type safety principles and introduces potential runtime errors. By adding type-safe signatures first, we ensure Plan 1 starts with a solid foundation.

## Implementation

### Step 1: Define PathSignature Enum

Add to `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`:

```rust
/// A signature for grouping PathKinds that have similar structure
/// Used as a HashMap key for deduplication in output stage grouping
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSignature {
    Root { type_name: BrpTypeName },
    Field { type_name: BrpTypeName },
    Index { type_name: BrpTypeName },
    Array { type_name: BrpTypeName },
}

impl PathSignature {
    /// Create a signature from a PathKind for grouping purposes
    pub fn from_path_kind(path_kind: &PathKind) -> Self {
        match path_kind {
            PathKind::RootValue { type_name, .. } =>
                PathSignature::Root { type_name: type_name.clone() },
            PathKind::StructField { type_name, .. } =>
                PathSignature::Field { type_name: type_name.clone() },
            PathKind::IndexedElement { type_name, .. } =>
                PathSignature::Index { type_name: type_name.clone() },
            PathKind::ArrayElement { type_name, .. } =>
                PathSignature::Array { type_name: type_name.clone() },
        }
    }
}
```

### Step 2: Add Signature Method to MutationPathInternal

Add to `MutationPathInternal` implementation in `types.rs`:

```rust
impl MutationPathInternal {
    /// Get the signature of this path for grouping purposes
    pub fn signature(&self) -> PathSignature {
        PathSignature::from_path_kind(&self.path_kind)
    }
}
```

### Step 3: Update Plan 1's Functions to Use PathSignature

Plan 1's new `deduplicate_mutation_paths` function will use the type-safe signature:

```rust
/// Groups mutation paths by signature, keeping one representative per group
/// Called during final output processing to deduplicate similar paths
fn deduplicate_mutation_paths(all_paths: Vec<MutationPathInternal>) -> Vec<MutationPathInternal> {
    // Group paths by (path_string, signature) - using PathSignature enum
    let mut groups: HashMap<(String, PathSignature), Vec<MutationPathInternal>> = HashMap::new();

    for path in all_paths {
        let signature = path.signature();  // Returns PathSignature enum
        let key = (path.path.clone(), signature);
        groups.entry(key).or_default().push(path);
    }

    // Return one representative per group
    groups.into_values()
        .map(|mut group| group.pop().unwrap())
        .collect()
}
```

## Benefits

1. **Type Safety**: Compile-time checking prevents signature-related bugs
2. **Performance**: Enum comparison is faster than string comparison
3. **Maintainability**: IDE support, refactoring safety, and clearer intent
4. **No Runtime Parsing**: Eliminates potential string format errors

## Testing

This is a pure addition with no behavior changes. Testing involves:
1. Compile successfully with the new types
2. Verify Plan 1 can use `PathSignature` instead of strings
3. Confirm HashMap operations work with `PathSignature` as key

## Why This Is Independent

- **No changes to existing code** - purely additive
- **No functional changes** - just adds types for future use
- **Can be merged immediately** - provides foundation for Plan 1
- **Reduces risk** - separates type safety from algorithm changes