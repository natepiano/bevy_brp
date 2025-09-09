//! Builder for Set types (`HashSet`, `BTreeSet`, etc.)
//!
//! Unlike Lists, Sets can only be mutated at the top level (replacing/merging the entire set).
//! Sets don't support indexed access or element-level mutations through BRP.
//!
//! **Recursion**: NO - Sets are terminal mutation points. Elements have no stable
//! addresses (no indices or keys) and cannot be individually mutated. Only the entire
//! set can be replaced. Mutating an element could change its hash, breaking set invariants.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::brp_tools::brp_type_guide::example_builder::ExampleBuilder;
use crate::brp_tools::brp_type_guide::response_types::BrpTypeName;
use crate::error::Result;
use crate::json_types::SchemaField;
use crate::string_traits::JsonFieldAccess;

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
        // Generate the example using build_schema_example
        let example = self.build_schema_example(ctx, depth);

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
            // For Sets, these represent unique values to add
            let array = vec![item_example; 2];
            json!(array)
        })
    }
}

impl SetMutationBuilder {
    /// Build set example using extracted logic from `TypeInfo::build_type_example`
    /// This is the static method version that calls `TypeInfo` for element types
    pub fn build_set_example_static(
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
            // For Sets, these represent unique values to add
            let array = vec![item_example; 2];
            json!(array)
        })
    }

    /// Build a mutation path for the entire Set field

    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This set type cannot be mutated - {support}")
            }),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }
}
