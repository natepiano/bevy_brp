//! Builder for List types (Vec, etc.)
//!
//! Similar to `ArrayMutationBuilder` but for dynamic containers like Vec<T>.
//! Uses single-pass recursion to extract element type and recurse deeper.
//! use `std::collections::HashMap`;

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::example_builder::ExampleBuilder;
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, SchemaField};
use crate::brp_tools::brp_type_schema::type_info::TypeInfo;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

pub struct ListMutationBuilder;

impl MutationPathBuilder for ListMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_type) = RecursionContext::extract_list_element_type(schema) else {
            // If we have a schema but can't extract element type, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();

        // First, add the top-level list mutation path (for replacing entire list)
        if ctx.value_type_has_serialization(ctx.type_name()) {
            paths.push(Self::build_list_mutation_path(ctx, depth));
        }

        // RECURSE DEEPER - add element-level paths
        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(element_type),
            )]);
        };
        let element_kind = TypeKind::from_schema(element_schema, &element_type);

        // Create a child context for the element type using PathKind
        // Lists/Vecs use array notation [0], not tuple notation .0
        let element_path_kind =
            PathKind::new_array_element(0, element_type.clone(), ctx.type_name().clone());
        let element_ctx = ctx.create_field_context(element_path_kind);

        // Continue recursion to actual mutation endpoints
        let element_paths = element_kind.build_paths(&element_ctx, depth)?; // depth already incremented by TypeKind
        paths.extend(element_paths);

        Ok(paths)
    }
}

impl ListMutationBuilder {
    /// Build list example using extracted logic from `TypeInfo::build_type_example`
    /// This is the static method version that calls `TypeInfo` for element types
    pub fn build_list_example_static(
        schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        // Extract element type using the same logic as TypeInfo
        let item_type = schema
            .get_field(SchemaField::Items)
            .and_then(|items| items.get_field(SchemaField::Type))
            .and_then(TypeInfo::extract_type_ref_with_schema_field);

        item_type.map_or(json!(null), |item_type_name| {
            // Generate example value for the item type
            let item_example =
                ExampleBuilder::build_example(&item_type_name, registry, depth.increment());

            // Create array with 2 example elements
            // For Lists, these are ordered elements
            let array = vec![item_example; 2];
            json!(array)
        })
    }

    /// Build a top-level list mutation path (for replacing entire list)
    fn build_list_mutation_path(
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        // Build path using the context's prefix
        let path = if ctx.mutation_path.is_empty() {
            String::new() // Root level path is empty
        } else {
            ctx.mutation_path.clone() // Field level path uses the prefix
        };

        // Generate example value for the list type
        let example = ExampleBuilder::build_example(ctx.type_name(), &ctx.registry, depth);

        MutationPathInternal {
            path,
            example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        }
    }

    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This list type cannot be mutated - {support}")
            }),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }
}
