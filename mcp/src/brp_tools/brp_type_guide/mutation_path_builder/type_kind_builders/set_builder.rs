//! `PathBuilder` for Set types (`HashSet`, `BTreeSet`, etc.)
//!
//! Unlike Lists, Sets can only be mutated at the top level (replacing/merging the entire set).
//! Sets don't support indexed access or element-level mutations through BRP.
//!
//! **Recursion**: NO - Sets are terminal mutation points. Elements have no stable
//! addresses (no indices or keys) and cannot be individually mutated. Only the entire
//! set can be replaced. Mutating an element could change its hash, breaking set invariants.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::BuilderError;
use super::super::new_types::StructFieldName;
use super::super::path_kind::{MutationPathDescriptor, PathKind};
use super::super::recursion_context::RecursionContext;
use super::super::types::PathAction;
use super::type_kind_builder::TypeKindBuilder;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

pub struct SetMutationBuilder;

impl TypeKindBuilder for SetMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let schema = ctx.require_registry_schema()?;

        // Extract item type from schema
        let item_type = schema.get_type(SchemaField::Items);

        let Some(item_t) = item_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract item type from schema for type: {}",
                ctx.type_name()
            ))
            .into());
        };

        // Create PathKind for items (MutationPathBuilder will create context)
        Ok(vec![PathKind::StructField {
            field_name:  StructFieldName::from(SchemaField::Items),
            type_name:   item_t,
            parent_type: ctx.type_name().clone(),
        }]
        .into_iter())
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> std::result::Result<Value, BuilderError> {
        // At this point, children contains a COMPLETE example for the item type
        let Some(item_example) = children.get(SchemaField::Items.as_ref()) else {
            return Err(BuilderError::SystemError(
                Error::InvalidState(format!(
                    "Protocol violation: Set type {} missing required 'items' child example",
                    ctx.type_name()
                ))
                .into(),
            ));
        };

        // Check if the element is complex (non-primitive) type
        self.check_collection_element_complexity(item_example, ctx)?;

        // Create array with 2 example elements
        // For Sets, these represent unique values to add
        let array = vec![item_example.clone(); 2];
        Ok(json!(array))
    }

    fn child_path_action(&self) -> PathAction {
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
        PathAction::Skip
    }
}
