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
use super::super::mutation_support::MutationSupport;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, SchemaField};
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

pub struct MapMutationBuilder;

impl MutationPathBuilder for MapMutationBuilder {
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
        tracing::warn!(
            "MapMutationBuilder::collect_children called for type: {}",
            ctx.type_name()
        );

        let Some(schema) = ctx.require_schema() else {
            tracing::warn!("No schema found for map type: {}", ctx.type_name());
            return vec![];
        };

        tracing::warn!(
            "Map schema found for type: {}, schema: {}",
            ctx.type_name(),
            schema
        );
        
        // Debug schema structure in detail
        tracing::warn!("Schema keys: {:?}", schema.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        if let Some(obj) = schema.as_object() {
            for (key, value) in obj {
                tracing::warn!("Schema field '{}': {}", key, value);
            }
        }

        // Debug: Check what SchemaField::KeyType actually produces
        tracing::warn!("SchemaField::KeyType as_ref: '{}'", SchemaField::KeyType.as_ref());
        
        // Extract key and value types from schema
        let key_type = schema
            .get_field(SchemaField::KeyType)
            .and_then(|key_field| {
                tracing::warn!("Found KeyType field: {}", key_field);
                key_field.get_field(SchemaField::Type)
            })
            .and_then(|type_field| {
                tracing::warn!("Found key Type field: {}", type_field);
                // Now get the $ref field from the type object
                type_field.get_field(SchemaField::Ref)
            })
            .and_then(|ref_value| {
                tracing::warn!("Found key $ref field: {}", ref_value);
                ref_value.as_str()
            })
            .and_then(|type_ref| {
                tracing::warn!("Key type ref string: {}", type_ref);
                type_ref.strip_prefix("#/$defs/")
            })
            .map(|type_name| {
                tracing::warn!("Extracted key type name: {}", type_name);
                BrpTypeName::from(type_name)
            });

        let value_type = schema
            .get_field(SchemaField::ValueType)
            .and_then(|value_field| {
                tracing::warn!("Found ValueType field: {}", value_field);
                value_field.get_field(SchemaField::Type)
            })
            .and_then(|type_field| {
                tracing::warn!("Found value Type field: {}", type_field);
                // Now get the $ref field from the type object
                type_field.get_field(SchemaField::Ref)
            })
            .and_then(|ref_value| {
                tracing::warn!("Found value $ref field: {}", ref_value);
                ref_value.as_str()
            })
            .and_then(|type_ref| {
                tracing::warn!("Value type ref string: {}", type_ref);
                type_ref.strip_prefix("#/$defs/")
            })
            .map(|type_name| {
                tracing::warn!("Extracted value type name: {}", type_name);
                BrpTypeName::from(type_name)
            });

        let mut children = vec![];

        if let Some(key_t) = key_type {
            tracing::info!("Creating context for key type: {}", key_t);
            // Create context for key recursion
            let key_path_kind = super::super::path_kind::PathKind::new_root_value(key_t);
            let key_ctx = ctx.create_field_context(key_path_kind);
            children.push(("key".to_string(), key_ctx));
        } else {
            tracing::warn!(
                "Failed to extract key type from schema for type: {}",
                ctx.type_name()
            );
        }

        if let Some(val_t) = value_type {
            tracing::info!("Creating context for value type: {}", val_t);
            // Create context for value recursion
            let val_path_kind = super::super::path_kind::PathKind::new_root_value(val_t);
            let val_ctx = ctx.create_field_context(val_path_kind);
            children.push(("value".to_string(), val_ctx));
        } else {
            tracing::warn!(
                "Failed to extract value type from schema for type: {}",
                ctx.type_name()
            );
        }

        tracing::info!(
            "MapMutationBuilder::collect_children returning {} children for type: {}",
            children.len(),
            ctx.type_name()
        );
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
        
        tracing::warn!(
            "MapMutationBuilder::assemble_from_children called for type: {}, received {} children",
            ctx.type_name(),
            children.len()
        );
        
        for (name, value) in &children {
            tracing::warn!("  Child '{}': {}", name, value);
        }

        let Some(key_example) = children.get("key") else {
            tracing::warn!(
                "Missing key example for map type {}, using fallback",
                ctx.type_name()
            );
            return json!({"example_key": "example_value"});
        };

        let Some(value_example) = children.get("value") else {
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
        map.insert(key_str.clone(), value_example.clone());
        let result = json!(map);
        
        tracing::warn!(
            "MapMutationBuilder assembled final map for {}: {}",
            ctx.type_name(),
            result
        );
        
        result
    }
}

impl MapMutationBuilder {
    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This map type cannot be mutated - {support}")
            }),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }
}
