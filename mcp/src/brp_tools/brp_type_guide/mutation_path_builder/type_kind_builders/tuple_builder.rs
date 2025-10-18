//! `PathBuilder` for `Tuple` and `TupleStruct` types
//!
//! Handles tuple mutations by extracting prefix items (tuple elements) and building
//! paths for both the entire tuple and individual elements by index.
//!
//! **Recursion**: YES - Tuples recurse into each element to generate mutation paths
//! for nested structures (e.g., `EntityHashMap(HashMap)` generates `.0[key]`).
//! Elements are addressable by position indices `.0`, `.1`, etc.

use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use super::super::BuilderError;
use super::super::NotMutableReason;
use super::super::path_kind::MutationPathDescriptor;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::type_kind_builder::TypeKindBuilder;
use crate::error::Error;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;

pub struct TupleMutationBuilder;

impl TypeKindBuilder for TupleMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        // Use Result-returning API - MutationPathBuilder handles missing schema
        let schema = ctx.require_registry_schema()?;

        // Handle empty tuples (unit type `()`) - they don't have prefixItems
        let Some(prefix_items) = schema.get("prefixItems") else {
            // Empty tuple - no children to process
            return Ok(Vec::new().into_iter());
        };

        // Extract array of element schemas
        let Some(items_array) = prefix_items.as_array() else {
            return Err(Error::schema_processing_for_type(
                ctx.type_name().as_str(),
                "parse_prefix_items",
                "prefixItems is not an array",
            )
            .into());
        };

        // Build PathKind for each tuple element
        let mut children = Vec::new();
        for (index, element_schema) in items_array.iter().enumerate() {
            // Extract element type from schema
            let Some(element_type) = element_schema.extract_field_type() else {
                return Err(Error::schema_processing_for_type(
                    ctx.type_name().as_str(),
                    "extract_element_type",
                    format!("Failed to extract type for element {index}"),
                )
                .into());
            };

            // Create PathKind for this indexed element
            children.push(PathKind::new_indexed_element(
                index,
                element_type,
                ctx.type_name().clone(),
            ));
        }

        Ok(children.into_iter())
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> std::result::Result<Value, BuilderError> {
        // First extract element types to check for Handle wrapper
        let schema = ctx.require_registry_schema()?;
        let elements = RecursionContext::extract_tuple_element_types(schema).unwrap_or_default();

        // Check if this is a single-element Handle wrapper
        if elements.len() == 1 && elements[0].is_handle() {
            return Err(BuilderError::NotMutable(
                NotMutableReason::NonMutableHandle {
                    container_type: ctx.type_name().clone(),
                    element_type:   elements[0].clone(),
                },
            ));
        }

        // Assemble tuple from child examples in order
        // The HashMap keys are created by MutationPathBuilder from PathKind::IndexedElement
        // which converts to just the index as a string: "0", "1", "2" etc.
        let mut tuple_examples = Vec::new();
        for index in 0..elements.len() {
            // MutationPathBuilder creates descriptors from PathKind.to_mutation_path_descriptor()
            // For IndexedElement, this returns just the index as a string
            let key = MutationPathDescriptor::from(index.to_string());
            let example = children.get(&key).cloned().unwrap_or(json!(null));
            tuple_examples.push(example);
        }

        // Special case: single-field tuple structs are unwrapped by BRP
        // Return the inner value directly, not as an array
        if tuple_examples.len() == 1 {
            Ok(tuple_examples.into_iter().next().unwrap_or(json!(null)))
        } else if tuple_examples.is_empty() {
            Ok(json!(null))
        } else {
            Ok(json!(tuple_examples))
        }
    }
}

impl TupleMutationBuilder {}
