# Per-Variant Mutation Status - Collaborative Execution Plan

**Status**: Ready for implementation
**Original Plan**: `.claude/plans/mutation-status.md`
**Estimated Time**: 30-45 minutes

---

## Pre-Implementation Checklist

- [ ] Working directory is clean (`git status` shows no uncommitted changes)
- [ ] On correct branch (`main` or feature branch)
- [ ] All existing tests passing (`cargo nextest run`)
- [ ] Code compiles without warnings (`cargo build`)

---

## Implementation Sequence

### Phase 1: Extract Shared Aggregation Logic (Steps 1-2)

**These steps are independent and can be built/tested after each one**

#### Step 1: Add `aggregate_mutation_statuses` Helper

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Location**: Add directly before `determine_parent_mutation_status` function

**Action**: Add this function:

```rust
/// Aggregate multiple mutation statuses into a single status
///
/// Logic:
/// - If any `PartiallyMutable` OR (has both `Mutable` and `NotMutable`) → `PartiallyMutable`
/// - Else if any `NotMutable` → `NotMutable`
/// - Else → `Mutable`
pub fn aggregate_mutation_statuses(statuses: &[MutationStatus]) -> MutationStatus {
    let has_partially_mutable = statuses
        .iter()
        .any(|s| matches!(s, MutationStatus::PartiallyMutable));

    let has_mutable = statuses
        .iter()
        .any(|s| matches!(s, MutationStatus::Mutable));

    let has_not_mutable = statuses
        .iter()
        .any(|s| matches!(s, MutationStatus::NotMutable));

    if has_partially_mutable || (has_mutable && has_not_mutable) {
        MutationStatus::PartiallyMutable
    } else if has_not_mutable {
        MutationStatus::NotMutable
    } else {
        MutationStatus::Mutable
    }
}
```

**Checkpoint**:
```bash
cargo build
```
**Expected**: ✅ Builds successfully

---

#### Step 2: Refactor `determine_parent_mutation_status`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Location**: Around line 218 (the existing `determine_parent_mutation_status` function)

**Action**: Replace the function with:

```rust
pub fn determine_parent_mutation_status(
    ctx: &RecursionContext,
    child_paths: &[MutationPathInternal],
) -> (MutationStatus, Option<NotMutableReason>) {
    // Extract statuses and aggregate
    let statuses: Vec<MutationStatus> = child_paths
        .iter()
        .map(|p| p.mutation_status)
        .collect();

    let status = aggregate_mutation_statuses(&statuses);

    // Build detailed reason if not fully mutable
    let reason = match status {
        MutationStatus::PartiallyMutable => {
            let summaries: Vec<PathSummary> = child_paths
                .iter()
                .map(MutationPathInternal::to_path_summary)
                .collect();
            Some(NotMutableReason::from_partial_mutability(
                ctx.type_name().clone(),
                summaries,
            ))
        }
        MutationStatus::NotMutable => Some(ctx.create_no_mutable_children_error()),
        MutationStatus::Mutable => None,
    };

    (status, reason)
}
```

**Checkpoint**:
```bash
cargo build && cargo nextest run
```
**Expected**: ✅ Builds and all tests pass (this is a refactor, should not break anything)

---

### Phase 2: Add Per-Variant Mutation Status (Steps 3-4)

**⚠️ CRITICAL: Steps 3 and 4 MUST be completed together before building**

#### Step 3: Update `ExampleGroup` Struct

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
**Location**: Around line 280 (the `ExampleGroup` struct definition)

**Action**: Replace the struct definition with:

```rust
/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for NotMutable variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:             Option<Value>,  // CHANGED: Was Value, now Option<Value>
    /// The variant signature as a string
    pub signature:           String,
    /// Mutation status for this signature/variant group
    pub mutation_status:     MutationStatus,  // NEW FIELD
}
```

**⚠️ DO NOT BUILD YET** - Code will not compile until Step 4 is complete

---

#### Step 4: Update `process_children` to Determine Signature Status

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
**Location**: The `process_children` function (around line 470)

**Action**: Find the loop that processes variant groups and update it:

**FIND** (around line 491-593):
```rust
for (signature, variants_in_group) in variant_groups {
    let mut child_examples = HashMap::new();

    // ... existing code that builds child_examples and collects all_child_paths ...

    all_examples.push(ExampleGroup {
        applicable_variants,
        signature: signature.to_string(),
        example,  // Currently Value
    });
}
```

**REPLACE WITH**:
```rust
for (signature, variants_in_group) in variant_groups {
    let mut child_examples = HashMap::new();

    // ... existing code that builds child_examples and collects all_child_paths ...

    // NEW: Determine mutation status for this signature
    let signature_status = if matches!(signature, VariantSignature::Unit) {
        // Unit variants are always mutable (no fields to construct)
        MutationStatus::Mutable
    } else {
        // Aggregate field statuses from direct children at this depth
        // Filter all_child_paths to get only the direct children for this signature
        let signature_field_statuses: Vec<MutationStatus> = all_child_paths
            .iter()
            .filter(|p| p.is_direct_child_at_depth(*ctx.depth))
            .map(|p| p.mutation_status)
            .collect();

        if signature_field_statuses.is_empty() {
            // No fields (shouldn't happen, but handle gracefully)
            MutationStatus::Mutable
        } else {
            builder::aggregate_mutation_statuses(&signature_field_statuses)
        }
    };

    // Build example for this variant group
    let representative = variants_in_group
        .first()
        .ok_or_else(|| Report::new(Error::InvalidState("Empty variant group".to_string())))?;

    // NEW: Only build example for mutable variants
    // NotMutable variants get None (field omitted from JSON)
    let example = if matches!(signature_status, MutationStatus::NotMutable) {
        None  // Omit example field entirely for unmutable variants
    } else {
        Some(build_variant_example(
            signature,
            representative.name(),
            &child_examples,
            ctx.type_name(),
        ))
    };

    all_examples.push(ExampleGroup {
        applicable_variants,
        signature: signature.to_string(),
        example,                            // Now Option<Value>
        mutation_status: signature_status,  // NEW FIELD
    });
}
```

**Checkpoint**:
```bash
cargo build
```
**Expected**: ✅ Builds successfully (atomic group complete)

---

### Phase 3: Aggregate Enum-Level Status (Step 5)

#### Step 5: Update `create_result_paths`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
**Location**: The `create_result_paths` function (around line 907-924)

**Action**: Replace the mutation status determination logic:

**FIND** (lines 918-924):
```rust
let (enum_mutation_status, mutation_status_reason) = if has_unit_variant {
    (MutationStatus::Mutable, None)
} else {
    builder::determine_parent_mutation_status(ctx, &child_paths)
};
```

**REPLACE WITH**:
```rust
// NEW: Determine enum mutation status by aggregating signature statuses
let signature_statuses: Vec<MutationStatus> = enum_examples
    .iter()
    .map(|eg| eg.mutation_status)
    .collect();

let enum_mutation_status = builder::aggregate_mutation_statuses(&signature_statuses);

// NEW: Build reason for partially_mutable or not_mutable enums
let mutation_status_reason = match enum_mutation_status {
    MutationStatus::PartiallyMutable => {
        // Build reason explaining which variants are mutable vs not
        Some(json!({
            "reason": "enum_partial_mutability",
            "message": "Some variants are mutable while others are not",
            "variant_statuses": enum_examples.iter().map(|eg| {
                json!({
                    "signature": eg.signature,
                    "variants": eg.applicable_variants,
                    "mutation_status": eg.mutation_status
                })
            }).collect::<Vec<_>>()
        }))
    }
    MutationStatus::NotMutable => {
        // All variants are not mutable
        Some(json!({
            "reason": "enum_no_mutable_variants",
            "message": "No variants in this enum can be mutated"
        }))
    }
    MutationStatus::Mutable => None,
};
```

**Also remove** (around line 918): The `has_unit_variant` variable is no longer needed, you can delete these lines:
```rust
let has_unit_variant = variant_groups
    .keys()
    .any(|sig| matches!(sig, VariantSignature::Unit));
```

**Checkpoint**:
```bash
cargo build && cargo +nightly fmt
```
**Expected**: ✅ Builds successfully and code is formatted

---

## Validation

### Run Tests
```bash
cargo nextest run
```
**Expected**: All tests pass (may need to update tests that check enum mutation status)

### Manual Verification

Test with a type that has mixed variant mutability:

```bash
# Launch test app
cargo run --bin test-app

# In another terminal, query Option<NodeIndex> or similar enum
```

**Expected Output Example**:
```json
{
  "full_mutation_path": "",
  "mutation_status": "partially_mutable",
  "examples": [
    {
      "applicable_variants": ["Option<NodeIndex>::None"],
      "example": null,
      "signature": "unit",
      "mutation_status": "mutable"
    },
    {
      "applicable_variants": ["Option<NodeIndex>::Some"],
      "signature": "tuple(petgraph::graph::NodeIndex)",
      "mutation_status": "not_mutable"
      // Note: "example" field omitted
    }
  ]
}
```

---

## Rollback Instructions

If something goes wrong:

```bash
# Discard all changes
git checkout .

# Or revert specific files
git checkout mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
git checkout mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs
git checkout mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs
```

---

## Completion Checklist

- [ ] All 5 steps implemented
- [ ] Code builds without errors
- [ ] Code formatted with `cargo +nightly fmt`
- [ ] All tests passing
- [ ] Manual verification with mixed-mutability enum shows correct output
- [ ] Ready to commit

**Next Steps**:
- Run mutation tests if applicable
- Update documentation
- Create PR or commit to main
