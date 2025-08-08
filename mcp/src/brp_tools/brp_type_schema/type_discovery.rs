//! Type discovery and format building logic
//!
//! This module handles the discovery of type formats and mutation paths
//! by combining registry schema information with hardcoded BRP knowledge.

use serde_json::{Map, Value, json};
use tracing::debug;

use super::TypeKind;
use super::hardcoded_formats::BRP_FORMAT_KNOWLEDGE;
use super::registry_cache::REGISTRY_CACHE;
use super::types::{BrpSupportedOperation, BrpTypeName, CachedTypeInfo, MutationPath};
use super::wrapper_types::WrapperType;
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::Result;
use crate::tool::BrpMethod;

/// Extract reflect types from a registry schema
pub fn extract_reflect_types(type_schema: &Value) -> Vec<String> {
    type_schema
        .get("reflectTypes")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

/// Determine supported BRP operations based on reflection traits
pub fn determine_supported_operations(reflect_types: &[String]) -> Vec<BrpSupportedOperation> {
    let mut operations = vec![BrpSupportedOperation::Query];

    let has_component = reflect_types.contains(&"Component".to_string());
    let has_resource = reflect_types.contains(&"Resource".to_string());
    let has_serialize = reflect_types.contains(&"Serialize".to_string());
    let has_deserialize = reflect_types.contains(&"Deserialize".to_string());

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

/// Build enum spawn format from type schema
pub fn build_enum_spawn_format(type_schema: &Value) -> Value {
    if let Some(one_of) = type_schema.get("oneOf").and_then(Value::as_array) {
        if let Some(first_variant) = one_of.first() {
            if let Some(variant_name) = first_variant.get("shortPath").and_then(Value::as_str) {
                // Check variant type to build appropriate spawn format
                if let Some(prefix_items) =
                    first_variant.get("prefixItems").and_then(Value::as_array)
                {
                    // Tuple variant
                    if let Some(first_item) = prefix_items.first() {
                        if let Some(type_ref) = first_item
                            .get("type")
                            .and_then(|t| t.get("$ref"))
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
                } else if first_variant.get("properties").is_some() {
                    // Struct variant
                    return json!({ variant_name: {} });
                } else {
                    // Unit variant
                    return json!(variant_name);
                }
            }
        }
    }
    json!({})
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
            if let Some(type_path) = value.get("typePath").and_then(Value::as_str) {
                if type_path == type_name {
                    return Ok(value);
                }
            }
        }
    }

    Err(crate::error::Error::BrpCommunication(format!(
        "Type '{}' not found in registry",
        type_name
    ))
    .into())
}

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
        // Add the cached type's mutation paths prefixed with our field name
        for path in &cached_info.mutation_paths {
            let nested_path = if path.path.starts_with('.') {
                format!(".{field_name}{}", path.path)
            } else {
                format!(".{field_name}.{}", path.path)
            };
            nested_paths.push(MutationPath {
                path:          nested_path,
                example_value: path.example_value.clone(),
                enum_variants: path.enum_variants.clone(),
                type_name:     path.type_name.clone(),
            });
        }
        return Ok(nested_paths);
    }

    // Not in cache, make a registry call for this specific type
    debug!("Making registry call for nested type: {}", field_type);

    let client = BrpClient::new(
        BrpMethod::BevyRegistrySchema,
        port,
        Some(json!({
            "with_types": [field_type]
        })),
    );

    match client.execute_raw().await {
        Ok(ResponseStatus::Success(Some(registry_data))) => {
            // Try to find this type in the response
            if let Ok(type_schema) = require_type_in_registry(field_type, &registry_data) {
                // Check the kind of type
                let type_kind = type_schema.get("kind").and_then(Value::as_str);

                match type_kind {
                    Some("Struct") => {
                        // Extract properties and generate mutation paths for structs
                        if let Some(props) =
                            type_schema.get("properties").and_then(Value::as_object)
                        {
                            // Build paths for immediate return with field_name prefix
                            for (nested_field_name, nested_field_info) in props {
                                let nested_path = format!(".{field_name}.{nested_field_name}");

                                // Check if this nested field has a known type for recursive
                                // discovery
                                let nested_field_type = nested_field_info
                                    .get("type")
                                    .and_then(|t| t.get("$ref"))
                                    .and_then(Value::as_str)
                                    .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"));

                                // Add base path for this nested field
                                nested_paths.push(MutationPath {
                                    path:          nested_path.clone(),
                                    example_value: json!(null),
                                    enum_variants: None,
                                    type_name:     nested_field_type.map(String::from),
                                });

                                // Recursively discover paths for nested structs
                                if let Some(nft) = nested_field_type {
                                    if !nft.starts_with("core::")
                                        && !nft.starts_with("alloc::")
                                        && !BRP_FORMAT_KNOWLEDGE.contains_key(&nft.into())
                                    {
                                        // Recursive call for deeper nesting
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

                            // Build relative paths for caching (without field_name prefix)
                            let mut cache_paths = Vec::new();
                            for (nested_field_name, _nested_field_info) in props {
                                cache_paths.push(MutationPath {
                                    path:          format!(".{nested_field_name}"),
                                    example_value: json!(null),
                                    enum_variants: None,
                                    type_name:     None,
                                });
                            }

                            // Cache this type for future use with relative paths
                            let reflect_types = extract_reflect_types(&type_schema);
                            let supported_operations =
                                determine_supported_operations(&reflect_types);

                            let cached_info = CachedTypeInfo {
                                mutation_paths: cache_paths, // Store relative paths in cache
                                registry_schema: type_schema.clone(),
                                reflect_types,
                                spawn_format: json!({}),
                                supported_operations,
                                type_category: TypeKind::Struct,
                                enum_variants: None,
                            };

                            REGISTRY_CACHE.insert(type_name, cached_info);
                            debug!(
                                "Cached struct type {} with {} mutation paths",
                                field_type,
                                nested_paths.len()
                            );
                        }
                    }
                    Some("Enum") => {
                        // For enums, we generate different mutation paths
                        // Enums in Bevy are serialized as { "variant_name": variant_data }
                        // For Color, you'd have paths like .color with examples showing the
                        // variants

                        // Get the first variant as the default example
                        if let Some(one_of) = type_schema.get("oneOf").and_then(Value::as_array) {
                            // Build spawn format from first variant
                            let spawn_format = build_enum_spawn_format(&type_schema);

                            // Create a list of all variant options for documentation
                            let variant_options: Vec<String> = one_of
                                .iter()
                                .filter_map(|v| v.get("shortPath").and_then(Value::as_str))
                                .map(|s| s.to_string())
                                .collect();

                            debug!(
                                "Found enum type {} with {} variants",
                                field_type,
                                variant_options.len()
                            );

                            // Cache enum info with variant information
                            let reflect_types = extract_reflect_types(&type_schema);
                            let supported_operations =
                                determine_supported_operations(&reflect_types);

                            let cached_info = CachedTypeInfo {
                                mutation_paths: vec![], // Enums don't have nested mutation paths
                                registry_schema: type_schema.clone(),
                                reflect_types,
                                spawn_format,
                                supported_operations,
                                type_category: TypeKind::Enum,
                                enum_variants: Some(variant_options),
                            };

                            REGISTRY_CACHE.insert(type_name, cached_info);
                            debug!("Cached enum type {} with spawn format", field_type);
                        }
                    }
                    _ => {
                        debug!("Unknown type kind for {}: {:?}", field_type, type_kind);
                        // Cache with empty paths for unknown types
                        let reflect_types = extract_reflect_types(&type_schema);
                        let supported_operations = determine_supported_operations(&reflect_types);
                        let type_category: TypeKind = type_schema
                            .get("kind")
                            .and_then(Value::as_str)
                            .map_or(TypeKind::Unknown, Into::into);

                        let cached_info = CachedTypeInfo {
                            mutation_paths: vec![],
                            registry_schema: type_schema.clone(),
                            reflect_types,
                            spawn_format: json!({}),
                            supported_operations,
                            type_category,
                            enum_variants: None,
                        };

                        REGISTRY_CACHE.insert(type_name, cached_info);
                    }
                }
            }
        }
        Ok(_) => {
            debug!("Registry call for {} returned no data", field_type);
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

/// Build spawn format and mutation paths for a type
///
/// This function analyzes a type's registry schema and builds:
/// - A spawn format example for use with bevy/spawn operations
/// - Mutation paths for use with bevy/mutate_component operations
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

    let properties = type_schema.get("properties").and_then(Value::as_object);

    if let Some(props) = properties {
        // Build spawn format and mutation paths, discovering types as needed
        for (field_name, field_info) in props {
            // Extract type from {"type": {"$ref": "#/$defs/glam::Vec3"}} structure
            let field_type = field_info
                .get("type")
                .and_then(|t| t.get("$ref"))
                .and_then(Value::as_str)
                .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"));

            // Always generate base field mutation path for every field
            let base_path = format!(".{field_name}");

            match field_type {
                Some(ft) => {
                    // Check if this is a well-known wrapper type (Option, Handle, etc.)
                    let (actual_type, wrapper_type) =
                        if let Some((wrapper, inner_type)) = WrapperType::detect(ft) {
                            (inner_type, Some(wrapper))
                        } else {
                            (ft, None)
                        };

                    if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&actual_type.into()) {
                        // We have hardcoded knowledge for this type (or its inner type)
                        let example_value = if let Some(wrapper) = wrapper_type {
                            wrapper.wrap_example(hardcoded.example_value.clone())
                        } else {
                            hardcoded.example_value.clone()
                        };

                        spawn_format.insert(field_name.clone(), example_value.clone());
                        debug!(
                            "Added field '{}' from hardcoded knowledge{}",
                            field_name,
                            wrapper_type
                                .map_or("".to_string(), |w| format!(" ({} wrapper)", w.as_ref()))
                        );

                        // For wrapper types (Option, Handle), we don't want to show enum variants
                        // as they're trivial For other enums, try to
                        // discover enum variants
                        let enum_variants = if wrapper_type.is_some() {
                            None
                        } else {
                            // Make a quick registry call to get enum variants
                            let client = BrpClient::new(
                                BrpMethod::BevyRegistrySchema,
                                port,
                                Some(json!({
                                    "with_types": [ft]
                                })),
                            );

                            match client.execute_raw().await {
                                Ok(ResponseStatus::Success(Some(registry_data))) => {
                                    if let Ok(type_schema) =
                                        require_type_in_registry(ft, &registry_data)
                                    {
                                        if type_schema.get("kind").and_then(Value::as_str)
                                            == Some("Enum")
                                        {
                                            // Extract enum variants
                                            if let Some(one_of) =
                                                type_schema.get("oneOf").and_then(Value::as_array)
                                            {
                                                Some(
                                                    one_of
                                                        .iter()
                                                        .filter_map(|v| {
                                                            v.get("shortPath")
                                                                .and_then(Value::as_str)
                                                        })
                                                        .map(|s| s.to_string())
                                                        .collect(),
                                                )
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        };

                        // For Option wrapper types in mutation paths, show the unwrapped value
                        // since mutations accept raw values for Some and null for None
                        let mutation_example = if let Some(wrapper) = wrapper_type {
                            if wrapper == WrapperType::Option {
                                // For Option mutation paths, use the mutation examples format
                                // which shows both Some and None examples
                                // Use the actual inner value, not the wrapped spawn format
                                wrapper.mutation_examples(hardcoded.example_value.clone())
                            } else {
                                // Handle wrapper still uses wrapped format in mutations
                                example_value.clone()
                            }
                        } else {
                            // Not a wrapper type
                            hardcoded.example_value.clone()
                        };

                        mutation_paths.push(MutationPath {
                            path: base_path,
                            example_value: mutation_example,
                            enum_variants,
                            type_name: Some(ft.to_string()),
                        });

                        // Generate component mutation paths if available (but NOT for wrapper
                        // types)
                        if wrapper_type.is_none() {
                            if let Some(component_paths) = &hardcoded.subfield_paths {
                                for (component_name, example_value) in component_paths {
                                    let component_path = format!(".{field_name}.{component_name}");

                                    mutation_paths.push(MutationPath {
                                        path:          component_path,
                                        example_value: example_value.clone(),
                                        enum_variants: None,
                                        type_name:     None,
                                    });
                                }
                            }
                        }
                    } else {
                        // No hardcoded knowledge - check if it's a wrapper type we should handle
                        // specially
                        if let Some(wrapper) = wrapper_type {
                            // For wrapper types without hardcoded inner type, try recursive
                            // discovery
                            debug!(
                                "Handling {} wrapper type {} - attempting recursive discovery for inner type",
                                wrapper.as_ref(),
                                ft
                            );

                            // Try to discover the inner type recursively
                            match Box::pin(discover_nested_type_paths(
                                actual_type,
                                field_name,
                                port,
                            ))
                            .await
                            {
                                Ok(_) => {
                                    // Check if we cached the inner type during discovery
                                    let type_name_key: BrpTypeName = actual_type.into();
                                    let example_value = if let Some(cached_info) =
                                        REGISTRY_CACHE.get(&type_name_key)
                                    {
                                        // Use the discovered spawn format for the inner type
                                        wrapper.wrap_example(cached_info.spawn_format.clone())
                                    } else {
                                        // Fallback to simple defaults
                                        wrapper.default_example()
                                    };

                                    // For Option wrapper, show both Some and None examples
                                    let final_example = if wrapper == WrapperType::Option {
                                        // Get the inner example and create mutation examples
                                        if let Some(cached_info) =
                                            REGISTRY_CACHE.get(&type_name_key)
                                        {
                                            wrapper
                                                .mutation_examples(cached_info.spawn_format.clone())
                                        } else {
                                            wrapper.mutation_examples(json!(null))
                                        }
                                    } else {
                                        // Handle wrapper uses regular wrapped format
                                        example_value
                                    };

                                    mutation_paths.push(MutationPath {
                                        path:          base_path,
                                        example_value: final_example,
                                        enum_variants: None, /* Don't show variants for wrapper
                                                              * types */
                                        type_name:     Some(ft.to_string()),
                                    });
                                }
                                Err(e) => {
                                    debug!("Failed to discover inner type {}: {}", actual_type, e);
                                    // Fallback to simple defaults
                                    let example_value = wrapper.default_example();

                                    // For Option wrapper, show both Some and None examples
                                    let final_example = if wrapper == WrapperType::Option {
                                        wrapper.mutation_examples(json!(null))
                                    } else {
                                        example_value
                                    };

                                    mutation_paths.push(MutationPath {
                                        path:          base_path,
                                        example_value: final_example,
                                        enum_variants: None,
                                        type_name:     Some(ft.to_string()),
                                    });
                                }
                            }
                        } else {
                            // Not an Option - try recursive discovery
                            debug!(
                                "Attempting recursive discovery for type {} in field '{}'",
                                ft, field_name
                            );

                            // Try recursive discovery for this type
                            match Box::pin(discover_nested_type_paths(ft, field_name, port)).await {
                                Ok(discovered_paths) => {
                                    if !discovered_paths.is_empty() {
                                        debug!(
                                            "Discovered {} nested paths for field '{}'",
                                            discovered_paths.len(),
                                            field_name
                                        );
                                        // Add all discovered paths
                                        mutation_paths.extend(discovered_paths);
                                    } else {
                                        // Check cache to see if we discovered an enum or other type
                                        // info
                                        let type_name_key: BrpTypeName = ft.into();
                                        if let Some(cached_info) =
                                            REGISTRY_CACHE.get(&type_name_key)
                                        {
                                            let (example_value, enum_variants) =
                                                match cached_info.type_category {
                                                    TypeKind::Enum => {
                                                        // Use spawn format as example for enums
                                                        (
                                                            cached_info.spawn_format.clone(),
                                                            cached_info.enum_variants.clone(),
                                                        )
                                                    }
                                                    _ => (json!(null), None),
                                                };

                                            mutation_paths.push(MutationPath {
                                                path: base_path.clone(),
                                                example_value,
                                                enum_variants,
                                                type_name: Some(ft.to_string()),
                                            });
                                        } else {
                                            // No cache info, add base path with null
                                            debug!(
                                                "No nested paths discovered for {}, adding base path only",
                                                ft
                                            );
                                            mutation_paths.push(MutationPath {
                                                path:          base_path.clone(),
                                                example_value: json!(null),
                                                enum_variants: None,
                                                type_name:     Some(ft.to_string()),
                                            });
                                        }
                                    }

                                    // Always check cache for spawn format after discovery
                                    let type_name_key: BrpTypeName = ft.into();
                                    if let Some(cached_info) = REGISTRY_CACHE.get(&type_name_key) {
                                        if cached_info.type_category == TypeKind::Enum {
                                            spawn_format.insert(
                                                field_name.clone(),
                                                cached_info.spawn_format.clone(),
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Failed to discover nested paths for {}: {}", ft, e);
                                    // Check cache anyway in case some discovery happened before the
                                    // error
                                    let type_name_key: BrpTypeName = ft.into();
                                    if let Some(cached_info) = REGISTRY_CACHE.get(&type_name_key) {
                                        let (example_value, enum_variants) =
                                            match cached_info.type_category {
                                                TypeKind::Enum => {
                                                    spawn_format.insert(
                                                        field_name.clone(),
                                                        cached_info.spawn_format.clone(),
                                                    );
                                                    (
                                                        cached_info.spawn_format.clone(),
                                                        cached_info.enum_variants.clone(),
                                                    )
                                                }
                                                _ => (json!(null), None),
                                            };

                                        mutation_paths.push(MutationPath {
                                            path: base_path.clone(),
                                            example_value,
                                            enum_variants,
                                            type_name: Some(ft.to_string()),
                                        });
                                    } else {
                                        // Fallback to base path only
                                        mutation_paths.push(MutationPath {
                                            path:          base_path.clone(),
                                            example_value: json!(null),
                                            enum_variants: None,
                                            type_name:     Some(ft.to_string()),
                                        });
                                    }
                                }
                            }

                            // Check for special cases like Option<Vec2> that might have array
                            // access
                            if ft.starts_with("core::option::Option<") && ft.contains("Vec") {
                                // Add array-style mutation paths for optional vectors
                                mutation_paths.push(MutationPath {
                                    path:          format!(".{field_name}[0]"),
                                    example_value: json!(null),
                                    enum_variants: None,
                                    type_name:     None,
                                });
                                mutation_paths.push(MutationPath {
                                    path:          format!(".{field_name}[1]"),
                                    example_value: json!(null),
                                    enum_variants: None,
                                    type_name:     None,
                                });
                            }
                        }
                    } // End of is_option else block
                }
                None => {
                    // No type info, but still generate base mutation path
                    debug!(
                        "No type info for field '{}' in '{}' - generating base mutation path only",
                        field_name, type_name
                    );
                    mutation_paths.push(MutationPath {
                        path:          base_path,
                        example_value: json!(null),
                        enum_variants: None,
                        type_name:     None,
                    });
                }
            }
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
