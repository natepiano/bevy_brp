//! Schema adapters for the unified type system
//!
//! Converts different schemas to `UnifiedTypeInfo` without data loss:
//! - `TypeDiscoveryResponse` → `UnifiedTypeInfo`
//! - Registry schema → `UnifiedTypeInfo`

use std::collections::HashMap;

use serde_json::Value;

use super::unified_types::{
    DiscoverySource, EnumInfo, EnumVariant, FormatInfo, RegistryStatus, SerializationSupport,
    TypeCategory, UnifiedTypeInfo,
};

/// Parse a type category string to the corresponding enum variant
fn parse_type_category(category_str: &str) -> TypeCategory {
    match category_str {
        "Struct" => TypeCategory::Struct,
        "TupleStruct" => TypeCategory::TupleStruct,
        "Enum" => TypeCategory::Enum,
        "MathType" => TypeCategory::MathType,
        "Component" => TypeCategory::Component,
        _ => TypeCategory::Unknown,
    }
}

/// Convert `TypeDiscoveryResponse` JSON to `UnifiedTypeInfo`
///
/// Preserves all fields including `mutation_paths` and metadata.
/// Note: Uses JSON until extras crate types are directly accessible.
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
        .map_or(TypeCategory::Unknown, parse_type_category);

    // Extract child types
    let mut child_types = HashMap::new();
    if let Some(children) = obj.get("child_types").and_then(Value::as_object) {
        for (name, type_path) in children {
            if let Some(path_str) = type_path.as_str() {
                child_types.insert(name.clone(), path_str.to_string());
            }
        }
    }

    // Extract enum info and convert to proper structure
    let enum_info = obj
        .get("enum_info")
        .and_then(convert_enum_info_from_discovery);

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

/// Convert `enum_info` from type discovery response to `EnumInfo` structure
fn convert_enum_info_from_discovery(enum_obj: &Value) -> Option<EnumInfo> {
    enum_obj
        .get("variants")
        .and_then(Value::as_array)
        .map(|variants_array| {
            let variants = variants_array
                .iter()
                .filter_map(|variant| {
                    if let Some(variant_obj) = variant.as_object() {
                        let name = variant_obj.get("name")?.as_str()?.to_string();
                        let variant_type = variant_obj
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("Unit")
                            .to_string();
                        Some(EnumVariant { name, variant_type })
                    } else {
                        None
                    }
                })
                .collect();

            EnumInfo { variants }
        })
}

/// Generate mutation paths from registry schema structure
fn generate_mutation_paths_from_schema(schema_data: &Value) -> HashMap<String, String> {
    let mut paths = HashMap::new();

    // Get the type kind
    let kind = schema_data
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");

    match kind {
        "TupleStruct" => {
            // For tuple structs, generate paths based on prefixItems
            if let Some(prefix_items) = schema_data.get("prefixItems").and_then(Value::as_array) {
                for (index, item) in prefix_items.iter().enumerate() {
                    // Basic tuple access path
                    paths.insert(
                        format!(".{index}"),
                        format!("Access field {index} of the tuple struct"),
                    );

                    // Check if this field is a Color type
                    if let Some(type_ref) = item
                        .get("type")
                        .and_then(|t| t.get("$ref"))
                        .and_then(Value::as_str)
                    {
                        if type_ref.contains("Color") {
                            // Add common color field paths
                            paths.insert(
                                format!(".{index}.red"),
                                "Access the red component (if Color is an enum with named fields)"
                                    .to_string(),
                            );
                            paths.insert(
                                format!(".{index}.green"),
                                "Access the green component (if Color is an enum with named fields)".to_string()
                            );
                            paths.insert(
                                format!(".{index}.blue"),
                                "Access the blue component (if Color is an enum with named fields)"
                                    .to_string(),
                            );
                            paths.insert(
                                format!(".{index}.alpha"),
                                "Access the alpha component (if Color is an enum with named fields)".to_string()
                            );

                            // Also add potential enum variant access
                            paths.insert(
                                format!(".{index}.0"),
                                "Access the first field if Color is an enum variant".to_string(),
                            );
                        }
                    }
                }
            }
        }
        "Struct" => {
            // For regular structs, use property names
            if let Some(properties) = schema_data.get("properties").and_then(Value::as_object) {
                for (field_name, _field_type) in properties {
                    paths.insert(
                        format!(".{field_name}"),
                        format!("Access the '{field_name}' field"),
                    );
                }
            }
        }
        _ => {
            // For other types (enums, values), mutation typically replaces the whole value
            if kind == "Enum" {
                paths.insert(
                    String::new(),
                    "Replace the entire enum value (use empty path)".to_string(),
                );
            }
        }
    }

    paths
}

/// Extract enum variant information from registry schema
fn extract_enum_info_from_schema(schema_data: &Value) -> Option<EnumInfo> {
    // Look for the "oneOf" field which contains enum variants
    schema_data
        .get("oneOf")
        .and_then(Value::as_array)
        .and_then(|one_of| {
            let variants: Vec<EnumVariant> = one_of
                .iter()
                .filter_map(|variant| {
                    match variant {
                        // Simple string variant (unit variants)
                        Value::String(variant_name) => Some(EnumVariant {
                            name:         variant_name.clone(),
                            variant_type: "Unit".to_string(),
                        }),
                        // Object variant (struct or tuple variants)
                        Value::Object(variant_obj) => {
                            variant_obj
                                .get("shortPath")
                                .and_then(Value::as_str)
                                .map(|short_path| EnumVariant {
                                    name:         short_path.to_string(),
                                    variant_type: "Unit".to_string(), /* Most registry enums are
                                                                       * unit variants */
                                })
                        }
                        _ => None,
                    }
                })
                .collect();

            if variants.is_empty() {
                None
            } else {
                Some(EnumInfo { variants })
            }
        })
}

/// Convert Bevy registry schema to `UnifiedTypeInfo`
///
/// Extracts registry status, reflection traits, and serialization support.
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
        .map_or(TypeCategory::Unknown, parse_type_category);

    // Extract basic structure information for supported operations
    let supported_operations = if serialization.brp_compatible {
        match type_category {
            TypeCategory::Struct | TypeCategory::TupleStruct => vec![
                "query".to_string(),
                "get".to_string(),
                "spawn".to_string(),
                "insert".to_string(),
                "mutate".to_string(),
            ],
            TypeCategory::Enum => vec![
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

    // Extract enum information if this is an enum
    let enum_info = if type_category == TypeCategory::Enum {
        extract_enum_info_from_schema(schema_data)
    } else {
        None
    };

    // Generate mutation paths based on schema structure
    let mutation_paths = generate_mutation_paths_from_schema(schema_data);

    UnifiedTypeInfo {
        type_name: type_name.to_string(),
        registry_status,
        serialization,
        format_info: FormatInfo {
            examples: HashMap::new(),
            mutation_paths,
            original_format: None,
            corrected_format: None,
        },
        supported_operations,
        type_category,
        child_types: HashMap::new(),
        enum_info,
        discovery_source: DiscoverySource::TypeRegistry,
    }
}
