/// Default builder for simple types
///
/// Handles simple types that don't need complex logic - just creates a standard mutation path
/// use `std::collections::HashMap`;
use serde_json::json;

use super::super::MutationPathBuilder;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::error::Result;

pub struct DefaultMutationBuilder;

impl MutationPathBuilder for DefaultMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        _depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        Ok(vec![MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!(null),
            enum_variants:   None,
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason:    None,
        }])
    }
}
