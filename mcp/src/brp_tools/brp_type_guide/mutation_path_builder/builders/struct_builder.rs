//! Builder for Struct types
//!
//! Handles the most complex case - struct mutations with one-level recursion.
//! For field contexts, adds both the struct field itself and nested field paths.
//!
//! **Recursion**: YES - Structs recurse into each field to generate mutation paths
//! for nested structures (e.g., `Transform.translation.x`). Each field has a stable
//! name that can be used in paths, allowing deep mutation of nested structures.

use serde_json::{Value, json};
use tracing::warn;

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::not_mutatable_reason::NotMutableReason;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::brp_tools::brp_type_guide::response_types::{BrpTypeName, MathComponent};
use crate::error::Result;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

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
                NotMutableReason::RecursionLimitExceeded(ctx.type_name().clone()),
            )]);
        }

        let Some(_schema) = ctx.require_registry_schema() else {
            return Ok(vec![Self::build_not_mutatable_path_from_support(
                ctx,
                NotMutableReason::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();
        let mut struct_example = serde_json::Map::new();
        let properties = Self::extract_properties(ctx);

        for (field_name, field_info) in properties {
            let (field_example, field_paths) =
                Self::process_field(ctx, &field_name, field_info, depth)?;
            paths.extend(field_paths);
            struct_example.insert(field_name, field_example);
        }

        // Add the root struct path with the accumulated example
        // Always add root path - all PathKind variants can contain structs that may need direct
        // access
        {
            paths.insert(
                0,
                MutationPathInternal {
                    path:                   ctx.mutation_path.clone(),
                    example:                json!(struct_example),
                    type_name:              ctx.type_name().clone(),
                    path_kind:              ctx.path_kind.clone(),
                    mutation_status:        MutationStatus::Mutatable,
                    mutation_status_reason: None,
                },
            );
        }

        Self::propagate_struct_immutability(&mut paths);

        Ok(paths)
    }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        // Check depth limit to prevent infinite recursion
        if depth.exceeds_limit() {
            return json!("...");
        }

        let Some(schema) = ctx.require_registry_schema() else {
            return json!(null);
        };

        // Extract properties using the same logic as the static method
        schema
            .get_field(SchemaField::Properties)
            .map_or(json!(null), |properties| {
                Self::build_struct_example_from_properties_with_context(
                    properties,
                    ctx,
                    depth.increment(),
                )
            })
    }
}

impl StructMutationBuilder {
    /// Process a single struct field and return its example and paths
    fn process_field(
        ctx: &RecursionContext,
        field_name: &str,
        field_info: &Value,
        depth: RecursionDepth,
    ) -> Result<(Value, Vec<MutationPathInternal>)> {
        let mut paths = Vec::new();

        let field_type = SchemaField::extract_field_type(field_info)
            .unwrap_or_else(|| BrpTypeName::from(field_name));

        // Create field context using PathKind
        let field_path_kind = PathKind::new_struct_field(
            field_name.to_string(),
            field_type.clone(),
            ctx.type_name().clone(),
        );
        let field_ctx = ctx.create_unmigrated_recursion_context(field_path_kind);

        // If type extraction failed, handle it
        if SchemaField::extract_field_type(field_info).is_none() {
            paths.push(Self::build_not_mutatable_field_from_support(
                &field_ctx,
                NotMutableReason::NotInRegistry(field_type.clone()),
            ));
            return Ok((json!(null), paths));
        }

        // Check if field is a Value type needing serialization
        let Some(field_schema) = ctx.get_registry_schema(&field_type) else {
            paths.push(Self::build_not_mutatable_field_from_support(
                &field_ctx,
                NotMutableReason::NotInRegistry(field_type.clone()),
            ));
            return Ok((json!(null), paths));
        };

        let field_kind = TypeKind::from_schema(field_schema, &field_type);
        let has_hardcoded_knowledge = BRP_MUTATION_KNOWLEDGE
            .get(&KnowledgeKey::exact(&field_type))
            .is_some();

        let field_example = if matches!(field_kind, TypeKind::Value) {
            Self::process_value_field(&field_ctx, &field_type, depth, &mut paths)
        } else {
            Self::process_complex_field(&field_ctx, &field_kind, depth, &mut paths)?
        };

        // Handle direct field path creation and hardcoded knowledge
        let final_example = Self::finalize_field_paths(
            &field_ctx,
            &field_type,
            &field_kind,
            has_hardcoded_knowledge,
            field_example,
            &mut paths,
        );

        Ok((final_example, paths))
    }

    /// Process a value field type
    fn process_value_field(
        field_ctx: &RecursionContext,
        field_type: &BrpTypeName,
        depth: RecursionDepth,
        paths: &mut Vec<MutationPathInternal>,
    ) -> Value {
        if field_ctx.value_type_has_serialization(field_type) {
            let path = Self::build_field_mutation_path(field_ctx, depth);
            let example = path.example.clone();
            paths.push(path);
            example
        } else {
            paths.push(Self::build_not_mutatable_field_from_support(
                field_ctx,
                NotMutableReason::MissingSerializationTraits(field_type.clone()),
            ));
            json!(null)
        }
    }

    /// Process a complex field (struct, array, etc.)
    fn process_complex_field(
        field_ctx: &RecursionContext,
        field_kind: &TypeKind,
        depth: RecursionDepth,
        paths: &mut Vec<MutationPathInternal>,
    ) -> Result<Value> {
        let field_paths = field_kind.build_paths(field_ctx, depth)?;

        // CRITICAL DEBUG: Log what ProtocolEnforcer returns vs what we expect
        // Extract the field example from the root path
        let field_example = Self::extract_field_example(&field_paths, field_ctx, field_kind, depth);

        paths.extend(field_paths);

        Ok(field_example)
    }

    /// Extract field example from paths
    fn extract_field_example(
        field_paths: &[MutationPathInternal],
        field_ctx: &RecursionContext,
        field_kind: &TypeKind,
        depth: RecursionDepth,
    ) -> Value {
        field_paths
            .iter()
            .find(|p| p.path == field_ctx.mutation_path)
            .map_or_else(
                || {
                    // If no direct path, generate example using trait dispatch
                    // For struct root paths with enum fields, this ensures concrete
                    // examples (like "Active") instead of
                    // __enum_signature_groups documentation format
                    field_kind
                        .builder()
                        .build_schema_example(field_ctx, depth.increment())
                },
                |p| {
                    // Check if this is signature groups array from enum builder
                    p.example.as_array().map_or_else(
                        || p.example.clone(),
                        |signature_groups| {
                            // Extract first concrete example from signature groups
                            signature_groups
                                .first()
                                .and_then(|group| group.get("example"))
                                .cloned()
                                .unwrap_or_else(|| p.example.clone())
                        },
                    )
                },
            )
    }

    /// Finalize field paths and handle hardcoded knowledge
    fn finalize_field_paths(
        field_ctx: &RecursionContext,
        field_type: &BrpTypeName,
        field_kind: &TypeKind,
        has_hardcoded_knowledge: bool,
        field_example: Value,
        paths: &mut Vec<MutationPathInternal>,
    ) -> Value {
        // Always create a direct field path if it doesn't exist yet
        if field_ctx.value_type_has_serialization(field_type) {
            // Check if a direct field path already exists
            if !paths.iter().any(|p| p.path == field_ctx.mutation_path) {
                // Create direct field path with computed example
                let field_path = MutationPathInternal {
                    path:                   field_ctx.mutation_path.clone(),
                    example:                field_example.clone(),
                    type_name:              field_type.clone(),
                    path_kind:              field_ctx.path_kind.clone(),
                    mutation_status:        MutationStatus::Mutatable,
                    mutation_status_reason: None,
                };
                paths.push(field_path);
            }

            // Special case: Types with hardcoded knowledge that are also structs
            // (like Vec3, Quat, etc.) should have their direct path updated with hardcoded
            // example
            if has_hardcoded_knowledge && matches!(field_kind, TypeKind::Struct) {
                // Find and update the direct field path to use hardcoded example
                if let Some(path) = paths.iter_mut().find(|p| p.path == field_ctx.mutation_path)
                    && let Some(knowledge) =
                        BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(field_type))
                {
                    path.example = knowledge.example().clone();
                    return path.example.clone();
                }
            }
        }

        field_example
    }

    /// Build a not mutatable path from `MutationSupport` for struct-level errors
    fn build_not_mutatable_path_from_support(
        ctx: &RecursionContext,
        support: NotMutableReason,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:                   ctx.mutation_path.clone(),
            example:                json!(null), // No example for NotMutatable paths
            type_name:              ctx.type_name().clone(),
            path_kind:              ctx.path_kind.clone(),
            mutation_status:        MutationStatus::NotMutatable,
            mutation_status_reason: Option::<String>::from(&support),
        }
    }

    /// Build a not mutatable field path from `MutationSupport` for field-level errors
    fn build_not_mutatable_field_from_support(
        field_ctx: &RecursionContext,
        support: NotMutableReason,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:                   field_ctx.mutation_path.clone(),
            example:                json!(null), // No example for NotMutatable paths
            type_name:              field_ctx.type_name().clone(),
            path_kind:              field_ctx.path_kind.clone(),
            mutation_status:        MutationStatus::NotMutatable,
            mutation_status_reason: Option::<String>::from(&support),
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
                            // Generate example using trait dispatch
                            field_ctx
                                .get_registry_schema(field_type)
                                .map(|schema| {
                                    let kind = TypeKind::from_schema(schema, field_type);
                                    kind.builder().build_schema_example(field_ctx, depth)
                                })
                                .unwrap_or(json!(null))
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
                                        // Generate example using trait dispatch
                                        field_ctx
                                            .get_registry_schema(field_type)
                                            .map(|schema| {
                                                let kind =
                                                    TypeKind::from_schema(schema, field_type);
                                                kind.builder()
                                                    .build_schema_example(field_ctx, depth)
                                            })
                                            .unwrap_or(json!(null))
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
            mutation_status_reason: None,
        }
    }

    /// Extract properties from the schema
    fn extract_properties(ctx: &RecursionContext) -> Vec<(String, &Value)> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Vec::new();
        };

        let Some(properties) = schema.get_properties() else {
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
                        path.mutation_status_reason = Some("non_mutatable_fields".to_string());
                        path.example = json!(null); // No example for NotMutatable paths
                    }
                }
            }
        }
    }

    /// Build example struct from properties with context (trait method version)
    fn build_struct_example_from_properties_with_context(
        properties: &Value,
        ctx: &RecursionContext,
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
            // Use trait dispatch for each field type with depth tracking
            let field_value = SchemaField::extract_field_type(field_schema)
                .map(|field_type| {
                    // First check for hardcoded knowledge
                    BRP_MUTATION_KNOWLEDGE
                        .get(&KnowledgeKey::exact(&field_type))
                        .map_or_else(
                            || {
                                // Get field schema and use trait dispatch
                                ctx.get_registry_schema(&field_type)
                                    .map_or(json!(null), |_| {
                                        // Create field context for recursive building
                                        let field_path_kind = PathKind::new_struct_field(
                                            field_name.clone(),
                                            field_type.clone(),
                                            ctx.type_name().clone(),
                                        );
                                        let field_ctx = ctx
                                            .create_unmigrated_recursion_context(field_path_kind);
                                        // Use trait dispatch directly
                                        ctx.get_registry_schema(&field_type)
                                            .map(|schema| {
                                                let kind =
                                                    TypeKind::from_schema(schema, &field_type);
                                                kind.builder()
                                                    .build_schema_example(&field_ctx, depth)
                                            })
                                            .unwrap_or(json!(null))
                                    })
                            },
                            |k| k.example().clone(),
                        )
                })
                .unwrap_or(json!(null));

            example.insert(field_name.clone(), field_value);
        }

        json!(example)
    }
}
