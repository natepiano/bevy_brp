//! Builder for List types (Vec, etc.)
//!
//! Similar to `ArrayMutationBuilder` but for dynamic containers like Vec<T>.
//! Uses single-pass recursion to extract element type and recurse deeper.
//!
//! **Recursion**: YES - Lists recurse into elements to generate mutation paths
//! for nested structures (e.g., `Vec<Transform>` generates `[0].translation`).
//! Elements are addressable by index, though indices may change as list mutates.
//! use `std::collections::HashMap`;

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::example_builder::ExampleBuilder;
use crate::brp_tools::brp_type_schema::response_types::BrpTypeName;
use crate::error::Result;
use crate::json_types::SchemaField;
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
        let element_paths = element_kind.build_paths(&element_ctx, depth)?;

        // Extract element example from child paths
        let element_example = element_paths
            .iter()
            .find(|p| p.path == element_ctx.mutation_path)
            .map(|p| p.example.clone())
            .unwrap_or_else(|| {
                // If no direct path, generate example using trait dispatch
                element_kind
                    .builder()
                    .build_schema_example(&element_ctx, depth.increment())
            });

        // Build the main list path using the element example (like Array builder does)
        let list_example = vec![element_example.clone(); 2];
        paths.push(MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!(list_example),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason:    None,
        });

        // Build the indexed element path (like Array builder does)
        let indexed_path = format!("{}[0]", ctx.mutation_path);
        paths.push(MutationPathInternal {
            path:            indexed_path,
            example:         element_example,
            type_name:       element_type.clone(),
            path_kind:       element_ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason:    None,
        });

        // Add the nested paths
        paths.extend(element_paths);

        Ok(paths)
    }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        let Some(schema) = ctx.require_schema() else {
            return json!(null);
        };

        // Extract element type using the same logic as the static method
        let item_type = schema
            .get_field(SchemaField::Items)
            .and_then(SchemaField::extract_field_type);

        item_type.map_or(json!(null), |item_type_name| {
            // Generate example value for the item type using trait dispatch
            // First check for hardcoded knowledge
            let item_example = BRP_MUTATION_KNOWLEDGE
                .get(&KnowledgeKey::exact(&item_type_name))
                .map_or_else(
                    || {
                        // Get the element type schema and use trait dispatch
                        ctx.get_type_schema(&item_type_name)
                            .map_or(json!(null), |element_schema| {
                                let element_kind =
                                    TypeKind::from_schema(element_schema, &item_type_name);
                                // Create element context for recursive building
                                let element_path_kind = PathKind::new_array_element(
                                    0,
                                    item_type_name.clone(),
                                    ctx.type_name().clone(),
                                );
                                let element_ctx = ctx.create_field_context(element_path_kind);
                                // Use trait dispatch directly
                                element_kind
                                    .builder()
                                    .build_schema_example(&element_ctx, depth.increment())
                            })
                    },
                    |k| k.example().clone(),
                );

            // Create array with 2 example elements
            // For Lists, these are ordered elements
            let array = vec![item_example; 2];
            json!(array)
        })
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
            .and_then(SchemaField::extract_field_type);

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
