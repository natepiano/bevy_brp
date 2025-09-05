//! Builder for List types (Vec, etc.)
//!
//! Similar to `ArrayMutationBuilder` but for dynamic containers like Vec<T>.
//! Uses single-pass recursion to extract element type and recurse deeper.
//! use `std::collections::HashMap`;

use std::sync::Arc;

use serde_json::json;

use super::super::TypeKind;
use super::super::types::{MutationPathBuilder, MutationPathContext, RootOrField};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use super::super::types::{MutationPathInternal, MutationPathKind, MutationStatus};
use crate::brp_tools::brp_type_schema::type_info::MutationSupport;
use crate::error::Result;

pub struct ListMutationBuilder;

impl MutationPathBuilder for ListMutationBuilder {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_type) = MutationPathContext::extract_list_element_type(schema) else {
            // If we have a schema but can't extract element type, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        // RECURSE DEEPER - don't stop at list level
        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(element_type),
            )]);
        };
        let element_kind = TypeKind::from_schema(element_schema, &element_type);

        // Create a new root context for the element type
        let element_ctx =
            MutationPathContext::new(RootOrField::root(&element_type), Arc::clone(&ctx.registry));

        // Continue recursion to actual mutation endpoints
        element_kind.build_paths(&element_ctx, depth) // depth already incremented by TypeKind
    }
}

impl ListMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &MutationPathContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This list type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This list field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }
}
