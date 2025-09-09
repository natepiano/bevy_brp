//! Builder for Tuple and `TupleStruct` types
//!
//! Handles tuple mutations by extracting prefix items (tuple elements) and building
//! paths for both the entire tuple and individual elements by index.
//!
//! **Recursion**: YES - Tuples recurse into each element to generate mutation paths
//! for nested structures (e.g., `EntityHashMap(HashMap)` generates `.0[key]`).
//! Elements are addressable by position indices `.0`, `.1`, etc.
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

pub struct TupleMutationBuilder;

impl MutationPathBuilder for TupleMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();
        let elements = RecursionContext::extract_tuple_element_types(schema).unwrap_or_default();

        // Check if this is a single-element TupleStruct containing only a Handle type
        if elements.len() == 1
            && elements[0]
                .as_str()
                .starts_with("bevy_asset::handle::Handle<")
        {
            // This is a Handle-only component wrapper that cannot be mutated
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NonMutatableHandle {
                    container_type: ctx.type_name().clone(),
                    element_type:   elements[0].clone(),
                },
            )]);
        }

        // Check if parent knowledge indicates this should be treated as opaque
        let should_stop_recursion = ctx.parent_knowledge.is_some_and(|knowledge| {
            matches!(
                knowledge.guidance(),
                super::super::mutation_knowledge::KnowledgeGuidance::TreatAsValue { .. }
            )
        });

        let mut tuple_examples = Vec::new();

        // Build paths for each element and collect examples (unless parent knowledge says to treat
        // as opaque)
        if !should_stop_recursion {
            for (index, element_type) in elements.iter().enumerate() {
                let element_path_kind = PathKind::new_indexed_element(
                    index,
                    element_type.clone(),
                    ctx.type_name().clone(),
                );
                let element_ctx = ctx.create_field_context(element_path_kind);

                let Some(element_schema) = ctx.get_registry_schema(element_type) else {
                    // Element not in registry - create error path
                    let path = if ctx.mutation_path.is_empty() {
                        format!(".{index}")
                    } else {
                        format!("{}.{index}", ctx.mutation_path)
                    };
                    paths.push(MutationPathInternal {
                        path,
                        example: json!({
                            "NotMutatable": format!("{}", MutationSupport::NotInRegistry(element_type.clone())),
                            "agent_directive": "Element type not found in registry"
                        }),
                        type_name: element_type.clone(),
                        path_kind: element_ctx.path_kind.clone(),
                        mutation_status: MutationStatus::NotMutatable,
                        error_reason: Option::<String>::from(&MutationSupport::NotInRegistry(element_type.clone())),
                    });
                    tuple_examples.push(json!(null));
                    continue;
                };

                let element_kind = TypeKind::from_schema(element_schema, element_type);

                if matches!(element_kind, TypeKind::Value) {
                    // For Value types, check serialization and build directly
                    if ctx.value_type_has_serialization(element_type) {
                        let element_example = element_kind
                            .builder()
                            .build_schema_example(&element_ctx, depth.increment());
                        tuple_examples.push(element_example.clone());
                        paths.push(MutationPathInternal {
                            path:            element_ctx.mutation_path.clone(),
                            example:         element_example,
                            type_name:       element_type.clone(),
                            path_kind:       element_ctx.path_kind.clone(),
                            mutation_status: MutationStatus::Mutatable,
                            error_reason:    None,
                        });
                    } else {
                        tuple_examples.push(json!(null));
                        paths.push(MutationPathInternal {
                            path: element_ctx.mutation_path.clone(),
                            example: json!({
                                "NotMutatable": format!("{}", MutationSupport::MissingSerializationTraits(element_type.clone())),
                                "agent_directive": "Element type cannot be mutated through BRP"
                            }),
                            type_name: element_type.clone(),
                            path_kind: element_ctx.path_kind.clone(),
                            mutation_status: MutationStatus::NotMutatable,
                            error_reason: Option::<String>::from(&MutationSupport::MissingSerializationTraits(element_type.clone())),
                        });
                    }
                } else {
                    // Recurse for complex types
                    let element_paths = element_kind.build_paths(&element_ctx, depth)?;

                    // Extract the element example from the root path
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

                    tuple_examples.push(element_example);
                    paths.extend(element_paths);
                }
            }
        } else {
            // Use parent knowledge for example if not recursing
            if let Some(knowledge) = ctx.parent_knowledge {
                let example = knowledge.example();
                if let Some(arr) = example.as_array() {
                    tuple_examples = arr.clone();
                }
            }
        }

        // Build root tuple path with accumulated examples
        let root_example = if tuple_examples.len() == 1 {
            // Single-field tuple structs are unwrapped by BRP
            tuple_examples.into_iter().next().unwrap_or(json!(null))
        } else if tuple_examples.is_empty() {
            json!(null)
        } else {
            json!(tuple_examples)
        };

        paths.insert(
            0,
            MutationPathInternal {
                path:            ctx.mutation_path.clone(),
                example:         root_example,
                type_name:       ctx.type_name().clone(),
                path_kind:       ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                error_reason:    None,
            },
        );

        // Propagate mixed mutability status to root path
        Self::propagate_tuple_mixed_mutability(&mut paths);
        Ok(paths)
    }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        let Some(schema) = ctx.require_registry_schema() else {
            return json!(null);
        };

        // Extract prefixItems using the same logic as the static method
        schema
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array)
            .map_or(json!(null), |prefix_items| {
                let tuple_examples: Vec<Value> = prefix_items
                    .iter()
                    .map(|item| {
                        SchemaField::extract_field_type(item).map_or_else(
                            || json!(null),
                            |element_type| {
                                // First check for hardcoded knowledge
                                BRP_MUTATION_KNOWLEDGE
                                    .get(&KnowledgeKey::exact(&element_type))
                                    .map_or_else(
                                        || {
                                            // Get the element type schema and use trait
                                            // dispatch
                                            ctx.get_registry_schema(&element_type).map_or(
                                                json!(null),
                                                |element_schema| {
                                                    let element_kind = TypeKind::from_schema(
                                                        element_schema,
                                                        &element_type,
                                                    );
                                                    // Create element context for recursive
                                                    // building
                                                    let element_path_kind =
                                                        PathKind::new_indexed_element(
                                                            0,
                                                            element_type.clone(),
                                                            ctx.type_name().clone(),
                                                        );
                                                    let element_ctx =
                                                        ctx.create_field_context(element_path_kind);
                                                    // Use trait dispatch directly
                                                    element_kind.builder().build_schema_example(
                                                        &element_ctx,
                                                        depth.increment(),
                                                    )
                                                },
                                            )
                                        },
                                        |k| k.example().clone(),
                                    )
                            },
                        )
                    })
                    .collect();

                if tuple_examples.is_empty() {
                    json!(null)
                } else {
                    // Special case: single-field tuple structs are unwrapped by BRP
                    // Return the inner value directly, not as an array
                    if tuple_examples.len() == 1 {
                        tuple_examples.into_iter().next().unwrap_or(json!(null))
                    } else {
                        json!(tuple_examples)
                    }
                }
            })
    }
}

impl TupleMutationBuilder {
    /// Build tuple example using extracted logic from `TypeGuide::build_type_example`
    /// This is the static method version that calls ``TypeGuide`` for element types
    pub fn build_tuple_example_static(
        schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        // Extract prefixItems using the same logic as `TypeGuide`
        schema
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array)
            .map_or(json!(null), |prefix_items| {
                let tuple_examples: Vec<Value> = prefix_items
                    .iter()
                    .map(|item| {
                        SchemaField::extract_field_type(item).map_or_else(
                            || json!(null),
                            |ft| ExampleBuilder::build_example(&ft, registry, depth.increment()),
                        )
                    })
                    .collect();

                if tuple_examples.is_empty() {
                    json!(null)
                } else {
                    json!(tuple_examples)
                }
            })
    }

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
                                        // Use `TypeGuide` instead of null
                                        ExampleBuilder::build_example(
                                            &element_type,
                                            registry,
                                            depth,
                                        )
                                    },
                                    |k| k.example().clone(),
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
        let Some(element_schema) = ctx.get_registry_schema(&element_type) else {
            // Element type not in registry - build error path
            return Some(MutationPathInternal {
                path,
                example: json!({
                    "NotMutatable": format!("{}", MutationSupport::NotInRegistry(element_type.clone())),
                    "agent_directive": "Element type not found in registry"
                }),
                type_name: element_type.clone(),
                path_kind: PathKind::new_indexed_element(
                    index,
                    element_type.clone(),
                    parent_type.clone(),
                ),
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
                        // Use `TypeGuide` instead of null
                        ExampleBuilder::build_example(&element_type, &ctx.registry, depth)
                    },
                    |k| k.example().clone(),
                );

            Some(MutationPathInternal {
                path,
                example: elem_example,
                type_name: element_type.clone(),
                path_kind: PathKind::new_indexed_element(
                    index,
                    element_type.clone(),
                    parent_type.clone(),
                ),
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
                type_name: element_type.clone(),
                path_kind: PathKind::new_indexed_element(
                    index,
                    element_type.clone(),
                    parent_type.clone(),
                ),
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
        // Use parent knowledge if available
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
            |k| k.example().clone(),
        );

        paths.push(MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        });
    }

    fn build_tuple_element_paths(
        paths: &mut Vec<MutationPathInternal>,
        ctx: &RecursionContext,
        schema: &Value,
        elements: &[BrpTypeName],
        depth: RecursionDepth,
    ) -> Result<()> {
        for (index, element_type) in elements.iter().enumerate() {
            // Create field context using PathKind
            let element_path_kind =
                PathKind::new_indexed_element(index, element_type.clone(), ctx.type_name().clone());
            let element_ctx = ctx.create_field_context(element_path_kind);
            let Some(element_schema) = ctx.get_registry_schema(element_type) else {
                // Build not mutatable element path for missing registry entry
                let path = if ctx.mutation_path.is_empty() {
                    format!(".{index}")
                } else {
                    format!("{}.{index}", ctx.mutation_path)
                };
                paths.push(MutationPathInternal {
                    path,
                    example: json!({
                        "NotMutatable": format!("{}", MutationSupport::NotInRegistry(element_type.clone())),
                        "agent_directive": "Element type not found in registry"
                    }),
                                        type_name: element_type.clone(),
                    path_kind: PathKind::new_indexed_element(index, element_type.clone(), ctx.type_name().clone()),
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
                            &ctx.mutation_path,
                            ctx.type_name(),
                            depth,
                        )
                    {
                        paths.push(element_path);
                    }
                } else {
                    // Build not mutatable element path inline
                    let path = if ctx.mutation_path.is_empty() {
                        format!(".{index}")
                    } else {
                        format!("{}.{index}", ctx.mutation_path)
                    };
                    paths.push(MutationPathInternal {
                        path,
                        example: json!({
                            "NotMutatable": format!("{}", MutationSupport::MissingSerializationTraits(element_type.clone())),
                            "agent_directive": "Element type cannot be mutated through BRP"
                        }),
                                                type_name: element_type.clone(),
                        path_kind: PathKind::new_indexed_element(index, element_type.clone(), ctx.type_name().clone()),
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
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This tuple type cannot be mutated - {support}")
            }),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }
}
