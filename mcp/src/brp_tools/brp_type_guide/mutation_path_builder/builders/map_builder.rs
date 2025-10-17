//! `PathBuilder` for Map types (`HashMap`, `BTreeMap`, etc.)
//!
//! Like Sets, Maps can only be mutated at the top level (replacing the entire map).
//! Maps don't support individual key mutations through BRP's reflection path system.
//!
//! **Recursion**: NO - Maps are terminal mutation points. Only the entire map can be
//! replaced, not individual entries. BRP reflection expects integer indices `[0]` for
//! arrays, not string keys `["key"]` for maps, making individual entry paths impossible.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::BuilderError;
use super::super::new_types::StructFieldName;
use super::super::path_builder::PathBuilder;
use super::super::path_kind::{MutationPathDescriptor, PathKind};
use super::super::recursion_context::RecursionContext;
use super::super::types::PathAction;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

pub struct MapMutationBuilder;

impl PathBuilder for MapMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn child_path_action(&self) -> PathAction {
        // Maps DON'T include child paths in the result
        //
        // Why: A HashMap<String, Transform> should only expose:
        //   Path: ""  ->  {"key1": {transform1}, "key2": {transform2}}
        //
        // It should NOT expose Transform's internal paths like:
        //   Path: ".rotation"     -> [0,0,0,1]  // Makes no sense for a map!
        //   Path: ".rotation.x"   -> 0.0        // These aren't valid map mutations
        //
        // The recursion still happens (we need Transform examples to build the map),
        // but those paths aren't included in the final mutation paths list.
        PathAction::Skip
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let schema = ctx.require_registry_schema()?;

        // Extract key and value types from schema
        let key_type = schema.get_type(SchemaField::KeyType);
        let value_type = schema.get_type(SchemaField::ValueType);

        let Some(key_type_name) = key_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract key type from schema for type: {}",
                ctx.type_name()
            ))
            .into());
        };

        let Some(val_type_name) = value_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract value type from schema for type: {}",
                ctx.type_name()
            ))
            .into());
        };

        // Create PathKinds for key and value (MutationPathBuilder will create contexts)
        Ok(vec![
            PathKind::StructField {
                field_name:  StructFieldName::from(SchemaField::Key),
                type_name:   key_type_name,
                parent_type: ctx.type_name().clone(),
            },
            PathKind::StructField {
                field_name:  StructFieldName::from(SchemaField::Value),
                type_name:   val_type_name,
                parent_type: ctx.type_name().clone(),
            },
        ]
        .into_iter())
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> std::result::Result<Value, BuilderError> {
        let Some(key_example) = children.get(SchemaField::Key.as_ref()) else {
            return Err(BuilderError::SystemError(
                Error::InvalidState(format!(
                    "Protocol violation: Map type {} missing required 'key' child example",
                    ctx.type_name()
                ))
                .into(),
            ));
        };

        let Some(value_example) = children.get(SchemaField::Value.as_ref()) else {
            return Err(BuilderError::SystemError(
                Error::InvalidState(format!(
                    "Protocol violation: Map type {} missing required 'value' child example",
                    ctx.type_name()
                ))
                .into(),
            ));
        };

        // Check if the key is complex (non-primitive) type
        self.check_collection_element_complexity(key_example, ctx)?;

        // Convert key to string (JSON maps need string keys)
        let key_str = match key_example {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            other => {
                // This should not happen since we checked for complex keys above
                return Err(BuilderError::SystemError(
                    Error::schema_processing_for_type(
                        ctx.type_name(),
                        "serialize_map_key",
                        format!("Unexpected complex key type after complexity check: {other:?}"),
                    )
                    .into(),
                ));
            }
        };

        // Build final map with the COMPLETE value example
        // For HashMap<String, Transform>, value_example is the full Transform
        let mut map = serde_json::Map::new();
        map.insert(key_str, value_example.clone());
        Ok(json!(map))
    }
}
