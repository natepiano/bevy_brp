//! Builder for List types (Vec, etc.)
//!
//! Similar to `ArrayMutationBuilder` but for dynamic containers like Vec<T>.
//! Lists support indexed access and element-level mutations through BRP.
//!
//! **Recursion**: YES - Lists recurse into elements to generate mutation paths
//! for nested structures (e.g., `Vec<Transform>` generates `[0].translation`).
//! Elements are addressable by index, though indices may change as list mutates.

use std::collections::HashMap;
use std::ops::Deref;

use serde_json::{Value, json};

use super::super::MutationPathBuilder;
use super::super::path_kind::{MutationPathDescriptor, PathKind};
use super::super::recursion_context::RecursionContext;
use super::super::types::MutationPathInternal;
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

pub struct ListMutationBuilder;

impl MutationPathBuilder for ListMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        _depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        Err(Error::InvalidState(format!(
            "ListMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
            ctx.type_name()
        )).into())
    }

    fn is_migrated(&self) -> bool {
        true // MIGRATED!
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Err(Error::SchemaProcessing {
                message:   format!("No schema found for list type: {}", ctx.type_name()),
                type_name: Some(ctx.type_name().to_string()),
                operation: Some("collect_children".to_string()),
                details:   None,
            }
            .into());
        };

        // Extract element type from schema
        let Some(element_type) = schema.get_type(SchemaField::Items) else {
            return Err(Error::SchemaProcessing {
                message:   format!(
                    "Failed to extract element type from schema for list: {}",
                    ctx.type_name()
                ),
                type_name: Some(ctx.type_name().to_string()),
                operation: Some("extract_items_type".to_string()),
                details:   None,
            }
            .into());
        };

        // Lists use indexed PathKind for the element at [0]
        // We only recurse into one element for efficiency
        Ok(vec![PathKind::ArrayElement {
            index:       0,
            type_name:   element_type,
            parent_type: ctx.type_name().clone(),
        }])
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> Result<Value> {
        // Get the single element at index 0
        // The key is just "0", not "[0]" - that's how ArrayElement converts to
        // MutationPathDescriptor
        let element_example = children.get("0").ok_or_else(|| {
            Error::InvalidState(format!(
                "Protocol violation: List {} missing element at index 0. Available keys: {:?}",
                ctx.type_name(),
                children.keys().map(|k| k.deref()).collect::<Vec<_>>()
            ))
        })?;

        // Create single-element array to show it's a list
        // One element is sufficient to demonstrate the pattern
        // Create single-element array to show it's a list
        // One element is sufficient to demonstrate the pattern
        Ok(json!([element_example]))
    }

    // NO child_path_action() override - Lists DO expose indexed child paths
    // This allows mutations like: myList[0].field = value
}
