use std::collections::HashMap;

use serde_json::{Value, json};

use super::mutation_knowledge::KnowledgeKey;
use super::type_kind::TypeKind;
use super::{MutationPathBuilder, MutationPathInternal, MutationStatus, RecursionContext};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

pub struct ProtocolEnforcer {
    inner: Box<dyn MutationPathBuilder>,
}

impl ProtocolEnforcer {
    pub fn new(inner: Box<dyn MutationPathBuilder>) -> Self {
        Self { inner }
    }
}

impl MutationPathBuilder for ProtocolEnforcer {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        tracing::debug!("ProtocolEnforcer processing type: {}", ctx.type_name());

        // 1. Check depth limit for THIS level
        if depth.exceeds_limit() {
            return Ok(vec![MutationPathInternal {
                path:            ctx.mutation_path.clone(),
                example:         json!({"error": "recursion limit exceeded"}),
                type_name:       ctx.type_name().clone(),
                path_kind:       ctx.path_kind.clone(),
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Some("Recursion limit exceeded".to_string()),
            }]);
        }

        // 2. Check knowledge for THIS level
        if let Some(example) = KnowledgeKey::find_example_for_type(ctx.type_name()) {
            return Ok(vec![MutationPathInternal {
                path:            ctx.mutation_path.clone(),
                example:         example.clone(),
                type_name:       ctx.type_name().clone(),
                path_kind:       ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                error_reason:    None,
            }]);
        }

        // 3. Collect children for depth-first traversal
        let children = self.inner.collect_children(ctx);
        let mut all_paths = vec![];
        let mut child_examples = HashMap::new();

        // 4. Recurse to each child (they handle their own protocol)
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

        // 5. Assemble THIS level from children (post-order)
        let parent_example = self.inner.assemble_from_children(ctx, child_examples);

        // 6. Add THIS level's path at the beginning
        all_paths.insert(
            0,
            MutationPathInternal {
                path:            ctx.mutation_path.clone(),
                example:         parent_example,
                type_name:       ctx.type_name().clone(),
                path_kind:       ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                error_reason:    None,
            },
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
    ) -> Value {
        self.inner.assemble_from_children(ctx, children)
    }

    // fn build_example_with_knowledge(&self, ctx: &RecursionContext, depth: RecursionDepth) ->
    // Value {     self.inner.build_example_with_knowledge(ctx, depth)
    // }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        self.inner.build_schema_example(ctx, depth)
    }
}
