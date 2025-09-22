# Plan 1: Defer Enum Grouping to Output Stage

## Goal
Refactor enum example generation to defer signature grouping from recursion time to output time. This is a pure refactoring that should produce IDENTICAL output to the current implementation, just achieved through a different algorithm.

## Motivation
Currently, we group enum variants by signature early in `enum_builder.rs::collect_children()`. This premature grouping loses information we need for Plan 2. By deferring grouping to the output stage, we maintain ALL variant information through recursion while still producing the same final output.

## Current vs. New Algorithm

### Current Algorithm
1. `collect_children()` groups variants by signature immediately
2. Returns one `PathKindWithVariants` per signature group with `applicable_variants`
3. `assemble_from_children()` builds one example per signature group
4. Output stage uses pre-grouped examples

### New Algorithm
1. `collect_children()` returns ALL variants individually (no grouping)
2. Remove `PathKindWithVariants` type entirely
3. `assemble_from_children()` builds examples for EVERY variant
4. Output stage performs grouping to:
   - Create `ExampleGroup` with correct `applicable_variants`
   - Select one representative mutation path per signature

## Implementation Changes

### Phase 1: Remove Early Grouping in enum_builder.rs

#### Remove `PathKindWithVariants`
```rust
// DELETE this entire type
pub struct PathKindWithVariants {
    pub path: Option<PathKind>,
    pub applicable_variants: Vec<String>,
}
```

#### Update `collect_children()`
```rust
impl PathBuilder for EnumMutationBuilder {
    type Item = Option<PathKind>;  // Changed from PathKindWithVariants

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let schema = ctx.require_registry_schema()?;
        let variants = extract_enum_variants(schema, &ctx.registry);

        let mut children = Vec::new();

        // Process EACH variant individually (no grouping!)
        for variant in variants {
            match variant {
                EnumVariantInfo::Unit(_) => {
                    children.push(None);  // Unit variants have no fields
                }
                EnumVariantInfo::Tuple(name, types) => {
                    for (index, type_name) in types.iter().enumerate() {
                        children.push(Some(PathKind::IndexedElement {
                            index,
                            type_name: type_name.clone(),
                            parent_type: ctx.type_name().clone(),
                        }));
                    }
                }
                EnumVariantInfo::Struct(name, fields) => {
                    for field in fields {
                        children.push(Some(PathKind::StructField {
                            field_name: field.field_name.clone(),
                            type_name: field.type_name.clone(),
                            parent_type: ctx.type_name().clone(),
                        }));
                    }
                }
            }
        }

        Ok(children.into_iter())
    }
}
```

### Phase 2: Update Example Assembly

#### Modify `assemble_from_children()`
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,
) -> Result<Value> {
    let schema = ctx.require_registry_schema()?;
    let all_variants = extract_enum_variants(schema, &ctx.registry);

    match &ctx.enum_context {
        Some(EnumContext::Root) => {
            // Build examples for ALL variants (no grouping!)
            let mut all_examples = Vec::new();

            for variant in all_variants {
                let variant_name = variant.name();
                let signature = variant.signature();

                let example = Self::build_variant_example(
                    &signature,
                    variant_name,
                    &children,
                    ctx.type_name(),
                );

                // Store each variant example individually
                all_examples.push(json!({
                    "variant": variant_name,
                    "signature": signature.to_string(),
                    "example": example,
                }));
            }

            // Return ALL examples for later grouping
            Ok(json!({
                "enum_root_data": {
                    "all_variant_examples": all_examples,
                    "enum_root_example_for_parent": all_examples.first()
                        .map(|e| e["example"].clone())
                        .unwrap_or(json!(null))
                }
            }))
        }
        // ... rest of the match cases
    }
}
```

### Phase 3: Implement Output-Stage Grouping

#### New Grouping Function
```rust
/// Groups variant examples by signature to create ExampleGroups
fn group_variant_examples(all_examples: Vec<Value>) -> Vec<ExampleGroup> {
    // Parse all examples and group by signature
    let mut groups: HashMap<String, Vec<(String, Value)>> = HashMap::new();

    for example_json in all_examples {
        let variant = example_json["variant"].as_str().unwrap();
        let signature = example_json["signature"].as_str().unwrap();
        let example = example_json["example"].clone();

        groups.entry(signature.to_string())
            .or_default()
            .push((variant.to_string(), example));
    }

    // Create ExampleGroup for each signature
    groups.into_iter().map(|(signature, variant_examples)| {
        let applicable_variants: Vec<String> = variant_examples.iter()
            .map(|(variant, _)| variant.clone())
            .collect();

        let representative_example = variant_examples.first()
            .map(|(_, example)| example.clone())
            .unwrap_or(json!(null));

        ExampleGroup {
            applicable_variants,
            signature,
            example: representative_example,
        }
    }).collect()
}
```

#### Mutation Path Deduplication
```rust
/// Groups mutation paths by signature, keeping one representative per group
fn deduplicate_mutation_paths(all_paths: Vec<MutationPathInternal>) -> Vec<MutationPathInternal> {
    // Group paths by (path_string, type_signature)
    let mut groups: HashMap<(String, String), Vec<MutationPathInternal>> = HashMap::new();

    for path in all_paths {
        // Extract signature (this might need path_kind analysis)
        let signature = extract_path_signature(&path);
        let key = (path.path.clone(), signature);
        groups.entry(key).or_default().push(path);
    }

    // Return one representative per group
    groups.into_values()
        .map(|mut group| group.pop().unwrap())
        .collect()
}
```

## Success Criteria

1. **Identical Output**: The generated type guide for `TestVariantChainEnum` should be byte-for-byte identical to the current implementation.

2. **All Variants Processed**: Verify that `assemble_from_children()` processes every variant, not just representatives.

3. **Correct Grouping**: `ExampleGroup` objects should have the same `applicable_variants` lists as before.

4. **No Information Loss**: Although we deduplicate at the end, we should have access to all variant information during recursion (setting up for Plan 2).

## Testing Strategy

1. **Snapshot Test**: Capture current output for `TestVariantChainEnum`, verify new implementation produces identical JSON.

2. **Intermediate Verification**: Add debug logging to verify all variants are being processed.

3. **Group Validation**: Verify that signature grouping produces correct `applicable_variants` lists.

## Why This Matters for Plan 2

By deferring grouping, we'll have ALL variant-specific examples available during recursion. This allows Plan 2 to:
- Track variant chains for each mutation path
- Build a lookup map from variant chain to root example
- Provide the correct root example for each mutation path

## Notes

- This is purely a refactoring - no new features or changed output
- Keep the existing `enum_variant_path` mechanism unchanged
- Performance should be similar since we're just moving where grouping happens
- This sets up the foundation for Plan 2's variant chain tracking