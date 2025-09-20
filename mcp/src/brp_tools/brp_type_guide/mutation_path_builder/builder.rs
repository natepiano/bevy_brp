//! This is the main `MutationPathBuilder` implementation which
//! recursively uses the `PathBuilder` trait to build mutation paths for a given type.
use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::mutation_path_builder::EnumContext;
use super::builders::{
    ArrayMutationBuilder, EnumMutationBuilder, ListMutationBuilder, MapMutationBuilder,
    SetMutationBuilder, StructMutationBuilder, TupleMutationBuilder, ValueMutationBuilder,
};
use super::mutation_knowledge::MutationKnowledge;
use super::path_builder::{MaybeVariants, PathBuilder};
use super::type_kind::TypeKind;
use super::types::PathSummary;
use super::{
    MutationPathDescriptor, MutationPathInternal, MutationStatus, NotMutableReason, PathAction,
    PathKind, RecursionContext,
};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

pub struct MutationPathBuilder<B: PathBuilder> {
    inner: B,
}

impl<B: PathBuilder> PathBuilder for MutationPathBuilder<B> {
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
        tracing::debug!("MutationPathBuilder processing type: {}", ctx.type_name());

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
        tracing::debug!(
            "Processing assembled example for {} with path '{}' and enum_context: {:?}",
            ctx.type_name(),
            ctx.mutation_path,
            ctx.enum_context
        );
        let (parent_example, enum_root_examples, enum_root_example_for_parent) = match &ctx
            .enum_context
        {
            Some(EnumContext::Root) => {
                tracing::debug!(
                    "Type {} at path '{}' has EnumContext::Root, checking for enum_root_data",
                    ctx.type_name(),
                    ctx.mutation_path
                );
                // Check if the assembled_example contains enum_root_data marker
                assembled_example
                    .get("enum_root_data")
                    .cloned()
                    .map_or_else(
                        || {
                            // Fallback if structure is unexpected
                            tracing::debug!(
                                "EnumRoot for {} at path '{}' has no enum_root_data in assembled_example: {}",
                                ctx.type_name(),
                                ctx.mutation_path,
                                assembled_example
                            );
                            (assembled_example, None, None)
                        },
                        |enum_data| {
                            let default_example = enum_data
                                .get("enum_root_example_for_parent")
                                .cloned()
                                .unwrap_or(json!(null));
                            let examples_json = enum_data
                                .get("enum_root_examples")
                                .cloned()
                                .unwrap_or(json!([]));
                            // Deserialize the examples array into Vec<ExampleGroup>
                            let examples: Vec<super::types::ExampleGroup> =
                                serde_json::from_value(examples_json.clone()).unwrap_or_default();

                            tracing::debug!(
                                "EnumRoot extraction for {} at path '{}': found {} examples, default_example: {}",
                                ctx.type_name(),
                                ctx.mutation_path,
                                examples.len(),
                                default_example
                            );

                            // For enum root paths: no single example, store default separately for parent
                            (json!(null), Some(examples), Some(default_example))
                        },
                    )
            }
            Some(EnumContext::Child) => {
                // Trust the enum builder's result - it already computed applicable_variants
                // EnumChild returns: {"value": example, "applicable_variants": [...]}
                (assembled_example, None, None)
            }
            None => {
                // Regular non-enum types pass through unchanged
                (assembled_example, None, None)
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
        let mutation_status_reason = reason_enum.as_ref().and_then(Option::<Value>::from);

        // Fix: PartiallyMutable paths should not provide misleading examples
        let example_to_use = match parent_status {
            MutationStatus::PartiallyMutable => json!(null),
            MutationStatus::NotMutable => json!(null),
            MutationStatus::Mutable => final_example,
        };

        // Decide what to return based on PathAction
        match ctx.path_action {
            PathAction::Create => {
                // Normal mode: Add root path and return only paths marked for exposure
                paths_to_expose.insert(
                    0,
                    Self::build_mutation_path_internal_with_enum_examples(
                        ctx,
                        example_to_use,
                        enum_root_examples,
                        enum_root_example_for_parent,
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
                    example_to_use,
                    enum_root_examples,
                    enum_root_example_for_parent,
                    parent_status,
                    mutation_status_reason,
                )])
            }
        }
    }
}

/// Result of processing all children during mutation path building
struct ChildProcessingResult {
    /// All child paths (used for mutation status determination)
    all_paths:       Vec<MutationPathInternal>,
    /// Only paths that should be exposed (filtered by `PathAction`)
    paths_to_expose: Vec<MutationPathInternal>,
    /// Examples for each child path
    child_examples:  HashMap<MutationPathDescriptor, Value>,
}

/// Single dispatch point for creating builders - used for both entry and recursion
/// This is the ONLY place where we match on `TypeKind` to create builders
pub fn recurse_mutation_paths(
    type_kind: TypeKind,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<Vec<MutationPathInternal>> {
    use PathBuilder;

    tracing::debug!(
        "recurse_mutation_paths: Dispatching {} as TypeKind::{:?}",
        ctx.type_name(),
        type_kind
    );

    match type_kind {
        TypeKind::Struct => {
            tracing::debug!("Using StructMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(StructMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Tuple | TypeKind::TupleStruct => {
            tracing::debug!("Using TupleMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(TupleMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Array => {
            tracing::debug!("Using ArrayMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(ArrayMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::List => {
            tracing::debug!("Using ListMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(ListMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Map => {
            tracing::debug!("Using MapMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(MapMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Set => {
            tracing::debug!("Using SetMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(SetMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Enum => {
            tracing::debug!("Using NewEnumMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(EnumMutationBuilder).build_paths(ctx, depth)
        }
        TypeKind::Value => {
            tracing::debug!("Using ValueMutationBuilder for {}", ctx.type_name());
            MutationPathBuilder::new(ValueMutationBuilder).build_paths(ctx, depth)
        }
    }
}

impl<B: PathBuilder> MutationPathBuilder<B> {
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
            let variant_info = item.applicable_variants().map(<[String]>::to_vec);

            // Always try to extract the PathKind first (may be None for unit variants)
            if let Some(path_kind) = item.into_path_kind() {
                let mut child_ctx =
                    ctx.create_recursion_context(path_kind.clone(), self.inner.child_path_action());

                // Check if we need special variant handling
                if let Some(variants) = variant_info {
                    // Special handling for enum items: Set up variant chain
                    if let Some(representative_variant) = variants.first() {
                        // Extend the inherited variant chain with this enum's variant
                        child_ctx
                            .variant_chain
                            .push(super::types::VariantPathEntry {
                                path:    ctx.mutation_path.clone(),
                                variant: representative_variant.clone(),
                            });
                    }

                    child_ctx.enum_context = Some(super::recursion_context::EnumContext::Child);
                } else {
                    // Check if this child is an enum and set EnumContext appropriately
                    if let Some(child_schema) = child_ctx.get_registry_schema(child_ctx.type_name())
                    {
                        let child_type_kind =
                            TypeKind::from_schema(child_schema, child_ctx.type_name());
                        if matches!(child_type_kind, TypeKind::Enum) {
                            // This child is an enum
                            match &ctx.enum_context {
                                Some(super::recursion_context::EnumContext::Child) => {
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
                    Self::process_child(&child_key, &mut child_ctx, depth)?;
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
        child_ctx: &mut RecursionContext,
        depth: RecursionDepth,
    ) -> Result<(Vec<MutationPathInternal>, Value)> {
        tracing::debug!(
            "MutationPathBuilder recursing to child '{}' of type '{}'",
            &**descriptor,
            child_ctx.type_name()
        );

        // Get child's schema and create its builder
        let child_schema = child_ctx
            .require_registry_schema()
            .unwrap_or_else(|err| {
                tracing::warn!(
                    "🔥 CHOKE POINT: Type '{}' not found in registry - swallowing NotInRegistry error! Error: {:?}",
                    child_ctx.type_name(),
                    err
                );
                &json!(null)
            });
        tracing::debug!(
            "Child '{}' schema found: {}",
            &**descriptor,
            child_schema != &json!(null)
        );

        let child_type = child_ctx.type_name().clone();
        let child_kind = TypeKind::from_schema(child_schema, &child_type);
        tracing::debug!("Child '{}' TypeKind: {:?}", &**descriptor, child_kind);

        // If child is an enum and we're building a non-root path for it, set EnumContext::Root
        // This ensures the enum generates proper examples for its mutation path
        // Check if it's either None OR if it's a Child context (which needs to become Root for this
        // enum)
        let should_set_enum_root = matches!(child_kind, TypeKind::Enum)
            && (child_ctx.enum_context.is_none()
                || matches!(
                    &child_ctx.enum_context,
                    Some(super::recursion_context::EnumContext::Child)
                ));

        if should_set_enum_root {
            tracing::debug!(
                "Detected enum field '{}' with type '{}', current context: {:?}, checking if should set EnumContext::Root",
                &**descriptor,
                child_ctx.type_name(),
                child_ctx.enum_context
            );
            match child_ctx.path_kind {
                PathKind::StructField { .. }
                | PathKind::IndexedElement { .. }
                | PathKind::ArrayElement { .. } => {
                    tracing::debug!(
                        "Setting EnumContext::Root for enum field '{}' with PathKind {:?}",
                        &**descriptor,
                        child_ctx.path_kind
                    );
                    child_ctx.enum_context = Some(super::recursion_context::EnumContext::Root);
                }
                PathKind::RootValue { .. } => {
                    // RootValue paths don't need EnumContext::Root
                    tracing::debug!(
                        "Skipping EnumContext::Root for RootValue path '{}'",
                        &**descriptor
                    );
                }
            }
        }

        // Use the single dispatch point for recursion
        // THIS is the recursion point - after this everything pops back up to build examples
        tracing::debug!(
            "MutationPathBuilder: Calling build_paths on child '{}' of type '{}'",
            &**descriptor,
            child_ctx.type_name()
        );

        let child_paths = recurse_mutation_paths(child_kind, &child_ctx, depth.increment())?;
        tracing::debug!(
            "MutationPathBuilder: Child '{}' returned {} paths",
            &**descriptor,
            child_paths.len()
        );

        // Extract child's example - handle both simple and enum root cases
        let child_example = child_paths.first().map_or(json!(null), |p| {
            if let Some(parent_example) = &p.enum_root_example_for_parent {
                // This child is an enum root - use the parent example
                parent_example.clone()
            } else {
                // Simple case - just use the example
                p.example.clone()
            }
        });

        Ok((child_paths, child_example))
    }

    pub const fn new(inner: B) -> Self {
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
        enum_root_example_for_parent: Option<Value>,
        status: MutationStatus,
        mutation_status_reason: Option<Value>,
    ) -> MutationPathInternal {
        // Build complete path_requirement if variant chain exists
        let path_requirement = if !ctx.variant_chain.is_empty() {
            Some(super::types::PathRequirement {
                description:  Self::generate_variant_description(&ctx.variant_chain),
                example:      example.clone(), // Use the example we already built!
                variant_path: ctx.variant_chain.clone(), /* Already Vec<VariantPathEntry> from
                                                * Step 2 */
            })
        } else {
            None
        };

        let result = MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example: example.clone(),
            enum_root_examples: enum_root_examples.clone(),
            enum_root_example_for_parent: enum_root_example_for_parent.clone(),
            type_name: ctx.type_name().display_name(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: status,
            mutation_status_reason,
            path_requirement, // Now populated based on enum context
        };

        tracing::debug!(
            "Created MutationPathInternal for {} at path '{}': example={}, enum_root_examples={}, enum_root_example_for_parent={}",
            ctx.type_name(),
            ctx.mutation_path,
            example,
            enum_root_examples.is_some(),
            enum_root_example_for_parent.is_some()
        );

        result
    }

    /// Generate a human-readable description of variant requirements
    fn generate_variant_description(variant_chain: &[super::types::VariantPathEntry]) -> String {
        if variant_chain.len() == 1 {
            let entry = &variant_chain[0];
            if entry.path.is_empty() {
                format!(
                    "To use this mutation path, the root must be set to {}",
                    entry.variant
                )
            } else {
                format!(
                    "To use this mutation path, {} must be set to {}",
                    entry.path, entry.variant
                )
            }
        } else {
            let requirements: Vec<String> = variant_chain
                .iter()
                .map(|entry| {
                    if entry.path.is_empty() {
                        format!("root must be set to {}", entry.variant)
                    } else {
                        format!("{} must be set to {}", entry.path, entry.variant)
                    }
                })
                .collect();
            format!("To use this mutation path, {}", requirements.join(" and "))
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
    /// Returns `(should_stop_recursion, Option<knowledge_example>)`
    fn check_knowledge(
        ctx: &RecursionContext,
    ) -> (Option<Result<Vec<MutationPathInternal>>>, Option<Value>) {
        tracing::debug!(
            "MutationPathBuilder checking knowledge for type: {}",
            ctx.type_name()
        );

        // Use unified knowledge lookup that handles all cases
        if let Some(knowledge) = ctx.find_knowledge() {
            let example = knowledge.example().clone();
            tracing::debug!(
                "MutationPathBuilder found knowledge for {}: {:?} with knowledge {:?}",
                ctx.type_name(),
                example,
                knowledge
            );

            // Only return early for TreatAsValue types - they should not recurse
            if matches!(knowledge, MutationKnowledge::TreatAsRootValue { .. }) {
                tracing::debug!(
                    "MutationPathBuilder stopping recursion for TreatAsValue type: {}",
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
                "MutationPathBuilder continuing recursion for Teach type: {}, will use knowledge example",
                ctx.type_name()
            );
            return (None, Some(example));
        }
        tracing::debug!(
            "MutationPathBuilder NO knowledge found for: {}",
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
                let summaries: Vec<PathSummary> = child_paths
                    .iter()
                    .map(MutationPathInternal::to_path_summary)
                    .collect();
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
