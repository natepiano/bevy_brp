# Plan: Improved Enum Variant Signature Knowledge - Single Choke Point

## Problem with Current Implementation

The current implementation applies knowledge in `build_variant_group_example` which:
1. Only affects the HashMap used for root example assembly
2. Does NOT update the child path's own `.example` field
3. Results in `.0` path showing π (3.14...) instead of 0.5
4. Requires checking knowledge in two places (ugly, no "taste")

**Current state**: Root example correctly shows `{"Mask": 0.5}`, but `.0` path's example still shows `3.1415927...`

## Root Cause

The f32 child gets its example from `RecursionContext::find_knowledge()` during recursion, but at that point it doesn't know it's part of an enum signature. The enum builder applies knowledge later during assembly, but the child path object already has the wrong value.

## Solution: Single Choke Point via Context Propagation

Add parent enum signature information to `RecursionContext` so that when the child calls `find_knowledge()`, it can check signature-specific knowledge first. This creates a single point where ALL knowledge is applied.

### Benefits
1. **Single source of truth**: Knowledge checked in ONE place (`find_knowledge`)
2. **Automatic propagation**: Both child path example AND parent assembly get the correct value
3. **Clean code**: No manual syncing, no duplicate logic
4. **Extensible**: Works for struct variant fields too

## Implementation Steps

### Step 1: Back Out Current Changes

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Remove the knowledge application logic from `build_variant_group_example`:

```rust
// REMOVE these lines (630-641):
        // Apply signature-specific knowledge to override child examples
        // Mutate in place since we own child_examples now
        if let VariantSignature::Tuple(types) = signature {
            for (index, _type_name) in types.iter().enumerate() {
                if let Some(knowledge_value) =
                    check_signature_element_knowledge(ctx.type_name(), signature, index)?
                {
                    let descriptor = MutationPathDescriptor::from(index.to_string());
                    child_examples.insert(descriptor, knowledge_value);
                }
            }
        }
```

Change function signature back to borrowing:
```rust
// Change from:
fn build_variant_group_example(
    signature: &VariantSignature,
    variants_in_group: &[EnumVariantInfo],
    mut child_examples: HashMap<MutationPathDescriptor, Value>,  // owns
    ...

// Back to:
fn build_variant_group_example(
    signature: &VariantSignature,
    variants_in_group: &[EnumVariantInfo],
    child_examples: &HashMap<MutationPathDescriptor, Value>,  // borrows
    ...
```

Update call site back to borrowing:
```rust
// Change from:
let example = build_variant_group_example(
    signature,
    variants_in_group,
    child_examples,  // pass ownership
    ...

// Back to:
let example = build_variant_group_example(
    signature,
    variants_in_group,
    &child_examples,  // borrow
    ...
```

Remove the `check_signature_element_knowledge` helper function (lines 582-611) - we'll integrate this into `find_knowledge` instead.

### Step 2: Add Parent Variant Signature to Context

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

Add field to `RecursionContext` to track parent enum variant signature:

```rust
pub struct RecursionContext {
    pub path_kind:          PathKind,
    pub registry:           Arc<HashMap<BrpTypeName, Value>>,
    pub full_mutation_path: FullMutationPath,
    pub path_action:        PathAction,
    pub variant_chain:      Vec<VariantName>,
    pub depth:              RecursionDepth,
    /// Parent enum variant signature (only set when processing enum variant children)
    /// The enum type is available via path_kind.parent_type - no need to store it redundantly
    pub parent_variant_signature: Option<VariantSignature>,  // NEW
}
```

Update `new()` method:
```rust
pub fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
    Self {
        path_kind,
        registry,
        full_mutation_path: FullMutationPath::from(""),
        path_action: PathAction::Create,
        variant_chain: Vec::new(),
        depth: RecursionDepth::ZERO,
        parent_variant_signature: None,  // NEW
    }
}
```

Update `create_recursion_context()` to propagate parent variant signature:
```rust
pub fn create_recursion_context(
    &self,
    path_kind: PathKind,
    child_path_action: PathAction,
) -> std::result::Result<Self, BuilderError> {
    // ... existing depth checking ...

    Ok(Self {
        path_kind,
        registry: Arc::clone(&self.registry),
        full_mutation_path: new_path_prefix,
        path_action,
        variant_chain: self.variant_chain.clone(),
        depth: new_depth,
        parent_variant_signature: self.parent_variant_signature.clone(),  // NEW: inherit from parent
    })
}
```

### Step 3: Update find_knowledge() to Check Enum Signature

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

Update the `find_knowledge()` method to check enum signature knowledge for `IndexedElement`:

```rust
pub fn find_knowledge(&self) -> Option<&'static super::mutation_knowledge::MutationKnowledge> {
    match &self.path_kind {
        PathKind::StructField { field_name, parent_type, .. } => {
            // Existing struct field logic...
        }
        PathKind::IndexedElement { index, parent_type, .. } => {
            // NEW: Check if we're a child of an enum variant signature
            if let Some(parent_sig) = &self.parent_enum_signature {
                // Validate index is within bounds for tuple signatures
                if let VariantSignature::Tuple(types) = &parent_sig.signature {
                    if *index >= types.len() {
                        tracing::warn!(
                            "Knowledge index {} out of bounds for enum {} tuple signature with {} elements",
                            index,
                            parent_sig.enum_type.display_name(),
                            types.len()
                        );
                    } else {
                        let key = KnowledgeKey::enum_variant_signature(
                            parent_sig.enum_type.clone(),
                            parent_sig.signature.clone(),
                            *index,
                        );
                        if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&key) {
                            tracing::debug!(
                                "Found enum signature match for {}[{}] in enum {}: {:?}",
                                parent_type.display_name(),
                                index,
                                parent_sig.enum_type.display_name(),
                                knowledge.example()
                            );
                            return Some(knowledge);
                        }
                    }
                }
            }
            // Fall through to exact type match
        }
        PathKind::RootValue { .. } | PathKind::ArrayElement { .. } => {
            // Existing logic...
        }
    }

    // Exact type match as fallback
    let exact_key = KnowledgeKey::exact(self.type_name());
    BRP_MUTATION_KNOWLEDGE.get(&exact_key).map_or_else(|| None, Some)
}
```

Add import at top of file:
```rust
use super::types::ParentEnumSignature;
```

### Step 4: Set Parent Enum Signature in Enum Builder

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Update `process_signature_path` to set the parent enum signature when creating child contexts:

```rust
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],
    signature: &VariantSignature,  // NEW parameter
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    let mut child_ctx = ctx.create_recursion_context(path.clone(), PathAction::Create)?;

    // NEW: Set parent enum signature context for the child
    child_ctx.parent_enum_signature = Some(ParentEnumSignature {
        enum_type: ctx.type_name().clone(),
        signature: signature.clone(),
    });

    // Set up enum context for children - just push the variant name
    if let Some(representative_variant) = applicable_variants.first() {
        child_ctx.variant_chain.push(representative_variant.clone());
    }

    // ... rest of function unchanged ...
}
```

Update call site in `process_children` to pass signature:

```rust
// Process each path
for path in paths.into_iter().flatten() {
    let child_paths = process_signature_path(
        path,
        &applicable_variants,
        signature,  // NEW: pass signature
        ctx,
        &mut child_examples,
    )?;
    signature_child_paths.extend(child_paths);
}
```

Add import at top of file:
```rust
use super::types::ParentEnumSignature;
```

## Testing

After implementation:

1. **Verify `.0` path shows 0.5**:
   ```bash
   # Generate type guide for AlphaMode2d
   # Check that `.0` path example shows 0.5 (not π)
   ```

2. **Verify root example shows 0.5**:
   ```bash
   # Check that root example shows {"Mask": 0.5}
   ```

3. **Verify other f32 fields still use π**:
   ```bash
   # Generate type guide for Transform
   # Check that .translation.x/y/z still show π
   ```

4. **Run mutation test**:
   ```bash
   # Test TilemapChunk mutation - should not crash
   ```

## Expected Behavior

**Before (current broken state)**:
- Root example: `{"Mask": 0.5}` ✅ (correct due to HashMap override)
- `.0` path example: `3.1415927...` ❌ (wrong, still has π)

**After (fixed)**:
- Root example: `{"Mask": 0.5}` ✅ (uses child's example during assembly)
- `.0` path example: `0.5` ✅ (gets it from find_knowledge)

## Benefits of This Approach

1. **Single choke point**: ALL knowledge application happens in `find_knowledge()`
2. **Automatic consistency**: Child example and parent assembly always match
3. **Clean code**: No manual syncing, no duplicate logic
4. **Type-safe**: Leverages existing `VariantSignature` type
5. **Extensible**: Will work for struct variant fields when needed
6. **Debuggable**: Single place to add tracing/logging for knowledge application

## Migration Notes

This is a pure refactor with no breaking changes to the knowledge system itself. The `EnumVariantSignature` key and knowledge entries remain unchanged - we're just moving WHERE the knowledge is checked to a better location in the code flow.
