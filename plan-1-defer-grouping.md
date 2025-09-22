# Plan 1: Defer Enum Grouping to Output Stage

## Goal
Refactor enum example generation to defer signature grouping from recursion time to output time. This is a pure refactoring that should produce IDENTICAL output to the current implementation, just achieved through a different algorithm.

## Motivation
Currently, we group enum variants by signature early in `enum_builder.rs::collect_children()`. This premature grouping loses information we need for Plan 2. By deferring grouping to the output stage, we maintain ALL variant information through recursion while still producing the same final output.

## Critical Bug This Fixes

### Bug Summary (from `.claude/bug_reports/bug-report-color-enum-fields.md`)
The type guide generator has a critical bug where enum variants with structurally identical signatures but different field names get their fields confused. For example, Color enum variants all have 4 f32 fields but with different names:
- `Srgba`: `red`, `green`, `blue`, `alpha`
- `Hsla`: `hue`, `saturation`, `lightness`, `alpha`
- `Xyza`: `x`, `y`, `z`, `alpha`

The current grouping logic only compares field types, not names. This causes ALL variants in a group to incorrectly share the field names from whichever variant is processed first (non-deterministic due to HashMap iteration). This results in mutations failing with "missing field" errors.

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

**Migration Strategy: Phased**

## Implementation Changes

### Phase 1: Remove Early Grouping in enum_builder.rs

#### Keep Signature-Based Grouping in collect_children()

**Current Implementation Problem**: The current `collect_children()` groups variants by signature, which is actually necessary to avoid HashMap key collisions. The real issue is that we need to defer EXAMPLE grouping, not path grouping.

**Key Insight**: Processing variants individually would create HashMap collisions when multiple variants have the same structure (e.g., Variant1(i32) and Variant2(i32) both create index 0). The signature grouping prevents this collision.

**Changes Needed**:
1. Keep signature-based grouping to prevent collisions
2. Preserve all variant information for later use
3. Focus changes on `assemble_from_children()` instead

```rust
impl PathBuilder for EnumMutationBuilder {
    type Item = PathKindWithVariants;  // Keep this for now to avoid collisions

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let schema = ctx.require_registry_schema()?;
        let variants = extract_enum_variants(schema, &ctx.registry);

        // KEEP signature grouping to prevent HashMap key collisions
        let variant_groups = group_variants_by_signature(variants);
        let mut children = Vec::new();

        // Process by signature to avoid duplicate descriptors
        for (signature, variants_in_group) in variant_groups {
            let applicable_variants: Vec<String> = variants_in_group
                .iter()
                .map(|v| ctx.type_name().variant_name(v.name()))
                .collect();

            match signature {
                VariantSignature::Unit => {
                    children.push(PathKindWithVariants {
                        path: None,
                        applicable_variants,
                    });
                }
                VariantSignature::Tuple(types) => {
                    for (index, type_name) in types.iter().enumerate() {
                        children.push(PathKindWithVariants {
                            path: Some(PathKind::IndexedElement {
                                index,
                                type_name: type_name.clone(),
                                parent_type: ctx.type_name().clone(),
                            }),
                            applicable_variants: applicable_variants.clone(),
                        });
                    }
                }
                VariantSignature::Struct(fields) => {
                    for field in fields {
                        children.push(PathKindWithVariants {
                            path: Some(PathKind::StructField {
                                field_name: field.field_name.clone(),
                                type_name: field.type_name.clone(),
                                parent_type: ctx.type_name().clone(),
                            }),
                            applicable_variants: applicable_variants.clone(),
                        });
                    }
                }
            }
        }

        Ok(children.into_iter())
    }
}

```

### Phase 2: Update Example Assembly with Signature-Aware Processing

#### Modify `assemble_from_children()` - Signature-Aware Approach
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,  // May have collision issues
) -> Result<Value> {
    let schema = ctx.require_registry_schema()?;
    let all_variants = extract_enum_variants(schema, &ctx.registry);

    match &ctx.enum_context {
        Some(EnumContext::Root) => {
            // Build examples for ALL variants independently
            // Process each signature group separately to avoid HashMap collisions
            let variant_groups = group_variants_by_signature(all_variants);
            let mut all_examples = Vec::new();

            for (signature, variants_in_group) in variant_groups {
                // Build examples for each variant in this signature group
                for variant in variants_in_group {
                    let variant_name = variant.name();

                    // Build example for this specific variant
                    // Note: For now we use the shared children HashMap, but in the future
                    // we might process each signature's children independently
                    let example = Self::build_variant_example(
                        &signature,
                        variant_name,
                        &children,
                        ctx.type_name(),
                    );

                    // Store structured data for each variant
                    all_examples.push(VariantExampleData {
                        variant_name: variant_name.to_string(),
                        signature: signature.clone(),
                        example,
                    });
                }
            }

            // Return ALL variant examples for later grouping
            Ok(json!({
                "enum_root_data": {
                    "all_variant_examples": all_examples,
                    "enum_root_example_for_parent": all_examples.first()
                        .map(|e| e.example.clone())
                        .unwrap_or(json!(null))
                }
            }))
        }
        // ... rest of the match cases
    }
}
```

### Phase 3: Move Grouping Logic to Output Stage

#### Why Move Grouping from EnumMutationBuilder

The current grouping happens inside `EnumMutationBuilder::collect_children()` and `assemble_from_children()`, but for Plan 2 we need ALL variant examples available during recursion. Since `EnumMutationBuilder` instances are destroyed after building paths, the grouping logic must move to the output stage where it operates on the final `Vec<MutationPathInternal>` data.

#### Remove Early Grouping from enum_builder.rs

**In `collect_children()`**: Remove the `group_variants_by_signature()` call and process each variant individually:
```rust
// REMOVE this grouping step:
// let variant_groups = group_variants_by_signature(variants);

// CHANGE: Process all variants individually instead of signature groups
for variant in variants {
    // Create children for each individual variant, not grouped by signature
}
```

**In `assemble_from_children()`**: Remove the re-grouping and build examples for all variants:
```rust
// REMOVE this grouping step:
// let variant_groups = group_variants_by_signature(all_variants);

// CHANGE: Build examples for each individual variant
for variant in all_variants {
    // Build example for this specific variant
}
```

#### Add Output-Stage Grouping to builder.rs

**Location in builder.rs**: Add the grouping functions after the `MutationPathBuilder` implementation but before the `recurse_mutation_paths()` function. This keeps them logically grouped with the path building functionality.

**Structured Data Types**:
```rust
/// Structured data for variant examples instead of JSON
#[derive(Debug, Clone)]
struct VariantExampleData {
    variant_name: String,
    signature: VariantSignature,
    example: Value,
}
```

**Grouping Functions**:
```rust
/// Groups variant examples by signature to create ExampleGroups
/// Called during output stage processing to group ungrouped variant data
fn group_variant_examples(all_examples: Vec<VariantExampleData>) -> Vec<ExampleGroup> {
    // Group by actual VariantSignature enum, not string
    let mut groups: HashMap<VariantSignature, Vec<VariantExampleData>> = HashMap::new();

    for example_data in all_examples {
        groups.entry(example_data.signature.clone())
            .or_default()
            .push(example_data);
    }

    // Create ExampleGroup for each signature
    groups.into_iter().map(|(signature, variant_examples)| {
        let applicable_variants: Vec<String> = variant_examples.iter()
            .map(|data| data.variant_name.clone())
            .collect();

        let representative_example = variant_examples.first()
            .map(|data| data.example.clone())
            .unwrap_or(json!(null));

        ExampleGroup {
            applicable_variants,
            signature: signature.to_string(),
            example: representative_example,
        }
    }).collect()
}

/// Groups mutation paths by signature, keeping one representative per group
/// Called during final output processing to deduplicate similar paths
fn deduplicate_mutation_paths(all_paths: Vec<MutationPathInternal>) -> Vec<MutationPathInternal> {
    // Group paths by (path_string, type_signature)
    let mut groups: HashMap<(String, String), Vec<MutationPathInternal>> = HashMap::new();

    for path in all_paths {
        // Extract signature from path_kind analysis
        let signature = extract_path_signature(&path);
        let key = (path.path.clone(), signature);
        groups.entry(key).or_default().push(path);
    }

    // Return one representative per group
    groups.into_values()
        .map(|mut group| group.pop().unwrap())
        .collect()
}

/// Extracts a signature string from a mutation path for grouping purposes
fn extract_path_signature(path: &MutationPathInternal) -> String {
    // Analyze path.path_kind to determine the signature
    match &path.path_kind {
        PathKind::StructField { type_name, .. } => format!("field:{}", type_name),
        PathKind::IndexedElement { type_name, .. } => format!("index:{}", type_name),
        PathKind::ArrayElement { type_name, .. } => format!("array:{}", type_name),
        PathKind::RootValue { type_name, .. } => format!("root:{}", type_name),
    }
}
```

#### Integration with MutationPathBuilder

**In `process_enum_context()` method**: Update to handle the new ungrouped format and call grouping functions:
```rust
// When processing enum root data, check for ungrouped format
if let Some(all_variant_examples) = enum_data.get("all_variant_examples") {
    // Deserialize ungrouped data
    let variant_data: Vec<VariantExampleData> = serde_json::from_value(all_variant_examples.clone())
        .unwrap_or_default();

    // Apply output-stage grouping
    let grouped_examples = group_variant_examples(variant_data);

    (json!(null), Some(grouped_examples), Some(default_example))
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

## Implementation: Fixing the Color Enum Field Bug

### The Core Problem
The bug occurs because `VariantSignature` comparison is too coarse:

```rust
// Current BUGGY implementation
impl EnumVariantInfo {
    fn signature(&self) -> VariantSignature {
        match self {
            Self::Struct(_, fields) => {
                // This creates signatures that look identical for Srgba, Hsla, etc.
                // because they all have 4 f32 fields, ignoring field NAMES
                let field_sig = fields
                    .iter()
                    .map(|f| (f.field_name.clone(), f.type_name.clone()))
                    .collect();
                VariantSignature::Struct(field_sig)
            }
        }
    }
}
```

### The Fix: Enhanced Signature Comparison

```rust
// FIXED implementation - include field names in signature comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariantSignature {
    Unit,
    Tuple(Vec<BrpTypeName>),
    // Change: Include field names as part of the signature
    Struct(Vec<(String, BrpTypeName)>),  // (field_name, field_type)
}

// Alternative approach if we want to keep some grouping:
// Create a more detailed signature that preserves field information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DetailedVariantSignature {
    pub structure: VariantStructure,  // Unit/Tuple/Struct
    pub field_names: Vec<String>,     // Preserve exact field names
    pub field_types: Vec<BrpTypeName>, // Preserve exact field types
}
```

### Implementation Changes Required

#### 1. Fix `group_variants_by_signature()` to preserve field names:

```rust
fn group_variants_by_detailed_signature(
    variants: Vec<EnumVariantInfo>,
) -> HashMap<DetailedVariantSignature, Vec<EnumVariantInfo>> {
    let mut groups = HashMap::new();
    for variant in variants {
        // Create a detailed signature that includes field names
        let detailed_sig = variant.detailed_signature();
        groups
            .entry(detailed_sig)
            .or_insert_with(Vec::new)
            .push(variant);
    }
    groups
}
```

#### 2. Update `collect_children()` to handle field-name-aware grouping:

```rust
fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
    let schema = ctx.require_registry_schema()?;
    let variants = extract_enum_variants(schema, &ctx.registry);

    // Use detailed grouping that preserves field names
    let variant_groups = group_variants_by_detailed_signature(variants);
    let mut children = Vec::new();

    for (detailed_sig, variants_in_group) in variant_groups {
        let applicable_variants: Vec<String> = variants_in_group
            .iter()
            .map(|v| ctx.type_name().variant_name(v.name()))
            .collect();

        match detailed_sig.structure {
            VariantStructure::Struct => {
                // Now each group has consistent field names
                // Srgba group: red, green, blue, alpha
                // Hsla group: hue, saturation, lightness, alpha
                // They are NOT mixed together!
                for field_name in &detailed_sig.field_names {
                    let field_index = /* find index */;
                    let type_name = &detailed_sig.field_types[field_index];

                    children.push(PathKindWithVariants {
                        path: Some(PathKind::StructField {
                            field_name: field_name.clone(),
                            type_name: type_name.clone(),
                            parent_type: ctx.type_name().clone(),
                        }),
                        applicable_variants: applicable_variants.clone(),
                    });
                }
            }
            // ... handle Tuple and Unit cases
        }
    }

    Ok(children.into_iter())
}
```

#### 3. Ensure examples use correct field names:

```rust
fn build_variant_example(
    signature: &DetailedVariantSignature,
    variant_name: &str,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
) -> Value {
    match signature.structure {
        VariantStructure::Struct => {
            let mut field_values = serde_json::Map::new();

            // Use the ACTUAL field names from the signature
            for field_name in &signature.field_names {
                let descriptor = MutationPathDescriptor::from(field_name.clone());
                let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                field_values.insert(field_name.clone(), value);
            }

            json!({ variant_name: field_values })
        }
        // ... handle other cases
    }
}
```

### Validation Test

After implementation, this MUST pass:

```rust
// Generate type guide for DistanceFog
let type_guide = generate_type_guide("bevy_pbr::fog::DistanceFog");

// Extract color mutation examples
let color_examples = type_guide[".color"]["examples"];

// Verify Srgba has correct fields
let srgba_example = find_example_for_variant(color_examples, "Color::Srgba");
assert!(srgba_example["Srgba"].has_field("red"));
assert!(srgba_example["Srgba"].has_field("green"));
assert!(srgba_example["Srgba"].has_field("blue"));
assert!(!srgba_example["Srgba"].has_field("x"));  // Should NOT have x,y,z

// Verify Hsla has correct fields
let hsla_example = find_example_for_variant(color_examples, "Color::Hsla");
assert!(hsla_example["Hsla"].has_field("hue"));
assert!(hsla_example["Hsla"].has_field("saturation"));
assert!(hsla_example["Hsla"].has_field("lightness"));
assert!(!hsla_example["Hsla"].has_field("red"));  // Should NOT have red,green,blue
```

## Notes

- This is purely a refactoring - no new features or changed output
- Keep the existing `enum_variant_path` mechanism unchanged
- Performance should be similar since we're just moving where grouping happens
- This sets up the foundation for Plan 2's variant chain tracking
- **Critical**: This fixes the non-deterministic field name bug that causes mutation failures