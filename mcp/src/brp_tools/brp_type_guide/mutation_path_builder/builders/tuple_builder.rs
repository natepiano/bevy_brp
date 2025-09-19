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

use super::super::MutationPathDescriptor;
use super::super::not_mutable_reason::NotMutableReason;
use super::super::path_builder::PathBuilder;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use crate::error::{Error, Result};
use crate::json_schema::SchemaField;

pub struct TupleMutationBuilder;

impl PathBuilder for TupleMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        // Use Result-returning API - MutationPathBuilder handles missing schema
        let schema = ctx.require_registry_schema()?;

        let Some(prefix_items) = schema.get("prefixItems") else {
            return Err(Error::SchemaProcessing {
                message:   format!(
                    "Missing prefixItems in tuple schema for: {}",
                    ctx.type_name()
                ),
                type_name: Some(ctx.type_name().to_string()),
                operation: Some("extract_prefix_items".to_string()),
                details:   None,
            }
            .into());
        };

        // Extract array of element schemas
        let Some(items_array) = prefix_items.as_array() else {
            return Err(Error::SchemaProcessing {
                message:   format!("prefixItems is not an array for tuple: {}", ctx.type_name()),
                type_name: Some(ctx.type_name().to_string()),
                operation: Some("parse_prefix_items".to_string()),
                details:   None,
            }
            .into());
        };

        // Build PathKind for each tuple element
        let mut children = Vec::new();
        for (index, element_schema) in items_array.iter().enumerate() {
            // Extract element type from schema
            let Some(element_type) = SchemaField::extract_field_type(element_schema) else {
                return Err(Error::SchemaProcessing {
                    message:   format!(
                        "Failed to extract type for tuple element {} in '{}'",
                        index,
                        ctx.type_name()
                    ),
                    type_name: Some(ctx.type_name().to_string()),
                    operation: Some("extract_element_type".to_string()),
                    details:   Some(format!("Element index: {index}")),
                }
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
    ) -> Result<Value> {
        // First extract element types to check for Handle wrapper
        let schema = ctx.require_registry_schema()?;
        let elements = RecursionContext::extract_tuple_element_types(schema).unwrap_or_default();

        // Check if this is a single-element Handle wrapper
        if elements.len() == 1 && elements[0].is_handle() {
            return Err(Error::NotMutable(NotMutableReason::NonMutableHandle {
                container_type: ctx.type_name().clone(),
                element_type:   elements[0].clone(),
            })
            .into());
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
