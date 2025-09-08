/// Default builder for simple types
///
/// Handles simple types that don't need complex logic - just creates a standard mutation path
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

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        // For default/simple types, delegate to ExampleBuilder
        ExampleBuilder::build_example(ctx.type_name(), &ctx.registry, depth)
    }
}
