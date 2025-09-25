# Plan: Set Standard Order to Fix Non-Deterministic Enum Variants

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Code examples showing before/after
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for user confirmation ("go ahead" or similar)

3. **IMPLEMENT**: Make the changes and stop

4. **BUILD & VALIDATE**: Execute the build process:
   ```bash
   cargo build && cargo +nightly fmt
   ```

5. **CONFIRM**: Wait for user to confirm the build succeeded

6. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

7. **PROCEED**: Move to next step only after confirmation
</Instructions>

<ExecuteImplementation>
    Find the next ⏳ PENDING step in the INTERACTIVE IMPLEMENTATION SEQUENCE below.

    For the current step:
    1. Follow the <Instructions/> above for executing the step
    2. When step is complete, use Edit tool to mark it as ✅ COMPLETED
    3. Continue to next PENDING step

    If all steps are COMPLETED:
        Display: "✅ Implementation complete! All steps have been executed."
</ExecuteImplementation>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### Step 1: Add Ord Trait Prerequisites [ATOMIC GROUP] - ⏳ PENDING

**Objective**: Add PartialOrd and Ord derives to StructFieldName and VariantSignature to enable BTreeMap usage

**Changes Required**:
- Add `PartialOrd, Ord` to StructFieldName derive attributes
- Add `PartialOrd, Ord` to VariantSignature derive attributes

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Code Changes**:
```rust
// BEFORE:
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructFieldName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariantSignature {
    Unit,
    Tuple(Vec<BrpTypeName>),
    Struct(Vec<(StructFieldName, BrpTypeName)>),
}

// AFTER:
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct StructFieldName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum VariantSignature {
    Unit,
    Tuple(Vec<BrpTypeName>),
    Struct(Vec<(StructFieldName, BrpTypeName)>),
}
```

**Build Command**: `cargo build && cargo +nightly fmt`

**Expected Impact**: Enables VariantSignature to be used as BTreeMap keys, providing deterministic ordering

**Notes**: Both types must be modified together since VariantSignature depends on StructFieldName implementing Ord

---

### Step 2: Replace HashMap with BTreeMap [SAFE] - ⏳ PENDING

**Objective**: Replace HashMap with BTreeMap in enum variant processing to get deterministic iteration order

**Changes Required**:
- Update import statement to include BTreeMap
- Change function return types from HashMap to BTreeMap
- Change function parameters from HashMap to BTreeMap
- Update HashMap::new() calls to BTreeMap::new()

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Code Changes**:
```rust
// BEFORE:
use std::collections::HashMap;

fn group_variants_by_signature(
    variants: Vec<EnumVariantInfo>,
) -> HashMap<VariantSignature, Vec<EnumVariantInfo>> {
    let mut groups = HashMap::new();
    // ...
}

// AFTER:
use std::collections::{BTreeMap, HashMap};

fn group_variants_by_signature(
    variants: Vec<EnumVariantInfo>,
) -> BTreeMap<VariantSignature, Vec<EnumVariantInfo>> {
    let mut groups = BTreeMap::new();
    // ...
}
```

**Additional Function Updates**:
- `extract_and_group_variants`: Change return type to `Result<BTreeMap<VariantSignature, Vec<EnumVariantInfo>>>`
- `build_enum_examples`: Change parameter type to `&BTreeMap<VariantSignature, Vec<EnumVariantInfo>>`
- `concrete_example`: Change parameter type to `&BTreeMap<VariantSignature, Vec<EnumVariantInfo>>`
- `process_children`: Change parameter type to `&BTreeMap<VariantSignature, Vec<EnumVariantInfo>>`

**Build Command**: `cargo build && cargo +nightly fmt`

**Expected Impact**: Eliminates 304+ false positive VALUE_CHANGE patterns in mutation tests due to deterministic iteration order

**Dependencies**: Requires Step 1 (Ord traits needed for BTreeMap keys)

---

### Step 3: Complete Validation - ⏳ PENDING

**Objective**: Verify that the changes eliminate non-deterministic behavior in mutation tests

**Validation Steps**:
1. Run complete test suite to ensure no regressions
2. Run mutation test generation twice to verify identical output
3. Compare mutation paths to confirm no differences
4. Verify 304+ VALUE_CHANGE false positives are eliminated

**Build & Test Commands**:
```bash
cargo nextest run
/create_mutation_test_json
/create_mutation_test_json  # Run second time
/compare_mutation_path      # Should show no differences
```

**Success Criteria**:
- All existing tests pass
- Two consecutive `/create_mutation_test_json` runs produce identical output
- `/compare_mutation_path` shows no differences between runs
- Test instability from non-deterministic HashMap iteration is eliminated

**Expected Impact**: Stable, deterministic mutation test results without false positives

**Dependencies**: Requires Steps 1-2

---

## Problem
Non-deterministic iteration order in enum variant processing causes 304+ false positive VALUE_CHANGE patterns and 13 TYPE_CHANGE patterns in mutation tests. The issue stems from using `HashMap` which has random iteration order.

## Quick Fix Solution
Replace `HashMap` with `BTreeMap` in `enum_path_builder.rs` to get deterministic iteration order based on variant signatures. This will ensure consistent ordering across test runs until we implement the full variant chain solution.

## Why This Works
- `BTreeMap` iterates in sorted key order, providing deterministic iteration
- `VariantSignature` comparison will ensure consistent ordering based on variant structure:
  - Unit variants sort first (no fields)
  - Tuple variants sort by their type sequences
  - Struct variants sort by field names and types
- This eliminates the 304+ false positive VALUE_CHANGE patterns
- Minimal code change with immediate effect

## Impact Analysis
- **Positive**: Deterministic test results, no false positives
- **Neutral**: Slight performance impact (BTreeMap vs HashMap) but negligible for our use case
- **No Breaking Changes**: Output format remains the same, just ordering changes

## Alternative Considered
Could sort variants after extraction, but BTreeMap is cleaner and ensures consistency throughout the processing pipeline.

## Migration Strategy
**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

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