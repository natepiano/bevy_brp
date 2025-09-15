use std::collections::HashMap;
use std::ops::Deref;

use serde_json::{Value, json};

use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeGuidance, KnowledgeKey};
use super::type_kind::TypeKind;
use super::types::PathSummary;
use super::{
    MutationPathBuilder, MutationPathDescriptor, MutationPathInternal, MutationStatus,
    NotMutableReason, PathAction, RecursionContext,
};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

impl MutationPathBuilder for ProtocolEnforcer {
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

        // Use knowledge example if available (for Teach types), otherwise use assembled
        let parent_example = if let Some(knowledge_example) = knowledge_example {
            tracing::debug!(
                "Using knowledge example for {} instead of assembled value",
                ctx.type_name()
            );
            knowledge_example
        } else {
            assembled_example
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
                    Self::build_mutation_path_internal(
                        ctx,
                        parent_example,
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
                Ok(vec![Self::build_mutation_path_internal(
                    ctx,
                    parent_example,
                    parent_status,
                    mutation_status_reason,
                )])
            }
        }
    }
}

pub struct ProtocolEnforcer {
    inner: Box<dyn MutationPathBuilder>,
}

/// Result of processing all children during mutation path building
struct ChildProcessingResult {
    /// All child paths (used for mutation status determination)
    all_paths:       Vec<MutationPathInternal>,
    /// Only paths that should be exposed (filtered by PathAction)
    paths_to_expose: Vec<MutationPathInternal>,
    /// Examples for each child path
    child_examples:  HashMap<MutationPathDescriptor, Value>,
}

impl ProtocolEnforcer {
    /// Process all children and collect their paths and examples
    fn process_all_children(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<ChildProcessingResult> {
        // Collect children for depth-first traversal
        let child_path_kinds = self.inner.collect_children(ctx)?;
        let mut all_paths = vec![];
        let mut paths_to_expose = vec![]; // Paths that should be included in final result
        let mut child_examples = HashMap::<MutationPathDescriptor, Value>::new();

        // Recurse to each child (they handle their own protocol)
        for path_kind in child_path_kinds {
            // ProtocolEnforcer creates the context with proper path_action handling
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

        // we need builder() so that unmigrated children return to their normal unmigrated path
        // as builder() in `TypeKind` will check the `is_migrated()` method
        //
        //  1. If child is migrated: child_builder becomes ANOTHER ProtocolEnforcer wrapping the
        //     migrated child
        //  - So it calls _ProtocolEnforcer.build_paths()_ (this again), NOT the migrated builder's
        //    build_paths()
        //    - The migrated builder's build_paths() that returns Error::InvalidState is NEVER
        //      called
        //  2. If child is unmigrated: child_builder is the raw unmigrated builder
        //  - So it calls the unmigrated builder's build_paths() directly
        let child_builder = child_kind.builder();

        // Child handles its OWN depth increment and protocol
        // If child is migrated -> wrapped with ProtocolEnforcer and calls back through
        // If not migrated -> uses old implementation
        // THIS is the recursion point - after this everything pops back up to build examples
        let child_paths = child_builder.build_paths(child_ctx, depth.increment())?;

        // Extract child's example from its root path
        let child_example = child_paths
            .first()
            .map(|p| p.example.clone())
            .unwrap_or(json!(null));

        Ok((child_paths, child_example))
    }

    pub fn new(inner: Box<dyn MutationPathBuilder>) -> Self {
        Self { inner }
    }

    /// Build a `MutationPathInternal` with the provided status and example
    fn build_mutation_path_internal(
        ctx: &RecursionContext,
        example: Value,
        status: MutationStatus,
        mutation_status_reason: Option<Value>,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            type_name: ctx.type_name().clone(),
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

        // Check if we have knowledge for this type
        if let Some(knowledge) =
            BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(ctx.type_name().to_string()))
        {
            let example = knowledge.example().clone();
            let guidance = knowledge.guidance();
            tracing::debug!(
                "ProtocolEnforcer found knowledge for {}: {:?} with guidance {:?}",
                ctx.type_name(),
                example,
                guidance
            );

            // Only return early for TreatAsValue types - they should not recurse
            if matches!(guidance, KnowledgeGuidance::TreatAsValue { .. }) {
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
        } else {
            tracing::debug!(
                "ProtocolEnforcer NO knowledge found for: {}",
                ctx.type_name()
            );
        }

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
