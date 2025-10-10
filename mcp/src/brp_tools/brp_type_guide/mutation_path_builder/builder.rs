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

use std::collections::{BTreeMap, HashMap};

use error_stack::Report;
use serde_json::{Value, json};

use super::super::constants::RecursionDepth;
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
        depth: RecursionDepth,
    ) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
        // Early returns for simple cases
        if let Some(result) = Self::check_depth_limit(ctx, depth) {
            return result;
        }

        if let Some(result) = Self::check_registry(ctx) {
            return result;
        }

        // Check knowledge - might return early or provide example
        let (knowledge_result, knowledge_example) = Self::check_knowledge(ctx, depth);
        if let Some(result) = knowledge_result {
            return result;
        }

        // Process all children and collect paths
        let ChildProcessingResult {
            all_paths,
            paths_to_expose,
            child_examples,
        } = self
            .process_all_children(ctx, depth)
            .map_err(BuilderError::SystemError)?;

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
        let assembled_partial_roots_new =
            Self::assemble_partial_roots_new(&self.inner, ctx, direct_children.as_slice())?;

        // Use knowledge example if available (for Teach types), otherwise use assembled example
        let final_example = knowledge_example.map_or(assembled_example, |knowledge_example| {
            tracing::debug!(
                "Using knowledge example for {} instead of assembled value",
                ctx.type_name()
            );
            knowledge_example
        });

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
                    assembled_partial_roots_new,
                    depth,
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
pub fn recurse_mutation_paths(
    type_kind: TypeKind,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<Vec<MutationPathInternal>> {
    tracing::debug!(
        "[DISPATCH] recurse_mutation_paths called: type_kind={:?}, type={}, path={}",
        type_kind,
        ctx.type_name(),
        ctx.full_mutation_path
    );

    let mutation_result = match type_kind {
        // Enum is distinct from the rest but now returns MutationResult too
        TypeKind::Enum => {
            tracing::debug!(
                "[DISPATCH] Dispatching to enum_path_builder::process_enum for {}",
                ctx.type_name()
            );
            enum_path_builder::process_enum(ctx, depth)
        }
        TypeKind::Struct => MutationPathBuilder::new(StructMutationBuilder).build_paths(ctx, depth),
        TypeKind::Tuple | TypeKind::TupleStruct => {
            MutationPathBuilder::new(TupleMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Array => MutationPathBuilder::new(ArrayMutationBuilder).build_paths(ctx, depth),
        TypeKind::List => MutationPathBuilder::new(ListMutationBuilder).build_paths(ctx, depth),
        TypeKind::Map => MutationPathBuilder::new(MapMutationBuilder).build_paths(ctx, depth),
        TypeKind::Set => MutationPathBuilder::new(SetMutationBuilder).build_paths(ctx, depth),
        TypeKind::Value => MutationPathBuilder::new(ValueMutationBuilder).build_paths(ctx, depth),
    };

    // Convert BuilderError to public Result interface at module boundary
    // This is the choke point where NotMutableReason becomes a success with NotMutable path
    match mutation_result {
        Ok(paths) => Ok(paths),
        Err(BuilderError::NotMutable(reason)) => Ok(vec![MutationPathBuilder::<
            ValueMutationBuilder,
        >::build_not_mutable_path(
            ctx, reason, depth
        )]),
        Err(BuilderError::SystemError(e)) => Err(e),
    }
}

/// Determine parent's mutation status based on children's statuses and return detailed reasons
///
/// This is a shared helper function used by both non-enum types (via `MutationPathBuilder`)
/// and enum types (via `enum_path_builder::create_result_paths`).
pub fn determine_parent_mutation_status(
    ctx: &RecursionContext,
    child_paths: &[MutationPathInternal],
) -> (MutationStatus, Option<NotMutableReason>) {
    // Check for any partially mutable children
    let has_partially_mutable = child_paths
        .iter()
        .any(|p| matches!(p.mutation_status, MutationStatus::PartiallyMutable));

    let has_mutable = child_paths
        .iter()
        .any(|p| matches!(p.mutation_status, MutationStatus::Mutable));

    let has_not_mutable = child_paths
        .iter()
        .any(|p| matches!(p.mutation_status, MutationStatus::NotMutable));

    // Determine status
    let status = if has_partially_mutable || (has_mutable && has_not_mutable) {
        MutationStatus::PartiallyMutable
    } else if has_not_mutable {
        MutationStatus::NotMutable
    } else {
        MutationStatus::Mutable
    };

    // Build detailed reason if not fully mutable
    let reason = match status {
        MutationStatus::PartiallyMutable => {
            let summaries: Vec<PathSummary> = child_paths
                .iter()
                .map(MutationPathInternal::to_path_summary)
                .collect();
            Some(NotMutableReason::from_partial_mutability(
                ctx.type_name().clone(),
                summaries,
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
        depth: RecursionDepth,
    ) -> Result<ChildProcessingResult> {
        // Collect children for depth-first traversal
        let child_items = self.inner.collect_children(ctx)?;
        let mut all_paths = vec![];
        let mut paths_to_expose = vec![]; // Paths that should be included in final result
        let mut child_examples = HashMap::<MutationPathDescriptor, Value>::new();

        // Recurse to each child (they handle their own protocol)
        for path_kind in child_items {
            let child_ctx =
                ctx.create_recursion_context(path_kind.clone(), self.inner.child_path_action());

            // Extract descriptor from PathKind for HashMap
            let child_key = path_kind.to_mutation_path_descriptor();

            let (child_paths, child_example) = Self::process_child(&child_key, &child_ctx, depth)?;
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
        depth: RecursionDepth,
    ) -> Result<(Vec<MutationPathInternal>, Value)> {
        tracing::debug!(
            "MutationPathBuilder recursing to child '{}' of type '{}'",
            &**descriptor,
            child_ctx.type_name()
        );

        // Check if child type is in registry first
        // If not, return NotMutable path directly without recursing
        let Ok(child_schema) = child_ctx.require_registry_schema() else {
            // Type not in registry - return NotMutable path directly
            let not_mutable_path = Self::build_not_mutable_path(
                child_ctx,
                NotMutableReason::NotInRegistry(child_ctx.type_name().clone()),
                depth,
            );
            return Ok((vec![not_mutable_path], json!(null)));
        };

        let child_kind = TypeKind::from_schema(child_schema);

        let child_paths = recurse_mutation_paths(child_kind, child_ctx, depth.increment())?;

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
        depth: RecursionDepth,
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
            depth: *depth,
            partial_root_examples,
        }
    }

    /// NEW: Assemble `partial_root_examples` from children using same bottom-up approach
    ///
    /// For each variant chain present in any child:
    /// 1. Collect each child's value for that chain
    /// 2. Assemble them using the builder's assembly logic (struct/tuple/etc)
    /// 3. Store the assembled value for that chain
    fn assemble_partial_roots_new(
        builder: &B,
        ctx: &RecursionContext,
        child_paths: &[&MutationPathInternal],
    ) -> std::result::Result<Option<BTreeMap<Vec<VariantName>, Value>>, BuilderError> {
        use std::collections::BTreeSet;

        // Collect all unique variant chains from all children
        let mut all_chains = BTreeSet::new();
        for child in child_paths {
            if let Some(child_partials) = &child.partial_root_examples {
                tracing::debug!(
                    "[BUILDER] Child {} has partial_roots_new with {} chains",
                    child.full_mutation_path,
                    child_partials.len()
                );
                for chain in child_partials.keys() {
                    all_chains.insert(chain.clone());
                }
            }
        }

        if all_chains.is_empty() {
            tracing::debug!(
                "[BUILDER] No partial roots found in children of {}",
                ctx.type_name()
            );
            return Ok(None);
        }

        tracing::debug!(
            "[BUILDER] Assembling partial_roots_new for {} from {} chains",
            ctx.type_name(),
            all_chains.len()
        );

        let mut assembled_partials = BTreeMap::new();

        // For each variant chain, assemble wrapped example from ALL children
        for chain in all_chains {
            let mut examples_for_chain = HashMap::new();

            // Collect from ALL children, using variant-specific value if available, otherwise
            // regular example
            for child in child_paths {
                let descriptor = child.path_kind.to_mutation_path_descriptor();

                // Try to get variant-specific value first
                if let Some(child_partials) = &child.partial_root_examples
                    && let Some(child_value) = child_partials.get(&chain)
                {
                    examples_for_chain.insert(descriptor, child_value.clone());
                    tracing::debug!(
                        "[BUILDER]   Got VARIANT-SPECIFIC value for chain {:?} from child {}",
                        chain
                            .iter()
                            .map(super::types::VariantName::as_str)
                            .collect::<Vec<_>>(),
                        child.full_mutation_path
                    );
                    continue;
                }

                // No variant-specific value, use regular example
                examples_for_chain.insert(descriptor, child.example.clone());
                tracing::debug!(
                    "[BUILDER]   Got REGULAR value for chain {:?} from child {}",
                    chain
                        .iter()
                        .map(super::types::VariantName::as_str)
                        .collect::<Vec<_>>(),
                    child.full_mutation_path
                );
            }

            // Assemble from all children
            let assembled = builder.assemble_from_children(ctx, examples_for_chain)?;
            tracing::debug!(
                "[BUILDER]   Assembled for chain {:?} -> {}",
                chain
                    .iter()
                    .map(super::types::VariantName::as_str)
                    .collect::<Vec<_>>(),
                serde_json::to_string(&assembled).unwrap_or_else(|_| "???".to_string())
            );
            assembled_partials.insert(chain, assembled);
        }

        Ok(Some(assembled_partials))
    }

    /// Build final result based on `PathAction`
    fn build_final_result(
        ctx: &RecursionContext,
        mut paths_to_expose: Vec<MutationPathInternal>,
        example_to_use: Value,
        parent_status: MutationStatus,
        mutation_status_reason: Option<Value>,
        partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
        depth: RecursionDepth,
    ) -> Vec<MutationPathInternal> {
        if let Some(ref partials) = partial_root_examples {
            tracing::debug!(
                "[BUILDER] Storing {} partial_roots_new chains in path {}",
                partials.len(),
                ctx.full_mutation_path
            );

            // Propagate assembled partial_root_examples to all children
            for child in &mut paths_to_expose {
                child.partial_root_examples = Some(partials.clone());
                tracing::debug!(
                    "[BUILDER] Propagated assembled partial_roots_new to child {}",
                    child.full_mutation_path
                );
            }
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
                        depth,
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
                    depth,
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
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        Self::build_mutation_path_internal(
            ctx,
            json!(null), // No example for NotMutable paths
            MutationStatus::NotMutable,
            Option::<Value>::from(&reason),
            None, // No partial roots for NotMutable paths
            depth,
        )
    }

    /// Check depth limit and return `NotMutable` path if exceeded
    fn check_depth_limit(
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Option<std::result::Result<Vec<MutationPathInternal>, BuilderError>> {
        if depth.exceeds_limit() {
            Some(Err(BuilderError::NotMutable(
                NotMutableReason::RecursionLimitExceeded(ctx.type_name().clone()),
            )))
        } else {
            None
        }
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
        depth: RecursionDepth,
    ) -> (
        Option<std::result::Result<Vec<MutationPathInternal>, BuilderError>>,
        Option<Value>,
    ) {
        // Use unified knowledge lookup that handles all cases
        if let Some(knowledge) = ctx.find_knowledge() {
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
                        depth,
                    )])),
                    None,
                );
            }

            return (None, Some(example));
        }

        // Continue with normal processing, no hard coded mutation knowledge found
        (None, None)
    }
}
