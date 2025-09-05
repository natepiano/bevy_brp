//! Builder for Enum types
//!
//! Handles enum mutation paths by extracting variant information and building
//! appropriate examples for each enum variant type (Unit, Tuple, Struct).
use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::TypeKind;
use super::super::types::{MutationPathBuilder, MutationPathContext, RootOrField};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use crate::brp_tools::brp_type_schema::response_types::{
    self, BrpTypeName, MutationPathInternal, MutationPathKind, MutationStatus, SchemaField,
    VariantAccess,
};
use crate::brp_tools::brp_type_schema::type_info::{MutationSupport, TypeInfo};
use crate::error::Result;
pub struct EnumMutationBuilder;

impl MutationPathBuilder for EnumMutationBuilder {
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

        // Check depth limit first (like StructMutationBuilder does)
        if depth.exceeds_limit() {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::RecursionLimitExceeded(ctx.type_name().clone()),
            )]);
        }

        let mut paths = Vec::new();

        // Step 1: Add the base enum path with ALL signature examples
        let enum_variants = Self::extract_enum_variants(schema);
        let enum_example = Self::build_enum_example(
            schema,
            &ctx.registry,
            Some(ctx.type_name()),
            depth, // No increment here - just pass current depth
        );

        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path: String::new(),
                    example: enum_example,
                    enum_variants,
                    type_name: type_name.clone(),
                    path_kind: MutationPathKind::RootValue {
                        type_name: type_name.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason: None,
                });
            }
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => {
                // When in field context, use the path_prefix which contains the full path
                let path = if ctx.path_prefix.is_empty() {
                    format!(".{field_name}")
                } else {
                    ctx.path_prefix.clone()
                };
                paths.push(MutationPathInternal {
                    path,
                    example: MutationPathContext::wrap_example(enum_example),
                    enum_variants,
                    type_name: field_type.clone(),
                    path_kind: MutationPathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason: None,
                });
            }
        }

        // Step 2: Recurse into unique signature inner types
        // ONLY add variant field paths when the enum is at the ROOT level
        // When an enum is a field, we don't recurse into its variants because:
        // 1. Only one variant can be active at a time
        // 2. The variant is selected when setting the field value
        // 3. Variant fields are accessed through the enum field path (e.g., .field.0.variant_field)
        if matches!(ctx.location, RootOrField::Root { .. }) {
            let variants = response_types::extract_enum_variants(schema, &ctx.registry, *depth);
            let unique_variants = response_types::deduplicate_variant_signatures(variants);

            for variant in unique_variants {
                for (type_name, variant_access) in variant.inner_types() {
                    // Get the schema for the inner type
                    let Some(inner_schema) = ctx.get_type_schema(&type_name) else {
                        continue; // Skip if we can't find the schema
                    };

                    let inner_kind = TypeKind::from_schema(inner_schema, &type_name);

                    // Create field context for recursion using existing infrastructure
                    let accessor = match &variant_access {
                        VariantAccess::TupleIndex(idx) => format!(".{idx}"),
                        VariantAccess::StructField(name) => format!(".{name}"),
                    };
                    let variant_ctx = ctx.create_field_context(&accessor, &type_name);

                    // Recurse with current depth (TypeKind::build_paths will increment if needed)
                    let nested_paths = inner_kind.build_paths(&variant_ctx, depth)?;
                    paths.extend(nested_paths);
                }
            }
        }

        Ok(paths)
    }
}

impl EnumMutationBuilder {
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
                    "agent_directive": format!("This enum type cannot be mutated - {support}")
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
                    "agent_directive": format!("This enum field cannot be mutated - {support}")
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

    /// Extract enum variants from type schema
    pub fn extract_enum_variants(type_schema: &Value) -> Option<Vec<String>> {
        use response_types::extract_enum_variants as extract_variants_new;

        let variants = extract_variants_new(type_schema, &HashMap::new(), 0);
        if variants.is_empty() {
            None
        } else {
            Some(variants.iter().map(|v| v.name().to_string()).collect())
        }
    }

    /// Build example value for an enum type
    /// CHANGED: Now returns ALL variant examples instead of just the first one
    /// by calling the existing `build_all_enum_examples` function
    pub fn build_enum_example(
        schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        enum_type: Option<&BrpTypeName>,
        depth: RecursionDepth,
    ) -> Value {
        // Check for exact enum type knowledge first
        if let Some(enum_type) = enum_type
            && let Some(knowledge) =
                BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(enum_type.type_string()))
        {
            return knowledge.example_value().clone();
        }

        // CRITICAL: Reuse EXISTING build_all_enum_examples function
        // DO NOT reimplement the deduplication logic - it already exists!
        let all_examples =
            response_types::build_all_enum_examples(schema, registry, *depth, enum_type);

        // Return all variant examples as JSON
        if all_examples.is_empty() {
            json!(null)
        } else {
            json!(all_examples)
        }
    }

    /// Build example struct from properties
    pub fn build_struct_example_from_properties(
        properties: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        // Check depth limit to prevent infinite recursion
        if depth.exceeds_limit() {
            return json!("...");
        }

        let Some(props_map) = properties.as_object() else {
            return json!({});
        };

        let mut example = serde_json::Map::new();

        for (field_name, field_schema) in props_map {
            // Use TypeInfo to build example for each field type with depth tracking
            let field_value = SchemaField::extract_field_type(field_schema)
                .map(|field_type| {
                    TypeInfo::build_example_value_for_type_with_depth(
                        &field_type,
                        registry,
                        depth, // Don't increment - TypeInfo will handle it
                    )
                })
                .unwrap_or(json!(null));

            example.insert(field_name.clone(), field_value);
        }

        json!(example)
    }
}
