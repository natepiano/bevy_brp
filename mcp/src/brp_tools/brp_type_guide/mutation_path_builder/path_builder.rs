//! Generic mutation path builder for non-enum types using the `PathBuilder` trait
//!
//! This module handles path building for all non-enum types (structs, tuples, arrays, etc.)
//! through a unified `MutationPathBuilder` that wraps type-specific builders. It manages:
//! - Recursive traversal of type hierarchies
//! - Mutation status determination based on child mutability
//! - Variant chain propagation for types inside enum variants
//!
//! ## Key Responsibilities
//!
//! 1. **Child Processing**: Recursively builds paths for all child elements
//! 2. **Status Aggregation**: Determines parent mutability from child statuses
//! 3. **Variant Chain Handling**: Passes through variant requirements from parent enums
//! 4. **Knowledge Integration**: Applies hardcoded mutation knowledge when available
//!
//! ## Central Dispatch
//!
//! The `recurse_mutation_paths` function is the single entry point that dispatches to either:
//! - `enum_path_builder::process_enum` for enum types
//! - `MutationPathBuilder` with appropriate builder for all other types
use std::collections::{BTreeMap, BTreeSet, HashMap};

use error_stack::Report;
use serde_json::{Value, json};

use super::super::type_kind::TypeKind;
use super::super::type_knowledge::TypeKnowledge;
use super::mutation_path_internal::MutationPathInternal;
use super::new_types::VariantName;
use super::not_mutable_reason::NotMutableReason;
use super::path_kind::{MutationPathDescriptor, PathKind};
use super::recursion_context::RecursionContext;
use super::type_kind_builders::{
    ArrayMutationBuilder, ListMutationBuilder, MapMutationBuilder, SetMutationBuilder,
    StructMutationBuilder, TupleMutationBuilder, TypeKindBuilder, ValueMutationBuilder,
};
use super::types::{EnumPathData, Mutability, MutabilityIssue, PathAction, PathExample};
use super::{BuilderError, enum_builder};
use crate::error::{Error, Result};

/// Result of processing all children during mutation path building
struct ChildProcessingResult {
    /// All child paths (used for mutation status determination)
    all_paths:       Vec<MutationPathInternal>,
    /// Only paths that should be exposed (filtered by `PathAction`)
    paths_to_expose: Vec<MutationPathInternal>,
    /// Examples for each child path
    child_examples:  HashMap<MutationPathDescriptor, Value>,
}

pub struct MutationPathBuilder<B: TypeKindBuilder> {
    inner: B,
}

impl<B: TypeKindBuilder<Item = PathKind>> TypeKindBuilder for MutationPathBuilder<B> {
    type Item = B::Item;
    type Iter<'a>
        = B::Iter<'a>
    where
        Self: 'a,
        B: 'a;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        // Delegate to the inner builder
        self.inner.collect_children(ctx)
    }

    fn build_paths(
        &self,
        ctx: &RecursionContext,
    ) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
        // Early returns for simple cases
        if let Some(result) = Self::check_registry(ctx) {
            return result;
        }

        // Check knowledge - might return early or provide example
        let (knowledge_result, knowledge_example) = Self::check_knowledge(ctx);
        if let Some(result) = knowledge_result {
            return result;
        }

        // Process all children and collect paths
        let ChildProcessingResult {
            all_paths,
            paths_to_expose,
            child_examples,
        } = self.process_all_children(ctx)?;

        // Assemble THIS level from children (post-order)
        let assembled_example = self
            .inner
            .assemble_from_children(ctx, child_examples.clone())?;

        // NEW: Assemble partial_root_examples from children (same bottom-up approach)
        // Filter to only direct children by matching against child_examples keys
        let direct_children: Vec<&MutationPathInternal> = all_paths
            .iter()
            .filter(|p| child_examples.contains_key(&p.path_kind.to_mutation_path_descriptor()))
            .collect();
        let partial_root_examples =
            Self::assemble_partial_root_examples(&self.inner, ctx, direct_children.as_slice())?;

        // Use knowledge example if available (for Teach types), otherwise use assembled example
        let final_example =
            knowledge_example.map_or(assembled_example, |knowledge_example| knowledge_example);

        // Compute parent's mutation status from children's statuses
        let (parent_status, reason_enum) = determine_parent_mutability(ctx, &all_paths);

        // Convert NotMutableReason to Value if present
        let mutability_reason = reason_enum.as_ref().and_then(Option::<Value>::from);

        // Build examples appropriately based on mutation status
        let example_to_use = match parent_status {
            Mutability::NotMutable => json!(null),
            Mutability::PartiallyMutable => {
                // Build partial example with only mutable children
                let mutable_child_examples: HashMap<_, _> = child_examples
                    .iter()
                    .filter(|(descriptor, _)| {
                        // Find the child path and check if it's mutable
                        all_paths.iter().any(|p| {
                            p.path_kind.to_mutation_path_descriptor() == **descriptor
                                && matches!(p.mutability, Mutability::Mutable)
                        })
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // Assemble from only mutable children
                self.inner
                    .assemble_from_children(ctx, mutable_child_examples)
                    .unwrap_or(json!(null))
            }
            Mutability::Mutable => final_example,
        };

        // Return error only for NotMutable, success for Mutable and PartiallyMutable
        match parent_status {
            Mutability::NotMutable => {
                let mutability_reason = reason_enum.ok_or_else(|| {
                    BuilderError::SystemError(Report::new(Error::InvalidState(
                        "NotMutable status must have a reason".to_string(),
                    )))
                })?;
                Err(BuilderError::NotMutable(mutability_reason))
            }
            Mutability::Mutable | Mutability::PartiallyMutable => Ok(Self::build_final_result(
                ctx,
                paths_to_expose,
                example_to_use,
                parent_status,
                mutability_reason,
                partial_root_examples,
            )),
        }
    }
}

// Feature flag removed - EnumPathBuilder is now the permanent implementation

/// Single dispatch point for creating builders - used for both entry and recursion
/// This is the ONLY place where we match on `TypeKind` to create builders
///
/// # Simplified Context Handling
///
/// With the removal of `EnumContext`, the `RecursionContext` is now immutable throughout
/// the recursion. Each type handles its own behavior without needing to coordinate context states.
///
/// # Depth Limit Checking
///
/// Depth limit checking is now automatic in `RecursionContext::create_recursion_context()`.
/// The check happens at the point where depth is incremented, ensuring developers cannot
/// accidentally skip the check.
pub fn recurse_mutation_paths(
    type_kind: TypeKind,
    ctx: &RecursionContext,
) -> Result<Vec<MutationPathInternal>> {
    let mutation_result = match type_kind {
        // Enum is distinct from the rest but now returns MutationResult too
        TypeKind::Enum => enum_builder::process_enum(ctx),
        TypeKind::Struct => MutationPathBuilder::new(StructMutationBuilder).build_paths(ctx),
        TypeKind::Tuple | TypeKind::TupleStruct => {
            MutationPathBuilder::new(TupleMutationBuilder).build_paths(ctx)
        }
        TypeKind::Array => MutationPathBuilder::new(ArrayMutationBuilder).build_paths(ctx),
        TypeKind::List => MutationPathBuilder::new(ListMutationBuilder).build_paths(ctx),
        TypeKind::Map => MutationPathBuilder::new(MapMutationBuilder).build_paths(ctx),
        TypeKind::Set => MutationPathBuilder::new(SetMutationBuilder).build_paths(ctx),
        TypeKind::Value => MutationPathBuilder::new(ValueMutationBuilder).build_paths(ctx),
    };

    // Convert BuilderError to public Result interface at module boundary
    // This is the choke point where NotMutableReason becomes a success with NotMutable path
    match mutation_result {
        Ok(paths) => Ok(paths),
        Err(BuilderError::NotMutable(reason)) => {
            Ok(vec![
                MutationPathBuilder::<ValueMutationBuilder>::build_not_mutable_path(ctx, reason),
            ])
        }
        Err(BuilderError::SystemError(e)) => Err(e),
    }
}

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
pub fn is_variant_chain_compatible(
    child: &MutationPathInternal,
    child_chain: &[VariantName],
) -> bool {
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
/// Used by both enum and non-enum types.
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

/// Determine parent's mutation status based on children's statuses and return detailed reasons
///
/// This is a shared helper function used by both non-enum types (via `MutationPathBuilder`)
/// and enum types (via `enum_path_builder::create_result_paths`).
///
/// ## Special Case: Maps and Sets
///
/// Maps and Sets require ALL children to be mutable for BRP operations:
/// - `HashMap<K, V>` needs both K and V mutable (can't insert with non-serializable key)
/// - `HashSet<T>` needs T mutable (can't insert non-serializable element)
///
/// Unlike Structs where some fields can be mutable and others not, collections are
/// all-or-nothing: either you can perform operations or you can't.
pub fn determine_parent_mutability(
    ctx: &RecursionContext,
    child_paths: &[MutationPathInternal],
) -> (Mutability, Option<NotMutableReason>) {
    // Get TypeKind for special case handling
    let schema = ctx.registry.get(ctx.type_name()).unwrap_or(&Value::Null);
    let type_kind = TypeKind::from_schema(schema);

    // SPECIAL CASE: Map and Set require ALL children to be mutable
    // Maps need both key AND value mutable for operations like insert(key, value)
    // Sets need element mutable for operations like insert(element)
    if matches!(type_kind, TypeKind::Map | TypeKind::Set) {
        let has_not_mutable = child_paths
            .iter()
            .any(|p| matches!(p.mutability, Mutability::NotMutable));

        if has_not_mutable {
            // Map/Set is NotMutable if ANY child is NotMutable
            let mutability_issues: Vec<MutabilityIssue> = child_paths
                .iter()
                .map(MutationPathInternal::to_mutability_issue)
                .collect();

            let collection_type = if matches!(type_kind, TypeKind::Map) {
                "Maps"
            } else {
                "Sets"
            };

            let reason = NotMutableReason::from_partial_mutability(
                ctx.type_name().clone(),
                mutability_issues,
                format!(
                    "{collection_type} require all {} to be mutable for BRP operations",
                    type_kind.child_terminology()
                ),
            );

            return (Mutability::NotMutable, Some(reason));
        }
    }

    // Extract statuses and aggregate (normal logic for non-Map/Set types)
    let statuses: Vec<Mutability> = child_paths.iter().map(|p| p.mutability).collect();

    let status = aggregate_mutability(&statuses);

    // Build detailed reason if not fully mutable
    let reason = match status {
        Mutability::PartiallyMutable => {
            let mutability_issues: Vec<MutabilityIssue> = child_paths
                .iter()
                .map(MutationPathInternal::to_mutability_issue)
                .collect();

            let message = format!(
                "Some {} are mutable while others are not",
                type_kind.child_terminology()
            );

            Some(NotMutableReason::from_partial_mutability(
                ctx.type_name().clone(),
                mutability_issues,
                message,
            ))
        }
        Mutability::NotMutable => Some(ctx.create_no_mutable_children_error()),
        Mutability::Mutable => None,
    };

    (status, reason)
}

impl<B: TypeKindBuilder<Item = PathKind>> MutationPathBuilder<B> {
    pub const fn new(inner: B) -> Self {
        Self { inner }
    }

    /// Process all children and collect their paths and examples
    fn process_all_children(
        &self,
        ctx: &RecursionContext,
    ) -> std::result::Result<ChildProcessingResult, BuilderError> {
        // Collect children for depth-first traversal
        let child_items = self
            .inner
            .collect_children(ctx)
            .map_err(BuilderError::SystemError)?;
        let mut all_paths = vec![];
        let mut paths_to_expose = vec![]; // Paths that should be included in final result
        let mut child_examples = HashMap::<MutationPathDescriptor, Value>::new();

        // Recurse to each child (they handle their own protocol)
        for path_kind in child_items {
            let child_ctx =
                ctx.create_recursion_context(path_kind.clone(), self.inner.child_path_action())?;

            // Extract descriptor from PathKind for HashMap
            let child_key = path_kind.to_mutation_path_descriptor();

            let (child_paths, child_example) = Self::process_child(&child_key, &child_ctx)?;
            child_examples.insert(child_key, child_example);

            // Always collect all paths for analysis
            all_paths.extend(child_paths.clone());

            // Only add to paths_to_expose if this child should be created
            if matches!(child_ctx.path_action, PathAction::Create) {
                paths_to_expose.extend(child_paths);
            }
        }

        Ok(ChildProcessingResult {
            all_paths,
            paths_to_expose,
            child_examples,
        })
    }

    /// Process a single child and return its paths and example value
    fn process_child(
        descriptor: &MutationPathDescriptor,
        child_ctx: &RecursionContext,
    ) -> Result<(Vec<MutationPathInternal>, Value)> {
        tracing::debug!(
            "PROCESS_CHILD: descriptor='{}', type='{}', path='{}', depth={}",
            &**descriptor,
            child_ctx.type_name(),
            child_ctx.mutation_path,
            *child_ctx.depth
        );

        // Check if child type is in registry first
        // If not, return NotMutable path directly without recursing
        let Ok(child_schema) = child_ctx.require_registry_schema() else {
            // Type not in registry - return NotMutable path directly
            let not_mutable_path = Self::build_not_mutable_path(
                child_ctx,
                NotMutableReason::NotInRegistry(child_ctx.type_name().clone()),
            );
            return Ok((vec![not_mutable_path], json!(null)));
        };

        let child_kind = TypeKind::from_schema(child_schema);

        tracing::debug!(
            "PROCESS_CHILD: calling recurse_mutation_paths, type_kind={:?}",
            child_kind
        );

        let child_paths = recurse_mutation_paths(child_kind, child_ctx)?;

        tracing::debug!(
            "PROCESS_CHILD: recurse returned {} paths",
            child_paths.len()
        );

        // Extract child's example - handle both simple and enum root cases
        let child_example = child_paths
            .first()
            .map_or(json!(null), |p| p.example.for_parent().clone());

        Ok((child_paths, child_example))
    }

    /// Build a `MutationPathInternal` with the provided status and example
    ///
    /// Used by `build_not_mutable_path` for `NotMutableReason`s and `check_knowledge`
    /// when we already have a hard coded example and don't need to try to build our own.
    ///
    /// Finally, used by `build_final_result`
    ///
    /// Generates enum variant selection instructions for any type (non-enum) that exists
    /// within an enum's variant tree. The instructions explain how many variant
    /// selections are needed (based on `variant_chain` length) to reach this mutation path.
    fn build_mutation_path_internal(
        ctx: &RecursionContext,
        example: PathExample,
        status: Mutability,
        mutability_reason: Option<Value>,
        partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
    ) -> MutationPathInternal {
        // Build enum data if variant chain exists
        let enum_path_data = if ctx.variant_chain.is_empty() {
            None
        } else {
            Some(EnumPathData {
                variant_chain:       ctx.variant_chain.clone(),
                applicable_variants: Vec::new(),
                root_example:        None,
            })
        };

        MutationPathInternal {
            mutation_path: ctx.mutation_path.clone(),
            example,
            type_name: ctx.type_name().display_name(),
            path_kind: ctx.path_kind.clone(),
            mutability: status,
            mutability_reason,
            enum_path_data,
            depth: *ctx.depth,
            partial_root_examples,
        }
    }

    /// Assemble `partial_root_examples` from children using same bottom-up approach
    ///
    /// For each variant chain present in any child:
    /// 1. Collect each child's value for that chain
    /// 2. Assemble them using the builder's assembly logic (struct/tuple/etc)
    /// 3. Store the assembled value for that chain
    fn assemble_partial_root_examples(
        builder: &B,
        ctx: &RecursionContext,
        child_paths: &[&MutationPathInternal],
    ) -> std::result::Result<Option<BTreeMap<Vec<VariantName>, Value>>, BuilderError> {
        // Collect all unique variant chains from all children
        let mut all_chains = BTreeSet::new();
        for child in child_paths {
            if let Some(partial_root_example) = &child.partial_root_examples {
                for chain in partial_root_example.keys() {
                    all_chains.insert(chain.clone());
                }
            }
        }

        if all_chains.is_empty() {
            return Ok(None);
        }

        let mut assembled_partial_root_examples = BTreeMap::new();

        // For each variant chain, assemble wrapped example from compatible children
        for chain in all_chains {
            tracing::debug!(
                "🔧 ASSEMBLE_PARTIAL: Processing chain {:?} for type {}",
                chain,
                ctx.type_name()
            );

            // Use shared choke point for filtering and value extraction
            let examples_for_chain = collect_children_for_chain(child_paths, ctx, Some(&chain));

            // Assemble from filtered children
            let assembled = builder.assemble_from_children(ctx, examples_for_chain)?;
            tracing::debug!(
                "🔧 ASSEMBLED for chain {:?}: {}",
                chain,
                serde_json::to_string(&assembled).unwrap_or_else(|_| "ERR".to_string())
            );

            assembled_partial_root_examples.insert(chain, assembled);
        }

        Ok(Some(assembled_partial_root_examples))
    }

    /// Build final result based on `PathAction`
    fn build_final_result(
        ctx: &RecursionContext,
        mut paths_to_expose: Vec<MutationPathInternal>,
        example_to_use: Value,
        parent_status: Mutability,
        mutability_reason: Option<Value>,
        partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
    ) -> Vec<MutationPathInternal> {
        if let Some(ref partials) = partial_root_examples {
            // Propagate assembled partial_root_examples to all children
            for child in &mut paths_to_expose {
                child.partial_root_examples = Some(partials.clone());
            }

            // Populate root_example from partial_root_examples for children with enum_path_data
            populate_root_examples_from_partials(&mut paths_to_expose, partials);
        }

        match ctx.path_action {
            PathAction::Create => {
                // Normal mode: Add root path and return only paths marked for exposure
                paths_to_expose.insert(
                    0,
                    Self::build_mutation_path_internal(
                        ctx,
                        PathExample::Simple(example_to_use),
                        parent_status,
                        mutability_reason,
                        partial_root_examples,
                    ),
                );
                paths_to_expose
            }
            PathAction::Skip => {
                // Skip mode: Return ONLY a root path with the example
                // This ensures the example is available for parent assembly
                // but child paths aren't exposed in the final result
                vec![Self::build_mutation_path_internal(
                    ctx,
                    PathExample::Simple(example_to_use),
                    parent_status,
                    mutability_reason,
                    partial_root_examples,
                )]
            }
        }
    }

    /// Build a `NotMutatable` path with consistent formatting (private to `MutationPathBuilder`)
    ///
    /// This centralizes `NotMutable` path creation, ensuring only `MutationPathBuilder`
    /// can create these paths while builders simply return `Error::NotMutable`.
    fn build_not_mutable_path(
        ctx: &RecursionContext,
        reason: NotMutableReason,
    ) -> MutationPathInternal {
        Self::build_mutation_path_internal(
            ctx,
            PathExample::Simple(json!(null)), // No example for NotMutable paths
            Mutability::NotMutable,
            Option::<Value>::from(&reason),
            None, // No partial roots for NotMutable paths
        )
    }

    /// Check if type is in registry and return `NotMutable` path if not found
    fn check_registry(
        ctx: &RecursionContext,
    ) -> Option<std::result::Result<Vec<MutationPathInternal>, BuilderError>> {
        if ctx.require_registry_schema().is_err() {
            Some(Err(BuilderError::NotMutable(
                NotMutableReason::NotInRegistry(ctx.type_name().clone()),
            )))
        } else {
            None
        }
    }

    /// Check knowledge base and handle based on guidance type
    fn check_knowledge(
        ctx: &RecursionContext,
    ) -> (
        Option<std::result::Result<Vec<MutationPathInternal>, BuilderError>>,
        Option<Value>,
    ) {
        // Use unified knowledge lookup that handles all cases
        let knowledge_result = ctx.find_knowledge();
        match knowledge_result {
            Ok(Some(knowledge)) => {
                let example = knowledge.example().clone();

                // Only return early for TreatAsValue types - they should not recurse
                if matches!(knowledge, TypeKnowledge::TreatAsRootValue { .. }) {
                    return (
                        Some(Ok(vec![Self::build_mutation_path_internal(
                            ctx,
                            PathExample::Simple(example),
                            Mutability::Mutable,
                            None,
                            None, // No partial roots for knowledge-based paths
                        )])),
                        None,
                    );
                }

                (None, Some(example))
            }
            Ok(None) => {
                // Continue with normal processing, no hard coded mutation knowledge found
                (None, None)
            }
            Err(e) => {
                // Propagate error from find_knowledge()
                (Some(Err(e)), None)
            }
        }
    }
}
