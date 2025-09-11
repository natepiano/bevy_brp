//! Builder for Set types (`HashSet`, `BTreeSet`, etc.)
//!
//! Unlike Lists, Sets can only be mutated at the top level (replacing/merging the entire set).
//! Sets don't support indexed access or element-level mutations through BRP.
//!
//! **Recursion**: NO - Sets are terminal mutation points. Elements have no stable
//! addresses (no indices or keys) and cannot be individually mutated. Only the entire
//! set can be replaced. Mutating an element could change its hash, breaking set invariants.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::MutationPathBuilder;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::types::MutationPathInternal;
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::{Error, Result};
use crate::json_types::SchemaField;
use crate::string_traits::JsonFieldAccess;

pub struct SetMutationBuilder;

impl MutationPathBuilder for SetMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        _depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        Err(Error::InvalidState(format!(
            "SetMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
            ctx.type_name()
        )).into())
    }

    fn is_migrated(&self) -> bool {
        true // MIGRATED!
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        let Some(schema) = ctx.require_registry_schema() else {
            tracing::debug!("No schema found for set type: {}", ctx.type_name());
            return vec![];
        };

        // Extract element type from schema
        let item_type = schema.get_type(SchemaField::Items);

        if let Some(item_type_name) = item_type {
            // Create context for item recursion
            let item_path_kind =
                PathKind::new_array_element(0, item_type_name, ctx.type_name().clone());
            let item_ctx = ctx.create_field_context(item_path_kind);
            vec![(SchemaField::Items.to_string(), item_ctx)]
        } else {
            tracing::debug!(
                "Failed to extract item type from schema for type: {}",
                ctx.type_name()
            );
            vec![]
        }
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<String, Value>,
    ) -> Result<Value> {
        // At this point, children contains a COMPLETE example for the item type
        let Some(item_example) = children.get(SchemaField::Items.as_ref()) else {
            return Err(Error::InvalidState(format!(
                "Protocol violation: Set type {} missing required 'items' child example",
                ctx.type_name()
            ))
            .into());
        };

        // Create array with 2 example elements
        // For Sets, these represent unique values to add
        let array = vec![item_example.clone(); 2];
        Ok(json!(array))
    }

    fn include_child_paths(&self) -> bool {
        // Sets DON'T include child paths in the result
        //
        // Why: A HashSet<String> should only expose:
        //   Path: ""  ->  ["example1", "example2"]
        //
        // It should NOT expose element paths like:
        //   Path: "[0]"  -> "example1"  // Makes no sense for a set!
        //
        // Sets are unordered collections - elements have no stable indices.
        // The recursion still happens (we need element examples to build the set),
        // but those paths aren't included in the final mutation paths list.
        false
    }
}
