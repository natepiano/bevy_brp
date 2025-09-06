//! Builder for Map types (`HashMap`, `BTreeMap`, etc.)
//!
//! Like Sets, Maps can only be mutated at the top level (replacing the entire map).
//! Maps don't support individual key mutations through BRP's reflection path system.
//!
//! The BRP reflection parser expects integer indices in brackets (e.g., `[0]`) for arrays,
//! not string keys (e.g., `["key"]`) for maps. Because of this limitation, we generate
//! a single terminal mutation path for the entire map field.

use serde_json::json;

use super::super::MutationPathBuilder;
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::{PathLocation, RecursionContext};
use super::super::types::{MutationPathInternal, MutationStatus};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::error::Result;

pub struct MapMutationBuilder;

impl MutationPathBuilder for MapMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        if ctx.require_schema().is_none() {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        }

        // Maps can only be mutated at the top level - no individual key access
        Ok(vec![Self::build_map_mutation_path(ctx, depth)])
    }
}

impl MapMutationBuilder {
    /// Build a mutation path for the entire Map field
    fn build_map_mutation_path(
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        use crate::brp_tools::brp_type_schema::type_info::TypeInfo;

        // Generate example value for the Map type
        let example = TypeInfo::build_type_example(ctx.type_name(), &ctx.registry, depth);

        match &ctx.location {
            PathLocation::Root { type_name } => MutationPathInternal {
                path: String::new(),
                example,
                enum_variants: None,
                type_name: type_name.clone(),
                path_kind: PathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            },
            PathLocation::Element {
                field_name,
                element_type: field_type,
                parent_type,
            } => MutationPathInternal {
                path: format!(".{field_name}"),
                example,
                enum_variants: None,
                type_name: field_type.clone(),
                path_kind: PathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            },
        }
    }

    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            PathLocation::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This map type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       PathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
            PathLocation::Element {
                field_name,
                element_type: field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This map field cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       PathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(&support),
            },
        }
    }
}
