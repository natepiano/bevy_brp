//! Shared utilities for mutation path building
//!
//! This module contains functions used by both `path_builder.rs` (non-enum types)
//! and `enum_path_builder.rs` (enum types) to avoid code duplication and improve
//! maintainability.

use std::collections::HashMap;

use itertools::Itertools;
use serde_json::Value;

use super::mutation_path_internal::MutationPathInternal;
use super::new_types::VariantName;
use super::path_kind::MutationPathDescriptor;
use super::recursion_context::RecursionContext;
use super::types::Mutability;
use super::types::RootExample;

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
/// Use to filter children when building variant-specific examples.
fn is_variant_chain_compatible(child: &MutationPathInternal, child_chain: &[VariantName]) -> bool {
    if let Some(child_enum_data) = &child.enum_path_info {
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
            .partial_root_examples // Changed from partial_root_examples
            .as_ref()
            .and_then(|partials| {
                // Helper to extract Value from RootExample
                let get_value = |root_ex: &RootExample| match root_ex {
                    RootExample::Available { root_example } if !root_example.is_null() => {
                        Some(root_example.clone())
                    } // Skip null to prefer data-filled variants
                    _ => None,
                };

                // Try exact match first
                partials.get(chain).and_then(get_value).or_else(|| {
                    // For enum children: find a mutable (Available) variant
                    partials
                        .iter()
                        .filter(|(child_chain, _)| {
                            child_chain.len() == chain.len() + 1 && child_chain.starts_with(chain)
                        })
                        .sorted_by_key(|(child_chain, _)| *child_chain)
                        .find_map(|(_, root_ex)| get_value(root_ex))
                })
            })
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
/// Used for assembling struct-like objects from child examples in both structs and enum struct
/// variants.
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
    new_partials: &HashMap<Vec<VariantName>, RootExample>,
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_info
            && !enum_data.variant_chain.is_empty()
        {
            // Populate new field
            if let Some(root_example) = new_partials.get(&enum_data.variant_chain) {
                enum_data.root_example = Some(root_example.clone());
            }
        }
    }
}

/// Wrap an example with availability status by checking children for unavailability reasons
///
/// Implements hierarchical unavailability propagation:
/// 1. If parent has unavailability reason, use it (child is unreachable)
/// 2. Otherwise, check if any child has unavailability reason for this chain in their
///    `partial_root_examples`
///
/// This is shared between `path_builder.rs` (non-enum types) and `enum_path_builder.rs`
/// (enum types) to avoid code duplication.
pub fn wrap_example_with_availability(
    example: Value,
    children: &[&MutationPathInternal],
    chain: &[VariantName],
    parent_unavailable_reason: Option<String>,
) -> RootExample {
    let unavailable_reason = parent_unavailable_reason.or_else(|| {
        // Check children's `partial_root_examples` for unavailability of this chain
        children.iter().find_map(|child| {
            child
                .partial_root_examples
                .as_ref()
                .and_then(|examples| examples.get(chain))
                .and_then(|root_example| match root_example {
                    RootExample::Unavailable {
                        root_example_unavailable_reason,
                    } => Some(root_example_unavailable_reason.clone()),
                    RootExample::Available { .. } => None,
                })
        })
    });

    unavailable_reason.map_or(
        RootExample::Available {
            root_example: example,
        },
        |reason| RootExample::Unavailable {
            root_example_unavailable_reason: reason,
        },
    )
}
