# Plan: Add Enum Variant Signature Knowledge Support

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

### Step 1: Implement Enum Variant Signature Knowledge Support ⏳ PENDING

**Objective:** Add signature-level knowledge targeting to fix the `AlphaMode2d::Mask(f32)` crash issue.

**Changes:**
1. Add `EnumVariantSignature` variant to `KnowledgeKey` enum
2. Add knowledge entry for `AlphaMode2d::Mask` targeting index 0 with value 0.5
3. Add `check_signature_element_knowledge` helper function with bounds validation
4. Modify `build_variant_group_example` to apply signature knowledge
5. Add `VariantSignature` import

**Files:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Change Type:** Additive (all changes are new additions, no breaking changes)

**Build Command:**
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact:**
- `AlphaMode2d::Mask(f32)` will use 0.5 instead of π (3.14159...)
- Other f32 fields remain unaffected
- Mutation test for `TilemapChunk` should pass without crash

### Step 2: Complete Validation ⏳ PENDING

**Objective:** Verify the implementation works correctly.

**Validation Steps:**
1. Run mutation test for `TilemapChunk` - should pass without crash
2. Generate type guide and verify `.alpha_mode` example shows `{"Mask": 0.5}`
3. Verify other f32 fields (e.g., `Transform.translation.x`) still use π

**Expected Result:** All tests pass, knowledge is applied correctly and surgically.

---

## Design Review Skip Notes

### DESIGN-1: Inconsistent type usage: BrpTypeName vs String in KnowledgeKey - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Solution: Add Signature-Level Knowledge
- **Issue**: The plan proposes EnumVariantSignature.enum_type as BrpTypeName, but the current KnowledgeKey implementation uses String for both Exact and StructField.struct_type
- **Reasoning**: The finding was based on stale code analysis. The actual current code (as of this review) already uses BrpTypeName for both Exact and StructField variants. The plan is correct and consistent with the current implementation.
- **Decision**: The plan's use of BrpTypeName is correct and no changes are needed

## Problem

The mutation test for `bevy_sprite_render::tilemap_chunk::TilemapChunk` crashes when mutating `.alpha_mode` because the type guide uses π (3.14159...) as the example value for `AlphaMode2d::Mask(f32)`. The `Mask` variant expects an alpha threshold in the range 0.0-1.0, but the default f32 value (π) is outside this range and causes the app to crash.

## Root Cause

The current `MutationKnowledge` system can target:
- Entire types via `KnowledgeKey::Exact` (e.g., all f32 globally)
- Struct fields via `KnowledgeKey::StructField` (e.g., `Camera3d.depth_texture_usages`)

But it **cannot** target tuple elements within enum variants. The signature `Tuple(vec![f32])` in `AlphaMode2d` needs a valid threshold value (0.0-1.0), but we have no way to specify this without affecting all f32 types globally.

## Why Existing Approaches Don't Work

### Approach 1: Target the struct field `.alpha_mode`
Using `KnowledgeKey::StructField` for `TilemapChunk.alpha_mode` would only affect the struct-level example, but the enum builder processes children independently and wouldn't see this knowledge.

### Approach 2: Target f32 directly
Using `KnowledgeKey::Exact("f32")` would affect **all** f32 fields globally, breaking other valid uses of π.

### Approach 3: Add knowledge checking to child recursion
The f32 child is processed through normal recursion, but at that point we've lost the context that it's part of a specific enum signature.

## Solution: Add Signature-Level Knowledge

Add a new `KnowledgeKey` variant that targets indexed elements within enum variant signatures:

```rust
pub enum KnowledgeKey {
    Exact(BrpTypeName),
    StructField {
        struct_type: BrpTypeName,
        field_name:  StructFieldName,
    },
    /// Match an indexed element within enum variants that share a signature
    EnumVariantSignature {
        enum_type: BrpTypeName,      // "bevy_sprite_render::mesh2d::material::AlphaMode2d"
        signature: VariantSignature, // Tuple(vec![BrpTypeName("f32")])
        index: usize,                // 0 (the f32 element)
    },
}
```

### Why This Works

1. **Matches by signature, not variant name**: Any variant in `AlphaMode2d` with signature `Tuple(vec![f32])` will match
2. **Doesn't affect other f32 usage**: Only f32 in this specific enum signature is affected
3. **Natural integration point**: The enum builder already groups by signature and has all needed context

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Implementation Steps

### 1. Add `EnumVariantSignature` to `KnowledgeKey`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`

```rust
pub enum KnowledgeKey {
    Exact(BrpTypeName),
    StructField {
        struct_type: BrpTypeName,
        field_name:  StructFieldName,
    },
    EnumVariantSignature {
        enum_type: BrpTypeName,
        signature: VariantSignature,
        index: usize,
    },
}

impl KnowledgeKey {
    // ... existing methods ...

    pub fn enum_variant_signature(
        enum_type: impl Into<BrpTypeName>,
        signature: VariantSignature,
        index: usize,
    ) -> Self {
        Self::EnumVariantSignature {
            enum_type: enum_type.into(),
            signature,
            index,
        }
    }
}
```

### 2. Add Knowledge Entry for `AlphaMode2d::Mask`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`

In the `BRP_MUTATION_KNOWLEDGE` map initialization section, add this entry among the other knowledge entries:

```rust
// ===== AlphaMode2d enum variant signatures =====
// Mask(f32) variant requires alpha threshold in 0.0-1.0 range
map.insert(
    KnowledgeKey::enum_variant_signature(
        "bevy_sprite_render::mesh2d::material::AlphaMode2d",
        VariantSignature::Tuple(vec![BrpTypeName::from("f32")]),
        0,
    ),
    MutationKnowledge::as_root_value(json!(0.5), TYPE_F32),
);
```

### 3. Add Signature Knowledge Check in Enum Builder

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Why check here instead of in `RecursionContext::find_knowledge()`?**

The signature knowledge check happens in `enum_path_builder.rs` because this is where the `VariantSignature` information is naturally available. When `find_knowledge()` processes a tuple element child (like the f32 in `AlphaMode2d::Mask(f32)`), it only has access to the child type and index, not the parent variant's signature. The signature grouping logic exists in the enum builder, so checking knowledge there leverages existing context without requiring complex signature reconstruction from child contexts.

**Add one helper function immediately before the `build_variant_group_example` function:**

```rust
/// Check if there's mutation knowledge for a specific signature element
///
/// Returns `Ok(Some(value))` if knowledge exists, `Ok(None)` if no knowledge,
/// or `Err` if the knowledge index is out of bounds for the signature.
fn check_signature_element_knowledge(
    enum_type: &BrpTypeName,
    signature: &VariantSignature,
    index: usize,
) -> std::result::Result<Option<Value>, BuilderError> {
    // Validate index is within bounds for tuple signatures
    if let VariantSignature::Tuple(types) = signature {
        if index >= types.len() {
            return Err(BuilderError::SystemError(Report::new(Error::InvalidState(
                format!(
                    "Knowledge index {index} out of bounds for enum {} tuple signature with {} elements",
                    enum_type.display_name(),
                    types.len()
                )
            ))));
        }
    }

    let key = KnowledgeKey::enum_variant_signature(
        enum_type.clone(),
        signature.clone(),
        index,
    );

    Ok(BRP_MUTATION_KNOWLEDGE
        .get(&key)
        .map(|knowledge| knowledge.example().clone()))
}
```

**Then modify the `build_variant_group_example` function signature** to take ownership of `child_examples` instead of borrowing it:

Change:
```rust
fn build_variant_group_example(
    signature: &VariantSignature,
    variants_in_group: &[EnumVariantInfo],
    child_examples: &HashMap<MutationPathDescriptor, Value>,  // <- change this
    signature_status: MutationStatus,
    ctx: &RecursionContext,
) -> std::result::Result<Option<Value>, BuilderError>
```

To:
```rust
fn build_variant_group_example(
    signature: &VariantSignature,
    variants_in_group: &[EnumVariantInfo],
    mut child_examples: HashMap<MutationPathDescriptor, Value>,  // <- takes ownership now
    signature_status: MutationStatus,
    ctx: &RecursionContext,
) -> std::result::Result<Option<Value>, BuilderError>
```

**Then locate the code block** where it checks `matches!(signature_status, MutationStatus::NotMutable)` and builds the variant example. Replace that section:

```rust
// Only build example for mutable variants
// `NotMutable` variants get None (field omitted from JSON)
let example = if matches!(signature_status, MutationStatus::NotMutable) {
    None // Omit example field entirely for unmutable variants
} else {
    Some(build_variant_example(
        signature,
        representative.name(),
        child_examples,
        ctx.type_name(),
    ))
};
```

**With this updated version that applies signature knowledge:**

```rust
// Only build example for mutable variants
// `NotMutable` variants get None (field omitted from JSON)
let example = if matches!(signature_status, MutationStatus::NotMutable) {
    None
} else {
    // Apply signature-specific knowledge to override child examples
    // Mutate in place since we own child_examples now
    if let VariantSignature::Tuple(types) = signature {
        for (index, _type_name) in types.iter().enumerate() {
            if let Some(knowledge_value) = check_signature_element_knowledge(
                ctx.type_name(),
                signature,
                index
            )? {
                let descriptor = MutationPathDescriptor::from(index.to_string());
                child_examples.insert(descriptor, knowledge_value);
            }
        }
    }

    Some(build_variant_example(
        signature,
        representative.name(),
        &child_examples,
        ctx.type_name(),
    ))
};
```

**Update the call site:** In the `process_children` function, change the call from `&child_examples` to `child_examples` (passing ownership):

```rust
let example = build_variant_group_example(
    signature,
    variants_in_group,
    child_examples,  // <- pass ownership instead of &child_examples
    signature_status,
    ctx,
)?;
```

### 4. Import `VariantSignature` in `mutation_knowledge.rs`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`

Add this import after the existing `use crate::brp_tools::BrpTypeName;` import and before the `use crate::brp_tools::brp_type_guide::constants` block:

```rust
use super::types::VariantSignature;
```

This import is needed because `EnumVariantSignature` variant includes a `VariantSignature` field.

## Testing

After implementation:

1. Run type guide generation for `TilemapChunk`
2. Verify `.alpha_mode` example shows `{"Mask": 0.5}` instead of `{"Mask": 3.14...}`
3. Run mutation test on `TilemapChunk` - should pass without crash
4. Verify that f32 fields OUTSIDE the targeted enum signature still use π:
   - Generate type guide for a type with regular f32 fields (e.g., `bevy_transform::components::transform::Transform`)
   - Check that `.translation.x`, `.translation.y`, `.translation.z` examples use π (3.14159...)
   - Verify that only `AlphaMode2d::Mask(f32)` uses 0.5, not all f32 fields globally

## Benefits

1. **Surgical precision**: Only affects the specific enum signature that needs it
2. **Natural integration**: Leverages existing signature grouping in enum builder
3. **Extensible**: Can be used for other enum signature issues in the future
4. **Type-safe**: Uses existing `VariantSignature` type rather than string matching

## Alternative Considered: Index-Level Knowledge in `find_knowledge`

We considered adding lookup logic in `RecursionContext::find_knowledge` for `PathKind::IndexedElement`, but this wouldn't work because:

1. The signature information doesn't exist in the recursion context
2. Would require reconstructing signature from parent schema (complex and error-prone)
3. The enum builder already has the signature readily available

The current approach is cleaner because it checks knowledge at the point where we have all the information we need.
