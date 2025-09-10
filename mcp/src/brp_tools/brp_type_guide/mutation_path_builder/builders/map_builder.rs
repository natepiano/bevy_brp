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
use crate::error::Result;
use crate::json_types::SchemaField;
use crate::string_traits::JsonFieldAccess;

pub struct MapMutationBuilder;

impl MutationPathBuilder for MapMutationBuilder {
    #[allow(clippy::panic)]
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        _depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        tracing::error!(
            "MapMutationBuilder::build_paths() called directly! Type: {}",
            ctx.type_name()
        );
        panic!(
            "MapMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
            ctx.type_name()
        );
    }

    fn is_migrated(&self) -> bool {
        true
    }

    fn include_child_paths(&self) -> bool {
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
        false
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        let Some(schema) = ctx.require_registry_schema() else {
            tracing::debug!("No schema found for map type: {}", ctx.type_name());
            return vec![];
        };

        // Extract key and value types from schema
        let key_type = schema.get_type(SchemaField::KeyType);
        let value_type = schema.get_type(SchemaField::ValueType);

        let mut children = vec![];

        if let Some(key_t) = key_type {
            // Create context for key recursion
            let key_path_kind = super::super::path_kind::PathKind::new_root_value(key_t);
            let key_ctx = ctx.create_field_context(key_path_kind);
            children.push((SchemaField::Key.to_string(), key_ctx));
        } else {
            tracing::debug!(
                "Failed to extract key type from schema for type: {}",
                ctx.type_name()
            );
        }

        if let Some(val_t) = value_type {
            // Create context for value recursion
            let val_path_kind = super::super::path_kind::PathKind::new_root_value(val_t);
            let val_ctx = ctx.create_field_context(val_path_kind);
            children.push((SchemaField::Value.to_string(), val_ctx));
        } else {
            tracing::debug!(
                "Failed to extract value type from schema for type: {}",
                ctx.type_name()
            );
        }

        children
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<String, Value>,
    ) -> Value {
        // At this point, children contains COMPLETE examples:
        // - "key": Full example for the key type (e.g., "example_key" for String)
        // - "value": Full example for the value type (e.g., complete Transform JSON)

        let Some(key_example) = children.get(SchemaField::Key.as_ref()) else {
            tracing::warn!(
                "Missing key example for map type {}, using fallback",
                ctx.type_name()
            );
            return json!({"example_key": "example_value"});
        };

        let Some(value_example) = children.get(SchemaField::Value.as_ref()) else {
            tracing::warn!(
                "Missing value example for map type {}, using fallback",
                ctx.type_name()
            );
            return json!({"example_key": "example_value"});
        };

        // Convert key to string (JSON maps need string keys)
        let key_str = match key_example {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => {
                tracing::warn!(
                    "Complex key type for map serialization, type: {}, falling back to generic key",
                    ctx.type_name()
                );
                serde_json::to_string(other).unwrap_or_else(|e| {
                    tracing::error!(
                        "Failed to serialize map key for type {}: {}",
                        ctx.type_name(),
                        e
                    );
                    "example_key".to_string()
                })
            }
        };

        // Build final map with the COMPLETE value example
        // For HashMap<String, Transform>, value_example is the full Transform
        let mut map = serde_json::Map::new();
        map.insert(key_str, value_example.clone());
        json!(map)
    }
}
