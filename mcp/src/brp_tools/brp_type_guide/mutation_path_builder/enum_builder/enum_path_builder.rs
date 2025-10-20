//! Enum-specific mutation path builder handling variant selection and path generation
//!
//! This module exclusively handles enum types, which require special processing due to:
//! - Multiple variant signatures (unit, tuple, struct) that share the same mutation interface
//! - Variant selection requirements that cascade through the type hierarchy
//! - Generation of variant path examples showing how to reach nested mutation targets these
//!   examples also show the signature and the qualifying applicable variants for each signature
//!   These more sophisticated examples are necessary because the bevy remote protocol mutation
//!   paths are the same for all variants with the same signature.
//!
//! ## Key Responsibilities
//!
//! 1. **Variant Processing**: Groups variants by signature and generates examples for each
//! 2. **Variant Path Management**: Creates and populates the variant chain showing how to select
//!    specific variants to reach a mutation target using the variant `root_example`
//!
//! ## Example
//!
//! For `Option<GameState>` where `GameState` has a `mode: GameMode` field:
//! - To mutate `.mode`, you must first select `Some` variant: `{"Some": {...}}`
//! - The variant path shows this requirement with instructions and examples
//!
//! ## Integration
//!
//! Called directly by `recurse_mutation_paths` in builder.rs when `TypeKind::Enum` is detected.
//! Unlike other types that use `MutationPathBuilder`, enums bypass the trait system for
//! their specialized processing, then calls back into `recurse_mutation_paths` for its children.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use error_stack::Report;
use itertools::Itertools;
use serde_json::Value;
use serde_json::json;

use super::super::super::type_kind::TypeKind;
use super::super::BuilderError;
use super::super::NotMutableReason;
use super::super::mutation_path_internal::MutationPathInternal;
use super::super::new_types::VariantName;
use super::super::path_builder;
use super::super::path_kind::MutationPathDescriptor;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::support;
use super::super::types::EnumPathData;
use super::super::types::ExampleGroup;
use super::super::types::Mutability;
use super::super::types::MutabilityIssue;
use super::super::types::PathAction;
use super::super::types::PathExample;
use super::option_classification::apply_option_transformation;
use super::variant_kind::VariantKind;
use super::variant_signature::VariantSignature;
use crate::brp_tools::brp_type_guide::BrpTypeName;
use crate::error::Error;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Result type for `process_children` containing example groups, child paths, and partial roots
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    BTreeMap<Vec<VariantName>, Value>,
);

/// Process enum type directly, bypassing `PathBuilder` trait
///
/// This function always generates examples arrays for all enums, anywhere in the type hierarchy
/// - Ensures all enum fields show their available variants
/// - Improves discoverability for nested enums
pub fn process_enum(
    ctx: &RecursionContext,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    tracing::debug!(
        "ENUM_PROCESS: type={}, path={}, depth={}",
        ctx.type_name(),
        ctx.mutation_path,
        *ctx.depth
    );

    // Use shared function to get variant information
    let variants_grouped_by_signature = group_variants_by_signature(ctx)?;

    // Process enum variants, grouped by signature
    let (enum_examples, child_mutation_paths, partial_root_examples) =
        process_signature_groups(&variants_grouped_by_signature, ctx)?;

    // Select default example from the generated examples
    let default_example = select_preferred_example(&enum_examples).unwrap_or(json!(null));

    // Create result paths including both root AND child paths
    Ok(create_enum_mutation_paths(
        ctx,
        enum_examples,
        default_example,
        child_mutation_paths,
        partial_root_examples,
    ))
}

/// Select the preferred example from a collection of `ExampleGroups`.
///
/// This function is critical for handling partially mutable enums where some variants
/// cannot be fully constructed.
///
/// # Invariant
///
/// After `build_variant_group_example`, only fully `Mutable` variants have `example: Some(value)`.
/// Both `NotMutable` and `PartiallyMutable` variants will have `example: None` because:
/// - `NotMutable`: The variant's fields cannot be serialized at all
/// - `PartiallyMutable`: Some variatns are mutable, some are not
///
/// This invariant ensures that any `Some(value)` we find is safe to use for spawning.
///
/// # Why This Matters
///
/// When an enum has mixed mutability, we must select a variant that can be fully constructed.
/// If we select a variant with `example: None`, it propagates up `PathExample.for_parent`,
/// causing parent enums to build invalid examples.
///
/// ## Example Problem Case
///
/// For `Option<Handle<Image>>` where `Handle<Image>` has:
/// - `Strong` variant → `partially_mutable`, example: `None` (has non-serializable `Arc` field)
/// - `Uuid` variant → `mutable`, example: `Some({"Uuid": "..."})`
///
/// If we pick `Strong` first (because it's non-unit), we get:
/// 1. `Strong`'s example is `None`
/// 2. This becomes `enum_example_for_parent: None` for `Handle<Image>`
/// 3. Parent `Option<Handle<Image>>::Some` uses this to build: `{"Some": null}`
/// 4. Result: Invalid `spawn_format` that crashes when used
///
/// # Selection Strategy
///
/// 1. **First priority**: Non-unit `Mutable` variant with a complete example
///    - Provides rich examples for tuple/struct variants
///    - Explicitly checks `mutability` to ensure spawnability
///
/// 2. **Second priority**: ANY `Mutable` variant with an example (including unit)
///    - Handles enums where all non-unit variants are `not_mutable`/`partially_mutable`
///    - Unit variants are always `Mutable` (no fields to construct)
///
/// 3. **Fallback**: Return `None` if no `Mutable` variants exist
///    - The entire enum is not spawnable
pub fn select_preferred_example(examples: &[ExampleGroup]) -> Option<Value> {
    // First priority: Find a non-unit Mutable variant with a complete example
    // Note: We check mutability explicitly for clarity and safety, even though
    // example.is_some() now implies Mutable due to build_variant_group_example's logic
    examples
        .iter()
        .find(|eg| {
            !matches!(eg.signature, VariantSignature::Unit)
                && eg.example.is_some()
                && eg.mutability == Mutability::Mutable
        })
        .or_else(|| {
            // Second priority: Fall back to ANY Mutable variant with an example (including unit)
            // This handles cases where all non-unit variants are not_mutable/partially_mutable
            examples
                .iter()
                .find(|eg| eg.example.is_some() && eg.mutability == Mutability::Mutable)
        })
        .and_then(|eg| eg.example.clone())
}

/// Extract all variants from schema and group them by signature
/// This is the single source of truth for enum variant processing
fn group_variants_by_signature(
    ctx: &RecursionContext,
) -> Result<BTreeMap<VariantSignature, Vec<VariantKind>>> {
    let schema = ctx.require_registry_schema()?;
    let mut variants = extract_variants(schema, &ctx.registry, ctx.type_name());

    variants.sort_by(|a, b| a.signature().cmp(b.signature()));

    Ok(variants
        .into_iter()
        .chunk_by(|v| v.signature().clone())
        .into_iter()
        .map(|(signature, signature_group)| (signature, signature_group.collect()))
        .collect())
}

/// Extract and parse all variants from an enum's JSON schema
///
/// Reads the `oneOf` field from the schema and converts each variant definition
/// into a `VariantKind` structure containing the variant's name and signature.
fn extract_variants(
    registry_schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    enum_type: &BrpTypeName,
) -> Vec<VariantKind> {
    let one_of_field = registry_schema.get_field(SchemaField::OneOf);

    one_of_field
        .and_then(Value::as_array)
        .map(|variants| {
            variants
                .iter()
                .filter_map(|v| VariantKind::from_schema_variant(v, registry, enum_type))
                .collect()
        })
        .unwrap_or_default()
}

/// Process a single path within a signature group, recursively building child paths
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],
    signature: &VariantSignature,
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    let mut child_ctx = ctx.create_recursion_context(path.clone(), PathAction::Create)?;

    // Set parent variant signature context for the child
    // Note: enum type is already in child_ctx.path_kind.parent_type
    child_ctx.parent_variant_signature = Some(signature.clone());

    // Set up enum context for children - just push the variant name
    if let Some(representative_variant) = applicable_variants.first() {
        child_ctx.variant_chain.push(representative_variant.clone());
    }

    // Recursively process child and collect paths
    let child_descriptor = path.to_mutation_path_descriptor();
    let child_schema = child_ctx.require_registry_schema()?;
    let child_type_kind = TypeKind::from_schema(child_schema);

    // Use the same recursion function as `MutationPathBuilder`
    let mut child_paths = path_builder::recurse_mutation_paths(child_type_kind, &child_ctx)?;

    // Track which variants make these child paths valid
    // Only populate for DIRECT children (not grandchildren nested deeper)
    for child_path in &mut child_paths {
        if let Some(enum_data) = &mut child_path.enum_path_data {
            // Check if this path is a direct child of the current enum level
            // Direct children have variant_chain.len() == ctx.variant_chain.len() + 1
            if enum_data.variant_chain.len() == ctx.variant_chain.len() + 1 {
                // Add all variants from this signature group
                // (all variants in a group share the same signature/structure)
                for variant_name in applicable_variants {
                    enum_data.applicable_variants.push(variant_name.clone());
                }
            }
        }
    }

    let child_example = child_paths
        .first()
        .ok_or_else(|| {
            tracing::error!("Empty child_paths for descriptor {child_descriptor:?}");
            Report::new(Error::InvalidState(format!(
                "Empty child_paths returned for descriptor {child_descriptor:?}"
            )))
        })
        .map(|p| p.example.for_parent().clone())?;

    child_examples.insert(child_descriptor, child_example);

    Ok(child_paths)
}

/// Determine the mutation status for a signature based on its child paths
fn determine_signature_mutability(
    signature: &VariantSignature,
    signature_child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Mutability {
    if matches!(signature, VariantSignature::Unit) {
        // Unit variants are always mutable (no fields to construct)
        return Mutability::Mutable;
    }

    // Aggregate field statuses from direct children at this depth
    // Use ONLY this signature's children (not all_child_paths from other signatures)
    let signature_field_statuses: Vec<Mutability> = signature_child_paths
        .iter()
        .filter(|p| p.is_direct_child_at_depth(*ctx.depth))
        .map(|p| p.mutability)
        .collect();

    if signature_field_statuses.is_empty() {
        // No fields (shouldn't happen, but handle gracefully)
        Mutability::Mutable
    } else {
        support::aggregate_mutability(&signature_field_statuses)
    }
}

/// Build an example for a variant group based on mutation status
/// Skip example generation for non-spawnable variants
///
/// We omit examples for `NotMutable` and `PartiallyMutable` variants because:
/// 1. `child_examples` only contains mutable fields (`Arc`/`Handle` fields are excluded)
/// 2. Building an example with incomplete fields would create invalid `spawn_format` values
/// 3. Attempting to spawn with incomplete examples causes Bevy reflection to panic
/// 4. `select_preferred_example()` will automatically skip variants with `None` examples and choose
///    a fully `Mutable` variant (or return `None` if no `Mutable` variants exist)
fn build_variant_group_example(
    signature: &VariantSignature,
    variants_in_group: &[VariantKind],
    child_examples: &HashMap<MutationPathDescriptor, Value>,
    mutability: Mutability,
    ctx: &RecursionContext,
) -> std::result::Result<Option<Value>, BuilderError> {
    let representative = variants_in_group
        .first()
        .ok_or_else(|| Report::new(Error::InvalidState("Empty variant group".to_string())))?;

    let example = if matches!(
        mutability,
        Mutability::NotMutable | Mutability::PartiallyMutable
    ) {
        None // Omit example field for variants that cannot be fully constructed
    } else {
        Some(build_variant_example(
            signature,
            representative.name(),
            child_examples,
            ctx.type_name(),
        ))
    };

    Ok(example)
}

/// Build a complete example for a variant with all its fields
///
/// For nested `Option` types, BRP collapses all nesting levels due to the wrap-unwrap pattern:
/// - Each `Option` level wraps as `{"Some": value}`, then `apply_option_transformation` unwraps it
/// - This happens at every level, producing complete flattening:
///   - `Some(Some(Some(5.0)))` → `5.0` (fully unwrapped)
///   - `Some(Some(None))` → `null` (any nested `None` collapses to `null`)
///   - `Some(None)` → `null`
///   - `None` → `null`
///
/// When children are empty (e.g., filtered `NotMutable` at recursion depth limits),
/// `unwrap_or(json!(null))` at line 347 provides a fallback, producing `{"Some": null}`
/// which `apply_option_transformation` at line 367 transforms to `null` - the correct BRP
/// representation.
fn build_variant_example(
    signature: &VariantSignature,
    variant_name: &str,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
) -> Value {
    let example = match signature {
        VariantSignature::Unit => {
            json!(variant_name)
        }
        VariantSignature::Tuple(types) => {
            let mut tuple_values = Vec::new();
            for index in 0..types.len() {
                let descriptor = MutationPathDescriptor::from(index.to_string());
                let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                tuple_values.push(value);
            }
            // Fix: Single-element tuples should not be wrapped in arrays
            // Vec<Value> always serializes as JSON array, but BRP expects single-element
            // tuples to use direct value format for mutations, not array format
            if tuple_values.len() == 1 {
                json!({ variant_name: tuple_values[0] })
            } else {
                json!({ variant_name: tuple_values })
            }
        }
        VariantSignature::Struct(_field_types) => {
            // Use shared function to assemble struct from children (only includes mutable fields)
            let field_values = support::assemble_struct_from_children(children);
            json!({ variant_name: field_values })
        }
    };

    // Apply `Option<T>` transformation only for actual Option types
    apply_option_transformation(example, variant_name, enum_type)
}

/// Process child paths - simplified version of `MutationPathBuilder`'s child processing
///
/// Builds examples immediately for each variant group to avoid `HashMap` collision issues
/// where multiple variant groups with the same signature would overwrite each other's examples.
fn process_signature_groups(
    variant_groups: &BTreeMap<VariantSignature, Vec<VariantKind>>,
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError> {
    let mut examples = Vec::new();
    let mut child_mutation_paths = Vec::new();

    // Process each variant group
    for (signature, variant_kinds) in variant_groups {
        // Create FRESH child_examples `HashMap` for each variant group to avoid overwrites
        let mut child_examples = HashMap::new();
        // Collect THIS signature's children separately to avoid mixing with other variants
        let mut signature_child_paths = Vec::new();

        let applicable_variants: Vec<VariantName> = variant_kinds
            .iter()
            .map(|v| v.variant_name().clone())
            .collect();

        // Create paths for this signature group
        let path_kinds = create_paths_for_signature(signature, ctx);

        // Process each path
        for path_kind in path_kinds.into_iter().flatten() {
            let child_paths = process_signature_path(
                path_kind,
                &applicable_variants,
                signature,
                ctx,
                &mut child_examples,
            )?;
            signature_child_paths.extend(child_paths);
        }

        // Determine mutation status for this signature
        let mutability = determine_signature_mutability(signature, &signature_child_paths, ctx);

        // Build example for this variant group
        let example = build_variant_group_example(
            signature,
            variant_kinds,
            &child_examples,
            mutability,
            ctx,
        )?;

        examples.push(ExampleGroup {
            applicable_variants,
            signature: signature.clone(),
            example,
            mutability,
        });

        // Add this signature's children to the combined collection
        child_mutation_paths.extend(signature_child_paths);
    }

    // Build partial roots using assembly during ascent
    let partial_root_examples =
        build_partial_root_examples(variant_groups, &examples, &child_mutation_paths, ctx);

    Ok((examples, child_mutation_paths, partial_root_examples))
}

/// Create `PathKind` objects for a signature
fn create_paths_for_signature(
    signature: &VariantSignature,
    ctx: &RecursionContext,
) -> Option<Vec<PathKind>> {
    match signature {
        VariantSignature::Unit => None, // Unit variants have no paths
        VariantSignature::Tuple(types) => Some(
            types
                .iter()
                .enumerate()
                .map(|(index, type_name)| PathKind::IndexedElement {
                    index,
                    type_name: type_name.clone(),
                    parent_type: ctx.type_name().clone(),
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
    }
}

/// Collect all unique child chains that extend a given variant chain
fn collect_child_chains_to_wrap(
    child_paths: &[MutationPathInternal],
    our_chain: &[VariantName],
    ctx: &RecursionContext,
) -> BTreeSet<Vec<VariantName>> {
    child_paths
        .iter()
        // Only process direct children
        .filter(|child| child.is_direct_child_at_depth(*ctx.depth))
        // Flatten all matching child chains
        .flat_map(|child| {
            child
                .partial_root_examples
                .as_ref()
                .into_iter()
                .flat_map(|partials| {
                    partials
                        .keys()
                        .filter(|&child_chain| child_chain.starts_with(our_chain))
                        .cloned()
                })
        })
        .collect()
}

/// Build `partial_root_examples` by assembling variant-specific root examples during recursion
/// ascent
///
/// Creates a map from variant chains to complete root examples showing how to reach nested enum
/// paths. Each variant at this level gets entries for itself and all compatible child chains.
///
/// ## Variant Chain Compatibility
///
/// Multiple variants can share the same signature. When building examples for nested enums,
/// we must filter children by variant chain compatibility to prevent HashMap collisions.
///
/// **Example**: `Handle<Image>` enum with two variants:
/// - `Weak(AssetId<Image>)` where `AssetId` is an enum with `Uuid` and `Index` variants
/// - `Strong(Arc<StrongHandle>)` where the inner type is not an enum
///
/// Both variants use descriptor `"0"` for their tuple element, but have different nested
/// structures.
///
/// When building for chain `["Handle::Weak", "AssetId::Uuid"]`:
/// - Child with chain `["Handle::Weak"]` → compatible ✅ (prefix match)
/// - Child with chain `["Handle::Strong"]` → incompatible ❌ (different variant path)
///
/// Without filtering, both children would collide on descriptor `"0"`, and `Strong`'s `null` value
/// would overwrite `Weak`'s nested `AssetId` structure.
///
/// **Output for this example**:
/// - `["Handle::Weak"]` → `{"Weak": {"Uuid": "00000000-0000-0000-0000-000000000000"}}`
/// - `["Handle::Weak", "AssetId::Uuid"]` → `{"Weak": {"Uuid":
///   "00000000-0000-0000-0000-000000000000"}}`
/// - `["Handle::Strong"]` → `{"Strong": null}`
fn build_partial_root_examples(
    variant_groups: &BTreeMap<VariantSignature, Vec<VariantKind>>,
    enum_examples: &[ExampleGroup],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> BTreeMap<Vec<VariantName>, Value> {
    let mut partial_root_examples = BTreeMap::new();

    // For each variant at THIS level
    for (signature, variants) in variant_groups {
        for variant in variants {
            let our_variant = variant.variant_name().clone();

            // Build our variant chain by extending parent's chain
            let mut our_chain = ctx.variant_chain.clone();
            our_chain.push(our_variant.clone());

            // Get base example for this variant
            let base_example = enum_examples
                .iter()
                .find(|ex| ex.applicable_variants.contains(&our_variant))
                .and_then(|ex| ex.example.clone())
                .unwrap_or(json!(null));

            // Collect all unique child chains that start with our_chain
            let child_chains_to_wrap =
                collect_child_chains_to_wrap(child_mutation_paths, &our_chain, ctx);

            // Build wrapped examples for each compatible child chain
            let mut found_child_chains = false;
            for child_chain in &child_chains_to_wrap {
                let wrapped = build_variant_example_for_chain(
                    signature,
                    variant.name(),
                    child_mutation_paths,
                    child_chain,
                    ctx,
                );
                partial_root_examples.insert(child_chain.clone(), wrapped);
                found_child_chains = true;
            }

            // After processing all child chains, also create entry for n-variant chain
            // This handles paths that only specify the outer variant(s)
            if found_child_chains {
                let wrapped = build_variant_example_for_chain(
                    signature,
                    variant.name(),
                    child_mutation_paths,
                    &our_chain,
                    ctx,
                );
                partial_root_examples.insert(our_chain.clone(), wrapped);
            } else {
                // No child chains found, this is a leaf variant - store base example
                partial_root_examples.insert(our_chain, base_example);
            }
        }
    }

    partial_root_examples
}

/// Eliminate duplication in `build_partial_root_examples` by centralizing child collection and
/// example building
///
/// Collects children for a variant chain and calls `build_variant_example` to construct the
/// example.
fn build_variant_example_for_chain(
    signature: &VariantSignature,
    variant_name: &str,
    child_mutation_paths: &[MutationPathInternal],
    variant_chain: &[VariantName],
    ctx: &RecursionContext,
) -> Value {
    let child_refs: Vec<&MutationPathInternal> = child_mutation_paths.iter().collect();
    let children = support::collect_children_for_chain(&child_refs, ctx, Some(variant_chain));

    build_variant_example(signature, variant_name, &children, ctx.type_name())
}

/// Build mutation status reason for enums based on variant mutability
fn build_enum_mutability_reason(
    enum_mutability: Mutability,
    enum_examples: &[ExampleGroup],
    type_name: BrpTypeName,
) -> Option<Value> {
    match enum_mutability {
        Mutability::PartiallyMutable => {
            // Create `MutabilityIssue` for each variant using `from_variant_name`
            let mutability_issues: Vec<MutabilityIssue> = enum_examples
                .iter()
                .flat_map(|eg| {
                    eg.applicable_variants.iter().map(|variant| {
                        MutabilityIssue::from_variant_name(
                            variant.clone(),
                            type_name.clone(),
                            eg.mutability,
                        )
                    })
                })
                .collect();

            // Use unified `NotMutableReason` with TypeKind-based message
            let message = "Some variants are mutable while others are not".to_string();

            Option::<Value>::from(&NotMutableReason::from_partial_mutability(
                type_name,
                mutability_issues,
                message,
            ))
        }
        Mutability::NotMutable => {
            // All variants are not mutable
            Some(json!({
                "message": "No variants in this enum can be mutated"
            }))
        }
        Mutability::Mutable => None,
    }
}

/// Build the root `MutationPathInternal` for an enum
fn build_enum_root_path(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    enum_mutability: Mutability,
    mutability_reason: Option<Value>,
) -> MutationPathInternal {
    // Generate `EnumPathData` only if we have a variant chain (nested in another enum)
    let enum_path_data = if ctx.variant_chain.is_empty() {
        None
    } else {
        Some(EnumPathData {
            variant_chain:       ctx.variant_chain.clone(),
            applicable_variants: Vec::new(),
            root_example:        None,
        })
    };

    // Direct field assignment - enums ALWAYS generate examples arrays
    MutationPathInternal {
        mutation_path: ctx.mutation_path.clone(),
        example: PathExample::EnumRoot {
            groups:     enum_examples,
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

/// Propagate partial root examples to child paths at the root level
fn propagate_partial_root_examples_to_children(
    child_paths: &mut [MutationPathInternal],
    partial_root_examples: &BTreeMap<Vec<VariantName>, Value>,
    ctx: &RecursionContext,
) {
    if ctx.variant_chain.is_empty() {
        // Propagate to children (overwriting struct-level propagations)
        for child in child_paths.iter_mut() {
            child.partial_root_examples = Some(partial_root_examples.clone());
        }

        // Use shared helper function to populate root examples
        support::populate_root_examples_from_partials(child_paths, partial_root_examples);
    }
}

/// Create final result paths - includes both current and child paths
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: BTreeMap<Vec<VariantName>, Value>,
) -> Vec<MutationPathInternal> {
    // Determine enum mutation status by aggregating the mutability of all examples
    // and then using the shared (with path_builder) aggregate_mutability to determine
    // the mutability across all variants of this enum
    let mutability_statuses: Vec<Mutability> =
        enum_examples.iter().map(|eg| eg.mutability).collect();

    let enum_mutability = support::aggregate_mutability(&mutability_statuses);

    // Build reason for partially_mutable or not_mutable enums using unified approach
    let mutability_reason =
        build_enum_mutability_reason(enum_mutability, &enum_examples, ctx.type_name().clone());

    // Build root mutation path
    let mut root_mutation_path = build_enum_root_path(
        ctx,
        enum_examples,
        default_example,
        enum_mutability,
        mutability_reason,
    );

    // Store partial_root_examples built during ascent in process_children
    root_mutation_path.partial_root_examples = Some(partial_root_examples.clone());

    // Propagate partial root examples to children and populate root examples
    propagate_partial_root_examples_to_children(
        &mut child_mutation_paths,
        &partial_root_examples,
        ctx,
    );

    // Return root path plus all child paths (like `MutationPathBuilder` does)
    let mut result = vec![root_mutation_path];
    result.extend(child_mutation_paths);
    result
}
