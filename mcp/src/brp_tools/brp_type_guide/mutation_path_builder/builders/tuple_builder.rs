//! Builder for Tuple and `TupleStruct` types
//!
//! Handles tuple mutations by extracting prefix items (tuple elements) and building
//! paths for both the entire tuple and individual elements by index.
//!
//! **Recursion**: YES - Tuples recurse into each element to generate mutation paths
//! for nested structures (e.g., `EntityHashMap(HashMap)` generates `.0[key]`).
//! Elements are addressable by position indices `.0`, `.1`, etc.

use serde_json::{Value, json};

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::not_mutatable_reason::NotMutatableReason;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::brp_tools::brp_type_guide::response_types::BrpTypeName;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

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
                NotMutatableReason::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();
        let elements = RecursionContext::extract_tuple_element_types(schema).unwrap_or_default();

        // Check if this is a single-element TupleStruct containing only a Handle type
        if Self::is_handle_only_wrapper(&elements) {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                NotMutatableReason::NonMutatableHandle {
                    container_type: ctx.type_name().clone(),
                    element_type: elements[0].clone(),
                },
            )]);
        }

        // Check if parent knowledge indicates this should be treated as opaque
        let should_stop_recursion = Self::should_treat_as_opaque(ctx);

        let mut tuple_examples = Vec::new();

        // Build paths for each element and collect examples
        if should_stop_recursion {
            Self::collect_parent_knowledge_examples(ctx, &mut tuple_examples);
        } else {
            Self::build_element_paths(ctx, &elements, depth, &mut paths, &mut tuple_examples)?;
        }

        // Build root tuple path with accumulated examples
        let root_example = Self::build_root_example(tuple_examples);

        paths.insert(
            0,
            MutationPathInternal {
                path: ctx.mutation_path.clone(),
                example: root_example,
                type_name: ctx.type_name().clone(),
                path_kind: ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                mutation_status_reason: None,
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
    /// Check if this is a single-element tuple containing only a Handle type
    fn is_handle_only_wrapper(elements: &[BrpTypeName]) -> bool {
        elements.len() == 1
            && elements[0]
                .as_str()
                .starts_with("bevy_asset::handle::Handle<")
    }

    /// Check if parent knowledge indicates this should be treated as opaque
    fn should_treat_as_opaque(ctx: &RecursionContext) -> bool {
        ctx.parent_knowledge.is_some_and(|knowledge| {
            matches!(
                knowledge.guidance(),
                super::super::mutation_knowledge::KnowledgeGuidance::TreatAsValue { .. }
            )
        })
    }

    /// Collect examples from parent knowledge
    fn collect_parent_knowledge_examples(ctx: &RecursionContext, tuple_examples: &mut Vec<Value>) {
        if let Some(knowledge) = ctx.parent_knowledge {
            let example = knowledge.example();
            if let Some(arr) = example.as_array() {
                tuple_examples.clone_from(arr);
            }
        }
    }

    /// Build paths for all tuple elements
    fn build_element_paths(
        ctx: &RecursionContext,
        elements: &[BrpTypeName],
        depth: RecursionDepth,
        paths: &mut Vec<MutationPathInternal>,
        tuple_examples: &mut Vec<Value>,
    ) -> Result<()> {
        for (index, element_type) in elements.iter().enumerate() {
            let element_path_kind =
                PathKind::new_indexed_element(index, element_type.clone(), ctx.type_name().clone());
            let element_ctx = ctx.create_field_context(element_path_kind);

            let Some(element_schema) = ctx.get_registry_schema(element_type) else {
                Self::handle_missing_element(
                    index,
                    element_type,
                    &element_ctx,
                    paths,
                    tuple_examples,
                );
                continue;
            };

            let element_kind = TypeKind::from_schema(element_schema, element_type);

            if matches!(element_kind, TypeKind::Value) {
                Self::handle_value_element(
                    ctx,
                    element_type,
                    &element_ctx,
                    &element_kind,
                    depth,
                    paths,
                    tuple_examples,
                );
            } else {
                Self::handle_complex_element(
                    &element_ctx,
                    &element_kind,
                    depth,
                    paths,
                    tuple_examples,
                )?;
            }
        }
        Ok(())
    }

    /// Handle a missing element (not in registry)
    fn handle_missing_element(
        index: usize,
        element_type: &BrpTypeName,
        element_ctx: &RecursionContext,
        paths: &mut Vec<MutationPathInternal>,
        tuple_examples: &mut Vec<Value>,
    ) {
        let path = if element_ctx.mutation_path.is_empty() {
            format!(".{index}")
        } else {
            format!("{}.{index}", element_ctx.mutation_path)
        };
        paths.push(MutationPathInternal {
            path,
            example: json!(null), // No example for NotMutatable paths
            type_name: element_type.clone(),
            path_kind: element_ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            mutation_status_reason: Option::<String>::from(&NotMutatableReason::NotInRegistry(
                element_type.clone(),
            )),
        });
        tuple_examples.push(json!(null));
    }

    /// Handle a value element
    fn handle_value_element(
        ctx: &RecursionContext,
        element_type: &BrpTypeName,
        element_ctx: &RecursionContext,
        element_kind: &TypeKind,
        depth: RecursionDepth,
        paths: &mut Vec<MutationPathInternal>,
        tuple_examples: &mut Vec<Value>,
    ) {
        if ctx.value_type_has_serialization(element_type) {
            let element_example = element_kind
                .builder()
                .build_schema_example(element_ctx, depth.increment());
            tuple_examples.push(element_example.clone());
            paths.push(MutationPathInternal {
                path: element_ctx.mutation_path.clone(),
                example: element_example,
                type_name: element_type.clone(),
                path_kind: element_ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                mutation_status_reason: None,
            });
        } else {
            tuple_examples.push(json!(null));
            paths.push(MutationPathInternal {
                path: element_ctx.mutation_path.clone(),
                example: json!(null), // No example for NotMutatable paths
                type_name: element_type.clone(),
                path_kind: element_ctx.path_kind.clone(),
                mutation_status: MutationStatus::NotMutatable,
                mutation_status_reason: Option::<String>::from(
                    &NotMutatableReason::MissingSerializationTraits(element_type.clone()),
                ),
            });
        }
    }

    /// Handle a complex element (requires recursion)
    fn handle_complex_element(
        element_ctx: &RecursionContext,
        element_kind: &TypeKind,
        depth: RecursionDepth,
        paths: &mut Vec<MutationPathInternal>,
        tuple_examples: &mut Vec<Value>,
    ) -> Result<()> {
        let element_paths = element_kind.build_paths(element_ctx, depth)?;

        // Extract the element example from the root path
        let element_example = element_paths
            .iter()
            .find(|p| p.path == element_ctx.mutation_path)
            .map_or_else(
                || {
                    // If no direct path, generate example using trait dispatch
                    element_kind
                        .builder()
                        .build_schema_example(element_ctx, depth.increment())
                },
                |p| p.example.clone(),
            );

        tuple_examples.push(element_example);
        paths.extend(element_paths);
        Ok(())
    }

    /// Build the root example from collected tuple examples
    fn build_root_example(tuple_examples: Vec<Value>) -> Value {
        if tuple_examples.len() == 1 {
            // Single-field tuple structs are unwrapped by BRP
            tuple_examples.into_iter().next().unwrap_or(json!(null))
        } else if tuple_examples.is_empty() {
            json!(null)
        } else {
            json!(tuple_examples)
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
                        root.mutation_status_reason = Some("non_mutatable_elements".to_string());
                        root.example = json!(null); // No example for NotMutatable paths
                    }
                    (_, 0) => {
                        // All elements mutable - keep existing mutable root path
                    }
                    (_, _) => {
                        // Mixed mutability - root cannot be replaced, but individual elements can
                        // be mutated
                        root.mutation_status = MutationStatus::PartiallyMutatable;
                        root.mutation_status_reason =
                            Some("partially_mutable_elements".to_string());
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

    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: NotMutatableReason,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example: json!(null), // No example for NotMutatable paths
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            mutation_status_reason: Option::<String>::from(&support),
        }
    }
}
