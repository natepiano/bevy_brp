//! Utility functions for type discovery
//!
//! This module contains utility functions used by the V2 engine
//! for determining supported operations and extracting reflection traits.

use serde_json::Value;

use super::types::{BrpSupportedOperation, ReflectTrait, SchemaField};
use crate::string_traits::JsonFieldAccess;

/// Determine supported BRP operations based on reflection traits
pub fn determine_supported_operations(
    reflect_types: &[ReflectTrait],
) -> Vec<BrpSupportedOperation> {
    let mut operations = vec![BrpSupportedOperation::Query];

    let has_component = reflect_types.contains(&ReflectTrait::Component);
    let has_resource = reflect_types.contains(&ReflectTrait::Resource);
    let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
    let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

    if has_component {
        operations.push(BrpSupportedOperation::Get);
        if has_serialize && has_deserialize {
            operations.push(BrpSupportedOperation::Spawn);
            operations.push(BrpSupportedOperation::Insert);
        }
        if has_serialize {
            operations.push(BrpSupportedOperation::Mutate);
        }
    }

    if has_resource {
        if has_serialize && has_deserialize {
            operations.push(BrpSupportedOperation::Insert);
        }
        if has_serialize {
            operations.push(BrpSupportedOperation::Mutate);
        }
    }

    operations
}

/// Extract reflect types from a registry schema
pub fn extract_reflect_types(type_schema: &Value) -> Vec<ReflectTrait> {
    type_schema
        .get_field(SchemaField::ReflectTypes)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| s.parse::<ReflectTrait>().ok())
                .collect()
        })
        .unwrap_or_default()
}
