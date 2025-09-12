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
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

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

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Err(Error::InvalidState(format!(
                "No schema found for set type: {}",
                ctx.type_name()
            ))
            .into());
        };

        // Extract item type from schema
        let item_type = schema.get_type(SchemaField::Items);

        let Some(item_t) = item_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract item type from schema for type: {}",
                ctx.type_name()
            ))
            .into());
        };

        // Create PathKind for items (ProtocolEnforcer will create context)
        Ok(vec![PathKind::StructField {
            field_name:  SchemaField::Items.to_string(),
            type_name:   item_t.clone(),
            parent_type: ctx.type_name().clone(),
        }])
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

        // Check if the element is complex (non-primitive) type
        self.check_collection_element_complexity(item_example, ctx)?;

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
