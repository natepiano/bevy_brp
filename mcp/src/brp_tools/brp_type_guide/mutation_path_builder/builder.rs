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
//!
//! ## Integration
//!
//! Types inside enum variants inherit variant chains but don't populate them - parent enums
//! handle all variant path population through `update_child_variant_paths`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use error_stack::Report;
use serde_json::{Value, json};

use super::super::type_kind::TypeKind;
use super::builders::{
    ArrayMutationBuilder, ListMutationBuilder, MapMutationBuilder, SetMutationBuilder,
    StructMutationBuilder, TupleMutationBuilder, ValueMutationBuilder,
};
use super::mutation_knowledge::MutationKnowledge;
use super::path_builder::PathBuilder;
use super::types::{EnumPathData, PathAction, PathSummary, VariantName};
use super::{
    BuilderError, MutationPathDescriptor, MutationPathInternal, MutationStatus, NotMutableReason,
    PathKind, RecursionContext, enum_path_builder,
};
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

pub struct MutationPathBuilder<B: PathBuilder> {
    inner: B,
}

impl<B: PathBuilder<Item = PathKind>> PathBuilder for MutationPathBuilder<B> {
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
        let (parent_status, reason_enum) = determine_parent_mutation_status(ctx, &all_paths);

        // Convert NotMutableReason to Value if present
        let mutation_status_reason = reason_enum.as_ref().and_then(Option::<Value>::from);

        // Build examples appropriately based on mutation status
        let example_to_use = match parent_status {
            MutationStatus::NotMutable => json!(null),
            MutationStatus::PartiallyMutable => {
                // Build partial example with only mutable children
                let mutable_child_examples: HashMap<_, _> = child_examples
                    .iter()
                    .filter(|(descriptor, _)| {
                        // Find the child path and check if it's mutable
                        all_paths.iter().any(|p| {
                            p.path_kind.to_mutation_path_descriptor() == **descriptor
                                && matches!(p.mutation_status, MutationStatus::Mutable)
                        })
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // Assemble from only mutable children
                self.inner
                    .assemble_from_children(ctx, mutable_child_examples)
                    .unwrap_or(json!(null))
            }
            MutationStatus::Mutable => final_example,
        };

        // Return error only for NotMutable, success for Mutable and PartiallyMutable
        match parent_status {
            MutationStatus::NotMutable => {
                let reason = reason_enum.ok_or_else(|| {
                    BuilderError::SystemError(Report::new(Error::InvalidState(
                        "NotMutable status must have a reason".to_string(),
                    )))
                })?;
                Err(BuilderError::NotMutable(reason))
            }
            MutationStatus::Mutable | MutationStatus::PartiallyMutable => {
                Ok(Self::build_final_result(
                    ctx,
                    paths_to_expose,
                    example_to_use,
                    parent_status,
                    mutation_status_reason,
                    partial_root_examples,
                ))
            }
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
        TypeKind::Enum => enum_path_builder::process_enum(ctx),
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
pub fn aggregate_mutation_statuses(statuses: &[MutationStatus]) -> MutationStatus {
    let has_partially_mutable = statuses
        .iter()
        .any(|s| matches!(s, MutationStatus::PartiallyMutable));

    let has_mutable = statuses
        .iter()
        .any(|s| matches!(s, MutationStatus::Mutable));

    let has_not_mutable = statuses
        .iter()
        .any(|s| matches!(s, MutationStatus::NotMutable));

    if has_partially_mutable || (has_mutable && has_not_mutable) {
        MutationStatus::PartiallyMutable
    } else if has_not_mutable {
        MutationStatus::NotMutable
    } else {
        MutationStatus::Mutable
    }
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
        if let Some(enum_data) = &mut path.enum_path_data {
            if !enum_data.variant_chain.is_empty() {
                if let Some(root_example) = partials.get(&enum_data.variant_chain) {
                    enum_data.root_example = Some(root_example.clone());
                }
            }
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
pub fn determine_parent_mutation_status(
    ctx: &RecursionContext,
    child_paths: &[MutationPathInternal],
) -> (MutationStatus, Option<NotMutableReason>) {
    // Get TypeKind for special case handling
    let schema = ctx.registry.get(ctx.type_name()).unwrap_or(&Value::Null);
    let type_kind = TypeKind::from_schema(schema);

    // SPECIAL CASE: Map and Set require ALL children to be mutable
    // Maps need both key AND value mutable for operations like insert(key, value)
    // Sets need element mutable for operations like insert(element)
    if matches!(type_kind, TypeKind::Map | TypeKind::Set) {
        let has_not_mutable = child_paths
            .iter()
            .any(|p| matches!(p.mutation_status, MutationStatus::NotMutable));

        if has_not_mutable {
            // Map/Set is NotMutable if ANY child is NotMutable
            let summaries: Vec<PathSummary> = child_paths
                .iter()
                .map(MutationPathInternal::to_path_summary)
                .collect();

            let collection_type = if matches!(type_kind, TypeKind::Map) {
                "Maps"
            } else {
                "Sets"
            };

            let reason = NotMutableReason::from_partial_mutability(
                ctx.type_name().clone(),
                summaries,
                format!(
                    "{collection_type} require all {} to be mutable for BRP operations",
                    type_kind.child_terminology()
                ),
            );

            return (MutationStatus::NotMutable, Some(reason));
        }
    }

    // Extract statuses and aggregate (normal logic for non-Map/Set types)
    let statuses: Vec<MutationStatus> = child_paths.iter().map(|p| p.mutation_status).collect();

    let status = aggregate_mutation_statuses(&statuses);

    // Build detailed reason if not fully mutable
    let reason = match status {
        MutationStatus::PartiallyMutable => {
            let summaries: Vec<PathSummary> = child_paths
                .iter()
                .map(MutationPathInternal::to_path_summary)
                .collect();

            let message = format!(
                "Some {} are mutable while others are not",
                type_kind.child_terminology()
            );

            Some(NotMutableReason::from_partial_mutability(
                ctx.type_name().clone(),
                summaries,
                message,
            ))
        }
        MutationStatus::NotMutable => Some(ctx.create_no_mutable_children_error()),
        MutationStatus::Mutable => None,
    };

    (status, reason)
}

impl<B: PathBuilder<Item = PathKind>> MutationPathBuilder<B> {
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
            child_ctx.full_mutation_path,
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
        let child_example = child_paths.first().map_or(json!(null), |p| {
            p.enum_example_for_parent
                .as_ref()
                .map_or_else(|| p.example.clone(), Clone::clone)
        });

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
        example: Value,
        status: MutationStatus,
        mutation_status_reason: Option<Value>,
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
            full_mutation_path: ctx.full_mutation_path.clone(),
            example,
            enum_example_groups: None,
            enum_example_for_parent: None,
            type_name: ctx.type_name().display_name(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: status,
            mutation_status_reason,
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

        // For each variant chain, assemble wrapped example from ALL children
        for chain in all_chains {
            let mut examples_for_chain = HashMap::new();

            // Collect from ALL children, using variant-specific value if available, otherwise
            // regular example
            for child in child_paths {
                let descriptor = child.path_kind.to_mutation_path_descriptor();

                // Try to get variant-specific value first
                if let Some(partial_root_example) = &child.partial_root_examples
                    && let Some(child_value) = partial_root_example.get(&chain)
                {
                    examples_for_chain.insert(descriptor, child_value.clone());
                    continue;
                }

                // No variant-specific value, use regular example
                examples_for_chain.insert(descriptor, child.example.clone());
            }

            // Assemble from all children
            let assembled = builder.assemble_from_children(ctx, examples_for_chain)?;

            assembled_partial_root_examples.insert(chain, assembled);
        }

        Ok(Some(assembled_partial_root_examples))
    }

    /// Build final result based on `PathAction`
    fn build_final_result(
        ctx: &RecursionContext,
        mut paths_to_expose: Vec<MutationPathInternal>,
        example_to_use: Value,
        parent_status: MutationStatus,
        mutation_status_reason: Option<Value>,
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
                        example_to_use,
                        parent_status,
                        mutation_status_reason,
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
                    example_to_use,
                    parent_status,
                    mutation_status_reason,
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
            json!(null), // No example for NotMutable paths
            MutationStatus::NotMutable,
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
                if matches!(knowledge, MutationKnowledge::TreatAsRootValue { .. }) {
                    return (
                        Some(Ok(vec![Self::build_mutation_path_internal(
                            ctx,
                            example,
                            MutationStatus::Mutable,
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
