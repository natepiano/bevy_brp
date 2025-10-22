# Parallel Paths: Variant-Specific Array Output

## Goal

Create a parallel array structure that outputs individual enum variants at each mutation path instead of grouped variants with examples arrays. This allows the `mutation_paths_array` field to contain one entry per variant, while the `mutation_paths` HashMap maintains backward compatibility with grouped variants.

## Key Insight

Instead of creating special variant-specific `MutationPathInternal` entries separately, we modify the existing grouping pipeline to create BOTH:
- **Grouped entries**: Multiple variants sharing the same signature (existing behavior)
- **Individual entries**: One "fake" signature per variant (new behavior)

Both types flow through the existing processing pipeline. We filter at the output stage to include:
- Grouped entries in HashMap only (`mutation_paths`)
- Individual entries in Array only (`mutation_paths_array`)

## Implementation Steps

### Step 1: Extend `VariantSignature` enum

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/variant_signature.rs`

Add new variant to the enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VariantSignature {
    Unit,
    Tuple(Vec<BrpTypeName>),
    Struct(Vec<(String, BrpTypeName)>),

    /// Individual variant marker (for array-only output)
    /// Contains the variant name and its actual signature
    VariantSpecific {
        variant_name: VariantName,
        actual_signature: Box<VariantSignature>,
    },
}

impl VariantSignature {
    /// Check if this is a variant-specific signature (for filtering)
    pub fn is_variant_specific(&self) -> bool {
        matches!(self, Self::VariantSpecific { .. })
    }

    /// Get the actual signature (unwrapping VariantSpecific if needed)
    pub fn actual_signature(&self) -> &VariantSignature {
        match self {
            Self::VariantSpecific { actual_signature, .. } => actual_signature,
            other => other,
        }
    }
}
```

### Step 2: Modify `group_variants_by_signature()` to create both grouped and individual entries

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

```rust
fn group_variants_by_signature(
    ctx: &RecursionContext,
) -> std::result::Result<HashMap<VariantSignature, Vec<VariantName>>, BuilderError> {
    let schema = ctx.require_registry_schema()?;

    let one_of_array = schema
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Report::new(Error::InvalidState(format!(
                "Enum type {} missing oneOf field in schema",
                ctx.type_name()
            )))
        })?;

    // Parse all variants first (before grouping)
    let variant_kinds: Vec<VariantKind> = one_of_array
        .iter()
        .map(|v| VariantKind::from_schema_variant(v, &ctx.registry, ctx.type_name()))
        .collect::<Result<Vec<_>>>()?;

    let mut result = HashMap::new();

    // Create GROUPED entries (existing behavior - for HashMap output)
    for (signature, names) in variant_kinds
        .iter()
        .map(|vk| (vk.signature.clone(), vk.name.clone()))
        .into_group_map()
    {
        result.insert(signature, names);
    }

    // Create INDIVIDUAL variant entries (new behavior - for Array output)
    for variant_kind in &variant_kinds {
        let variant_specific_sig = VariantSignature::VariantSpecific {
            variant_name: variant_kind.name.clone(),
            actual_signature: Box::new(variant_kind.signature.clone()),
        };
        result.insert(variant_specific_sig, vec![variant_kind.name.clone()]);
    }

    Ok(result)
}
```

### Step 3: Update `create_paths_for_signature()` to handle VariantSpecific

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

```rust
fn create_paths_for_signature(
    signature: &VariantSignature,
    ctx: &RecursionContext,
) -> Option<Vec<PathKind>> {
    // Unwrap actual signature if this is VariantSpecific
    let actual_sig = signature.actual_signature();

    match actual_sig {
        VariantSignature::Unit => None,
        VariantSignature::Tuple(types) => Some(
            types
                .iter()
                .enumerate()
                .map(|(index, type_name)| {
                    let effective_type = OptionClassification::extract_leaf_type(type_name);
                    PathKind::IndexedElement {
                        index,
                        type_name: effective_type,
                        parent_type: ctx.type_name().clone(),
                    }
                })
                .collect_vec(),
        ),
        VariantSignature::Struct(fields) => Some(
            fields
                .iter()
                .map(|(field_name, type_name)| PathKind::StructField {
                    field_name:  field_name.clone(),
                    type_name:   type_name.clone(),
                    parent_type: ctx.type_name().clone(),
                })
                .collect(),
        ),
        VariantSignature::VariantSpecific { .. } => {
            unreachable!("Should have unwrapped VariantSpecific before matching")
        }
    }
}
```

### Step 4: Update `build_variant_example()` to handle VariantSpecific

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

```rust
fn build_variant_example(
    signature: &VariantSignature,
    variant_name: &VariantName,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
) -> Value {
    // Unwrap actual signature if VariantSpecific
    let actual_sig = signature.actual_signature();

    let example = match actual_sig {
        VariantSignature::Unit => json!(variant_name.short_name()),
        VariantSignature::Tuple(type_names) => {
            let tuple_values = support::assemble_tuple_from_children(type_names, children);
            if tuple_values.len() == 1 {
                json!({ variant_name.short_name(): tuple_values[0] })
            } else {
                json!({ variant_name.short_name(): tuple_values })
            }
        }
        VariantSignature::Struct(_field_types) => {
            let field_values = support::assemble_struct_from_children(children);
            json!({ variant_name.short_name(): field_values })
        }
        VariantSignature::VariantSpecific { .. } => {
            unreachable!("Should have unwrapped VariantSpecific before matching")
        }
    };

    apply_option_transformation(example, variant_name, enum_type)
}
```

### Step 5: Filter VariantSpecific from mutability aggregation

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

```rust
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: HashMap<Vec<VariantName>, Value>,
) -> Vec<MutationPathInternal> {
    // Filter OUT variant-specific entries from mutability calculation
    // (they represent individual variants, not groups)
    let mutability_statuses: Vec<Mutability> = enum_examples
        .iter()
        .filter(|eg| !eg.signature.is_variant_specific())
        .map(|eg| eg.mutability)
        .collect();

    let enum_mutability = support::aggregate_mutability(&mutability_statuses);

    // Build reason - also filter out variant-specific entries
    let filtered_examples: Vec<&ExampleGroup> = enum_examples
        .iter()
        .filter(|eg| !eg.signature.is_variant_specific())
        .collect();

    let mutability_reason = build_enum_mutability_reason(
        enum_mutability,
        &filtered_examples,
        ctx.type_name().clone()
    );

    // Build root mutation path (uses filtered examples for grouped behavior)
    let mut root_mutation_path = build_enum_root_path(
        ctx,
        enum_examples, // Pass all examples (will be filtered in build_enum_root_path)
        default_example,
        enum_mutability,
        mutability_reason,
    );

    // Store partial_root_examples built during ascent
    root_mutation_path.partial_root_examples = Some(partial_root_examples.clone());

    // Propagate partial root examples to ALL children (including variant-specific)
    propagate_partial_root_examples_to_children(
        &mut child_mutation_paths,
        &partial_root_examples,
        ctx,
    );

    // Return root path plus all child paths (including variant-specific)
    let mut result = vec![root_mutation_path];
    result.extend(child_mutation_paths);
    result
}
```

### Step 6: Filter VariantSpecific from `build_enum_root_path()`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

The root path should only include grouped examples in its `PathExample::EnumRoot`:

```rust
fn build_enum_root_path(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    enum_mutability: Mutability,
    mutability_reason: Option<Value>,
) -> MutationPathInternal {
    let enum_path_data = if ctx.variant_chain.is_empty() {
        None
    } else {
        Some(EnumPathData {
            variant_chain:       ctx.variant_chain.clone(),
            applicable_variants: Vec::new(),
            root_example:        None,
        })
    };

    // Filter out variant-specific entries (they go in child_mutation_paths instead)
    let grouped_examples: Vec<ExampleGroup> = enum_examples
        .into_iter()
        .filter(|eg| !eg.signature.is_variant_specific())
        .collect();

    MutationPathInternal {
        mutation_path: ctx.mutation_path.clone(),
        example: PathExample::EnumRoot {
            groups:     grouped_examples,
            for_parent: default_example,
        },
        type_name: ctx.type_name().display_name(),
        path_kind: ctx.path_kind.clone(),
        mutability: enum_mutability,
        mutability_reason,
        enum_path_data,
        depth: *ctx.depth,
        partial_root_examples: None,
    }
}
```

### Step 7: Filter VariantSpecific from `select_preferred_example()`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

```rust
pub fn select_preferred_example(groups: &[ExampleGroup]) -> Option<Value> {
    groups
        .iter()
        .filter(|g| !g.signature.is_variant_specific()) // Skip variant-specific
        .find(|g| g.mutability == Mutability::Mutable)
        .and_then(|g| g.example.clone())
}
```

### Step 8: Add `is_variant_specific_root` flag to `MutationPathInternal`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

```rust
pub struct MutationPathInternal {
    // ... existing fields

    /// Whether this path represents a variant-specific enum root (array-only)
    /// Derived from whether the parent signature is VariantSpecific
    pub is_variant_specific_root: bool,
}
```

### Step 9: Set the flag in `process_signature_groups()`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

When creating `ExampleGroup` entries, we need to mark whether they came from a VariantSpecific signature. However, the `ExampleGroup` itself doesn't need the flag - instead, we detect it during path creation.

Actually, the variant-specific entries will create their own child paths just like grouped entries. The difference is they'll be in `child_mutation_paths` not as a root. We need to mark them somehow.

Wait - I need to think about this more carefully. When `process_signature_groups()` processes a `VariantSpecific` signature, it creates child paths. Those child paths are nested field paths (like `.middle_struct.some_field`).

The variant-specific ROOT path needs to be created separately. Let me reconsider...

Actually, looking back at the flow:
1. `process_signature_groups()` creates child paths for each signature
2. Those child paths are returned in `child_mutation_paths`
3. `create_enum_mutation_paths()` creates the root path AND combines it with child paths

For variant-specific entries, we want to create a mutation path at the CURRENT level (not nested). So they should be created similarly to how the grouped root is created, but with individual variant examples instead of `PathExample::EnumRoot`.

Let me revise: The variant-specific paths should be created in `process_signature_groups()` as special entries that get added to `child_mutation_paths`, but they represent the current enum level (not nested children).

```rust
fn process_signature_groups(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError> {
    let mut examples = Vec::new();
    let mut child_mutation_paths = Vec::new();

    for (variant_signature, variant_names) in variant_groups.sorted() {
        let mut child_examples = HashMap::new();
        let mut signature_child_paths = Vec::new();

        let applicable_variants: Vec<VariantName> = variant_names.clone();

        // Create paths for this signature group
        let path_kinds = create_paths_for_signature(variant_signature, ctx);

        // Process each path
        for path_kind in path_kinds.into_iter().flatten() {
            let child_paths = process_signature_path(
                path_kind,
                &applicable_variants,
                variant_signature,
                ctx,
                &mut child_examples,
            )?;
            signature_child_paths.extend(child_paths);
        }

        // Determine mutation status
        let mutability = determine_signature_mutability(
            variant_signature,
            &signature_child_paths,
            ctx
        );

        // Build example
        let example = build_variant_group_example(
            variant_signature,
            variant_names,
            &child_examples,
            mutability,
            ctx,
        )?;

        examples.push(ExampleGroup {
            applicable_variants,
            signature: variant_signature.clone(),
            example: example.clone(),
            mutability,
        });

        // NEW: For VariantSpecific signatures, create a mutation path at current level
        if variant_signature.is_variant_specific() {
            if let Some(example_value) = example {
                let variant_specific_path = MutationPathInternal {
                    example: PathExample::Simple(example_value),
                    mutation_path: ctx.mutation_path.clone(),
                    type_name: ctx.type_name().display_name(),
                    path_kind: ctx.path_kind.clone(),
                    mutability,
                    mutability_reason: None,
                    enum_path_data: if ctx.variant_chain.is_empty() {
                        None
                    } else {
                        Some(EnumPathData {
                            variant_chain: ctx.variant_chain.clone(),
                            applicable_variants: variant_names.clone(),
                            root_example: None,
                        })
                    },
                    depth: *ctx.depth,
                    partial_root_examples: None,
                    is_variant_specific_root: true,
                };
                child_mutation_paths.push(variant_specific_path);
            }
        }

        // Add nested children to combined collection
        child_mutation_paths.extend(signature_child_paths);
    }

    // Build partial roots
    let partial_root_examples = build_partial_root_examples(
        variant_groups,
        &examples,
        &child_mutation_paths,
        ctx
    );

    Ok((examples, child_mutation_paths, partial_root_examples))
}
```

### Step 10: Update `build_partial_root_examples()` to handle variant-specific paths

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

The `build_partial_root_examples()` function needs to include variant-specific paths in its calculations:

```rust
fn build_partial_root_examples(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, Value> {
    let mut partial_roots = HashMap::new();

    // Process each variant group (including VariantSpecific)
    for (variant_signature, variant_names) in variant_groups.sorted() {
        // Get the example for this signature
        let example_value = examples
            .iter()
            .find(|eg| &eg.signature == variant_signature)
            .and_then(|eg| eg.example.as_ref());

        if let Some(example) = example_value {
            // For each variant in this group
            for variant_name in variant_names {
                let our_chain = {
                    let mut chain = ctx.variant_chain.clone();
                    chain.push(variant_name.clone());
                    chain
                };

                // Collect child chains that extend our chain
                let child_chains = collect_child_chains_to_wrap(child_paths, &our_chain, ctx);

                if child_chains.is_empty() {
                    // Leaf variant - use example directly
                    partial_roots.insert(our_chain, example.clone());
                } else {
                    // Variant with nested enums - wrap each child chain
                    for child_chain in child_chains {
                        let wrapped = wrap_child_partial(
                            child_paths,
                            &child_chain,
                            example,
                            ctx
                        );
                        partial_roots.insert(child_chain, wrapped);
                    }
                }
            }
        }
    }

    partial_roots
}
```

### Step 11: Filter at output in `build_mutation_paths()`

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`

```rust
pub fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<(
    HashMap<String, MutationPathExternal>,
    Vec<MutationPathExternalNew>,
)> {
    // ... existing setup code ...

    let internal_paths = recurse_mutation_paths(type_kind, &ctx)?;

    let mut hashmap_paths = HashMap::new();
    let mut array_paths = Vec::new();

    for mutation_path_internal in internal_paths {
        let key = (*mutation_path_internal.mutation_path).clone();

        // HashMap format: EXCLUDE variant-specific roots
        if !mutation_path_internal.is_variant_specific_root {
            let hashmap_external = mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry);
            hashmap_paths.insert(key, hashmap_external);
        }

        // Array format: INCLUDE ALL (both grouped and variant-specific)
        let array_external = mutation_path_internal.into_mutation_path_external_new(&registry);
        array_paths.push(array_external);
    }

    Ok((hashmap_paths, array_paths))
}
```

### Step 12: Update all `MutationPathInternal` constructors

Add `is_variant_specific_root: false` to all existing constructors in:
- `struct_builder.rs`
- `array_builder.rs`
- `tuple_builder.rs`
- `enum_path_builder.rs` (for non-variant-specific paths)

## Expected Result

### HashMap Output (backward compatible - unchanged)
```json
{
  "": {
    "examples": [
      {"applicable_variants": ["Empty"], "example": "Empty", ...},
      {"applicable_variants": ["WithMiddleStruct"], "example": {...}, ...}
    ]
  },
  ".middle_struct": { ... },
  ".middle_struct.some_field": { ... }
}
```

### Array Output (new - unrolled variants)
```json
[
  {
    "path": "",
    "description": "Select Empty variant",
    "example": "Empty",
    "path_info": {...}
  },
  {
    "path": "",
    "description": "Select WithMiddleStruct variant",
    "example": {"WithMiddleStruct": {...}},
    "path_info": {...}
  },
  {
    "path": ".middle_struct",
    "description": "Mutate the middle_struct field...",
    "example": {...},
    "path_info": {...}
  },
  {
    "path": ".middle_struct.some_field",
    ...
  }
]
```

## Benefits

1. **Reuses existing infrastructure**: Variant-specific paths flow through the same processing pipeline
2. **No duplication**: Shares logic for recursion, example building, mutability determination
3. **Clean filtering**: Single flag determines HashMap vs Array inclusion
4. **Backward compatible**: HashMap output unchanged
5. **Proper metadata**: Variant-specific paths get `partial_root_examples`, `enum_path_data`, etc. automatically
