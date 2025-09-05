//! Builder for Tuple and `TupleStruct` types
//!
//! Handles tuple mutations by extracting prefix items (tuple elements) and building
//! paths for both the entire tuple and individual elements by index.
use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::{RecursionContext, RootOrField};
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, SchemaField};
use crate::brp_tools::brp_type_schema::type_info::TypeInfo;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

pub struct TupleMutationBuilder;

impl MutationPathBuilder for TupleMutationBuilder {
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

        let mut paths = Vec::new();
        let elements = RecursionContext::extract_tuple_element_types(schema).unwrap_or_default();

        // Build root tuple path
        Self::build_root_tuple_path(&mut paths, ctx, schema, depth);

        // Check if parent knowledge indicates this should be treated as opaque
        let should_stop_recursion = ctx.parent_knowledge.is_some_and(|knowledge| {
            matches!(knowledge.guidance(), crate::brp_tools::brp_type_schema::mutation_knowledge::KnowledgeGuidance::TreatAsValue { .. })
        });

        // Build paths for each element (unless parent knowledge says to treat as opaque)
        if !should_stop_recursion {
            Self::build_tuple_element_paths(&mut paths, ctx, schema, &elements, depth)?;
        }

        // Propagate mixed mutability status to root path
        Self::propagate_tuple_mixed_mutability(&mut paths);
        Ok(paths)
    }
}

impl TupleMutationBuilder {
    /// Build example value for a tuple type
    pub fn build_tuple_example(
        prefix_items: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        prefix_items.as_array().map_or_else(
            || json!([]),
            |items| {
                let elements: Vec<Value> = items
                    .iter()
                    .map(|item| {
                        SchemaField::extract_field_type(item).map_or(json!(null), |element_type| {
                            BRP_MUTATION_KNOWLEDGE
                                .get(&KnowledgeKey::exact(&element_type))
                                .map_or_else(
                                    || {
                                        // Use TypeInfo instead of null
                                        TypeInfo::build_example_value_for_type_with_depth(
                                            &element_type,
                                            registry,
                                            depth,
                                        )
                                    },
                                    |k| k.example_value().clone(),
                                )
                        })
                    })
                    .collect();

                // Special case: single-field tuple structs are unwrapped by BRP
                // Return the inner value directly, not as an array
                if elements.len() == 1 {
                    elements.into_iter().next().unwrap_or(json!(null))
                } else {
                    json!(elements)
                }
            },
        )
    }

    /// Build a mutation path for a single tuple element with registry checking
    fn build_tuple_element_path(
        ctx: &RecursionContext,
        index: usize,
        element_info: &Value,
        path_prefix: &str,
        parent_type: &BrpTypeName,
        depth: RecursionDepth,
    ) -> Option<MutationPathInternal> {
        let element_type = SchemaField::extract_field_type(element_info)?;
        let path = if path_prefix.is_empty() {
            format!(".{index}")
        } else {
            format!("{path_prefix}.{index}")
        };

        // Inline validation for Value types only (similar to TypeKind::build_paths)
        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            // Element type not in registry - build error path
            return Some(MutationPathInternal {
                path,
                example: json!({
                    "NotMutatable": format!("{}", MutationSupport::NotInRegistry(element_type.clone())),
                    "agent_directive": "Element type not found in registry"
                }),
                enum_variants: None,
                type_name: element_type.clone(),
                path_kind: PathKind::IndexedElement {
                    index,
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason: Option::<String>::from(&MutationSupport::NotInRegistry(element_type)),
            });
        };

        let element_kind = TypeKind::from_schema(element_schema, &element_type);
        let supports_mutation = match element_kind {
            TypeKind::Value => ctx.value_type_has_serialization(&element_type),
            // Other types are assumed mutatable (their builders handle validation)
            _ => true,
        };

        if supports_mutation {
            // Element is mutatable, build normal path
            let elem_example = BRP_MUTATION_KNOWLEDGE
                .get(&KnowledgeKey::exact(&element_type))
                .map_or_else(
                    || {
                        // Use TypeInfo instead of null
                        TypeInfo::build_example_value_for_type_with_depth(
                            &element_type,
                            &ctx.registry,
                            depth,
                        )
                    },
                    |k| k.example_value().clone(),
                );

            Some(MutationPathInternal {
                path,
                example: elem_example,
                enum_variants: None,
                type_name: element_type,
                path_kind: PathKind::IndexedElement {
                    index,
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            })
        } else {
            // Element not mutatable, build error path
            let missing_support = MutationSupport::MissingSerializationTraits(element_type.clone());
            Some(MutationPathInternal {
                path,
                example: json!({
                    "NotMutatable": format!("{missing_support}"),
                    "agent_directive": "Element type cannot be mutated through BRP"
                }),
                enum_variants: None,
                type_name: element_type,
                path_kind: PathKind::IndexedElement {
                    index,
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason: Option::<String>::from(&missing_support),
            })
        }
    }

    /// Propagate mixed mutability from tuple elements to root path according to DESIGN-001
    fn propagate_tuple_mixed_mutability(paths: &mut [MutationPathInternal]) {
        let has_root = paths.iter().any(|p| p.path.is_empty());

        if has_root {
            let (mutable_count, immutable_count) =
                paths.iter().filter(|p| !p.path.is_empty()).fold(
                    (0, 0),
                    |(mut_count, immut_count), path| match path.mutation_status {
                        MutationStatus::NotMutatable => (mut_count, immut_count + 1),
                        _ => (mut_count + 1, immut_count),
                    },
                );

            // Root mutation strategy based on element composition
            if let Some(root) = paths.iter_mut().find(|p| p.path.is_empty()) {
                match (mutable_count, immutable_count) {
                    (0, _) => {
                        // All elements immutable - root cannot be mutated
                        root.mutation_status = MutationStatus::NotMutatable;
                        root.error_reason = Some("non_mutatable_elements".to_string());
                        root.example = json!({
                            "NotMutatable": format!("Type {} contains non-mutatable element types", root.type_name),
                            "agent_directive": "This tuple cannot be mutated - all elements contain non-mutatable types"
                        });
                    }
                    (_, 0) => {
                        // All elements mutable - keep existing mutable root path
                    }
                    (_, _) => {
                        // Mixed mutability - root cannot be replaced, but individual elements can
                        // be mutated
                        root.mutation_status = MutationStatus::PartiallyMutatable;
                        root.error_reason = Some("partially_mutable_elements".to_string());
                        root.example = json!({
                            "PartialMutation": format!("Some elements of {} are immutable", root.type_name),
                            "agent_directive": "Use individual element paths - root replacement not supported",
                            "mutable_elements": mutable_count,
                            "immutable_elements": immutable_count
                        });
                    }
                }
            }
        }
    }

    fn build_root_tuple_path(
        paths: &mut Vec<MutationPathInternal>,
        ctx: &RecursionContext,
        schema: &Value,
        depth: RecursionDepth,
    ) {
        match &ctx.location {
            RootOrField::Root { type_name } => {
                // Use parent knowledge if available (though rare for root)
                let example = ctx.parent_knowledge.map_or_else(
                    || {
                        Self::build_tuple_example(
                            schema
                                .get_field(SchemaField::PrefixItems)
                                .unwrap_or(&json!([])),
                            &ctx.registry,
                            depth,
                        )
                    },
                    |k| k.example_value().clone(),
                );
                paths.push(MutationPathInternal {
                    path: String::new(),
                    example,
                    enum_variants: None,
                    type_name: type_name.clone(),
                    path_kind: PathKind::RootValue {
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
                // Use parent knowledge if available (e.g., struct field knowledge)
                let example = ctx.parent_knowledge.map_or_else(
                    || {
                        Self::build_tuple_example(
                            schema
                                .get_field(SchemaField::PrefixItems)
                                .unwrap_or(&json!([])),
                            &ctx.registry,
                            depth,
                        )
                    },
                    |k| k.example_value().clone(),
                );
                paths.push(MutationPathInternal {
                    path,
                    example,
                    enum_variants: None,
                    type_name: field_type.clone(),
                    path_kind: PathKind::StructField {
                        field_name:  field_name.clone(),
                        parent_type: parent_type.clone(),
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason: None,
                });
            }
        }
    }

    fn build_tuple_element_paths(
        paths: &mut Vec<MutationPathInternal>,
        ctx: &RecursionContext,
        schema: &Value,
        elements: &[BrpTypeName],
        depth: RecursionDepth,
    ) -> Result<()> {
        for (index, element_type) in elements.iter().enumerate() {
            // Create field context with dot prefix for tuple elements
            let element_ctx = ctx.create_field_context(&format!(".{index}"), element_type);
            let Some(element_schema) = ctx.get_type_schema(element_type) else {
                // Build not mutatable element path for missing registry entry
                let path = if ctx.path_prefix.is_empty() {
                    format!(".{index}")
                } else {
                    format!("{}.{index}", ctx.path_prefix)
                };
                paths.push(MutationPathInternal {
                    path,
                    example: json!({
                        "NotMutatable": format!("{}", MutationSupport::NotInRegistry(element_type.clone())),
                        "agent_directive": "Element type not found in registry"
                    }),
                    enum_variants: None,
                    type_name: element_type.clone(),
                    path_kind: PathKind::IndexedElement {
                        index,
                        parent_type: ctx.type_name().clone(),
                    },
                    mutation_status: MutationStatus::NotMutatable,
                    error_reason: Option::<String>::from(&MutationSupport::NotInRegistry(element_type.clone())),
                });
                continue;
            };
            let element_kind = TypeKind::from_schema(element_schema, element_type);

            // Similar to struct fields - check Value types for serialization
            if matches!(element_kind, TypeKind::Value) {
                if ctx.value_type_has_serialization(element_type) {
                    // Use existing build_tuple_element_path method for Value types
                    if let Some(element_info) = schema
                        .get_field(SchemaField::PrefixItems)
                        .and_then(|items| items.as_array())
                        .and_then(|arr| arr.get(index))
                        && let Some(element_path) = Self::build_tuple_element_path(
                            ctx,
                            index,
                            element_info,
                            &ctx.path_prefix,
                            ctx.type_name(),
                            depth,
                        )
                    {
                        paths.push(element_path);
                    }
                } else {
                    // Build not mutatable element path inline
                    let path = if ctx.path_prefix.is_empty() {
                        format!(".{index}")
                    } else {
                        format!("{}.{index}", ctx.path_prefix)
                    };
                    paths.push(MutationPathInternal {
                        path,
                        example: json!({
                            "NotMutatable": format!("{}", MutationSupport::MissingSerializationTraits(element_type.clone())),
                            "agent_directive": "Element type cannot be mutated through BRP"
                        }),
                        enum_variants: None,
                        type_name: element_type.clone(),
                        path_kind: PathKind::IndexedElement {
                            index,
                            parent_type: ctx.type_name().clone(),
                        },
                        mutation_status: MutationStatus::NotMutatable,
                        error_reason: Option::<String>::from(&MutationSupport::MissingSerializationTraits(element_type.clone())),
                    });
                }
            } else {
                // Recurse for nested types
                let element_paths = element_kind.build_paths(&element_ctx, depth)?;
                paths.extend(element_paths);
            }
        }
        Ok(())
    }

    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This tuple type cannot be mutated - {support}")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       PathKind::RootValue {
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
                    "agent_directive": format!("This tuple field cannot be mutated - {support}")
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
