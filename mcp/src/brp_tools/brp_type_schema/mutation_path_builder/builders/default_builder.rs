/// Default builder for simple types
///
/// Handles simple types that don't need complex logic - just creates a standard mutation path
/// use `std::collections::HashMap`;
use serde_json::json;

use super::super::MutationPathBuilder;
use super::super::path_kind::PathKind;
use super::super::recursion_context::{PathLocation, RecursionContext};
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
        let mut paths = Vec::new();

        match &ctx.location {
            PathLocation::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path:            String::new(),
                    example:         json!(null),
                    enum_variants:   None,
                    type_name:       type_name.clone(),
                    path_kind:       PathKind::RootValue {
                        type_name: type_name.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason:    None,
                });
            }
            PathLocation::Element {
                field_name,
                element_type: field_type,
                parent_type,
            } => {
                paths.push(MutationPathInternal {
                    path:            format!(".{field_name}"),
                    example:         json!(null),
                    enum_variants:   None,
                    type_name:       field_type.clone(),
                    path_kind:       PathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason:    None,
                });
            }
        }

        Ok(paths)
    }
}
