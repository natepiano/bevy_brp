//! Shared utilities for mutation path building
//!
//! This module contains functions used by both `path_builder.rs` (non-enum types)
//! and `enum_path_builder.rs` (enum types) to avoid code duplication and improve
//! maintainability.

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use super::mutation_path_internal::MutationPathInternal;
use super::new_types::VariantName;
use super::path_kind::MutationPathDescriptor;
use super::recursion_context::RecursionContext;
use super::types::Mutability;

/// Aggregate multiple mutation statuses into a single status
///
/// Logic:
/// - If any `PartiallyMutable` OR (has both `Mutable` and `NotMutable`) → `PartiallyMutable`
/// - Else if any `NotMutable` → `NotMutable`
/// - Else → `Mutable`
pub fn aggregate_mutability(statuses: &[Mutability]) -> Mutability {
    let has_partially_mutable = statuses
        .iter()
        .any(|s| matches!(s, Mutability::PartiallyMutable));

    let has_mutable = statuses.iter().any(|s| matches!(s, Mutability::Mutable));

    let has_not_mutable = statuses.iter().any(|s| matches!(s, Mutability::NotMutable));

    if has_partially_mutable || (has_mutable && has_not_mutable) {
        Mutability::PartiallyMutable
    } else if has_not_mutable {
        Mutability::NotMutable
    } else {
        Mutability::Mutable
    }
}

/// Check if a child's `variant_chain` is compatible with a target chain
///
/// Compatibility means the child's `variant_chain` must be a prefix of the target `child_chain`.
///
/// This is shared between `path_builder.rs` and `enum_builder.rs` to filter children
/// when building variant-specific examples.
fn is_variant_chain_compatible(child: &MutationPathInternal, child_chain: &[VariantName]) -> bool {
    if let Some(child_enum_data) = &child.enum_path_data {
        // Child's variant_chain cannot be longer than target chain
        if child_enum_data.variant_chain.len() > child_chain.len() {
            return false;
        }

        // Check prefix compatibility: all elements must match
        child_enum_data
            .variant_chain
            .iter()
            .zip(child_chain.iter())
            .all(|(child_v, chain_v)| child_v == chain_v)
    } else {
        true // Non-enum children are always compatible
    }
}

/// Extract the appropriate value from a child path for assembly
///
/// Priority order:
/// 1. Variant-specific value from `partial_root_examples` (for deeply nested enums)
/// 2. `example.for_parent()` (fallback for all other cases)
fn extract_child_value_for_chain(
    child: &MutationPathInternal,
    child_chain: Option<&[VariantName]>,
) -> Value {
    let fallback = || child.example.for_parent().clone();

    child_chain.map_or_else(fallback, |chain| {
        child
            .partial_root_examples
            .as_ref()
            .and_then(|partials| partials.get(chain))
            .cloned()
            .unwrap_or_else(fallback)
    })
}

/// Collect children values for a specific variant chain
///
/// Used by both enum and non-enum types for constructing `root_example` fields
///
/// Filtering rules:
/// 1. Only direct children at current depth
/// 2. Only children compatible with target variant chain (if specified)
///
/// Value extraction:
/// 1. Variant-specific value from `partial_root_examples[chain]` if available
/// 2. Fallback to `example.for_parent()` otherwise
pub fn collect_children_for_chain(
    child_paths: &[&MutationPathInternal],
    ctx: &RecursionContext,
    target_chain: Option<&[VariantName]>,
) -> HashMap<MutationPathDescriptor, Value> {
    child_paths
        .iter()
        // Skip grandchildren - only process direct children
        .filter(|child| child.is_direct_child_at_depth(*ctx.depth))
        // Filter by variant-chain compatibility if target chain specified
        .filter(|child| target_chain.is_none_or(|chain| is_variant_chain_compatible(child, chain)))
        // Exclude NotMutable children - they can't be set in root_example
        // Include Mutable and PartiallyMutable (enum builder selects mutable variants)
        .filter(|child| !matches!(child.mutability, Mutability::NotMutable))
        // Map to (descriptor, value) pairs
        .map(|child| {
            let descriptor = child.path_kind.to_mutation_path_descriptor();
            let value = extract_child_value_for_chain(child, target_chain);
            (descriptor, value)
        })
        .collect()
}

/// Assemble a struct JSON object from child field examples
///
/// Only includes fields that exist in the `children` `HashMap` - does not add null defaults
/// for missing fields. This allows BRP to use the type's `Default` implementation to fill
/// in any missing required fields.
///
/// Used for assembling struct-like objects from child examples,
/// shared by both `StructMutationBuilder` and `build_variant_example` for enum struct variants.
pub fn assemble_struct_from_children(
    children: &HashMap<MutationPathDescriptor, Value>,
) -> serde_json::Map<String, Value> {
    let mut struct_obj = serde_json::Map::new();

    for (descriptor, example) in children {
        let field_name = (*descriptor).to_string();
        struct_obj.insert(field_name, example.clone());
    }

    struct_obj
}

/// Populate `root_example` from `partial_root_examples` for enum paths
///
/// Iterates through mutation paths and populates the `root_example` field for any paths
/// that have enum variant requirements (non-empty `variant_chain`).
///
/// This is shared between `builder.rs` and `enum_path_builder.rs` to avoid code duplication.
pub fn populate_root_examples_from_partials(
    paths: &mut [MutationPathInternal],
    partials: &BTreeMap<Vec<VariantName>, Value>,
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_data
            && !enum_data.variant_chain.is_empty()
            && let Some(root_example) = partials.get(&enum_data.variant_chain)
        {
            enum_data.root_example = Some(root_example.clone());
        }
    }
}
