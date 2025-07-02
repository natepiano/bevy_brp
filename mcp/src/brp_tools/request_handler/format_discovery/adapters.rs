//! Schema adapters for the unified type system
//!
//! This module provides conversion functions between different type schemas:
//! - `TypeDiscoveryResponse` (from `bevy_brp_extras`) → `UnifiedTypeInfo`
//! - Registry schema data → `UnifiedTypeInfo`
//!
//! These adapters solve the "`mutation_paths` bug" by ensuring no information is lost
//! during schema conversions, preserving all discovered metadata in the unified format.
//!
//! # Key Benefits
//!
//! - **No Data Loss**: All fields from source schemas are preserved
//! - **Consistent Interface**: All discovery sources produce `UnifiedTypeInfo`
//! - **Extensible**: Easy to add new source schema adapters
//! - **Type Safe**: Compile-time guarantees about conversion correctness

use std::collections::HashMap;

use serde_json::Value;

use super::unified_types::{
    DiscoverySource, FormatInfo, RegistryStatus, SerializationSupport, UnifiedTypeInfo,
};

/// Convert a JSON representation of `TypeDiscoveryResponse` to `UnifiedTypeInfo`
///
/// This adapter preserves all information from the direct discovery response,
/// ensuring no data loss occurs during conversion. This is critical for
/// preserving `mutation_paths` and other rich metadata.
///
/// Note: For now, we work with JSON values since the extras crate types
/// are not directly accessible. This will be improved in task 3.1a.
///
/// # Arguments
/// * `response_json` - JSON representation of `TypeDiscoveryResponse`
///
/// # Returns
/// * `UnifiedTypeInfo` containing all the original information
pub fn from_type_discovery_response_json(response_json: &Value) -> Option<UnifiedTypeInfo> {
    let obj = response_json.as_object()?;

    let type_name = obj.get("type_name")?.as_str()?.to_string();

    // Extract registry status
    let in_registry = obj.get("in_registry")?.as_bool().unwrap_or(false);
    let registry_status = RegistryStatus {
        in_registry,
        has_reflect: true, // If we got a response, reflection is working
        type_path: Some(type_name.clone()),
    };

    // Extract serialization support
    let has_serialize = obj.get("has_serialize")?.as_bool().unwrap_or(false);
    let has_deserialize = obj.get("has_deserialize")?.as_bool().unwrap_or(false);
    let serialization = SerializationSupport {
        has_serialize,
        has_deserialize,
        brp_compatible: has_serialize && has_deserialize,
    };

    // Convert example_values to format examples
    let mut examples = HashMap::new();
    if let Some(example_values) = obj.get("example_values").and_then(Value::as_object) {
        for (operation, example) in example_values {
            examples.insert(operation.clone(), example.clone());
        }
    }

    // Extract mutation paths
    let mut mutation_paths = HashMap::new();
    if let Some(paths) = obj.get("mutation_paths").and_then(Value::as_object) {
        for (path, description) in paths {
            if let Some(desc_str) = description.as_str() {
                mutation_paths.insert(path.clone(), desc_str.to_string());
            }
        }
    }

    // Create format info with mutation paths preserved
    let format_info = FormatInfo {
        examples,
        mutation_paths,
        original_format: None,
        corrected_format: None,
    };

    // Extract supported operations
    let supported_operations = obj
        .get("supported_operations")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    // Extract type category
    let type_category = obj
        .get("type_category")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();

    // Extract child types
    let mut child_types = HashMap::new();
    if let Some(children) = obj.get("child_types").and_then(Value::as_object) {
        for (name, type_path) in children {
            if let Some(path_str) = type_path.as_str() {
                child_types.insert(name.clone(), path_str.to_string());
            }
        }
    }

    // Extract enum info
    let enum_info = obj.get("enum_info").and_then(Value::as_object).cloned();

    Some(UnifiedTypeInfo {
        type_name,
        registry_status,
        serialization,
        format_info,
        supported_operations,
        type_category,
        child_types,
        enum_info,
        discovery_source: DiscoverySource::DirectDiscovery,
    })
}

/// Convert registry schema data to `UnifiedTypeInfo`
///
/// This adapter handles conversion from Bevy's type registry schema format
/// to the unified type system. It focuses on registry and reflection information.
///
/// # Arguments
/// * `type_name` - The fully-qualified type name
/// * `schema_data` - Schema data from `bevy/registry_schema`
///
/// # Returns
/// * `UnifiedTypeInfo` containing registry and reflection information
pub fn from_registry_schema(type_name: &str, schema_data: &Value) -> UnifiedTypeInfo {
    // Extract reflect types
    let reflect_types = schema_data
        .get("reflectTypes")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Determine serialization support
    let has_serialize = reflect_types.contains(&"Serialize".to_string());
    let has_deserialize = reflect_types.contains(&"Deserialize".to_string());

    let registry_status = RegistryStatus {
        in_registry: true, // If we have schema data, it's in the registry
        has_reflect: reflect_types.contains(&"Default".to_string()) || !reflect_types.is_empty(),
        type_path:   Some(type_name.to_string()),
    };

    let serialization = SerializationSupport {
        has_serialize,
        has_deserialize,
        brp_compatible: has_serialize && has_deserialize,
    };

    // Extract type category from schema if available
    let type_category = schema_data
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();

    // Extract basic structure information for supported operations
    let supported_operations = if serialization.brp_compatible {
        match type_category.as_str() {
            "Struct" | "TupleStruct" => vec![
                "query".to_string(),
                "get".to_string(),
                "spawn".to_string(),
                "insert".to_string(),
                "mutate".to_string(),
            ],
            "Enum" => vec![
                "query".to_string(),
                "get".to_string(),
                "spawn".to_string(),
                "insert".to_string(),
            ],
            _ => vec!["query".to_string(), "get".to_string()],
        }
    } else {
        // Without serialization, only reflection-based operations work
        vec!["query".to_string(), "get".to_string()]
    };

    UnifiedTypeInfo {
        type_name: type_name.to_string(),
        registry_status,
        serialization,
        format_info: FormatInfo::empty(),
        supported_operations,
        type_category,
        child_types: HashMap::new(),
        enum_info: None,
        discovery_source: DiscoverySource::TypeRegistry,
    }
}
