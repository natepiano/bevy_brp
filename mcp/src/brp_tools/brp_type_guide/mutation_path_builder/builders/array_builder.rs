//! Builder for Array types
//!
//! Handles both fixed-size arrays like `[Vec3; 3]` and dynamic arrays.
//! Creates mutation paths for both the entire array and individual elements.
//!
//! **Recursion**: YES - Arrays recurse into each element to generate mutation paths
//! for nested structures (e.g., `[Transform; 3]` generates paths for each Transform).
//! This is because array elements are addressable by stable indices `[0]`, `[1]`, etc.

use serde_json::{Value, json};

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::not_mutable_reason::NotMutableReason;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_guide::constants::{
    DEFAULT_EXAMPLE_ARRAY_SIZE, MAX_EXAMPLE_ARRAY_SIZE, RecursionDepth,
};
use crate::brp_tools::brp_type_guide::response_types::BrpTypeName;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

pub struct ArrayMutationBuilder;

impl MutationPathBuilder for ArrayMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Validate and extract array information
        let (element_type, element_schema) = match Self::validate_and_extract_array_info(ctx) {
            Ok(info) => info,
            Err(error_paths) => return Ok(error_paths),
        };

        let array_size = Self::extract_array_size(ctx.type_name());
        let mut paths = Vec::new();

        // First get nested paths for complex element types - we'll use these to build our examples
        let element_path_kind =
            PathKind::new_array_element(0, element_type.clone(), ctx.type_name().clone());
        let element_ctx = ctx.create_unmigrated_recursion_context(element_path_kind);
        let element_kind = TypeKind::from_schema(element_schema, &element_type);

        let element_paths = if matches!(element_kind, TypeKind::Value) {
            vec![]
        } else {
            element_kind.build_paths(&element_ctx, depth)?
        };

        // Extract the element example from the first element path
        // This is the indexed element path like "[0]"
        // IMPORTANT: Check hardcoded knowledge first to ensure correct BRP format
        let element_example =
            KnowledgeKey::find_example_for_type(&element_type).unwrap_or_else(|| {
                element_paths
                    .iter()
                    .find(|p| p.path == element_ctx.mutation_path)
                    .map_or_else(
                        || {
                            // For Value types or when no direct path exists, generate the example
                            // using trait dispatch
                            element_kind
                                .builder()
                                .build_schema_example(&element_ctx, depth.increment())
                        },
                        |first_element_path| first_element_path.example.clone(),
                    )
            });

        // Build the main array path using the element example
        let array_example = {
            let size = array_size.unwrap_or(2);
            vec![element_example.clone(); size]
        };

        paths.push(MutationPathInternal {
            path:                   ctx.mutation_path.clone(),
            example:                json!(array_example),
            type_name:              ctx.type_name().clone(),
            path_kind:              ctx.path_kind.clone(),
            mutation_status:        MutationStatus::Mutable,
            mutation_status_reason: None,
        });

        // Build the indexed element path
        // For individual array elements, check hardcoded knowledge to ensure correct format
        let indexed_path = format!("{}[0]", ctx.mutation_path);

        // Helper to create indexed path
        let create_indexed_path = |example: Value| MutationPathInternal {
            path: indexed_path.clone(),
            example,
            type_name: element_type.clone(),
            path_kind: element_ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutable,
            mutation_status_reason: None,
        };

        // Check if we need to override an existing indexed path with hardcoded knowledge
        if let Some(knowledge_example) = KnowledgeKey::find_example_for_type(&element_type) {
            // Override with hardcoded knowledge
            let filtered_paths: Vec<_> = element_paths
                .into_iter()
                .filter(|p| p.path != indexed_path)
                .collect();

            paths.push(create_indexed_path(knowledge_example));
            paths.extend(filtered_paths);
        } else {
            // Use generated example if no existing indexed path
            if !element_paths.iter().any(|p| p.path == indexed_path) {
                paths.push(create_indexed_path(element_example));
            }
            paths.extend(element_paths);
        }

        Ok(paths)
    }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        let Some(schema) = ctx.require_registry_schema() else {
            return json!(null);
        };

        // Extract array element type
        let item_type = schema.get_type(SchemaField::Items);

        item_type.map_or(json!(null), |item_type_name| {
            // Generate example value for the item type using trait dispatch
            // First check for hardcoded knowledge
            let item_example = BRP_MUTATION_KNOWLEDGE
                .get(&KnowledgeKey::exact(&item_type_name))
                .map_or_else(
                    || {
                        // Get the element type schema and use trait dispatch directly
                        ctx.get_registry_schema(&item_type_name).map_or(
                            json!(null),
                            |element_schema| {
                                let element_kind =
                                    TypeKind::from_schema(element_schema, &item_type_name);
                                // Create element context for recursive building
                                let element_path_kind = PathKind::new_array_element(
                                    0,
                                    item_type_name.clone(),
                                    ctx.type_name().clone(),
                                );
                                let element_ctx =
                                    ctx.create_unmigrated_recursion_context(element_path_kind);
                                // Use trait dispatch directly instead of ExampleBuilder
                                element_kind
                                    .builder()
                                    .build_schema_example(&element_ctx, depth.increment())
                            },
                        )
                    },
                    |k| k.example().clone(),
                );

            // Parse the array size from the type name (e.g., "[f32; 4]" -> 4)
            let size = ctx
                .type_name()
                .as_str()
                .rsplit_once("; ")
                .and_then(|(_, rest)| rest.strip_suffix(']'))
                .and_then(|s| s.parse::<usize>().ok())
                .map_or(DEFAULT_EXAMPLE_ARRAY_SIZE, |s| {
                    s.min(MAX_EXAMPLE_ARRAY_SIZE)
                });

            // Create array with the appropriate number of elements
            let array = vec![item_example; size];
            json!(array)
        })
    }
}

impl ArrayMutationBuilder {
    /// Validate and extract array information from context
    fn validate_and_extract_array_info(
        ctx: &RecursionContext,
    ) -> core::result::Result<(BrpTypeName, &Value), Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Err(vec![Self::build_not_mutable_path(
                ctx,
                NotMutableReason::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_type) = schema.get_type(SchemaField::Items) else {
            return Err(vec![Self::build_not_mutable_path(
                ctx,
                NotMutableReason::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_schema) = ctx.get_registry_schema(&element_type) else {
            return Err(vec![Self::build_not_mutable_path(
                ctx,
                NotMutableReason::NotInRegistry(element_type),
            )]);
        };

        Ok((element_type, element_schema))
    }

    /// Extract array size from type name (e.g., "[f32; 4]" -> 4)
    fn extract_array_size(type_name: &BrpTypeName) -> Option<usize> {
        let type_str = type_name.as_str();
        // Pattern: [ElementType; Size]
        type_str.rfind("; ").and_then(|size_start| {
            type_str.rfind(']').and_then(|size_end| {
                let size_str = &type_str[size_start + 2..size_end];
                size_str.parse().ok()
            })
        })
    }

    /// Build a not-mutatable path with structured error details
    fn build_not_mutable_path(
        ctx: &RecursionContext,
        support: NotMutableReason,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:                   ctx.mutation_path.clone(),
            example:                json!(null), // No example for NotMutatable paths
            type_name:              ctx.type_name().clone(),
            path_kind:              ctx.path_kind.clone(),
            mutation_status:        MutationStatus::NotMutable,
            mutation_status_reason: Option::<Value>::from(&support),
        }
    }
}
