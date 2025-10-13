# Plan: Add Enum Variant Signature Knowledge Support

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

In the `BRP_MUTATION_KNOWLEDGE` map initialization (around line 527):

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

Add a new helper function before `process_children`:

```rust
/// Check if there's mutation knowledge for a specific signature element
fn check_signature_element_knowledge(
    enum_type: &BrpTypeName,
    signature: &VariantSignature,
    index: usize,
) -> Option<Value> {
    let key = KnowledgeKey::enum_variant_signature(
        enum_type.clone(),
        signature.clone(),
        index,
    );

    BRP_MUTATION_KNOWLEDGE
        .get(&key)
        .map(|knowledge| knowledge.example().clone())
}
```

Then modify `process_children` at **lines 604-613**:

```rust
// Only build example for mutable variants
// NotMutable variants get None (field omitted from JSON)
let example = if matches!(signature_status, MutationStatus::NotMutable) {
    None
} else {
    // Check for signature-specific knowledge that overrides child examples
    let example_with_knowledge = apply_signature_knowledge(
        ctx.type_name(),
        signature,
        &child_examples,
    );

    Some(build_variant_example(
        signature,
        representative.name(),
        &example_with_knowledge,
        ctx.type_name(),
    ))
};
```

Add the helper function:

```rust
/// Apply signature-specific knowledge to child examples
fn apply_signature_knowledge(
    enum_type: &BrpTypeName,
    signature: &VariantSignature,
    child_examples: &HashMap<MutationPathDescriptor, Value>,
) -> HashMap<MutationPathDescriptor, Value> {
    let mut examples = child_examples.clone();

    // Only applies to tuple signatures
    if let VariantSignature::Tuple(types) = signature {
        for (index, _type_name) in types.iter().enumerate() {
            if let Some(knowledge_value) = check_signature_element_knowledge(enum_type, signature, index) {
                let descriptor = MutationPathDescriptor::from(index.to_string());
                examples.insert(descriptor, knowledge_value);
            }
        }
    }

    examples
}
```

### 4. Import `VariantSignature` in `mutation_knowledge.rs`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`

Add to imports:

```rust
use super::types::VariantSignature;
```

## Testing

After implementation:

1. Run type guide generation for `TilemapChunk`
2. Verify `.alpha_mode` example shows `{"Mask": 0.5}` instead of `{"Mask": 3.14...}`
3. Run mutation test on `TilemapChunk` - should pass without crash
4. Verify other f32 fields still use π correctly

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
