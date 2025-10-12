# Per-Variant Mutation Status Implementation Plan

## Problem Statement

Currently, enum types are marked as fully "mutable" if ANY variant is settable (including unit variants), even when some variants contain fields that lack required traits and cannot actually be mutated.

**Example**: `Option<NodeIndex>` where `NodeIndex` lacks required traits:
- `None` variant: Can be set (mutable)
- `Some(NodeIndex)` variant: Cannot be mutated (not mutable due to missing traits)

Current behavior incorrectly marks the entire enum as "mutable", creating false positives.

## Proposed Design

### 1. Per-Variant Granularity

Each `ExampleGroup` gets its own `mutation_status` field:
- `"mutable"` - this variant can be set/mutated
- `"partially_mutable"` - some fields in variant are mutable, others not
- `"not_mutable"` - this variant cannot be mutated

### 2. Overall Enum Status

The path-level `mutation_status` aggregates variant statuses:
- `"mutable"` - ALL variants are mutable
- `"partially_mutable"` - SOME variants mutable, others not (OR some variants are partially_mutable)
- `"not_mutable"` - NO variants are mutable

### 3. Example Output

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
      // Note: "example" field omitted for not_mutable variants
    }
  ]
}
```

**Key Behaviors**:
- Unit variants like `None`: `example: null` (this is the BRP mutation value)
- Mutable non-unit variants: `example: <constructed value>`
- NotMutable variants: `example` field omitted entirely (serde skip_serializing_if)

## Implementation Steps

### Step 1: Extract Pure Aggregation Logic

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Create new helper function before `determine_parent_mutation_status`:

```rust
/// Aggregate multiple mutation statuses into a single status
///
/// Logic:
/// - If any partially_mutable OR (has both mutable and not_mutable) → PartiallyMutable
/// - Else if any not_mutable → NotMutable
/// - Else → Mutable
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

### Step 2: Refactor `determine_parent_mutation_status`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Update function to use new aggregation helper (around line 218):

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

### Step 3: Add `mutation_status` to `ExampleGroup` and Make `example` Optional

**⚠️ BREAKING CHANGE - Must be done atomically with Step 4**

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

Update `ExampleGroup` struct (around line 280):

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

**Critical Constraint**: The `example` field must be:
- `Some(Value::Null)` for mutable unit variants (e.g., `None`) - this is how BRP sets them
- `Some(Value)` for mutable non-unit variants with constructed values
- `None` for NotMutable variants - field omitted from JSON entirely

**⚠️ COMPILATION IMPACT**:
This change will cause compilation errors at all ExampleGroup construction sites:
- `enum_path_builder.rs` line 589-593: `ExampleGroup { example, ... }` expects `Option<Value>`

**DO NOT ATTEMPT TO BUILD** after this step alone - the code will not compile until Step 4 is complete.
Step 4 updates the construction site to provide `Option<Value>`.

**These steps form an ATOMIC GROUP - both must be completed before building.**

### Step 4: Collect Signature Mutation Statuses and Fix Compilation

**⚠️ COMPLETES ATOMIC GROUP WITH STEP 3**

This step fixes the compilation errors caused by Step 3's type signature change.

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Update `process_children` function (around line 470):

```rust
fn process_children(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError> {
    let mut all_examples = Vec::new();
    let mut all_child_paths = Vec::new();

    // Process each variant group
    for (signature, variants_in_group) in variant_groups {
        let mut child_examples = HashMap::new();

        let applicable_variants: Vec<VariantName> = variants_in_group
            .iter()
            .map(|v| v.variant_name().clone())
            .collect();

        let paths = create_paths_for_signature(signature, ctx);

        for path in paths.into_iter().flatten() {
            // ... existing recursion code ...
            let mut child_paths = builder::recurse_mutation_paths(child_type_kind, &child_ctx)?;

            // ... rest of existing code ...
            all_child_paths.extend(child_paths);
        }

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
            example,                            // Now Option<Value> - fixes Step 3 compilation error
            mutation_status: signature_status,  // NEW
        });
    }

    // ... rest of function ...
}
```

**✅ ATOMIC GROUP COMPLETE**: After completing Steps 3 and 4 together, the code will compile successfully.
The struct definition (Step 3) and all construction sites (Step 4) are now aligned.

### Step 5: Aggregate Signature Statuses for Overall Enum Status

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Replace the current logic in `create_result_paths` (lines 907-924):

```rust
fn create_result_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_paths: Vec<MutationPathInternal>,
    partial_roots: BTreeMap<Vec<VariantName>, Value>,
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
) -> Vec<MutationPathInternal> {
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

    // ... rest of function remains the same ...
}
```

## Testing Strategy

### Test Cases

1. **Fully Mutable Enum**: All variants mutable → overall `"mutable"`
   - Example: `enum SimpleEnum { A, B, C }` (all unit variants)

2. **Partially Mutable Enum**: Mixed variant mutability → overall `"partially_mutable"`
   - Example: `Option<NodeIndex>` (None mutable, Some not mutable)
   - Example: `Option<Transform>` (both mutable but Some is partially_mutable due to nested fields)

3. **Not Mutable Enum**: No variants mutable → overall `"not_mutable"`
   - Example: Enum wrapping only types without required traits

4. **Nested Enums**: Verify status propagation through multiple enum levels
   - Example: `Option<Option<T>>` with various T types

### Validation Points

- Each `ExampleGroup` has correct `mutation_status` based on its fields
- Overall enum `mutation_status` correctly aggregates signature statuses
- `mutation_status_reason` provides clear explanation for partial/not mutable cases
- Existing tests continue to pass (no regression)

## Migration Notes

### Breaking Changes

- `ExampleGroup` struct gains new `mutation_status` field
  - JSON output will include this field for all enum examples
  - Existing parsers expecting old format will need updates

### Backward Compatibility

- Non-enum types unchanged
- Enum JSON structure extended (not replaced)
- New field provides additional information without removing existing data

## Benefits

1. **Accuracy**: No more false positives where enums appear fully mutable when they're not
2. **Granularity**: Users can see exactly which variants are usable
3. **Discoverability**: Clear feedback about why certain variants can't be mutated
4. **Consistency**: Uses same aggregation logic as struct/tuple/list types
5. **Extensibility**: Foundation for future variant-specific mutation operations

## Implementation Order

1. Extract `aggregate_mutation_statuses` helper ✓
2. Refactor `determine_parent_mutation_status` to use it ✓
3. Add field to `ExampleGroup` ✓
4. Collect signature statuses in `process_children` ✓
5. Aggregate for overall enum status ✓
6. Test with `Option<NodeIndex>` and other mixed-mutability enums
7. Update documentation and help text
