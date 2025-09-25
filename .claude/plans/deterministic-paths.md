# Plan: Set Standard Order to Fix Non-Deterministic Enum Variants

## Problem
Non-deterministic iteration order in enum variant processing causes 304+ false positive VALUE_CHANGE patterns and 13 TYPE_CHANGE patterns in mutation tests. The issue stems from using `HashMap` which has random iteration order.

## Quick Fix Solution
Replace `HashMap` with `BTreeMap` in `enum_path_builder.rs` to get deterministic iteration order based on variant signatures. This will ensure consistent ordering across test runs until we implement the full variant chain solution.

## Changes Required

### 1. Import BTreeMap (enum_path_builder.rs:3)
```rust
// Change from:
use std::collections::HashMap;

// To:
use std::collections::{BTreeMap, HashMap};
```

### 2. Update Function Signatures to Use BTreeMap

#### a. `group_variants_by_signature` (lines 208-220)
```rust
// Change return type from:
HashMap<VariantSignature, Vec<EnumVariantInfo>>

// To:
BTreeMap<VariantSignature, Vec<EnumVariantInfo>>

// Change implementation from:
let mut groups = HashMap::new();

// To:
let mut groups = BTreeMap::new();
```

#### b. `extract_and_group_variants` (lines 362-368)
```rust
// Change return type from:
Result<HashMap<VariantSignature, Vec<EnumVariantInfo>>>

// To:
Result<BTreeMap<VariantSignature, Vec<EnumVariantInfo>>>
```

#### c. `build_enum_examples` (lines 372-376)
```rust
// Change parameter from:
variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>

// To:
variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>
```

#### d. `concrete_example` (lines 329-333)
```rust
// Change parameter from:
variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>

// To:
variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>
```

#### e. `process_children` (lines 455-459)
```rust
// Change parameter from:
variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>

// To:
variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>
```

### 3. Ensure VariantSignature Implements Ord

Check if `VariantSignature` already derives `Ord` trait. If not, add:
```rust
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum VariantSignature {
    // ...
}
```

## Why This Works
- `BTreeMap` iterates in sorted key order, providing deterministic iteration
- `VariantSignature` comparison will ensure consistent ordering based on variant structure:
  - Unit variants sort first (no fields)
  - Tuple variants sort by their type sequences
  - Struct variants sort by field names and types
- This eliminates the 304+ false positive VALUE_CHANGE patterns
- Minimal code change with immediate effect

## Testing After Implementation
1. Run `/create_mutation_test_json` twice
2. Compare outputs - should be identical
3. Run `/compare_mutation_path` - should show no differences
4. Verify all existing tests still pass

## Impact Analysis
- **Positive**: Deterministic test results, no false positives
- **Neutral**: Slight performance impact (BTreeMap vs HashMap) but negligible for our use case
- **No Breaking Changes**: Output format remains the same, just ordering changes

## Alternative Considered
Could sort variants after extraction, but BTreeMap is cleaner and ensures consistency throughout the processing pipeline.

## Long-term Solution
This is a tactical fix while we implement the full variant chain solution from `plan-mutation-path-root-example.md`. The BTreeMap change is compatible with the future solution and can remain in place for overall stability.