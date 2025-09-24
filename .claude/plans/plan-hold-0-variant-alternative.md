# Plan 0: Simpler Variant Chain Fix - Preserve All Examples Through Recursion

## Goal
Fix the nested enum mutation bug where incorrect variant examples are provided, without major architectural changes. Enable Plan 2's variant chain tracking by preserving all variant examples during recursion.

## The Core Problem

Currently, when processing nested enums like `TestVariantChainEnum`:
1. `BottomEnum` correctly generates examples for ALL its variants (VariantA, VariantB, VariantC)
2. But when embedded in `MiddleStruct`, only ONE example (first) is selected
3. This causes `.middle_struct.nested_enum.name` to get VariantA example (which lacks a `name` field!)

The problem occurs in `enum_builder.rs::assemble_from_children()`:
```rust
Some(EnumContext::Child) => {
    // Building under another enum - return Simple example
    let example = Self::concrete_example(&variant_groups, &children, ctx.type_name());
    MutationExample::Simple(example)  // <-- Returns ONLY ONE example!
}
```

## The Solution

1. **enum_builder.rs always returns ALL variant examples** - remove special casing for root/child/none
2. **builder.rs handles context-aware decisions** - it knows when to create ExampleGroups vs build complete hierarchies
3. **Track variant chains during descent** - know which variants were selected at each level
4. **Build complete examples during ascent** - use variant chains to construct correct examples
5. **Map FullMutationPath to complete root examples** - direct lookup, no complex reconstruction

## Implementation Changes

### 0. Prerequisite: Add Associated Type for Builder-Specific Children Data

Before we can eliminate the redundant variant extraction, we need to enable enum_builder to receive the signature information it needs.

**Update PathBuilder trait**:
```rust
pub trait PathBuilder {
    type Item: MaybeVariants;
    type Iter<'a>: Iterator<Item = Self::Item> where Self: 'a;

    // NEW: Each builder can specify what data it needs
    type ChildrenData = HashMap<MutationPathDescriptor, Value>; // Default

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>>;

    // NEW: Default implementation for most builders
    fn prepare_children_data(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        _ctx: &RecursionContext,
    ) -> Result<Self::ChildrenData> {
        Ok(children)  // Most builders just pass through
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: Self::ChildrenData,  // Now builder-specific!
    ) -> Result<Value>;
}
```

**EnumMutationBuilder gets custom data**:
```rust
struct EnumChildrenData {
    /// The assembled examples by descriptor (from recursion)
    examples: HashMap<MutationPathDescriptor, Value>,

    /// The signature grouping from collect_children (no re-extraction needed!)
    signature_groups: Vec<(VariantSignature, Vec<VariantName>)>,
}

impl PathBuilder for EnumMutationBuilder {
    type ChildrenData = EnumChildrenData;

    fn prepare_children_data(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        ctx: &RecursionContext,
    ) -> Result<Self::ChildrenData> {
        // Extract variants and group by signature ONCE
        let schema = ctx.require_registry_schema()?;
        let variants = extract_enum_variants(schema, &ctx.registry);
        let signature_groups = group_variants_by_signature(variants);

        Ok(EnumChildrenData {
            examples: children,
            signature_groups,
        })
    }

    // assemble_from_children now has signature info without re-extraction!
}
```

**Update builder.rs to call prepare_children_data**:
```rust
fn process_all_children(&mut self, ctx: &RecursionContext) -> Result<()> {
    // ... recurse and build HashMap ...

    // NEW: Let builder prepare its specific data type
    let prepared_children = self.builder.prepare_children_data(child_examples, ctx)?;

    // Call assemble with prepared data
    let assembled = self.builder.assemble_from_children(ctx, prepared_children)?;
}
```

This eliminates the redundant variant extraction in enum_builder while keeping other builders unchanged.

### 1. Simplify enum_builder - Always Return All Examples

**In `enum_builder.rs::assemble_from_children()`**:
```rust
// Remove EnumContext checking - always build all examples
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: EnumChildrenData,  // Now gets enriched data!
) -> Result<Value> {
    // No need to extract variants or group them - it's in children.signature_groups!

    // Build raw examples for all signature groups
    let mut variant_examples = Vec::new();

    for (signature, variants_in_group) in &children.signature_groups {
        // Build one example per signature (all variants in group share same structure)
        let representative = variants_in_group.first().unwrap();
        let example = Self::build_variant_example(
            signature,
            representative,
            &children.examples,  // The HashMap of child examples
            ctx.type_name(),
        );

        // Store with all applicable variants
        variant_examples.push(json!({
            "variants": variants_in_group,  // All variants that share this signature
            "signature": signature.to_string(),
            "example": example,
        }));
    }

    // Always return raw variant examples - builder.rs will create ExampleGroups if needed
    Ok(json!({
        "enum_data": {
            "variants": variant_examples,
            "enum_type": ctx.type_name()
        }
    }))
}
```

### 2. Track Variant Chains During Descent

**In `builder.rs::recurse_mutation_paths()`**:
```rust
// Add variant chain to RecursionContext
struct RecursionContext {
    // ... existing fields ...

    /// Track which variants were selected during descent
    /// E.g., ["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantB"]
    variant_chain: Vec<VariantName>,
}

// When recursing into enum variants:
fn process_enum_variant(variant: &EnumVariant, ctx: &RecursionContext) {
    // Add this variant to the chain
    let mut child_ctx = ctx.clone();
    child_ctx.variant_chain.push(variant.name());

    // Recurse with updated chain
    recurse_mutation_paths(&child_ctx, ...);
}
```

### 3. Build Complete Examples During Ascent

**In `builder.rs::process_all_children()`**:
```rust
fn process_all_children(&mut self, ctx: &RecursionContext) -> Result<()> {
    // Collect children from all builders
    let children = self.collect_children(ctx)?;

    // Process each child
    for child in children {
        // If child has enum data with multiple examples
        if let Some(enum_data) = child.get("enum_data") {
            let all_examples = enum_data.get("all_examples");

            // For each variant example
            for example in all_examples {
                // Build version with this variant
                let mut child_ctx = ctx.clone();
                child_ctx.variant_chain.push(example.variant);

                // Recurse and collect paths
                let child_paths = recurse_mutation_paths(&child_ctx, ...);

                // Store paths with their variant chains
                for path in child_paths {
                    self.store_path_with_chain(path, child_ctx.variant_chain.clone());
                }
            }
        }
    }

    // Assemble examples using all children
    let assembled = self.builder.assemble_from_children(ctx, children)?;

    // If this is a struct with enum fields, it might return multiple versions
    // Each version corresponds to different enum variant combinations
}
```

### 4. Map FullMutationPath to Root Examples

**In `builder.rs` at the root level**:
```rust
struct MutationPathBuilder {
    // ... existing fields ...

    /// Map from variant chain to complete root example
    /// Used to provide correct examples for nested enum paths
    variant_chain_to_root: HashMap<Vec<VariantName>, Value>,

    /// Map from FullMutationPath to the variant chain it requires
    path_to_chain: HashMap<FullMutationPath, Vec<VariantName>>,
}

// After building all paths:
fn finalize_paths(&mut self) {
    // For each mutation path
    for path in &mut self.paths {
        // If this path has a variant chain requirement
        if let Some(chain) = self.path_to_chain.get(&path.full_mutation_path) {
            // Look up the complete root example for this chain
            if let Some(root_example) = self.variant_chain_to_root.get(chain) {
                path.root_variant_example = Some(root_example.clone());
            }
        }
    }
}
```

### 5. Handle Context in builder.rs

**Create ExampleGroups only for root enums**:
```rust
fn process_enum_data(
    &self,
    ctx: &RecursionContext,
    enum_data: &Value,
) -> Result<ProcessedEnum> {
    let variants = enum_data.get("variants");  // Raw variant examples from enum_builder

    if ctx.is_root_enum() {
        // Group variants by signature to create ExampleGroups for output
        let mut signature_groups: HashMap<String, Vec<VariantName>> = HashMap::new();
        let mut signature_examples: HashMap<String, Value> = HashMap::new();

        for variant_data in variants {
            let signature = variant_data.get("signature");
            let variant_name = variant_data.get("variant");
            let example = variant_data.get("example");

            signature_groups.entry(signature).or_default().push(variant_name);
            signature_examples.entry(signature).or_insert(example);
        }

        // Build ExampleGroups for root enum output
        let example_groups: Vec<ExampleGroup> = signature_groups.into_iter()
            .map(|(signature, applicable_variants)| ExampleGroup {
                applicable_variants,
                signature,
                example: signature_examples.get(&signature).cloned(),
            })
            .collect();

        ProcessedEnum::RootWithGroups(example_groups)
    } else {
        // For nested enums, just pass through the raw variants
        ProcessedEnum::NestedWithVariants(variants)
    }
}
```

## What Changes, What Doesn't

### Changes Required:
1. **enum_builder.rs**:
   - Remove EnumContext checking in `assemble_from_children()`
   - Always return all variant examples in consistent format

2. **builder.rs**:
   - Add variant chain tracking to RecursionContext
   - Update `process_all_children()` to handle multiple enum examples
   - Add variant_chain_to_root and path_to_chain mappings
   - Implement `finalize_paths()` to assign correct root examples

3. **RecursionContext**:
   - Add `variant_chain: Vec<VariantName>` field
   - Track variant selections during descent

### NO Changes Needed:
- `PathKindWithVariants` stays as-is
- HashMap key structure unchanged (for path descriptors)
- `collect_children()` logic unchanged
- Non-enum builders don't need signature changes
- Grouping by signature continues to work

## Example: How It Works

For `TestVariantChainEnum`:

### Current (Broken):
```
BottomEnum builds: [VariantA, VariantB, VariantC]
  ↓
MiddleStruct picks first: VariantA only
  ↓
TestVariantChainEnum uses: WithMiddleStruct(MiddleStruct(VariantA))
  ↓
Result: .middle_struct.nested_enum.name gets wrong variant
```

### With This Fix:
```
BottomEnum builds: [VariantA, VariantB, VariantC]
  ↓
MiddleStruct builds 3 versions:
  - MiddleStruct(VariantA)
  - MiddleStruct(VariantB)
  - MiddleStruct(VariantC)
  ↓
TestVariantChainEnum builds all combinations:
  - WithMiddleStruct(MiddleStruct(VariantA))
  - WithMiddleStruct(MiddleStruct(VariantB)) ✓
  - WithMiddleStruct(MiddleStruct(VariantC))
  - Empty
  ↓
Result: .middle_struct.nested_enum.name gets correct variant (VariantB)
```

## Benefits Over Original Plans

1. **Minimal Architecture Changes**: No HashMap key changes, no trait signature changes
2. **Localized Impact**: Changes mostly in enum_builder.rs
3. **Backwards Compatible**: Output format can remain the same initially
4. **Enables Plan 2**: Variant chains are naturally tracked
5. **Simpler to Implement**: Days instead of weeks

## Risks and Considerations

1. **Memory Usage**: Storing all variant combinations could be memory-intensive for deeply nested enums
   - Mitigation: Lazy evaluation or caching strategies

2. **Exponential Growth**: With multiple nested enums, combinations multiply
   - Mitigation: Practical limit on nesting depth

3. **Performance**: Building all combinations takes more time
   - Mitigation: Only build combinations that are actually referenced by paths

## Success Criteria

1. `.middle_struct.nested_enum.name` receives correct VariantB example
2. All nested enum paths get appropriate variant examples
3. Variant chains are tracked and available for Plan 2
4. No breaking changes to existing non-enum code
5. TestVariantChainEnum output shows correct examples for all paths

## Migration Path

1. **Phase 1**: Implement nested enum multi-example return
2. **Phase 2**: Add variant chain tracking
3. **Phase 3**: Build variant chain map at root
4. **Phase 4**: Update output to use correct examples
5. **Phase 5**: Remove old enum_variant_path once verified

## Open Questions

1. Should we build ALL combinations eagerly or on-demand?
2. How do we handle Option<T> special cases?
3. Should struct builders also return multiple examples when they have enum fields?
4. What's the memory limit we're willing to accept for variant combinations?