# Plan: Enum Variant Knowledge via Signature Matching

## Problem Statement

The current mutation knowledge system has a regression where newtype enum variants (like `Camera3dDepthLoadOp::Clear(f32)`) cannot properly provide specialized example values. The current exact-match lookup system fails because:

1. `MutationPathBuilder` only does exact type matches
2. Newtype variant knowledge requires both enum type + variant name + inner type
3. Variant names are not available in the `RecursionContext` during field processing

## Current Architecture

The `enum_builder.rs` groups variants by **signature** for deduplication:
- `Camera3dDepthLoadOp::Clear(f32)` → signature `tuple(f32)`
- `Camera3dDepthLoadOp::Load(f32)` → same signature `tuple(f32)`
- Only one example is generated per signature, showing all variants that share it

## Proposed Solution: Signature-Based Knowledge Matching

Instead of trying to track individual variant names through recursion, align the knowledge system with how `enum_builder` actually works.

### 1. New Knowledge Key Type

Add a new `KnowledgeKey` variant for enum signature matching:

```rust
pub enum KnowledgeKey {
    Exact(String),
    StructField { struct_type: String, field_name: String },

    // NEW: Match enum + signature pattern
    EnumSignature {
        enum_type: String,
        signature: VariantSignature,  // Reuse enum_builder's VariantSignature
    },
}
```

### 2. Knowledge Entries

Replace the current newtype variant entry:

```rust
// OLD: Requires variant name we don't have
KnowledgeKey::newtype_variant(
    "bevy_core_pipeline::core_3d::camera_3d::Camera3dDepthLoadOp",
    "Clear",  // ❌ Not available in context
    "f32",
)

// NEW: Uses signature pattern
KnowledgeKey::enum_signature(
    "bevy_core_pipeline::core_3d::camera_3d::Camera3dDepthLoadOp",
    VariantSignature::Tuple(vec![BrpTypeName::from("f32")])
)
```

The knowledge value specifies which variant to use:

```rust
MutationKnowledge::enum_example(json!({"Clear": 0.5}))
```

### 3. Unified Lookup Enhancement

Extend `RecursionContext::find_knowledge()` to handle enum signatures:

```rust
impl RecursionContext {
    pub fn find_knowledge(&self) -> Option<&'static MutationKnowledge> {
        // 1. Try exact type match
        if let Some(knowledge) = exact_match(self.type_name()) {
            return Some(knowledge);
        }

        // 2. Try struct field match
        if let PathKind::StructField { field_name, parent_type, .. } = &self.path_kind {
            if let Some(knowledge) = struct_field_match(parent_type, field_name) {
                return Some(knowledge);
            }
        }

        // 3. NEW: Try enum signature match
        if let PathKind::IndexedElement { parent_type, .. } = &self.path_kind {
            if let Some(knowledge) = enum_signature_match(parent_type, self.type_name()) {
                return Some(knowledge);
            }
        }

        None
    }

    fn enum_signature_match(&self, enum_type: &BrpTypeName, inner_type: &BrpTypeName) -> Option<&'static MutationKnowledge> {
        // Create signature for single-element tuple (newtype pattern)
        let signature = VariantSignature::Tuple(vec![inner_type.clone()]);
        let key = KnowledgeKey::enum_signature(enum_type.type_string(), signature);
        BRP_MUTATION_KNOWLEDGE.get(&key)
    }
}
```

### 4. Architecture After Migration

Once `enum_builder` is migrated, it will be wrapped by `MutationPathBuilder` like all other builders. This means:

1. **No direct integration needed**: `enum_builder` won't call knowledge lookup directly
2. **MutationPathBuilder handles everything**: All knowledge lookups happen via `ctx.find_knowledge()`
3. **Automatic enum signature matching**: When processing `PathKind::IndexedElement` with enum parent, `ctx.find_knowledge()` automatically checks for signature knowledge

## Benefits

1. **Aligns with existing architecture**: Uses the same signature concept as `enum_builder`
2. **No recursion context changes**: Works with existing `PathKind::IndexedElement`
3. **Extensible**: Can handle complex enum signatures (multiple tuple elements, struct variants)
4. **Backward compatible**: Existing exact and struct field matches continue working
5. **Performance**: Single HashMap lookup, no variant name iteration

## Migration Path

1. ✅ **Phase 1**: Implement unified lookup for exact + struct field (DONE)
2. **Phase 2**: Add `EnumSignature` key type and update knowledge entries
3. **Phase 3**: Extend `ctx.find_knowledge()` to handle enum signatures
4. **Phase 4**: Migrate `enum_builder` to be wrapped by `MutationPathBuilder`
5. **Phase 5**: Deprecate old `NewtypeVariant` key type and legacy lookup methods

## Test Cases

- `Camera3dDepthLoadOp::Clear(f32)` → signature `tuple(f32)` → returns `{"Clear": 0.5}`
- `Option<T>` variants → signature matching for `Some(T)` and `None`
- Complex tuple variants → `MyEnum::Config(String, i32)` → signature `tuple(String, i32)`
- Struct variants → `MyEnum::Settings{width: i32, height: i32}` → signature `struct{width: i32, height: i32}`

## Files to Modify

1. `mutation_knowledge.rs` - Add `EnumSignature` key type and update knowledge entries
2. `recursion_context.rs` - Extend `find_knowledge()` with enum signature matching
3. `enum_builder.rs` - Migrate to be wrapped by `MutationPathBuilder` (Phase 4)
4. Update knowledge entries from `NewtypeVariant` to `EnumSignature` format
