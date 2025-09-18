use std::collections::HashMap;
use std::ops::Deref;

use serde_json::{Value, json};

use super::builders::{
    ArrayMutationBuilder, EnumMutationBuilder, ListMutationBuilder, MapMutationBuilder,
    SetMutationBuilder, StructMutationBuilder, TupleMutationBuilder, ValueMutationBuilder,
};
use super::mutation_knowledge::MutationKnowledge;
use super::type_kind::TypeKind;
use super::types::PathSummary;
use super::{
    MaybeVariants, MutationPathBuilder, MutationPathDescriptor, MutationPathInternal,
    MutationStatus, NotMutableReason, PathAction, RecursionContext,
};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

impl<B: MutationPathBuilder> MutationPathBuilder for ProtocolEnforcer<B> {
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
    ) -> Result<Vec<MutationPathInternal>> {
        tracing::debug!("ProtocolEnforcer processing type: {}", ctx.type_name());

        // Check depth limit for THIS level
        if let Some(result) = Self::check_depth_limit(ctx, depth) {
            return result;
        }

        // Check if type is in registry
        if let Some(result) = Self::check_registry(ctx) {
            return result;
        }

        // Check knowledge for THIS level
        let (early_return, knowledge_example) = Self::check_knowledge(ctx);
        if let Some(result) = early_return {
            return result; // TreatAsValue case - stop recursion
        }

        // Process all children and collect paths
        let ChildProcessingResult {
            all_paths,
            mut paths_to_expose,
            child_examples,
        } = self.process_all_children(ctx, depth)?;

        // Assemble THIS level from children (post-order)
        let assembled_example = match self.inner.assemble_from_children(ctx, child_examples) {
            Ok(example) => example,
            Err(e) => {
                // Use helper method to handle NotMutatable errors cleanly
                return Self::handle_assemble_error(ctx, e);
            }
        };

        // Process the assembled example based on EnumContext
        // Extract enum_root_examples if this is an enum root
        let (parent_example, enum_root_examples) = match &ctx.enum_context {
            Some(super::recursion_context::EnumContext::Root) => {
                // Check if the assembled_example contains enum_root_data marker
                if let Some(enum_data) = assembled_example.get("enum_root_data") {
                    let default_example = enum_data.get("default").cloned().unwrap_or(json!(null));
                    let examples_json = enum_data.get("examples").cloned().unwrap_or(json!([]));
                    // Deserialize the examples array into Vec<ExampleGroup>
                    let examples: Vec<super::types::ExampleGroup> =
                        serde_json::from_value(examples_json.clone()).unwrap_or_default();

                    tracing::debug!(
                        "EnumRoot extraction for {}: found {} examples from JSON: {}",
                        ctx.type_name(),
                        examples.len(),
                        examples_json
                    );

                    (default_example, Some(examples))
                } else {
                    // Fallback if structure is unexpected
                    tracing::debug!(
                        "EnumRoot for {} has no enum_root_data in assembled_example: {}",
                        ctx.type_name(),
                        assembled_example
                    );
                    (assembled_example, None)
                }
            }
            Some(super::recursion_context::EnumContext::Child { .. }) => {
                // Trust the enum builder's result - it already computed applicable_variants
                // EnumChild returns: {"value": example, "applicable_variants": [...]}
                (assembled_example, None)
            }
            None => {
                // Regular non-enum types pass through unchanged
                (assembled_example, None)
            }
        };

        // Use knowledge example if available (for Teach types), otherwise use processed example
        let final_example = if let Some(knowledge_example) = knowledge_example {
            tracing::debug!(
                "Using knowledge example for {} instead of assembled value",
                ctx.type_name()
            );
            knowledge_example
        } else {
            parent_example
        };

        // Compute parent's mutation status from children's statuses
        let (parent_status, reason_enum) = Self::determine_parent_mutation_status(ctx, &all_paths);

        // Convert NotMutableReason to Value if present
        let mutation_status_reason = reason_enum.as_ref().and_then(|r| Option::<Value>::from(r));

        // Decide what to return based on PathAction
        match ctx.path_action {
            PathAction::Create => {
                // Normal mode: Add root path and return only paths marked for exposure
                paths_to_expose.insert(
                    0,
                    Self::build_mutation_path_internal_with_enum_examples(
                        ctx,
                        final_example,
                        enum_root_examples,
                        parent_status,
                        mutation_status_reason,
                    ),
                );
                Ok(paths_to_expose)
            }
            PathAction::Skip => {
                // Skip mode: Return ONLY a root path with the example
                // This ensures the example is available for parent assembly
                // but child paths aren't exposed in the final result
                Ok(vec![Self::build_mutation_path_internal_with_enum_examples(
                    ctx,
                    final_example,
                    enum_root_examples,
                    parent_status,
                    mutation_status_reason,
                )])
            }
        }
    }
}

pub struct ProtocolEnforcer<B: MutationPathBuilder> {
    inner: B,
}

/// Result of processing all children during mutation path building
struct ChildProcessingResult {
    /// All child paths (used for mutation status determination)
    all_paths: Vec<MutationPathInternal>,
    /// Only paths that should be exposed (filtered by PathAction)
    paths_to_expose: Vec<MutationPathInternal>,
    /// Examples for each child path
    child_examples: HashMap<MutationPathDescriptor, Value>,
}

/// Single dispatch point for creating builders - used for both entry and recursion
/// This is the ONLY place where we match on TypeKind to create builders
pub fn recurse_mutation_paths(
    type_kind: TypeKind,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<Vec<MutationPathInternal>> {
    use super::MutationPathBuilder;

    tracing::debug!(
        "recurse_mutation_paths: Dispatching {} as TypeKind::{:?}",
        ctx.type_name(),
        type_kind
    );

    match type_kind {
        TypeKind::Struct => {
            tracing::debug!("Using StructMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(StructMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Tuple | TypeKind::TupleStruct => {
            tracing::debug!("Using TupleMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(TupleMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Array => {
            tracing::debug!("Using ArrayMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(ArrayMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::List => {
            tracing::debug!("Using ListMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(ListMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Map => {
            tracing::debug!("Using MapMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(MapMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Set => {
            tracing::debug!("Using SetMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(SetMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Enum => {
            tracing::debug!("Using NewEnumMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(EnumMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Value => {
            tracing::debug!("Using ValueMutationBuilder for {}", ctx.type_name());
            ProtocolEnforcer::new(ValueMutationBuilder).build_paths(ctx, depth)
        }
    }
}

impl<B: MutationPathBuilder> ProtocolEnforcer<B> {
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
        for item in child_items {
            // Check if we have variant information (from enum builder)
            let variant_info = item.applicable_variants().map(|v| v.to_vec());

            // Always try to extract the PathKind first (may be None for unit variants)
            if let Some(path_kind) = item.into_path_kind() {
                let mut child_ctx =
                    ctx.create_recursion_context(path_kind.clone(), self.inner.child_path_action());

                // Check if we need special variant handling
                if let Some(variants) = variant_info {
                    // Special handling for enum items: Set up variant chain
                    let variant_chain = match &ctx.enum_context {
                        Some(super::recursion_context::EnumContext::Child {
                            variant_chain: parent_chain,
                        }) => {
                            // We're already in a variant - extend the chain
                            let mut extended = parent_chain.clone();
                            extended.push((ctx.type_name().clone(), variants.to_vec()));
                            extended
                        }
                        _ => {
                            // Start a new chain
                            vec![(ctx.type_name().clone(), variants.to_vec())]
                        }
                    };

                    child_ctx.enum_context =
                        Some(super::recursion_context::EnumContext::Child { variant_chain });
                } else {
                    // Check if this child is an enum and set EnumContext appropriately
                    if let Some(child_schema) = child_ctx.get_registry_schema(child_ctx.type_name())
                    {
                        let child_type_kind =
                            TypeKind::from_schema(child_schema, child_ctx.type_name());
                        if matches!(child_type_kind, TypeKind::Enum) {
                            // This child is an enum
                            match &ctx.enum_context {
                                Some(super::recursion_context::EnumContext::Child { .. }) => {
                                    // We're in a variant and found a nested enum
                                    // The nested enum gets Root context (to generate examples)
                                    // The chain will be extended when this enum's variants are
                                    // expanded
                                    child_ctx.enum_context =
                                        Some(super::recursion_context::EnumContext::Root);
                                }
                                _ => {
                                    // Check if parent has enum context
                                    match &ctx.enum_context {
                                        Some(_) => {
                                            // We're inside another enum - don't set enum context
                                            // for simple example
                                        }
                                        None => {
                                            // Not inside an enum - this enum gets Root treatment
                                            child_ctx.enum_context =
                                                Some(super::recursion_context::EnumContext::Root);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Extract descriptor from PathKind for HashMap
                let child_key = path_kind.to_mutation_path_descriptor();

                let (child_paths, child_example) =
                    Self::process_child(&child_key, &child_ctx, depth)?;
                child_examples.insert(child_key, child_example);

                // Always collect all paths for analysis
                all_paths.extend(child_paths.clone());

                // Only add to paths_to_expose if this child should be created
                if matches!(child_ctx.path_action, PathAction::Create) {
                    paths_to_expose.extend(child_paths);
                }
            }
            // If into_path_kind() returns None, skip this item (e.g., for filtering)
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
            "ProtocolEnforcer recursing to child '{}' of type '{}'",
            descriptor.deref(),
            child_ctx.type_name()
        );

        // Get child's schema and create its builder
        let child_schema = child_ctx
            .require_registry_schema()
            .unwrap_or_else(|_| &json!(null));
        tracing::debug!(
            "Child '{}' schema found: {}",
            descriptor.deref(),
            child_schema != &json!(null)
        );

        let child_type = child_ctx.type_name();
        let child_kind = TypeKind::from_schema(child_schema, child_type);
        tracing::debug!("Child '{}' TypeKind: {:?}", descriptor.deref(), child_kind);

        // Use the single dispatch point for recursion
        // THIS is the recursion point - after this everything pops back up to build examples
        tracing::debug!(
            "ProtocolEnforcer: Calling build_paths on child '{}' of type '{}'",
            descriptor.deref(),
            child_ctx.type_name()
        );

        let child_paths = recurse_mutation_paths(child_kind, child_ctx, depth.increment())?;
        tracing::debug!(
            "ProtocolEnforcer: Child '{}' returned {} paths",
            descriptor.deref(),
            child_paths.len()
        );

        // Extract child's example from its root path
        let child_example = child_paths
            .first()
            .map(|p| p.example.clone())
            .unwrap_or(json!(null));

        Ok((child_paths, child_example))
    }

    pub fn new(inner: B) -> Self {
        Self { inner }
    }

    /// Build a `MutationPathInternal` with the provided status and example
    fn build_mutation_path_internal(
        ctx: &RecursionContext,
        example: Value,
        status: MutationStatus,
        mutation_status_reason: Option<Value>,
    ) -> MutationPathInternal {
        Self::build_mutation_path_internal_with_enum_examples(
            ctx,
            example,
            None,
            status,
            mutation_status_reason,
        )
    }

    /// Build a `MutationPathInternal` with enum examples support
    fn build_mutation_path_internal_with_enum_examples(
        ctx: &RecursionContext,
        example: Value,
        enum_root_examples: Option<Vec<super::types::ExampleGroup>>,
        status: MutationStatus,
        mutation_status_reason: Option<Value>,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            enum_root_examples,
            type_name: ctx.type_name().display_name(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: status,
            mutation_status_reason,
        }
    }

    /// Build a `NotMutatable` path with consistent formatting (private to `ProtocolEnforcer`)
    ///
    /// This centralizes `NotMutable` path creation, ensuring only `ProtocolEnforcer`
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
        )
    }

    /// Check depth limit and return `NotMutable` path if exceeded
    fn check_depth_limit(
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Option<Result<Vec<MutationPathInternal>>> {
        if depth.exceeds_limit() {
            Some(Ok(vec![Self::build_not_mutable_path(
                ctx,
                NotMutableReason::RecursionLimitExceeded(ctx.type_name().clone()),
            )]))
        } else {
            None
        }
    }

    /// Check if type is in registry and return `NotMutable` path if not found
    fn check_registry(ctx: &RecursionContext) -> Option<Result<Vec<MutationPathInternal>>> {
        if ctx.require_registry_schema().is_err() {
            Some(Ok(vec![Self::build_not_mutable_path(
                ctx,
                NotMutableReason::NotInRegistry(ctx.type_name().clone()),
            )]))
        } else {
            None
        }
    }

    /// Check knowledge base and handle based on guidance type
    /// Returns (should_stop_recursion, Option<knowledge_example>)
    fn check_knowledge(
        ctx: &RecursionContext,
    ) -> (Option<Result<Vec<MutationPathInternal>>>, Option<Value>) {
        tracing::debug!(
            "ProtocolEnforcer checking knowledge for type: {}",
            ctx.type_name()
        );

        // Use unified knowledge lookup that handles all cases
        if let Some(knowledge) = ctx.find_knowledge() {
            let example = knowledge.example().clone();
            tracing::debug!(
                "ProtocolEnforcer found knowledge for {}: {:?} with knowledge {:?}",
                ctx.type_name(),
                example,
                knowledge
            );

            // Only return early for TreatAsValue types - they should not recurse
            if matches!(knowledge, MutationKnowledge::TreatAsRootValue { .. }) {
                tracing::debug!(
                    "ProtocolEnforcer stopping recursion for TreatAsValue type: {}",
                    ctx.type_name()
                );
                return (
                    Some(Ok(vec![Self::build_mutation_path_internal(
                        ctx,
                        example,
                        MutationStatus::Mutable,
                        None,
                    )])),
                    None,
                );
            }

            // For Teach guidance, we continue with normal recursion but save the knowledge example
            tracing::debug!(
                "ProtocolEnforcer continuing recursion for Teach type: {}, will use knowledge example",
                ctx.type_name()
            );
            return (None, Some(example));
        }
        tracing::debug!(
            "ProtocolEnforcer NO knowledge found for: {}",
            ctx.type_name()
        );

        (None, None) // Continue with normal processing, no knowledge
    }

    /// Handle errors from `assemble_from_children`, creating `NotMutatable` paths when appropriate
    fn handle_assemble_error(
        ctx: &RecursionContext,
        error: error_stack::Report<crate::error::Error>,
    ) -> Result<Vec<MutationPathInternal>> {
        // Check if it's a NotMutatable condition
        // Need to check the root cause of the error stack
        if let Some(reason) = error.current_context().as_not_mutable() {
            // Return a single NotMutatable path for this type
            return Ok(vec![Self::build_not_mutable_path(ctx, reason.clone())]);
        }
        // Real error - propagate it
        Err(error)
    }

    /// Determine parent's mutation status based on children's statuses and return detailed reasons
    fn determine_parent_mutation_status(
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
                let summaries: Vec<PathSummary> =
                    child_paths.iter().map(|p| p.to_path_summary()).collect();
                Some(NotMutableReason::from_partial_mutability(
                    ctx.type_name().clone(),
                    summaries,
                ))
            }
            MutationStatus::NotMutable => Some(NotMutableReason::NoMutableChildren {
                parent_type: ctx.type_name().clone(),
            }),
            MutationStatus::Mutable => None,
        };

        (status, reason)
    }
}
