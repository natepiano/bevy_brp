//! Builder for Set types (`HashSet`, `BTreeSet`, etc.)
//!
//! Unlike Lists, Sets can only be mutated at the top level (replacing/merging the entire set).
//! Sets don't support indexed access or element-level mutations through BRP.
//!
//! Because of this fundamental limitation, we do not attempt to recurse into the element type.
//! The mutation path generation stops at the Set field itself.

use serde_json::json;

use super::super::MutationPathBuilder;
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::{PathLocation, RecursionContext};
use super::super::types::{MutationPathInternal, MutationStatus};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::error::Result;

pub struct SetMutationBuilder;

impl MutationPathBuilder for SetMutationBuilder {
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

        // Sets can only be mutated at the top level - no element access
        Ok(vec![Self::build_set_mutation_path(ctx, depth)])
    }
}

impl SetMutationBuilder {
    /// Build a mutation path for the entire Set field
    fn build_set_mutation_path(
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        use crate::brp_tools::brp_type_schema::type_info::TypeInfo;

        // Generate example value for the Set type
        let example_value = TypeInfo::build_example_value_for_type_with_depth(
            ctx.type_name(),
            &ctx.registry,
            depth,
        );
        let final_example = RecursionContext::wrap_example(example_value);

        match &ctx.location {
            PathLocation::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         final_example,
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       PathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason:    None,
            },
            PathLocation::Element {
                mutation_path: field_name,
                element_type: field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         final_example,
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       PathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason:    None,
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
                    "agent_directive": format!("This set type cannot be mutated - {support}")
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
                mutation_path: field_name,
                element_type: field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This set field cannot be mutated - {support}")
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
