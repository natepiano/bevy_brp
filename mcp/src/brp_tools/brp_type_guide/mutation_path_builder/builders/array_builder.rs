//! `PathBuilder` for Array types
//!
//! Handles both fixed-size arrays like `[Vec3; 3]` and dynamic arrays.
//! Creates mutation paths for both the entire array and individual elements.
//!
//! **Recursion**: YES - Arrays recurse into each element to generate mutation paths
//! for nested structures (e.g., `[Transform; 3]` generates paths for each Transform).
//! This is because array elements are addressable by stable indices `[0]`, `[1]`, etc.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::path_builder::PathBuilder;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::{BuilderError, MutationPathDescriptor};
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

pub struct ArrayMutationBuilder;

impl PathBuilder for ArrayMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let schema = ctx.require_registry_schema()?;

        // Extract element type from schema
        let Some(element_type) = schema.get_type(SchemaField::Items) else {
            return Err(Error::SchemaProcessing {
                message:   format!(
                    "Failed to extract element type from schema for array: {}",
                    ctx.type_name()
                ),
                type_name: Some(ctx.type_name().to_string()),
                operation: Some("extract_items_type".to_string()),
                details:   None,
            }
            .into());
        };

        // Arrays use indexed PathKind for the element at [0]
        // We only recurse into one element for efficiency
        Ok(vec![PathKind::ArrayElement {
            index:       0,
            type_name:   element_type,
            parent_type: ctx.type_name().clone(),
        }]
        .into_iter())
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> std::result::Result<Value, BuilderError> {
        // Get the single element at index 0
        // The key is just "0", not "[0]" - that's how ArrayElement converts to
        // MutationPathDescriptor
        let element_example =
            children.get("0").ok_or_else(|| {
                BuilderError::SystemError(Error::InvalidState(format!(
                "Protocol violation: Array {} missing element at index 0. Available keys: {:?}",
                ctx.type_name(),
                children.keys().collect::<Vec<_>>()
            )).into())
            })?;

        // Create array with appropriate size
        let array_size = Self::extract_array_size(ctx.type_name());
        let size = array_size.unwrap_or(2);

        // Create array filled with the element example
        let array = vec![element_example.clone(); size];
        Ok(json!(array))
    }

    // NO child_path_action() override - Arrays DO expose indexed child paths
    // This allows mutations like: myArray[0].field = value
}

impl ArrayMutationBuilder {
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
}
