//! Builder for Map types (`HashMap`, `BTreeMap`, etc.)
//!
//! Like Sets, Maps can only be mutated at the top level (replacing the entire map).
//! Maps don't support individual key mutations through BRP's reflection path system.
//!
//! **Recursion**: NO - Maps are terminal mutation points. Only the entire map can be
//! replaced, not individual entries. BRP reflection expects integer indices `[0]` for
//! arrays, not string keys `["key"]` for maps, making individual entry paths impossible.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::MutationPathBuilder;
use super::super::recursion_context::RecursionContext;
use super::super::types::MutationPathInternal;
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::brp_tools::brp_type_guide::mutation_path_builder::{
    MutationPathDescriptor, PathAction, PathKind,
};
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

pub struct MapMutationBuilder;

impl MutationPathBuilder for MapMutationBuilder {
    #[allow(clippy::panic)]
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        _depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        Err(Error::InvalidState(format!(
            "MapMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
            ctx.type_name()
        )).into())
    }

    fn is_migrated(&self) -> bool {
        true
    }

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

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Err(Error::InvalidState(format!(
                "No schema found for map type: {}",
                ctx.type_name()
            ))
            .into());
        };

        // Extract key and value types from schema
        let key_type = schema.get_type(SchemaField::KeyType);
        let value_type = schema.get_type(SchemaField::ValueType);

        let Some(key_t) = key_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract key type from schema for type: {}",
                ctx.type_name()
            ))
            .into());
        };

        let Some(val_t) = value_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract value type from schema for type: {}",
                ctx.type_name()
            ))
            .into());
        };

        // Create PathKinds for key and value (ProtocolEnforcer will create contexts)
        Ok(vec![
            PathKind::StructField {
                field_name:  SchemaField::Key.to_string(),
                type_name:   key_t,
                parent_type: ctx.type_name().clone(),
            },
            PathKind::StructField {
                field_name:  SchemaField::Value.to_string(),
                type_name:   val_t,
                parent_type: ctx.type_name().clone(),
            },
        ])
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> Result<Value> {
        // At this point, children contains COMPLETE examples:
        // - "key": Full example for the key type (e.g., "example_key" for String)
        // - "value": Full example for the value type (e.g., complete Transform JSON)

        let Some(key_example) = children.get(SchemaField::Key.as_ref()) else {
            return Err(Error::InvalidState(format!(
                "Protocol violation: Map type {} missing required 'key' child example",
                ctx.type_name()
            ))
            .into());
        };

        let Some(value_example) = children.get(SchemaField::Value.as_ref()) else {
            return Err(Error::InvalidState(format!(
                "Protocol violation: Map type {} missing required 'value' child example",
                ctx.type_name()
            ))
            .into());
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
                return Err(Error::schema_processing_for_type(
                    ctx.type_name(),
                    "serialize_map_key",
                    format!("Unexpected complex key type after complexity check: {other:?}"),
                )
                .into());
            }
        };

        // Build final map with the COMPLETE value example
        // For HashMap<String, Transform>, value_example is the full Transform
        let mut map = serde_json::Map::new();
        map.insert(key_str, value_example.clone());
        Ok(json!(map))
    }
}
