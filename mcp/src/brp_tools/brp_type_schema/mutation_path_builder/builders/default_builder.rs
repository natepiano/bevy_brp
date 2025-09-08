//! Default builder for simple types
//!
//! Handles simple types that don't need complex logic - just creates a standard mutation path
//!
//! **Recursion**: NO - Default builder handles Value types (primitives like i32, f32, String)
//! which are leaf nodes in the type tree. These cannot be decomposed further and are
//! mutated as atomic values.
use serde_json::Value;

use super::super::MutationPathBuilder;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::example_builder::ExampleBuilder;
use crate::error::Result;

pub struct DefaultMutationBuilder;

impl MutationPathBuilder for DefaultMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Generate a proper example value for this type instead of null
        let example = ExampleBuilder::build_example(ctx.type_name(), &ctx.registry, depth);

        Ok(vec![MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        }])
    }

    fn build_schema_example(&self, ctx: &RecursionContext, _depth: RecursionDepth) -> Value {
        // For default/simple Value types, return a simple example without recursion
        // Check for hardcoded knowledge first
        use serde_json::json;

        use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};

        BRP_MUTATION_KNOWLEDGE
            .get(&KnowledgeKey::exact(ctx.type_name()))
            .map(|k| k.example().clone())
            .unwrap_or_else(|| json!(null))
    }
}
