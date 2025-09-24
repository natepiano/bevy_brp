# Plan 0 Prerequisite: Pass Signature Grouping Data Through Associated Types

## Goal
Eliminate redundant variant extraction and grouping in `enum_builder.rs` by passing the already-computed signature groups from `collect_children()` to `assemble_from_children()` via associated types.

## Current Problem
Currently, we extract and group enum variants **twice**:
1. In `collect_children()` - lines 350-353
2. In `assemble_from_children()` - lines 410-411

This is inefficient and violates DRY principles.

## Solution: Use Associated Types for Builder-Specific Data

### 1. Update PathBuilder Trait

```rust
// In path_builder.rs
pub trait PathBuilder {
    type Item: MaybeVariants;
    type Iter<'a>: Iterator<Item = Self::Item> where Self: 'a;

    // NEW: Each builder can specify what data it needs from collect phase
    type ChildrenData = HashMap<MutationPathDescriptor, Value>; // Default for most builders

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>>;

    // NEW: Transform collected data before assembly
    // Default implementation for builders that don't need special handling
    fn prepare_children_data(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        _ctx: &RecursionContext,
    ) -> Result<Self::ChildrenData> {
        Ok(children)  // Most builders just pass through
    }

    // CHANGED: Now accepts builder-specific data type
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: Self::ChildrenData,  // Now builder-specific!
    ) -> Result<Value>;
}
```

### 2. Define EnumChildrenData Type

```rust
// In enum_builder.rs
/// Data passed from collect phase to assembly phase for enum builders
#[derive(Debug)]
struct EnumChildrenData {
    /// The assembled examples by descriptor (from recursion)
    examples: HashMap<MutationPathDescriptor, Value>,

    /// The signature grouping extracted once during collect_children
    /// Map from signature to all variants with that signature
    signature_groups: HashMap<VariantSignature, Vec<EnumVariantInfo>>,
}
```

### 3. Implement for EnumMutationBuilder

```rust
impl PathBuilder for EnumMutationBuilder {
    type Item = PathKindWithVariants;
    type Iter<'a> = std::vec::IntoIter<PathKindWithVariants> where Self: 'a;

    // NEW: Specify we need enriched data
    type ChildrenData = EnumChildrenData;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let schema = ctx.require_registry_schema()?;

        // Extract and group variants ONCE
        let variants = extract_enum_variants(schema, &ctx.registry);
        let variant_groups = group_variants_by_signature(variants);

        // Store the groups for later use in prepare_children_data
        // Note: We'll need to pass this through somehow - see section 4

        let mut children = Vec::new();
        // ... rest of existing collect_children logic ...

        Ok(children.into_iter())
    }

    fn prepare_children_data(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        ctx: &RecursionContext,
    ) -> Result<Self::ChildrenData> {
        let schema = ctx.require_registry_schema()?;

        // Extract and group variants ONCE (we do it here since we have ctx)
        let variants = extract_enum_variants(schema, &ctx.registry);
        let signature_groups = group_variants_by_signature(variants);

        Ok(EnumChildrenData {
            examples: children,
            signature_groups,
        })
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: Self::ChildrenData,  // Now EnumChildrenData!
    ) -> Result<Value> {
        // NO MORE redundant extraction! Use children.signature_groups directly
        let variant_groups = &children.signature_groups;
        let child_examples = &children.examples;

        let mutation_example = match &ctx.enum_context {
            Some(EnumContext::Root) => {
                let mut examples = Vec::new();

                for (signature, variants_in_group) in variant_groups {
                    let representative = variants_in_group.first().ok_or_else(|| {
                        Report::new(Error::InvalidState("Empty variant group".to_string()))
                    })?;

                    let example = Self::build_variant_example(
                        signature,
                        representative.name(),
                        child_examples,  // Use from EnumChildrenData
                        ctx.type_name(),
                    );

                    // ... rest of example building ...
                }

                MutationExample::EnumRoot(examples)
            }
            // ... other cases also use children.signature_groups ...
        }

        // ... rest of method ...
    }
}
```

### 4. Update builder.rs to Call prepare_children_data

```rust
// In builder.rs, wherever we call assemble_from_children
fn process_all_children(&mut self, ctx: &RecursionContext) -> Result<()> {
    // ... existing code to recurse and build HashMap ...

    // NEW: Let builder prepare its specific data type
    let prepared_children = self.builder.prepare_children_data(child_examples, ctx)?;

    // Call assemble with prepared data
    let assembled = self.builder.assemble_from_children(ctx, prepared_children)?;

    // ... rest of processing ...
}
```

## Benefits

1. **No Redundant Work**: Variants extracted and grouped only once
2. **Type Safety**: Each builder specifies exactly what data it needs
3. **Backward Compatible**: Default implementation means non-enum builders unchanged
4. **Clean Separation**: Data preparation logic separated from assembly logic
5. **No Output Changes**: This is pure refactoring - output remains identical

## Implementation Steps

1. Add associated type `ChildrenData` to `PathBuilder` trait with default
2. Add `prepare_children_data` method with default implementation
3. Update `assemble_from_children` signature to use `Self::ChildrenData`
4. Create `EnumChildrenData` struct in enum_builder.rs
5. Implement `type ChildrenData = EnumChildrenData` for `EnumMutationBuilder`
6. Implement `prepare_children_data` for `EnumMutationBuilder`
7. Update `assemble_from_children` to use `children.signature_groups`
8. Update builder.rs to call `prepare_children_data`
9. Verify all other builders still work with default implementation

## Testing

Since this is pure refactoring with no output changes:
1. Run existing tests - all should pass
2. Generate type guide for `TestVariantChainEnum` - should be identical
3. Generate type guide for `Color` enum - should be identical
4. Memory usage should be slightly better (one less HashMap created)

## Notes

- We extract variants in `prepare_children_data` rather than storing them from `collect_children` because we need the `RecursionContext` to access the schema
- The default implementation of `prepare_children_data` ensures other builders continue to work unchanged
- This sets the foundation for Plan 0's variant chain tracking by establishing the data flow pattern