//! Type discovery and format building logic
//!
//! This module handles the discovery of type formats and mutation paths
//! by combining registry schema information with hardcoded BRP knowledge.

use std::collections::HashMap;

use serde_json::{Map, Value, json};
use tracing::debug;

use super::TypeCategory;
use super::hardcoded_formats::BRP_FORMAT_KNOWLEDGE;
use super::registry_cache::REGISTRY_CACHE;
use super::types::{BrpSupportedOperation, BrpTypeName, CachedTypeInfo, MutationPath};
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

/// Batch discover types by crate to minimize registry calls
pub async fn batch_discover_types_by_crate(
    types_to_discover: Vec<String>,
    port: Port,
) -> Result<()> {
    // Group types by crate
    let mut types_by_crate: HashMap<String, Vec<String>> = HashMap::new();

    for type_name in types_to_discover {
        // Skip if already in cache
        if REGISTRY_CACHE.get(&type_name.as_str().into()).is_some() {
            continue;
        }

        // Extract crate name (first part before ::)
        let crate_name = type_name
            .split("::")
            .next()
            .unwrap_or(&type_name)
            .to_string();
        types_by_crate
            .entry(crate_name)
            .or_insert_with(Vec::new)
            .push(type_name);
    }

    // Make one registry call per crate
    for (crate_name, type_names) in types_by_crate {
        debug!(
            "Batching {} types from crate {}",
            type_names.len(),
            crate_name
        );

        // Use with_crates instead of with_types for better efficiency
        let client = BrpClient::new(
            BrpMethod::BevyRegistrySchema,
            port,
            Some(json!({
                "with_crates": [crate_name]
            })),
        );

        match client.execute_raw().await {
            Ok(ResponseStatus::Success(Some(registry_data))) => {
                // Cache all types we were looking for from this crate
                for type_name in &type_names {
                    if let Ok(type_schema) = require_type_in_registry(type_name, &registry_data) {
                        // Process and cache the type based on its kind
                        let type_kind = type_schema.get("kind").and_then(Value::as_str);
                        let type_name_key: BrpTypeName = type_name.as_str().into();

                        match type_kind {
                            Some("Struct") => {
                                // Process struct type
                                let mut cache_paths = Vec::new();
                                if let Some(props) =
                                    type_schema.get("properties").and_then(Value::as_object)
                                {
                                    for (field_name, _) in props {
                                        cache_paths.push(MutationPath {
                                            path:          format!(".{field_name}"),
                                            example_value: json!(null),
                                            enum_variants: None,
                                            type_name:     None,
                                        });
                                    }
                                }

                                let reflect_types = extract_reflect_types(&type_schema);
                                let supported_operations =
                                    determine_supported_operations(&reflect_types);

                                let cached_info = CachedTypeInfo {
                                    mutation_paths: cache_paths,
                                    registry_schema: type_schema.clone(),
                                    reflect_types,
                                    spawn_format: json!({}),
                                    supported_operations,
                                    type_category: TypeCategory::Struct,
                                    enum_variants: None,
                                };

                                REGISTRY_CACHE.insert(type_name_key, cached_info);
                                debug!("Cached struct {} from batch", type_name);
                            }
                            Some("Enum") => {
                                // Process enum type
                                let spawn_format = build_enum_spawn_format(&type_schema);
                                let reflect_types = extract_reflect_types(&type_schema);
                                let supported_operations =
                                    determine_supported_operations(&reflect_types);

                                // Extract variant names
                                let enum_variants = if let Some(one_of) =
                                    type_schema.get("oneOf").and_then(Value::as_array)
                                {
                                    Some(
                                        one_of
                                            .iter()
                                            .filter_map(|v| {
                                                v.get("shortPath").and_then(Value::as_str)
                                            })
                                            .map(|s| s.to_string())
                                            .collect(),
                                    )
                                } else {
                                    None
                                };

                                let cached_info = CachedTypeInfo {
                                    mutation_paths: vec![], /* Enums don't have nested mutation
                                                             * paths */
                                    registry_schema: type_schema.clone(),
                                    reflect_types,
                                    spawn_format,
                                    supported_operations,
                                    type_category: TypeCategory::Enum,
                                    enum_variants,
                                };

                                REGISTRY_CACHE.insert(type_name_key, cached_info);
                                debug!("Cached enum {} from batch", type_name);
                            }
                            _ => {
                                // Cache unknown types with empty data
                                let reflect_types = extract_reflect_types(&type_schema);
                                let supported_operations =
                                    determine_supported_operations(&reflect_types);
                                let type_category: TypeCategory = type_schema
                                    .get("kind")
                                    .and_then(Value::as_str)
                                    .map_or(TypeCategory::Unknown, Into::into);

                                let cached_info = CachedTypeInfo {
                                    mutation_paths: vec![],
                                    registry_schema: type_schema.clone(),
                                    reflect_types,
                                    spawn_format: json!({}),
                                    supported_operations,
                                    type_category,
                                    enum_variants: None,
                                };

                                REGISTRY_CACHE.insert(type_name_key, cached_info);
                                debug!(
                                    "Cached {} type {} from batch",
                                    type_kind.unwrap_or("unknown"),
                                    type_name
                                );
                            }
                        }
                    }
                }
            }
            Ok(_) => {
                debug!("Registry call for crate {} returned no data", crate_name);
            }
            Err(e) => {
                debug!("Failed to fetch registry for crate {}: {}", crate_name, e);
            }
        }
    }

    Ok(())
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
    let mut types_to_discover = Vec::new();

    let properties = type_schema.get("properties").and_then(Value::as_object);

    if let Some(props) = properties {
        // First pass: collect all types we need to discover
        for (_field_name, field_info) in props {
            let field_type = field_info
                .get("type")
                .and_then(|t| t.get("$ref"))
                .and_then(Value::as_str)
                .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"));

            if let Some(ft) = field_type {
                if BRP_FORMAT_KNOWLEDGE.get(&ft.into()).is_none() {
                    // No hardcoded knowledge - add to discovery list
                    let should_discover = !ft.starts_with("core::")
                        && !ft.starts_with("alloc::")
                        && !matches!(
                            ft,
                            "bool"
                                | "u8"
                                | "u16"
                                | "u32"
                                | "u64"
                                | "i8"
                                | "i16"
                                | "i32"
                                | "i64"
                                | "f32"
                                | "f64"
                                | "usize"
                                | "isize"
                        );

                    if should_discover {
                        types_to_discover.push(ft.to_string());
                    }
                }
            }
        }

        // Batch discover all types we need
        if !types_to_discover.is_empty() {
            debug!("Batch discovering {} types", types_to_discover.len());
            let _ = batch_discover_types_by_crate(types_to_discover, port).await;
        }

        // Second pass: build spawn format and mutation paths with discovered types
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
                    if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&ft.into()) {
                        // We have hardcoded knowledge for this type
                        spawn_format.insert(field_name.clone(), hardcoded.example_value.clone());
                        debug!("Added field '{}' from hardcoded knowledge", field_name);

                        mutation_paths.push(MutationPath {
                            path:          base_path,
                            example_value: hardcoded.example_value.clone(),
                            enum_variants: None,
                            type_name:     Some(ft.to_string()),
                        });

                        // Generate component mutation paths if available
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
                    } else {
                        // Check if type is now in cache from batch discovery
                        let type_name_key: BrpTypeName = ft.into();

                        if let Some(cached_info) = REGISTRY_CACHE.get(&type_name_key) {
                            debug!("Using cached type {} for field '{}'", ft, field_name);

                            // For enums, use the spawn format as the mutation example
                            let mutation_example =
                                if cached_info.type_category == TypeCategory::Enum {
                                    cached_info.spawn_format.clone()
                                } else {
                                    json!(null)
                                };

                            // Add base mutation path with appropriate example and variants
                            mutation_paths.push(MutationPath {
                                path:          base_path.clone(),
                                example_value: mutation_example,
                                enum_variants: cached_info.enum_variants.clone(),
                                type_name:     Some(ft.to_string()),
                            });

                            // Add nested paths from cache (for structs)
                            for path in &cached_info.mutation_paths {
                                let nested_path = format!(".{field_name}{}", path.path);
                                mutation_paths.push(MutationPath {
                                    path:          nested_path,
                                    example_value: path.example_value.clone(),
                                    enum_variants: path.enum_variants.clone(),
                                    type_name:     path.type_name.clone(),
                                });
                            }

                            // Use spawn format from cache for enums
                            if cached_info.type_category == TypeCategory::Enum {
                                spawn_format
                                    .insert(field_name.clone(), cached_info.spawn_format.clone());
                            }
                        } else {
                            debug!("Type {} not found in cache after batch discovery", ft);
                            // Type not in cache - use null as example
                            mutation_paths.push(MutationPath {
                                path:          base_path.clone(),
                                example_value: json!(null),
                                enum_variants: None,
                                type_name:     Some(ft.to_string()),
                            });
                        }

                        // Check for special cases like Option<Vec2> that might have array access
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
