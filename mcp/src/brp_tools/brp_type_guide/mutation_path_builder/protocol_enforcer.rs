use std::collections::HashMap;

use serde_json::{Value, json};

use super::mutation_knowledge::KnowledgeKey;
use super::type_kind::TypeKind;
use super::{
    MutationPathBuilder, MutationPathInternal, MutationStatus, NotMutatableReason, RecursionContext,
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
    fn build_mutation_path(
        ctx: &RecursionContext,
        example: Value,
        status: MutationStatus,
        error_reason: Option<String>,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: status,
            mutation_status_reason: error_reason,
        }
    }

    /// Build a `NotMutatable` path with consistent formatting (private to `ProtocolEnforcer`)
    ///
    /// This centralizes `NotMutatable` path creation, ensuring only `ProtocolEnforcer`
    /// can create these paths while builders simply return `Error::NotMutatable`.
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        reason: NotMutatableReason,
    ) -> MutationPathInternal {
        Self::build_mutation_path(
            ctx,
            json!(null), // No example for NotMutatable paths
            MutationStatus::NotMutatable,
            Option::<String>::from(&reason),
        )
    }

    /// Check depth limit and return `NotMutatable` path if exceeded
    fn check_depth_limit(
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Option<Result<Vec<MutationPathInternal>>> {
        if depth.exceeds_limit() {
            Some(Ok(vec![Self::build_not_mutatable_path(
                ctx,
                NotMutatableReason::RecursionLimitExceeded(ctx.type_name().clone()),
            )]))
        } else {
            None
        }
    }

    /// Check if type is in registry and return `NotMutatable` path if not found
    fn check_registry(ctx: &RecursionContext) -> Option<Result<Vec<MutationPathInternal>>> {
        if ctx.require_registry_schema().is_none() {
            Some(Ok(vec![Self::build_not_mutatable_path(
                ctx,
                NotMutatableReason::NotInRegistry(ctx.type_name().clone()),
            )]))
        } else {
            None
        }
    }

    /// Check knowledge base and return path with known example if found
    fn check_knowledge(ctx: &RecursionContext) -> Option<Result<Vec<MutationPathInternal>>> {
        KnowledgeKey::find_example_for_type(ctx.type_name()).map(|example| {
            Ok(vec![Self::build_mutation_path(
                ctx,
                example,
                MutationStatus::Mutatable,
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
        if let Some(reason) = error.current_context().as_not_mutatable() {
            // Return a single NotMutatable path for this type
            return Ok(vec![Self::build_not_mutatable_path(ctx, reason.clone())]);
        }
        // Real error - propagate it
        Err(error)
    }

    /// Determine parent's mutation status based on children's statuses
    fn determine_parent_mutation_status(child_paths: &[MutationPathInternal]) -> MutationStatus {
        // Fast path: if ANY child is PartiallyMutatable, parent must be too
        if child_paths
            .iter()
            .any(|p| matches!(p.mutation_status, MutationStatus::PartiallyMutatable))
        {
            return MutationStatus::PartiallyMutatable;
        }

        let has_mutatable = child_paths
            .iter()
            .any(|p| matches!(p.mutation_status, MutationStatus::Mutatable));
        let has_not_mutatable = child_paths
            .iter()
            .any(|p| matches!(p.mutation_status, MutationStatus::NotMutatable));

        match (has_mutatable, has_not_mutatable) {
            (true, true) => MutationStatus::PartiallyMutatable, // Mixed
            (true, false) => MutationStatus::Mutatable,         // All mutatable
            (false, true) => MutationStatus::NotMutatable,      // All not mutatable
            (false, false) => MutationStatus::Mutatable,        // No children (leaf)
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
        let children = self.inner.collect_children(ctx);
        let mut all_paths = vec![];
        let mut child_examples = HashMap::new();

        // Recurse to each child (they handle their own protocol)
        for (name, child_ctx) in children {
            tracing::debug!(
                "ProtocolEnforcer recursing to child '{}' of type '{}'",
                name,
                child_ctx.type_name()
            );

            // Get child's schema and create its builder
            let child_schema = child_ctx.require_registry_schema().unwrap_or(&json!(null));
            tracing::debug!(
                "Child '{}' schema found: {}",
                name,
                child_schema != &json!(null)
            );

            let child_type = child_ctx.type_name();
            let child_kind = TypeKind::from_schema(child_schema, child_type);
            tracing::debug!("Child '{}' TypeKind: {:?}", name, child_kind);

            let child_builder = child_kind.builder();

            // Child handles its OWN depth increment and protocol
            // If child is migrated -> wrapped with ProtocolEnforcer
            // If not migrated -> uses old implementation
            let child_paths = child_builder.build_paths(&child_ctx, depth.increment())?;
            tracing::debug!("Child '{}' returned {} paths", name, child_paths.len());

            // Extract child's example from its root path
            let child_example = child_paths
                .first()
                .map(|p| p.example.clone())
                .unwrap_or(json!(null));

            tracing::debug!("Child '{}' example: {}", name, child_example);

            child_examples.insert(name, child_example);

            // Only include child paths if the builder wants them
            // Container types (like Maps) don't want child paths exposed
            if self.inner.include_child_paths() {
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

        // Set appropriate error reason based on computed status
        let error_reason = match parent_status {
            MutationStatus::NotMutatable => Some("all_children_not_mutatable".to_string()),
            MutationStatus::PartiallyMutatable => Some("mixed_mutability_children".to_string()),
            MutationStatus::Mutatable => None,
        };

        // Add THIS level's path at the beginning with computed status
        all_paths.insert(
            0,
            Self::build_mutation_path(ctx, parent_example, parent_status, error_reason),
        );

        Ok(all_paths)
    }

    // Delegate all other methods to inner builder
    fn is_migrated(&self) -> bool {
        self.inner.is_migrated()
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        self.inner.collect_children(ctx)
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<String, Value>,
    ) -> Result<Value> {
        self.inner.assemble_from_children(ctx, children)
    }

    // fn build_example_with_knowledge(&self, ctx: &RecursionContext, depth: RecursionDepth) ->
    // Value {     self.inner.build_example_with_knowledge(ctx, depth)
    // }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        self.inner.build_schema_example(ctx, depth)
    }
}
