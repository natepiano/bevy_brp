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
use std::collections::HashMap;

use error_stack::Report;
use serde_json::Value;
use serde_json::json;

use super::super::type_kind::TypeKind;
use super::super::type_knowledge::KnowledgeAction;
use super::BuilderError;
use super::enum_builder;
use super::mutation_path_internal::MutationPathInternal;
use super::mutation_path_internal::MutationPathSliceExt;
use super::new_types::VariantName;
use super::not_mutable_reason::NotMutableReason;
use super::path_example::PathExample;
use super::path_kind::MutationPathDescriptor;
use super::path_kind::PathKind;
use super::recursion_context::RecursionContext;
use super::support;
use super::type_kind_builders::ArrayMutationBuilder;
use super::type_kind_builders::ListMutationBuilder;
use super::type_kind_builders::MapMutationBuilder;
use super::type_kind_builders::SetMutationBuilder;
use super::type_kind_builders::StructMutationBuilder;
use super::type_kind_builders::TupleMutationBuilder;
use super::type_kind_builders::TypeKindBuilder;
use super::type_kind_builders::ValueMutationBuilder;
use super::types::EnumPathInfo;
use super::types::Example;
use super::types::Mutability;
use super::types::MutabilityIssue;
use super::types::PathAction;
use super::types::RootExample;
use crate::error::Error;
use crate::error::Result;

/// Result of processing all children during mutation path building
struct ChildProcessingResult {
    /// All child paths (used for mutation status determination)
    all_paths:       Vec<MutationPathInternal>,
    /// Only paths that should be exposed (filtered by `PathAction`)
    paths_to_expose: Vec<MutationPathInternal>,
    /// Examples for each child path
    child_examples:  HashMap<MutationPathDescriptor, Example>,
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
        let knowledge_example = match ctx.check_knowledge()? {
            KnowledgeAction::CompleteWithExample(example) => {
                // Build single root path and return immediately
                // Note: build_mutation_path_internal() returns MutationPathInternal,
                // so we wrap in vec![] to match build_paths() return type
                return Ok(vec![Self::build_mutation_path_internal(
                    ctx,
                    PathExample::Simple(Example::Json(example)),
                    Mutability::Mutable,
                    None,
                    None,
                )]);
            },
            KnowledgeAction::UseExampleAndRecurse(example) => Some(Example::Json(example)),
            KnowledgeAction::NoHardcodedKnowledge => None,
        };

        // Process all children and collect paths
        let ChildProcessingResult {
            all_paths,
            paths_to_expose,
            child_examples,
        } = self.process_all_children(ctx)?;

        // Assemble THIS level from children (post-order)
        // Clone child_examples since we need it later for filtering
        let assembled_value = self
            .inner
            .assemble_from_children(ctx, child_examples.clone())?;

        // Wrap result in Example
        let assembled_example = Example::Json(assembled_value);

        // Assemble partial_root_examples from children (same bottom-up approach)
        // Filter to only direct children by matching against child_examples keys
        let direct_children: Vec<&MutationPathInternal> = all_paths
            .iter()
            .filter(|p| child_examples.contains_key(&p.path_kind.to_mutation_path_descriptor()))
            .collect();
        let partial_root_examples =
            Self::build_partial_root_examples(&self.inner, ctx, direct_children.as_slice())?;

        // Use knowledge example if available (for Teach types), otherwise use assembled example
        let final_example = knowledge_example.unwrap_or(assembled_example);

        // Compute parent's mutation status from children's statuses
        let (parent_status, mutability_reason) = determine_parent_mutability(ctx, &all_paths);

        // Build examples appropriately based on mutation status
        let example_to_use: Example = match parent_status {
            Mutability::NotMutable => Example::NotApplicable,
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
                    .map(|(k, ex)| (k.clone(), ex.clone()))
                    .collect();

                // Assemble from only mutable children
                let assembled = self
                    .inner
                    .assemble_from_children(ctx, mutable_child_examples)
                    .unwrap_or_else(|_| json!(null));

                Example::Json(assembled)
            },
            Mutability::Mutable => final_example,
        };

        // Return error only for NotMutable, success for Mutable and PartiallyMutable
        match parent_status {
            Mutability::NotMutable => {
                let reason = mutability_reason.ok_or_else(|| {
                    BuilderError::SystemError(Report::new(Error::InvalidState(
                        "NotMutable status must have a reason".to_string(),
                    )))
                })?;
                Err(BuilderError::NotMutable(reason))
            },
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

/// Single dispatch point for creating builders - used for both entry and recursion
/// This is the ONLY place where we match on `TypeKind` to create builders
///
/// # Context Handling
///
/// The `RecursionContext` is immutable throughout recursion.
/// Each type handles its own behavior without needing to coordinate context states.
///
/// # Depth Limit Checking
///
/// Depth limit checking is automatic in `RecursionContext::create_recursion_context()`.
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
        },
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
        },
        Err(BuilderError::SystemError(e)) => Err(e),
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
    // Note: Tuples use normal aggregation - PartiallyMutable tuples expose mutable child paths
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

    let status = support::aggregate_mutability(&statuses);

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
        },
        Mutability::NotMutable => Some(ctx.create_no_mutable_children_error()),
        Mutability::Mutable => None,
    };

    (status, reason)
}

impl<B: TypeKindBuilder<Item = PathKind>> MutationPathBuilder<B> {
    pub const fn new(inner: B) -> Self { Self { inner } }

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
        let mut child_examples = HashMap::<MutationPathDescriptor, Example>::new();

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
    ) -> Result<(Vec<MutationPathInternal>, Example)> {
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
            return Ok((vec![not_mutable_path], Example::NotApplicable));
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
            .map_or(Example::NotApplicable, |p| p.example.for_parent().clone());

        Ok((child_paths, child_example))
    }

    /// Build a `MutationPathInternal` with the provided status and example
    ///
    /// Used by `build_not_mutable_path` for `NotMutableReason`s and knowledge handling
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
        mutability_reason: Option<NotMutableReason>,
        partial_root_examples: Option<HashMap<Vec<VariantName>, RootExample>>,
    ) -> MutationPathInternal {
        // Build enum data if variant chain exists
        let enum_path_data = if ctx.variant_chain.is_empty() {
            None
        } else {
            Some(EnumPathInfo {
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
            enum_path_info: enum_path_data,
            depth: *ctx.depth,
            partial_root_examples,
        }
    }

    /// Build `partial_root_examples` from children on ascending from recursion
    ///
    /// This method propagates partial root examples needed for enum variants to
    /// ensure the root example is set so that this particular variant chain can be mutated
    /// `path_builder` propagates these for all the non-enum path builders generically
    /// a similar thing is done within `enum_path_builder` with necessarily more complex logic
    ///
    /// For each variant chain present in any child:
    /// 1. Collect each child's value for that chain
    /// 2. Assemble them using the builder's assembly logic (struct/tuple/etc)
    /// 3. Store the assembled value for that chain
    fn build_partial_root_examples(
        builder: &B,
        ctx: &RecursionContext,
        child_paths: &[&MutationPathInternal],
    ) -> std::result::Result<
        Option<HashMap<Vec<VariantName>, RootExample>>, // NEW
        BuilderError,
    > {
        // Special case: Skip partial root examples for Maps/Sets with NotMutable children
        // These collections require ALL children to be present for assembly, but our
        // filter excludes NotMutable children, causing assembly validation errors
        let schema = ctx.registry.get(ctx.type_name()).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(schema);

        if matches!(type_kind, TypeKind::Map | TypeKind::Set) {
            let has_not_mutable = child_paths
                .iter()
                .any(|p| matches!(p.mutability, Mutability::NotMutable));

            if has_not_mutable {
                // Map/Set with NotMutable children can't have valid partial root examples
                return Ok(None);
            }
        }

        // Collect all unique variant chains from all children
        let all_chains = child_paths.child_variant_chains(*ctx.depth);

        if all_chains.is_empty() {
            return Ok(None);
        }

        let mut partial_root_examples = HashMap::new();

        // For each variant chain, assemble wrapped example from compatible children
        for chain in all_chains {
            // Use shared choke point for filtering and value extraction
            let examples_for_chain =
                support::collect_children_for_chain(child_paths, ctx, Some(&chain));

            // Convert to values for assembly
            // Assemble from filtered children
            let assembled_value = builder.assemble_from_children(ctx, examples_for_chain)?;

            // Use shared helper to wrap with availability status
            let root_example = support::wrap_example_with_availability(
                Example::Json(assembled_value),
                child_paths,
                &chain,
                None,
            );

            partial_root_examples.insert(chain, root_example);
        }

        Ok(Some(partial_root_examples))
    }

    /// Build final result based on `PathAction`
    fn build_final_result(
        ctx: &RecursionContext,
        mut paths_to_expose: Vec<MutationPathInternal>,
        example_to_use: Example,
        parent_status: Mutability,
        mutability_reason: Option<NotMutableReason>,
        partial_root_examples: Option<HashMap<Vec<VariantName>, RootExample>>,
    ) -> Vec<MutationPathInternal> {
        if let Some(ref partial_root_examples) = partial_root_examples {
            // Propagate assembled partial_root_examples to all children
            for child in &mut paths_to_expose {
                child.partial_root_examples = Some(partial_root_examples.clone());
            }

            // Populate root_example from partial_root_examples for children with enum_path_data
            support::populate_root_examples_from_partials(
                &mut paths_to_expose,
                partial_root_examples,
            );
        }

        let mutation_path_internal = Self::build_mutation_path_internal(
            ctx,
            PathExample::Simple(example_to_use),
            parent_status,
            mutability_reason,
            partial_root_examples,
        );

        match ctx.path_action {
            PathAction::Create => {
                // Normal mode: Add root path and return only paths marked for exposure
                paths_to_expose.insert(0, mutation_path_internal);
                paths_to_expose
            },
            PathAction::Skip => {
                // Skip mode: Return ONLY a root path with the example
                // This ensures the example is available for parent assembly
                // but child paths aren't exposed in the final result
                vec![mutation_path_internal]
            },
        }
    }

    /// Build a `NotMutable` path with consistent formatting (private to `MutationPathBuilder`)
    ///
    /// This centralizes `NotMutable` path creation, ensuring only `MutationPathBuilder`
    /// can create these paths while builders simply return `Error::NotMutable`.
    fn build_not_mutable_path(
        ctx: &RecursionContext,
        reason: NotMutableReason,
    ) -> MutationPathInternal {
        Self::build_mutation_path_internal(
            ctx,
            PathExample::Simple(Example::NotApplicable), // Self-documenting!
            Mutability::NotMutable,
            Some(reason),
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
}
