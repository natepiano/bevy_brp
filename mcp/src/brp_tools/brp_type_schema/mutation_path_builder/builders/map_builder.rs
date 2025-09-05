//! Builder for Map types
//!
//! Handles Map mutation paths with inline value type checking to avoid redundant precheck

use serde_json::json;

use super::super::types::{MutationPathBuilder, MutationPathContext, RootOrField};
use super::default_builder::DefaultMutationBuilder;
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use super::super::types::{MutationPathInternal, MutationPathKind, MutationStatus};
use crate::brp_tools::brp_type_schema::type_info::MutationSupport;
use crate::error::Result;
pub struct MapMutationBuilder;

impl MutationPathBuilder for MapMutationBuilder {
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

        let Some(value_type) = MutationPathContext::extract_map_value_type(schema) else {
            // If we have a schema but can't extract value type, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        // Maps are currently treated as opaque (cannot mutate individual keys)
        // So we just validate value type has serialization and build a single path
        if !ctx.value_type_has_serialization(&value_type) {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::MissingSerializationTraits(value_type),
            )]);
        }

        // Build single opaque mutation path for the entire map
        DefaultMutationBuilder.build_paths(ctx, depth)
    }
}

impl MapMutationBuilder {
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
                    "agent_directive": format!("This map type cannot be mutated - {support}")
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
                    "agent_directive": format!("This map field cannot be mutated - {support}")
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
