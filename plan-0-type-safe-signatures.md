# Plan 0: Type-Safe Path Signatures

## Goal
Add type-safe signatures for path grouping to replace string-based signatures proposed in Plan 1. This is a prerequisite improvement that provides the foundation for Plan 1's deferred grouping.

## Motivation
Plan 1 proposes using string-based signatures like `format!("field:{}", type_name)` for grouping paths. This violates Rust's type safety principles and introduces potential runtime errors. By adding type-safe signatures first, we ensure Plan 1 starts with a solid foundation.

## Implementation

### Step 1: Define PathSignature Enum

Add to `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` after line 64 (after the VariantSignature Display implementation) and before line 66 (before the MutationPathInternal struct):

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
```

### Step 2: Add to_signature Method to PathKind

Add to `PathKind` implementation in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_kind.rs`:

```rust
impl PathKind {
    /// Convert this PathKind to a PathSignature for grouping purposes
    pub fn to_signature(&self) -> PathSignature {
        match self {
            Self::RootValue { type_name, .. } =>
                PathSignature::Root { type_name: type_name.clone() },
            Self::StructField { type_name, .. } =>
                PathSignature::Field { type_name: type_name.clone() },
            Self::IndexedElement { type_name, .. } =>
                PathSignature::Index { type_name: type_name.clone() },
            Self::ArrayElement { type_name, .. } =>
                PathSignature::Array { type_name: type_name.clone() },
        }
    }
}
```

### Step 3: Add Signature Method to MutationPathInternal

Add to `MutationPathInternal` implementation in `types.rs`:

```rust
impl MutationPathInternal {
    /// Get the signature of this path for grouping purposes
    pub fn signature(&self) -> PathSignature {
        self.path_kind.to_signature()
    }
}
```

### Step 4: Update Plan 1's Functions to Use PathSignature

Plan 1's new `deduplicate_mutation_paths` function will use the type-safe signature:

```rust
/// Groups mutation paths by signature, keeping one representative per group
/// Called during final output processing to deduplicate similar paths
fn deduplicate_mutation_paths(all_paths: Vec<MutationPathInternal>) -> Vec<MutationPathInternal> {
    // Group paths by (full_mutation_path, signature) - using PathSignature enum
    let mut groups: HashMap<(FullMutationPath, PathSignature), Vec<MutationPathInternal>> = HashMap::new();

    for path in all_paths {
        let signature = path.signature();  // Returns PathSignature enum
        let key = (path.full_mutation_path.clone(), signature);
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