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

use std::collections::HashMap;
use std::collections::HashSet;

use error_stack::Report;
use itertools::Itertools;
use serde_json::Value;
use serde_json::json;

use super::super::super::type_kind::TypeKind;
use super::super::BuilderError;
use super::super::NotMutableReason;
use super::super::mutation_path_internal::MutationPathInternal;
use super::super::mutation_path_internal::MutationPathSliceExt;
use super::super::new_types::VariantName;
use super::super::path_builder;
use super::super::path_example::PathExample;
use super::super::path_kind::MutationPathDescriptor;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::support;
use super::super::types::EnumPathInfo;
use super::super::types::ExampleGroup;
use super::super::types::Mutability;
use super::super::types::MutabilityIssue;
use super::super::types::PathAction;
use super::super::types::RootExample;
use super::option_classification::apply_option_transformation;
use super::variant_kind::VariantKind;
use super::variant_signature::VariantSignature;
use crate::brp_tools::brp_type_guide::BrpTypeName;
use crate::error::Error;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Extension trait for sorting variant groups deterministically
trait SortedVariantGroups {
    fn sorted(&self) -> Vec<(&VariantSignature, &Vec<VariantName>)>;
}

impl SortedVariantGroups for HashMap<VariantSignature, Vec<VariantName>> {
    fn sorted(&self) -> Vec<(&VariantSignature, &Vec<VariantName>)> {
        let mut sorted_groups: Vec<_> = self.iter().collect();
        sorted_groups.sort_by_key(|(signature, _)| *signature);
        sorted_groups
    }
}

/// Result type for `process_children` containing example groups, child paths, and partial roots
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, RootExample>,
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

    // Select default example - check knowledge first, then fall back to enum examples
    // Knowledge allows struct-field-specific overrides (e.g., Camera.target should use
    // Window::Primary)
    let default_example = ctx
        .find_knowledge()
        .ok()
        .flatten()
        .map(|knowledge| knowledge.example().clone())
        .or_else(|| select_preferred_example(&enum_examples))
        .ok_or_else(|| {
            BuilderError::SystemError(Report::new(Error::InvalidState(format!(
                "Enum {} has no valid example: no struct field knowledge and no mutable variants",
                ctx.type_name()
            ))))
        })?;

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
fn group_variants_by_signature(
    ctx: &RecursionContext,
) -> Result<HashMap<VariantSignature, Vec<VariantName>>> {
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

    // the first map gets a VariantKind which contains the variant signature and name
    // the second map iterates over the result and then groups them by signature
    // returning the HashMap via .into_group_map()
    Ok(one_of_array
        .iter()
        .map(|v| VariantKind::from_schema_variant(v, &ctx.registry, ctx.type_name()))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|variant_kind| (variant_kind.signature, variant_kind.name))
        .into_group_map())
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
        if let Some(enum_path_info) = &mut child_path.enum_path_info {
            // Check if this path is a direct child of the current enum level
            // Direct children have variant_chain.len() == ctx.variant_chain.len() + 1
            if enum_path_info.variant_chain.len() == ctx.variant_chain.len() + 1 {
                // Add all variants from this signature group
                // (all variants in a group share the same signature/structure)
                for variant_name in applicable_variants {
                    enum_path_info
                        .applicable_variants
                        .push(variant_name.clone());
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
    variants_in_group: &[VariantName],
    child_examples: &HashMap<MutationPathDescriptor, Value>,
    mutability: Mutability,
    ctx: &RecursionContext,
) -> std::result::Result<Option<Value>, BuilderError> {
    let representative_variant_name = variants_in_group
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
            representative_variant_name,
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
    variant_name: &VariantName,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
) -> Value {
    let example = match signature {
        VariantSignature::Unit => {
            json!(variant_name.short_name())
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
                json!({ variant_name.short_name(): tuple_values[0] })
            } else {
                json!({ variant_name.short_name(): tuple_values })
            }
        }
        VariantSignature::Struct(_field_types) => {
            // Use shared function to assemble struct from children (only includes mutable fields)
            let field_values = support::assemble_struct_from_children(children);
            json!({ variant_name.short_name(): field_values })
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
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError> {
    let mut examples = Vec::new();
    let mut child_mutation_paths = Vec::new();

    // Process each variant group in deterministic order
    for (variant_signature, variant_names) in variant_groups.sorted() {
        // Create FRESH child_examples `HashMap` for each variant group to avoid overwrites
        let mut child_examples = HashMap::new();
        // Collect THIS signature's children separately to avoid mixing with other variants
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

        // Determine mutation status for this signature
        let mutability =
            determine_signature_mutability(variant_signature, &signature_child_paths, ctx);

        // Build example for this variant group
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
            example,
            mutability,
        });

        // Add this signature's children to the combined collection
        child_mutation_paths.extend(signature_child_paths);
    }

    // Build partial root examples using assembly during ascent
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

/// Build `partial_root_examples` by assembling variant-specific root examples during recursion
/// ascent
///
/// Creates a map from variant chains to complete root examples showing how to reach nested enum
/// paths. Each variant at this level gets entries for itself and all compatible child chains.
///
/// ## Variant Chain Compatibility
///
/// Multiple variants can share the same signature. When building examples for nested enums,
/// we must filter children by variant chain compatibility to prevent `HashMap` collisions.
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
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    enum_examples: &[ExampleGroup],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, RootExample> {
    let mut partial_root_examples = HashMap::new();

    // For each variant at THIS level in deterministic order
    for (signature, variants) in variant_groups.sorted() {
        for variant_name in variants {
            // Build the chain for this variant by extending parent's chain
            let mut this_variant_chain = ctx.variant_chain.clone();
            this_variant_chain.push(variant_name.clone());

            let spawn_example = enum_examples
                .iter()
                .find(|ex| ex.applicable_variants.contains(variant_name))
                .and_then(|ex| ex.example.clone())
                .or_else(|| select_preferred_example(enum_examples))
                .unwrap_or(json!(null));

            // Find this variant's mutability status
            // DEFENSIVE: This lookup should always succeed because enum_examples is built by
            // iterating over variant_groups (see process_signature_groups line 408-449), so
            // every variant in variant_groups is guaranteed to exist in enum_examples.
            // The unwrap_or fallback handles theoretical future refactoring errors by treating
            // unknown variants as NotMutable (safest choice - prevents construction attempts).
            let variant_mutability = enum_examples
                .iter()
                .find(|ex| ex.applicable_variants.contains(variant_name))
                .map_or(Mutability::NotMutable, |ex| ex.mutability);

            // Determine if this variant can be constructed via BRP
            let variant_unavailable_reason = analyze_variant_constructibility(
                variant_name,
                signature,
                variant_mutability,
                child_mutation_paths,
                ctx,
            )
            .err();

            // Find all deeper nested chains that extend this variant
            let child_refs: Vec<_> = child_mutation_paths.iter().collect();
            let nested_enum_chains: HashSet<_> = child_refs
                .child_variant_chains(*ctx.depth)
                .into_iter()
                .filter(|chain| chain.starts_with(&this_variant_chain))
                .collect();

            // Build root examples for each nested enum chain
            for nested_chain in &nested_enum_chains {
                let example = build_variant_example_for_chain(
                    signature,
                    variant_name,
                    child_mutation_paths,
                    nested_chain,
                    ctx,
                );

                // Use shared helper to wrap with availability status
                // Hierarchical selection: parent reason takes precedence (child is unreachable),
                // otherwise check if nested chain has its own unavailability reason
                let root_example = support::wrap_example_with_availability(
                    example,
                    &child_refs,
                    nested_chain,
                    variant_unavailable_reason.clone(),
                );

                partial_root_examples.insert(nested_chain.clone(), root_example);
            }

            // Build root example for this variant's chain itself
            let example = if nested_enum_chains.is_empty() {
                // Leaf variant (no nested enums) - use spawn example directly
                spawn_example
            } else {
                // This variant contains nested enums - build by wrapping children
                build_variant_example_for_chain(
                    signature,
                    variant_name,
                    child_mutation_paths,
                    &this_variant_chain,
                    ctx,
                )
            };

            // Use shared helper to wrap with availability status
            let root_example = support::wrap_example_with_availability(
                example,
                &child_refs,
                &this_variant_chain,
                variant_unavailable_reason,
            );

            partial_root_examples.insert(this_variant_chain, root_example);
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
    variant_name: &VariantName,
    child_mutation_paths: &[MutationPathInternal],
    variant_chain: &[VariantName],
    ctx: &RecursionContext,
) -> Value {
    let child_refs: Vec<&MutationPathInternal> = child_mutation_paths.iter().collect();
    let children = support::collect_children_for_chain(&child_refs, ctx, Some(variant_chain));

    build_variant_example(signature, variant_name, &children, ctx.type_name())
}

/// Analyze if a variant can be constructed via BRP and build detailed reason if not
///
/// Returns `Ok(())` if variant is constructible (Mutable variants, Unit variants)
/// Returns `Err(reason)` if variant cannot be constructed, with human-readable explanation
///
/// For `PartiallyMutable` variants, collects actual reasons from `NotMutable` child fields.
/// For `NotMutable` variants, indicates all fields are problematic.
fn analyze_variant_constructibility(
    variant_name: &VariantName,
    signature: &VariantSignature,
    mutability: Mutability,
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> std::result::Result<(), String> {
    // Unit variants are always constructible (no fields to serialize)
    if matches!(signature, VariantSignature::Unit) {
        return Ok(());
    }

    // Fully Mutable variants are constructible
    if matches!(mutability, Mutability::Mutable) {
        return Ok(());
    }

    // NotMutable variants - all fields are problematic
    if matches!(mutability, Mutability::NotMutable) {
        let message = format!(
            "Cannot construct {} variant via BRP - all fields are non-mutable. \
            This variant cannot be mutated via BRP.",
            variant_name.short_name()
        );
        return Err(message);
    }

    // PartiallyMutable variants - collect problematic field reasons
    // A variant is unconstructible if it has:
    // 1. NotMutable fields (cannot provide values)
    // 2. PartiallyMutable fields (contain NotMutable descendants, cannot provide complete values)
    let problematic_fields: Vec<String> = child_paths
        .iter()
        .filter(|p| p.is_direct_child_at_depth(*ctx.depth))
        // Filter to only paths belonging to the current variant
        .filter(|p| {
            p.enum_path_info.as_ref().is_some_and(|data| {
                !data.variant_chain.is_empty() && &data.variant_chain[0] == variant_name
            })
        })
        .filter(|p| {
            matches!(
                p.mutability,
                Mutability::NotMutable | Mutability::PartiallyMutable
            )
        })
        .map(|p| {
            let type_name = p.type_name.short_name();

            // Generate descriptive label based on PathKind and variant signature
            let field_label = match &p.path_kind {
                PathKind::StructField { field_name, .. } => field_name.to_string(),
                PathKind::IndexedElement { index, .. } => {
                    // For tuple variants, specify "tuple element"
                    if matches!(signature, VariantSignature::Tuple(_)) {
                        format!("tuple element {index}")
                    } else {
                        format!("element {index}")
                    }
                }
                PathKind::ArrayElement { index, .. } => format!("array element {index}"),
                PathKind::RootValue { .. } => "root".to_string(),
            };

            // For PartiallyMutable, explain that it contains non-mutable descendants
            // For NotMutable, show the actual reason
            let reason_detail = if matches!(p.mutability, Mutability::PartiallyMutable) {
                format!("contains non-mutable descendants (see '{type_name}' mutation_paths for details)")
            } else {
                p.mutability_reason
                    .as_ref()
                    .map_or_else(|| "unknown reason".to_string(), |reason| format!("{reason}"))
            };

            format!("{field_label} ({type_name}): {reason_detail}")
        })
        .collect();

    if problematic_fields.is_empty() {
        // Shouldn't happen for PartiallyMutable, but handle gracefully
        return Ok(());
    }

    let field_list = problematic_fields.join("; ");
    let message = format!(
        "Cannot construct {} variant via BRP due to incomplete field data: {}. \
        This variant's mutable fields can only be mutated if the entity is \
        already set to this variant by your code.",
        variant_name.short_name(),
        field_list
    );

    Err(message)
}

/// Build mutation status reason for enums based on variant mutability
fn build_enum_mutability_reason(
    enum_mutability: Mutability,
    enum_examples: &[ExampleGroup],
    type_name: BrpTypeName,
) -> Option<NotMutableReason> {
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

            Some(NotMutableReason::from_partial_mutability(
                type_name,
                mutability_issues,
                message,
            ))
        }
        Mutability::NotMutable => {
            // Use NoMutableChildren variant instead of raw JSON
            Some(NotMutableReason::NoMutableChildren {
                parent_type: type_name,
            })
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
    mutability_reason: Option<NotMutableReason>,
) -> MutationPathInternal {
    // Generate `EnumPathData` only if we have a variant chain (nested in another enum)
    let enum_path_data = if ctx.variant_chain.is_empty() {
        None
    } else {
        Some(EnumPathInfo {
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
        enum_path_info: enum_path_data,
        depth: *ctx.depth,
        partial_root_examples: None,
    }
}

/// Propagate partial root examples to child paths at the root level
fn propagate_partial_root_examples_to_children(
    child_paths: &mut [MutationPathInternal],
    partial_root_examples: &HashMap<Vec<VariantName>, RootExample>,
    ctx: &RecursionContext,
) {
    if ctx.variant_chain.is_empty() {
        // Propagate HashMap to children
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
    partial_root_examples: HashMap<Vec<VariantName>, RootExample>,
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
