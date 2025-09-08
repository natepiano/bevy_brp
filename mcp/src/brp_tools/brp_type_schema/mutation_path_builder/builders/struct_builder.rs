//! Builder for Struct types
//!
//! Handles the most complex case - struct mutations with one-level recursion.
//! For field contexts, adds both the struct field itself and nested field paths.
use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::warn;

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, MathComponent, SchemaField};
use crate::brp_tools::brp_type_schema::type_info::TypeInfo;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

pub struct StructMutationBuilder;

impl MutationPathBuilder for StructMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Check depth limit to prevent infinite recursion
        if depth.exceeds_limit() {
            return Ok(vec![Self::build_not_mutatable_path_from_support(
                ctx,
                MutationSupport::RecursionLimitExceeded(ctx.type_name().clone()),
            )]);
        }

        let Some(_schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path_from_support(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();
        let properties = Self::extract_properties(ctx);

        for (field_name, field_info) in properties {
            let field_type = SchemaField::extract_field_type(field_info)
                .unwrap_or_else(|| BrpTypeName::from(field_name.as_str()));

            // Create field context using PathKind
            let field_path_kind = PathKind::new_struct_field(
                field_name.clone(),
                field_type.clone(),
                ctx.type_name().clone(),
            );
            let field_ctx = ctx.create_field_context(field_path_kind);

            // If type extraction failed, handle it
            if SchemaField::extract_field_type(field_info).is_none() {
                paths.push(Self::build_not_mutatable_field_from_support(
                    &field_ctx,
                    MutationSupport::NotInRegistry(field_type.clone()),
                ));
                continue;
            }

            // Check if field is a Value type needing serialization
            let Some(field_schema) = ctx.get_type_schema(&field_type) else {
                paths.push(Self::build_not_mutatable_field_from_support(
                    &field_ctx,
                    MutationSupport::NotInRegistry(field_type.clone()),
                ));
                continue;
            };
            let field_kind = TypeKind::from_schema(field_schema, &field_type);

            // Check if this type has hardcoded knowledge (like Vec3, Vec4, etc.)
            let has_hardcoded_knowledge = BRP_MUTATION_KNOWLEDGE
                .get(&KnowledgeKey::exact(&field_type))
                .is_some();

            if matches!(field_kind, TypeKind::Value) {
                if ctx.value_type_has_serialization(&field_type) {
                    paths.push(Self::build_field_mutation_path(&field_ctx, depth));
                } else {
                    paths.push(Self::build_not_mutatable_field_from_support(
                        &field_ctx,
                        MutationSupport::MissingSerializationTraits(field_type.clone()),
                    ));
                }
            } else {
                // Recurse for nested containers or structs
                let field_paths = field_kind.build_paths(&field_ctx, depth)?;
                paths.extend(field_paths);
            }

            // Special case: Types with hardcoded knowledge that are also structs
            // (like Vec3, Quat, etc.) should have their direct path AND nested paths
            if has_hardcoded_knowledge && matches!(field_kind, TypeKind::Struct) {
                // We already added paths above through normal recursion,
                // but we also need the direct field path with hardcoded example
                if ctx.value_type_has_serialization(&field_type) {
                    // Find and update the direct field path to use hardcoded example
                    if let Some(path) = paths.iter_mut().find(|p| p.path == field_ctx.mutation_path)
                    {
                        if let Some(knowledge) =
                            BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(&field_type))
                        {
                            path.example = knowledge.example().clone();
                        }
                    } else {
                        // If no direct path was created, add it now with hardcoded example
                        paths.push(Self::build_field_mutation_path(&field_ctx, depth));
                    }
                }
            }
        }

        Self::propagate_struct_immutability(&mut paths);
        Ok(paths)
    }
}

impl StructMutationBuilder {
    /// Build a not mutatable path from `MutationSupport` for struct-level errors
    fn build_not_mutatable_path_from_support(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This struct type cannot be mutated - {support}")
            }),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }

    /// Build a not mutatable field path from `MutationSupport` for field-level errors
    fn build_not_mutatable_field_from_support(
        field_ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:            field_ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": "This field cannot be mutated - see error message for details"
            }),
            type_name:       field_ctx.type_name().clone(),
            path_kind:       field_ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }

    /// Build a single field mutation path
    fn build_field_mutation_path(
        field_ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        let field_type = field_ctx.type_name();
        let (field_name, parent_type) = match &field_ctx.path_kind {
            PathKind::StructField {
                field_name,
                parent_type,
                ..
            } => (field_name.as_str(), parent_type),
            _ => unreachable!("build_field_mutation_path should only be called for struct fields"),
        };

        // First check if parent has math components and this field is a component
        let example = field_ctx.parent_knowledge.map_or_else(
            || {
                // No parent knowledge, check struct field first, then type
                BRP_MUTATION_KNOWLEDGE
                    .get(&KnowledgeKey::struct_field(parent_type, field_name))
                    .or_else(|| BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(field_type)))
                    .map_or_else(
                        || {
                            // Don't increment - TypeInfo will handle it
                            TypeInfo::build_type_example(field_type, &field_ctx.registry, depth)
                        },
                        |k| k.example().clone(),
                    )
            },
            |parent_knowledge| {
                MathComponent::try_from(field_name)
                    .ok()
                    .and_then(|component| parent_knowledge.get_component_example(component))
                    .map_or_else(
                        || {
                            // Either not a math component or no example available
                            BRP_MUTATION_KNOWLEDGE
                                .get(&KnowledgeKey::struct_field(parent_type, field_name))
                                .or_else(|| {
                                    BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(field_type))
                                })
                                .map_or_else(
                                    || {
                                        // Don't increment - TypeInfo will handle it
                                        TypeInfo::build_type_example(
                                            field_type,
                                            &field_ctx.registry,
                                            depth,
                                        )
                                    },
                                    |k| k.example().clone(),
                                )
                        },
                        std::clone::Clone::clone,
                    )
            },
        );

        MutationPathInternal {
            path: field_ctx.mutation_path.clone(),
            example,
            type_name: field_type.clone(),
            path_kind: field_ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        }
    }

    /// Extract properties from the schema
    fn extract_properties(ctx: &RecursionContext) -> Vec<(String, &Value)> {
        let Some(schema) = ctx.require_schema() else {
            return Vec::new();
        };

        let Some(properties) = schema
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        else {
            warn!(
                type_name = %ctx.type_name(),
                "No properties field found in struct schema - mutation paths may be incomplete"
            );
            return Vec::new();
        };

        properties.iter().map(|(k, v)| (k.clone(), v)).collect()
    }

    /// Propagate `NotMutatable` status from all struct fields to the root path
    fn propagate_struct_immutability(paths: &mut [MutationPathInternal]) {
        let field_paths: Vec<_> = paths
            .iter()
            .filter(|p| matches!(p.path_kind, PathKind::StructField { .. }))
            .collect();

        if !field_paths.is_empty() {
            let all_fields_not_mutatable = field_paths
                .iter()
                .all(|p| matches!(p.mutation_status, MutationStatus::NotMutatable));

            if all_fields_not_mutatable {
                // Mark any root-level paths as NotMutatable
                for path in paths.iter_mut() {
                    if matches!(path.path_kind, PathKind::RootValue { .. }) {
                        path.mutation_status = MutationStatus::NotMutatable;
                        path.error_reason = Some("non_mutatable_fields".to_string());
                        path.example = json!({
                            "NotMutatable": format!("Type {} contains non-mutatable field types", path.type_name),
                            "agent_directive": "This struct cannot be mutated - all fields contain non-mutatable types"
                        });
                    }
                }
            }
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
                    TypeInfo::build_type_example(
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
