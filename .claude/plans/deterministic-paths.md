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

#### a. `group_variants_by_signature` function
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

#### b. `extract_and_group_variants` function in the public functions section
```rust
// Change return type from:
Result<HashMap<VariantSignature, Vec<EnumVariantInfo>>>

// To:
Result<BTreeMap<VariantSignature, Vec<EnumVariantInfo>>>
```

#### c. `build_enum_examples` function in the enum examples section
```rust
// Change parameter from:
variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>

// To:
variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>
```

#### d. `concrete_example` function in the variant processing helpers
```rust
// Change parameter from:
variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>

// To:
variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>
```

#### e. `process_children` function in the child processing section
```rust
// Change parameter from:
variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>

// To:
variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>
```

### 3. Ensure VariantSignature Implements Ord

**CRITICAL**: VariantSignature depends on StructFieldName, which must implement Ord first.

#### Step 3a: Add Ord to StructFieldName (prerequisite)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct StructFieldName(String);
```

#### Step 3b: Add Ord to VariantSignature
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum VariantSignature {
    Unit,
    Tuple(Vec<BrpTypeName>),
    Struct(Vec<(StructFieldName, BrpTypeName)>),
}
```

**Location**: Both changes in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

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

## Design Review Skip Notes

## DESIGN-2: Missing consistency analysis across codebase - **Verdict**: CONFIRMED
- **Status**: SKIPPED
- **Location**: Section: Why This Works
- **Issue**: The plan only addresses enum_path_builder.rs but doesn't analyze whether similar non-deterministic HashMap usage exists elsewhere in the mutation path building system that could cause the same test instability.
- **Reasoning**: Investigation found multiple HashMap iteration patterns beyond the one addressed in the plan. Struct builders and other components may also have non-deterministic iteration that contributes to test instability.
- **Decision**: User elected to skip this recommendation

## DESIGN-3: Missing Ord traits prevent BTreeMap usage for deterministic ordering - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Why This Works
- **Issue**: The plan expects BTreeMap usage for deterministic iteration, but VariantSignature lacks Ord/PartialOrd traits required for BTreeMap keys. Additionally, StructFieldName (used in VariantSignature::Struct) also lacks these traits, preventing compilation.
- **Existing Implementation**: Section 3: Ensure VariantSignature Implements Ord already addresses this with Step 3a (StructFieldName Ord implementation) and Step 3b (VariantSignature Ord implementation)
- **Plan Section**: Section: Ensure VariantSignature Implements Ord
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting