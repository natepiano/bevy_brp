//! Type discovery and format building logic
//!
//! This module handles the discovery of type formats and mutation paths
//! by combining registry schema information with hardcoded BRP knowledge.

use std::str::FromStr;

use serde_json::{Map, Value, json};
use tracing::debug;

use super::TypeKind;
use super::hardcoded_formats::BRP_FORMAT_KNOWLEDGE;
use super::registry_cache::REGISTRY_CACHE;
use super::types::{
    BrpFormatKnowledge, BrpSupportedOperation, BrpTypeName, CachedTypeInfo, MutationPath,
    ReflectTrait, SchemaField,
};
use super::wrapper_types::WrapperType;
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;
use crate::tool::BrpMethod;

// ===== Public Functions (Alphabetical) =====

/// Build enum spawn format from type schema
pub fn build_enum_spawn_format(type_schema: &Value) -> Value {
    if let Some(one_of) = type_schema
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
    {
        if let Some(first_variant) = one_of.first() {
            if let Some(variant_name) = first_variant
                .get_field(SchemaField::ShortPath)
                .and_then(Value::as_str)
            {
                // Check variant type to build appropriate spawn format
                if let Some(prefix_items) = first_variant
                    .get_field(SchemaField::PrefixItems)
                    .and_then(Value::as_array)
                {
                    // Tuple variant
                    if let Some(first_item) = prefix_items.first() {
                        if let Some(type_ref) = first_item
                            .get_field(SchemaField::Type)
                            .and_then(|t| t.get_field(SchemaField::Ref))
                            .and_then(Value::as_str)
                        {
                            let inner_type = type_ref.strip_prefix("#/$defs/").unwrap_or(type_ref);

                            let inner_value = if inner_type.contains("Srgba") {
                                json!({
                                    "red": 1.0,
                                    "green": 0.0,
                                    "blue": 0.0,
                                    "alpha": 1.0
                                })
                            } else {
                                json!({})
                            };

                            return json!({
                                variant_name: [inner_value]
                            });
                        }
                    }
                    return json!({ variant_name: [] });
                } else if first_variant.get_field(SchemaField::Properties).is_some() {
                    // Struct variant
                    return json!({ variant_name: {} });
                }
                // Unit variant
                return json!(variant_name);
            }
        }
    }
    json!({})
}

/// Build spawn format and mutation paths for a type
///
/// This function analyzes a type's registry schema and builds:
/// - A spawn format example for use with bevy/spawn operations
/// - Mutation paths for use with `bevy/mutate_component` operations
///
/// It combines hardcoded knowledge for known types with recursive discovery
/// for unknown types.
pub async fn build_spawn_format_and_mutation_paths(
    type_schema: &Value,
    type_name: &str,
    port: Port,
) -> (Map<String, Value>, Vec<MutationPath>) {
    let mut spawn_format = Map::new();
    let mut mutation_paths = Vec::new();

    let properties = type_schema
        .get_field(SchemaField::Properties)
        .and_then(Value::as_object);

    if let Some(props) = properties {
        // Build spawn format and mutation paths, discovering types as needed
        for (field_name, field_info) in props {
            process_field(
                field_name,
                field_info,
                port,
                &mut spawn_format,
                &mut mutation_paths,
            )
            .await;
        }
    } else {
        debug!(
            "No properties for {} (marker component or primitive type)",
            type_name
        );
    }

    debug!(
        "Generated {} mutation paths for {}",
        mutation_paths.len(),
        type_name
    );
    (spawn_format, mutation_paths)
}

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

/// Require that a type exists in the registry data
pub fn require_type_in_registry<'a>(
    type_name: &str,
    registry_data: &'a Value,
) -> Result<&'a Value> {
    // Try direct lookup first
    if let Some(type_obj) = registry_data.get(type_name) {
        return Ok(type_obj);
    }

    // If not found directly, search through all types
    if let Some(obj) = registry_data.as_object() {
        for (_key, value) in obj {
            if let Some(type_path) = value
                .get_field(SchemaField::TypePath)
                .and_then(Value::as_str)
            {
                if type_path == type_name {
                    return Ok(value);
                }
            }
        }
    }

    Err(
        crate::error::Error::BrpCommunication(format!("Type '{type_name}' not found in registry"))
            .into(),
    )
}

// ===== Async Functions (Alphabetical) =====

/// Discover nested type paths by fetching registry schema for unknown types
async fn discover_nested_type_paths(
    field_type: &str,
    field_name: &str,
    port: Port,
) -> Result<Vec<MutationPath>> {
    let mut nested_paths = Vec::new();

    // Check cache first
    let type_name: BrpTypeName = field_type.into();
    if let Some(cached_info) = REGISTRY_CACHE.get(&type_name) {
        debug!("Found {} in cache, using cached mutation paths", field_type);
        add_cached_paths_with_prefix(&cached_info, field_name, &mut nested_paths);
        return Ok(nested_paths);
    }

    // Not in cache, make a registry call for this specific type
    debug!("Making registry call for nested type: {}", field_type);

    match fetch_type_registry(field_type, port).await {
        Ok(registry_data) => {
            // Try to find this type in the response
            if let Ok(type_schema) = require_type_in_registry(field_type, &registry_data) {
                // Check the kind of type
                let type_kind = type_schema
                    .get_field(SchemaField::Kind)
                    .and_then(Value::as_str);

                let type_category = type_kind
                    .and_then(|s| TypeKind::from_str(s).ok())
                    .unwrap_or(TypeKind::Value);

                match type_category {
                    TypeKind::Struct => {
                        let struct_paths = process_struct_type(type_schema, field_name, type_name);

                        // Recursively discover paths for nested non-primitive fields
                        if let Some(props) = type_schema
                            .get_field(SchemaField::Properties)
                            .and_then(Value::as_object)
                        {
                            for (nested_field_name, nested_field_info) in props {
                                let nested_field_type = extract_field_type(nested_field_info);

                                if let Some(nft) = nested_field_type {
                                    if !is_primitive_type(nft) {
                                        let full_nested_name =
                                            format!("{field_name}.{nested_field_name}");
                                        if let Ok(deeper_paths) =
                                            Box::pin(discover_nested_type_paths(
                                                nft,
                                                &full_nested_name,
                                                port,
                                            ))
                                            .await
                                        {
                                            nested_paths.extend(deeper_paths);
                                        }
                                    }
                                }
                            }
                        }

                        nested_paths.extend(struct_paths);
                        debug!(
                            "Cached struct type {} with {} mutation paths",
                            field_type,
                            nested_paths.len()
                        );
                    }
                    TypeKind::Enum => {
                        process_enum_type(type_schema, field_type, type_name);
                    }
                    _ => {
                        debug!("Unknown type kind for {}: {:?}", field_type, type_kind);
                        // Cache with empty paths for unknown types
                        cache_type_info(
                            type_name,
                            type_schema,
                            vec![],
                            json!({}),
                            type_category,
                            None,
                        );
                    }
                }
            }
        }
        Err(e) => {
            debug!(
                "Failed to fetch registry for nested type {}: {}",
                field_type, e
            );
        }
    }

    Ok(nested_paths)
}

/// Fetch registry schema for a type
async fn fetch_type_registry(type_name: &str, port: Port) -> Result<Value> {
    let client = BrpClient::new(
        BrpMethod::BevyRegistrySchema,
        port,
        Some(json!({
            "with_types": [type_name]
        })),
    );

    match client.execute_raw().await {
        Ok(ResponseStatus::Success(Some(registry_data))) => Ok(registry_data),
        Ok(_) => Err(crate::error::Error::BrpCommunication(format!(
            "Registry call for {type_name} returned no data"
        ))
        .into()),
        Err(e) => Err(e),
    }
}

/// Try to get enum variants for a type
async fn get_enum_variants_for_type(type_name: &str, port: Port) -> Option<Vec<String>> {
    fetch_type_registry(type_name, port)
        .await
        .ok()
        .and_then(|registry_data| {
            require_type_in_registry(type_name, &registry_data)
                .ok()
                .filter(|type_schema| {
                    type_schema
                        .get_field(SchemaField::Kind)
                        .and_then(Value::as_str)
                        .and_then(|s| TypeKind::from_str(s).ok())
                        == Some(TypeKind::Enum)
                })
                .and_then(extract_enum_variants)
        })
}

/// Process a single field to generate spawn format and mutation paths
async fn process_field(
    field_name: &str,
    field_info: &Value,
    port: Port,
    spawn_format: &mut Map<String, Value>,
    mutation_paths: &mut Vec<MutationPath>,
) {
    let field_type = extract_field_type(field_info);
    let base_path = format!(".{field_name}");

    if let Some(ft) = field_type {
        // Check if this is a well-known wrapper type (Option, Handle, etc.)
        let (actual_type, wrapper_type) =
            if let Some((wrapper, inner_type)) = WrapperType::detect(ft) {
                (inner_type, Some(wrapper))
            } else {
                (ft, None)
            };

        if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&actual_type.into()) {
            // We have hardcoded knowledge for this type
            process_hardcoded_type(
                field_name,
                ft,
                base_path,
                wrapper_type,
                hardcoded,
                spawn_format,
                mutation_paths,
            );

            // Get enum variants if not a wrapper type
            if wrapper_type.is_none() {
                if let Some(variants) = get_enum_variants_for_type(ft, port).await {
                    // Update the last mutation path with enum variants
                    if let Some(last_path) = mutation_paths.last_mut() {
                        if last_path.path == format!(".{field_name}") {
                            last_path.enum_variants = Some(variants);
                        }
                    }
                }
            }
        } else if let Some(wrapper) = wrapper_type {
            // Wrapper type without hardcoded knowledge
            process_wrapper_type(
                field_name,
                ft,
                actual_type,
                base_path,
                wrapper,
                port,
                mutation_paths,
            )
            .await;
        } else {
            // Unknown type - try recursive discovery
            process_unknown_type(
                field_name,
                ft,
                base_path,
                port,
                spawn_format,
                mutation_paths,
            )
            .await;
        }
    } else {
        // No type info, but still generate base mutation path
        debug!(
            "No type info for field '{}' - generating base mutation path only",
            field_name
        );
        mutation_paths.push(MutationPath {
            path:          base_path,
            example:       json!(null),
            enum_variants: None,
            type_name:     None,
        });
    }
}

/// Process non-wrapped, non-hardcoded type
async fn process_unknown_type(
    field_name: &str,
    field_type: &str,
    base_path: String,
    port: Port,
    spawn_format: &mut Map<String, Value>,
    mutation_paths: &mut Vec<MutationPath>,
) {
    debug!(
        "Attempting recursive discovery for type {} in field '{}'",
        field_type, field_name
    );

    // Try recursive discovery for this type
    match Box::pin(discover_nested_type_paths(field_type, field_name, port)).await {
        Ok(discovered_paths) => {
            if discovered_paths.is_empty() {
                // Check cache to see if we discovered an enum or other type info
                let (example_value, enum_variants) =
                    get_cached_example_or_default(field_type, None);

                mutation_paths.push(MutationPath {
                    path: base_path.clone(),
                    example: example_value,
                    enum_variants,
                    type_name: Some(field_type.to_string()),
                });
            } else {
                debug!(
                    "Discovered {} nested paths for field '{}'",
                    discovered_paths.len(),
                    field_name
                );
                mutation_paths.extend(discovered_paths);
            }

            // Always check cache for spawn format after discovery
            let type_name_key: BrpTypeName = field_type.into();
            if let Some(cached_info) = REGISTRY_CACHE.get(&type_name_key) {
                if cached_info.type_kind == TypeKind::Enum {
                    spawn_format.insert(field_name.to_string(), cached_info.spawn_format);
                }
            }
        }
        Err(e) => {
            debug!("Failed to discover nested paths for {}: {}", field_type, e);
            // Check cache anyway in case some discovery happened before the error
            let (example_value, enum_variants) = get_cached_example_or_default(field_type, None);

            let type_name_key: BrpTypeName = field_type.into();
            if let Some(cached_info) = REGISTRY_CACHE.get(&type_name_key) {
                if cached_info.type_kind == TypeKind::Enum {
                    spawn_format.insert(field_name.to_string(), cached_info.spawn_format);
                }
            }

            mutation_paths.push(MutationPath {
                path: base_path.clone(),
                example: example_value,
                enum_variants,
                type_name: Some(field_type.to_string()),
            });
        }
    }

    // Check for special cases like Option<Vec2> that might have array access
    if field_type.starts_with("core::option::Option<") && field_type.contains("Vec") {
        // Add array-style mutation paths for optional vectors
        mutation_paths.push(MutationPath {
            path:          format!(".{field_name}[0]"),
            example:       json!(null),
            enum_variants: None,
            type_name:     None,
        });
        mutation_paths.push(MutationPath {
            path:          format!(".{field_name}[1]"),
            example:       json!(null),
            enum_variants: None,
            type_name:     None,
        });
    }
}

/// Process wrapper type without hardcoded knowledge
async fn process_wrapper_type(
    field_name: &str,
    field_type: &str,
    actual_type: &str,
    base_path: String,
    wrapper: WrapperType,
    port: Port,
    mutation_paths: &mut Vec<MutationPath>,
) {
    debug!(
        "Handling {} wrapper type {} - attempting recursive discovery for inner type",
        String::from(wrapper),
        field_type
    );

    // Try to discover the inner type recursively
    match Box::pin(discover_nested_type_paths(actual_type, field_name, port)).await {
        Ok(_) => {
            let (example_value, _) = get_cached_example_or_default(actual_type, Some(wrapper));

            // For Option wrapper, show both Some and None examples
            let final_example = if wrapper == WrapperType::Option {
                let type_name_key: BrpTypeName = actual_type.into();
                if let Some(cached_info) = REGISTRY_CACHE.get(&type_name_key) {
                    wrapper.mutation_examples(cached_info.spawn_format)
                } else {
                    wrapper.mutation_examples(json!(null))
                }
            } else {
                example_value
            };

            mutation_paths.push(MutationPath {
                path:          base_path,
                example:       final_example,
                enum_variants: None,
                type_name:     Some(field_type.to_string()),
            });
        }
        Err(e) => {
            debug!("Failed to discover inner type {}: {}", actual_type, e);
            let example_value = wrapper.default_example();
            let final_example = if wrapper == WrapperType::Option {
                wrapper.mutation_examples(json!(null))
            } else {
                example_value
            };

            mutation_paths.push(MutationPath {
                path:          base_path,
                example:       final_example,
                enum_variants: None,
                type_name:     Some(field_type.to_string()),
            });
        }
    }
}

// ===== Helper Functions (Alphabetical) =====

/// Add cached mutation paths with field name prefix
fn add_cached_paths_with_prefix(
    cached_info: &CachedTypeInfo,
    field_name: &str,
    nested_paths: &mut Vec<MutationPath>,
) {
    for path in &cached_info.mutation_paths {
        let nested_path = if path.path.starts_with('.') {
            format!(".{field_name}{}", path.path)
        } else {
            format!(".{field_name}.{}", path.path)
        };
        nested_paths.push(MutationPath {
            path:          nested_path,
            example:       path.example.clone(),
            enum_variants: path.enum_variants.clone(),
            type_name:     path.type_name.clone(),
        });
    }
}

/// Cache type information
fn cache_type_info(
    type_name: BrpTypeName,
    type_schema: &Value,
    mutation_paths: Vec<MutationPath>,
    spawn_format: Value,
    type_category: TypeKind,
    enum_variants: Option<Vec<String>>,
) {
    let reflect_types = extract_reflect_types(type_schema);
    let supported_operations = determine_supported_operations(&reflect_types);

    let cached_info = CachedTypeInfo {
        mutation_paths,
        registry_schema: type_schema.clone(),
        reflect_types,
        spawn_format,
        supported_operations,
        type_kind: type_category,
        enum_variants,
    };

    REGISTRY_CACHE.insert(type_name, cached_info);
}

/// Get enum variants from a type schema
fn extract_enum_variants(type_schema: &Value) -> Option<Vec<String>> {
    type_schema
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
        .map(|one_of| {
            one_of
                .iter()
                .filter_map(|v| v.get_field(SchemaField::ShortPath).and_then(Value::as_str))
                .map(std::string::ToString::to_string)
                .collect()
        })
}

/// Extract field type from field info
fn extract_field_type(field_info: &Value) -> Option<&str> {
    field_info
        .get_field(SchemaField::Type)
        .and_then(|t| t.get_field(SchemaField::Ref))
        .and_then(Value::as_str)
        .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"))
}

/// Get the example value from cache or create a default
fn get_cached_example_or_default(
    type_name: &str,
    wrapper: Option<WrapperType>,
) -> (Value, Option<Vec<String>>) {
    let type_name_key: BrpTypeName = type_name.into();
    if let Some(cached_info) = REGISTRY_CACHE.get(&type_name_key) {
        let example = if let Some(w) = wrapper {
            w.wrap_example(cached_info.spawn_format.clone())
        } else {
            cached_info.spawn_format
        };

        let variants = match cached_info.type_kind {
            TypeKind::Enum => cached_info.enum_variants,
            _ => None,
        };
        (example, variants)
    } else {
        let example = wrapper.map_or(json!(null), WrapperType::default_example);
        (example, None)
    }
}

/// Check if a type is a primitive type we should skip recursive discovery for
fn is_primitive_type(type_name: &str) -> bool {
    type_name.starts_with("core::")
        || type_name.starts_with("alloc::")
        || BRP_FORMAT_KNOWLEDGE.contains_key(&type_name.into())
}

/// Process enum type schema
fn process_enum_type(type_schema: &Value, field_type: &str, type_name: BrpTypeName) {
    if let Some(_one_of) = type_schema
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
    {
        let spawn_format = build_enum_spawn_format(type_schema);
        let variant_options = extract_enum_variants(type_schema).unwrap_or_default();

        debug!(
            "Found enum type {} with {} variants",
            field_type,
            variant_options.len()
        );

        // Cache enum info with variant information
        cache_type_info(
            type_name,
            type_schema,
            vec![], // Enums don't have nested mutation paths
            spawn_format,
            TypeKind::Enum,
            Some(variant_options),
        );
        debug!("Cached enum type {} with spawn format", field_type);
    }
}

/// Process hardcoded type with optional wrapper
fn process_hardcoded_type(
    field_name: &str,
    field_type: &str,
    base_path: String,
    wrapper_type: Option<WrapperType>,
    hardcoded: &BrpFormatKnowledge,
    spawn_format: &mut Map<String, Value>,
    mutation_paths: &mut Vec<MutationPath>,
) {
    // Build spawn format value
    let example_value = wrapper_type.map_or_else(
        || hardcoded.example_value.clone(),
        |wrapper| wrapper.wrap_example(hardcoded.example_value.clone()),
    );

    spawn_format.insert(field_name.to_string(), example_value.clone());
    debug!(
        "Added field '{}' from hardcoded knowledge{}",
        field_name,
        wrapper_type.map_or(String::new(), |w| format!(" ({} wrapper)", String::from(w)))
    );

    // Build mutation example
    let mutation_example = wrapper_type.map_or_else(
        || hardcoded.example_value.clone(),
        |wrapper| {
            if wrapper == WrapperType::Option {
                wrapper.mutation_examples(hardcoded.example_value.clone())
            } else {
                example_value
            }
        },
    );

    mutation_paths.push(MutationPath {
        path:          base_path,
        example:       mutation_example,
        enum_variants: None, // Will be populated later if needed
        type_name:     Some(field_type.to_string()),
    });

    // Generate component mutation paths if available (but NOT for wrapper types)
    if wrapper_type.is_none() {
        if let Some(component_paths) = &hardcoded.subfield_paths {
            for (component, example_value) in component_paths {
                let component_path = format!(".{field_name}.{}", String::from(*component));
                mutation_paths.push(MutationPath {
                    path:          component_path,
                    example:       example_value.clone(),
                    enum_variants: None,
                    type_name:     None,
                });
            }
        }
    }
}

/// Process struct type schema for mutation paths
fn process_struct_type(
    type_schema: &Value,
    field_name: &str,
    type_name: BrpTypeName,
) -> Vec<MutationPath> {
    let mut nested_paths = Vec::new();
    let mut cache_paths = Vec::new();

    if let Some(props) = type_schema
        .get_field(SchemaField::Properties)
        .and_then(Value::as_object)
    {
        // Build paths for immediate return with field_name prefix
        for (nested_field_name, nested_field_info) in props {
            let nested_path = format!(".{field_name}.{nested_field_name}");
            let nested_field_type = extract_field_type(nested_field_info);

            // Add base path for this nested field
            nested_paths.push(MutationPath {
                path:          nested_path,
                example:       json!(null),
                enum_variants: None,
                type_name:     nested_field_type.map(String::from),
            });

            // Build relative paths for caching
            cache_paths.push(MutationPath {
                path:          format!(".{nested_field_name}"),
                example:       json!(null),
                enum_variants: None,
                type_name:     None,
            });
        }

        // Cache this type for future use with relative paths
        cache_type_info(
            type_name,
            type_schema,
            cache_paths,
            json!({}),
            TypeKind::Struct,
            None,
        );
    }

    nested_paths
}
