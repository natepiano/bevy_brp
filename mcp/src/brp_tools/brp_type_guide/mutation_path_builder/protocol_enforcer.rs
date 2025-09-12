use std::collections::HashMap;
use std::ops::Deref;

use serde_json::{Value, json};

use super::mutation_knowledge::KnowledgeKey;
use super::type_kind::TypeKind;
use super::{
    MutationPathBuilder, MutationPathDescriptor, MutationPathInternal, MutationStatus,
    NotMutableReason, PathAction, RecursionContext,
};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

pub struct ProtocolEnforcer {
    inner: Box<dyn MutationPathBuilder>,
}

impl ProtocolEnforcer {
    pub fn new(inner: Box<dyn MutationPathBuilder>) -> Self {
        Self { inner }
    }

    /// Build a `MutationPathInternal` with the provided status and example
    fn build_mutation_path_internal(
        ctx: &RecursionContext,
        example: Value,
        status: MutationStatus,
        mutation_status_reason: Option<String>,
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
            Option::<String>::from(&reason),
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
        if ctx.require_registry_schema().is_none() {
            Some(Ok(vec![Self::build_not_mutable_path(
                ctx,
                NotMutableReason::NotInRegistry(ctx.type_name().clone()),
            )]))
        } else {
            None
        }
    }

    /// Check knowledge base and return path with known example if found
    fn check_knowledge(ctx: &RecursionContext) -> Option<Result<Vec<MutationPathInternal>>> {
        KnowledgeKey::find_example_for_type(ctx.type_name()).map(|example| {
            Ok(vec![Self::build_mutation_path_internal(
                ctx,
                example,
                MutationStatus::Mutable,
                None,
            )])
        })
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
        let child_schema = child_ctx.require_registry_schema().unwrap_or(&json!(null));
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
        tracing::debug!(
            "Child '{}' returned {} paths",
            descriptor.deref(),
            child_paths.len()
        );

        // Extract child's example from its root path
        let child_example = child_paths
            .first()
            .map(|p| p.example.clone())
            .unwrap_or(json!(null));

        tracing::debug!("Child '{}' example: {}", descriptor.deref(), child_example);

        Ok((child_paths, child_example))
    }

    /// Determine parent's mutation status based on children's statuses
    fn determine_parent_mutation_status(child_paths: &[MutationPathInternal]) -> MutationStatus {
        // Fast path: if ANY child is PartiallyMutable, parent must be too
        if child_paths
            .iter()
            .any(|p| matches!(p.mutation_status, MutationStatus::PartiallyMutable))
        {
            return MutationStatus::PartiallyMutable;
        }

        let has_mutatable = child_paths
            .iter()
            .any(|p| matches!(p.mutation_status, MutationStatus::Mutable));
        let has_not_mutable = child_paths
            .iter()
            .any(|p| matches!(p.mutation_status, MutationStatus::NotMutable));

        match (has_mutatable, has_not_mutable) {
            (true, true) => MutationStatus::PartiallyMutable, // Mixed
            (_, false) => MutationStatus::Mutable,            /* All mutatable or no children */
            // (leaf)
            (false, true) => MutationStatus::NotMutable, // All not mutatable
        }
    }
}

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
        if let Some(result) = Self::check_knowledge(ctx) {
            return result;
        }

        // Collect children for depth-first traversal
        // calls the migrated builders (the inner) collect_children method
        let child_path_kinds = self.inner.collect_children(ctx)?;
        let mut all_paths = vec![];
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

            // Only extend paths when child's path_action is Create
            // WE NEED TO REMOVE THE CONDITIONAL WHEN ALL ARE MIGRATED
            // as the new create_recursion_context will ensure children DON'T build paths
            if matches!(child_ctx.path_action, PathAction::Create) {
                all_paths.extend(child_paths);
            }
        }

        // Assemble THIS level from children (post-order)
        let parent_example = match self.inner.assemble_from_children(ctx, child_examples) {
            Ok(example) => example,
            Err(e) => {
                // Use helper method to handle NotMutatable errors cleanly
                return Self::handle_assemble_error(ctx, e);
            }
        };

        // Compute parent's mutation status from children's statuses
        let parent_status = Self::determine_parent_mutation_status(&all_paths);

        // Set appropriate error reason based on computed status using enum variants
        let mutation_status_reason = match parent_status {
            MutationStatus::NotMutable => Some(NotMutableReason::NoMutableChildren {
                parent_type: ctx.type_name().clone(),
            }),
            MutationStatus::PartiallyMutable => Some(NotMutableReason::PartialChildMutability {
                parent_type: ctx.type_name().clone(),
            }),
            MutationStatus::Mutable => None,
        }
        .and_then(|reason| Option::<String>::from(&reason));

        // Add THIS level's path at the beginning with computed status
        // Only create path if path_action is Create (skipped for Map/Set children)
        if matches!(ctx.path_action, PathAction::Create) {
            all_paths.insert(
                0,
                Self::build_mutation_path_internal(
                    ctx,
                    parent_example,
                    parent_status,
                    mutation_status_reason,
                ),
            );
        }

        Ok(all_paths)
    }
}
