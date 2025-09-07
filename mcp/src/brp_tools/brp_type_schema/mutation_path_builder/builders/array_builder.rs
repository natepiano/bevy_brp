//! Builder for Array types
//!
//! Handles both fixed-size arrays like `[Vec3; 3]` and dynamic arrays.
//! Creates mutation paths for both the entire array and individual elements.
use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::super::mutation_support::MutationSupport;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use super::super::{MutationPathBuilder, TypeKind};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::response_types::BrpTypeName;
use crate::brp_tools::brp_type_schema::type_info::TypeInfo;
use crate::error::Result;

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

        // Build the main array path
        paths.push(Self::build_main_array_path(
            ctx,
            &element_type,
            array_size,
            depth,
        ));

        // Build the indexed element path
        paths.push(Self::build_indexed_element_path(ctx, &element_type, depth));

        // Add nested paths for complex element types
        Self::add_nested_paths(ctx, &element_type, element_schema, depth, &mut paths)?;

        Ok(paths)
    }
}

impl ArrayMutationBuilder {
    /// Validate and extract array information from context
    fn validate_and_extract_array_info(
        ctx: &RecursionContext,
    ) -> core::result::Result<(BrpTypeName, &Value), Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Err(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_type) = RecursionContext::extract_list_element_type(schema) else {
            return Err(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            return Err(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(element_type),
            )]);
        };

        Ok((element_type, element_schema))
    }

    /// Build the main array path
    fn build_main_array_path(
        ctx: &RecursionContext,
        element_type: &BrpTypeName,
        array_size: Option<usize>,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        let array_example =
            Self::build_array_example(element_type, &ctx.registry, array_size, depth);

        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!(array_example),
            enum_variants:   None,
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason:    None,
        }
    }

    /// Build the indexed element path
    fn build_indexed_element_path(
        ctx: &RecursionContext,
        element_type: &BrpTypeName,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        let element_example = Self::build_element_example(element_type, &ctx.registry, depth);

        // Build array element path using PathKind
        let array_element_path_kind =
            PathKind::new_array_element(0, element_type.clone(), ctx.type_name().clone());
        let indexed_path = format!("{}[0]", ctx.mutation_path);

        MutationPathInternal {
            path:            indexed_path,
            example:         element_example,
            enum_variants:   None,
            type_name:       element_type.clone(),
            path_kind:       array_element_path_kind,
            mutation_status: MutationStatus::Mutatable,
            error_reason:    None,
        }
    }

    /// Add nested paths for complex element types
    fn add_nested_paths(
        ctx: &RecursionContext,
        element_type: &BrpTypeName,
        element_schema: &Value,
        depth: RecursionDepth,
        paths: &mut Vec<MutationPathInternal>,
    ) -> Result<()> {
        let element_path_kind =
            PathKind::new_array_element(0, element_type.clone(), ctx.type_name().clone());
        let element_ctx = ctx.create_field_context(element_path_kind);
        let element_kind = TypeKind::from_schema(element_schema, element_type);
        if !matches!(element_kind, TypeKind::Value) {
            let element_paths = element_kind.build_paths(&element_ctx, depth)?;
            paths.extend(element_paths);
        }
        Ok(())
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

    /// Build array example with repeated element examples
    fn build_array_example(
        element_type: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
        array_size: Option<usize>,
        depth: RecursionDepth,
    ) -> Vec<Value> {
        let element_example = Self::build_element_example(element_type, registry, depth);
        let size = array_size.unwrap_or(2);
        vec![element_example; size]
    }

    /// Build example value for an element
    fn build_element_example(
        element_type: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        // Check for hardcoded knowledge first
        BRP_MUTATION_KNOWLEDGE
            .get(&KnowledgeKey::exact(element_type))
            .map_or_else(
                || {
                    // Pass depth through - TypeInfo will handle incrementing
                    TypeInfo::build_type_example(element_type, registry, depth)
                },
                |k| k.example().clone(),
            )
    }

    // Note: Removed static helper methods build_root_array_path, build_indexed_element_path,
    // and build_field_array_path as we now build paths inline following StructMutationBuilder
    // pattern

    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This array type cannot be mutated - {support}")
            }),
            enum_variants:   None,
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }
}
