//! `brp_type_schema` tool - Local registry-based type schema discovery
//!
//! This tool provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide the same information as `brp_extras_discover_format`.

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tracing::debug;

use super::hardcoded_formats::BRP_FORMAT_KNOWLEDGE;
use super::registry_cache::global_cache;
use super::types::{BrpSupportedOperation, CachedTypeInfo, MutationPath};
use crate::brp_tools::brp_client::{BrpTypeName, TypeCategory};
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, HandlerContext, HandlerResult, ToolFn, ToolResult};

/// Parameters for the `brp_type_schema` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct TypeSchemaParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_type_schema` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct TypeSchemaResult {
    /// The type schema information containing format discovery results
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of types discovered
    #[to_metadata(result_operation = "count_type_info")]
    pub type_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Discovered {type_count} format(s)")]
    pub message_template: String,
}

impl TypeSchemaParams {
    /// Execute the type schema discovery operation
    #[allow(dead_code)]
    pub async fn execute(self) -> Result<TypeSchemaResult> {
        debug!("Executing brp_type_schema for {} types", self.types.len());

        // Convert types to BrpTypeName and prepare for registry calls
        let type_value_pairs: Vec<(BrpTypeName, Value)> = self
            .types
            .iter()
            .map(|type_str| (type_str.as_str().into(), json!({})))
            .collect();

        // Fetch registry schemas for all types at once
        let registry_data = fetch_registry_schemas(&type_value_pairs, self.port).await?;

        // Build local type info for each type (allow partial failures)
        for (type_name, _) in &type_value_pairs {
            // Continue processing even if individual types fail
            if let Err(e) =
                build_local_type_info_for_type(type_name, &registry_data, self.port).await
            {
                debug!("Failed to build type info for {}: {}", type_name, e);
            }
        }

        // Build the full response matching extras format
        let result = build_type_schema_response(&self.types);
        let type_count = count_type_info(&result);

        Ok(TypeSchemaResult {
            result: Some(result),
            type_count,
            message_template: String::new(),
        })
    }
}

/// Fetch registry schemas for all types at once
#[allow(dead_code)]
async fn fetch_registry_schemas(
    type_value_pairs: &[(BrpTypeName, Value)],
    port: Port,
) -> Result<Value> {
    debug!(
        "fetch_registry_schemas: Fetching registry schemas for {} types",
        type_value_pairs.len()
    );

    let type_names: Vec<String> = type_value_pairs
        .iter()
        .map(|(type_name, _)| type_name.to_string())
        .collect();

    let client = BrpClient::new(
        BrpMethod::BevyRegistrySchema,
        port,
        Some(json!({
            "with_types": type_names
        })),
    );
    let registry_response = client.execute_raw().await?;

    match registry_response {
        ResponseStatus::Success(Some(result)) => {
            debug!("Successfully fetched registry schemas");
            Ok(result)
        }
        ResponseStatus::Success(None) => {
            debug!("Registry call succeeded but returned no data");
            Ok(json!({}))
        }
        ResponseStatus::Error(brp_error) => {
            Err(Error::BrpCommunication(format!("{brp_error:?}")).into())
        }
    }
}

/// Build local type info for a single type and store in cache
#[allow(dead_code)]
async fn build_local_type_info_for_type(
    type_name: &BrpTypeName,
    registry_data: &Value,
    port: Port,
) -> Result<()> {
    let type_name_str = type_name.as_str();
    debug!("Building local type info for {}", type_name_str);

    // Check if already cached
    if global_cache().get(type_name).is_some() {
        debug!("Type {} already in cache", type_name_str);
        return Ok(());
    }

    // Find this type in the registry response
    let type_schema = require_type_in_registry(type_name_str, registry_data)?;

    // Extract serialization flags from registry schema directly
    let reflect_types = extract_reflect_types(&type_schema);

    // Build spawn format and mutation paths from properties using hardcoded knowledge
    let (spawn_format, mutation_paths) =
        build_spawn_format_and_mutation_paths(&type_schema, type_name_str, port).await;

    // Determine supported operations based on reflection types
    let supported_operations = determine_supported_operations(&reflect_types);

    // Extract type category from registry schema
    let type_category: TypeCategory = type_schema
        .get("kind")
        .and_then(Value::as_str)
        .map_or(TypeCategory::Unknown, Into::into);

    // Create complete CachedTypeInfo
    let cached_info = CachedTypeInfo {
        mutation_paths,
        registry_schema: type_schema,
        reflect_types,
        spawn_format: Value::Object(spawn_format),
        supported_operations,
        type_category,
        enum_variants: None,
    };

    // Store in permanent cache
    global_cache().insert(type_name.clone(), cached_info);

    debug!("Successfully cached type info for {}", type_name_str);
    Ok(())
}

/// Find type in registry response and return error if not found
#[allow(dead_code)]
fn require_type_in_registry(type_name: &str, registry_data: &Value) -> Result<Value> {
    // Try object format first (direct key lookup)
    if let Some(obj) = registry_data.as_object() {
        if let Some(type_data) = obj.get(type_name) {
            debug!("Found {} in registry (object format)", type_name);
            return Ok(type_data.clone());
        }
    }

    // Try array format (search by typePath field)
    if let Some(arr) = registry_data.as_array() {
        for item in arr {
            if let Some(type_path) = item.get("typePath").and_then(Value::as_str) {
                if type_path == type_name {
                    debug!("Found {} in registry (array format)", type_name);
                    return Ok(item.clone());
                }
            }
        }
    }

    Err(
        Error::BrpCommunication(format!("Type '{type_name}' not found in registry response",))
            .into(),
    )
}

/// Discover nested type paths by fetching registry schema for unknown types
#[allow(dead_code)]
async fn discover_nested_type_paths(
    field_type: &str,
    field_name: &str,
    port: Port,
) -> Result<Vec<MutationPath>> {
    let mut nested_paths = Vec::new();

    // Check cache first
    let type_name: BrpTypeName = field_type.into();
    if let Some(cached_info) = global_cache().get(&type_name) {
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
                            for (nested_field_name, _nested_field_info) in props {
                                let nested_path = format!(".{field_name}.{nested_field_name}");
                                nested_paths.push(MutationPath {
                                    path:          nested_path,
                                    example_value: json!(null),
                                    enum_variants: None,
                                    type_name:     None,
                                });
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
                                registry_schema: type_schema,
                                reflect_types,
                                spawn_format: json!({}),
                                supported_operations,
                                type_category: TypeCategory::Struct,
                                enum_variants: None,
                            };

                            global_cache().insert(type_name, cached_info);
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
                            let spawn_format = if let Some(first_variant) = one_of.first() {
                                if let Some(variant_name) =
                                    first_variant.get("shortPath").and_then(Value::as_str)
                                {
                                    // Check variant type to build appropriate spawn format
                                    if let Some(prefix_items) =
                                        first_variant.get("prefixItems").and_then(Value::as_array)
                                    {
                                        // Tuple variant - need to discover the inner type
                                        if let Some(first_item) = prefix_items.first() {
                                            if let Some(type_ref) = first_item
                                                .get("type")
                                                .and_then(|t| t.get("$ref"))
                                                .and_then(Value::as_str)
                                            {
                                                // Extract the type name from the $ref
                                                let inner_type = type_ref
                                                    .strip_prefix("#/$defs/")
                                                    .unwrap_or(type_ref);

                                                // For known types like Srgba, provide example
                                                // values
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

                                                json!({
                                                    variant_name: [inner_value]
                                                })
                                            } else {
                                                json!({ variant_name: [] })
                                            }
                                        } else {
                                            json!({ variant_name: [] })
                                        }
                                    } else if first_variant.get("properties").is_some() {
                                        // Struct variant
                                        json!({ variant_name: {} })
                                    } else {
                                        // Unit variant
                                        json!(variant_name)
                                    }
                                } else {
                                    json!({})
                                }
                            } else {
                                json!({})
                            };

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
                                type_category: TypeCategory::Enum,
                                enum_variants: Some(variant_options),
                            };

                            global_cache().insert(type_name, cached_info);
                            debug!("Cached enum type {} with spawn format", field_type);
                        }
                    }
                    _ => {
                        debug!("Unknown type kind for {}: {:?}", field_type, type_kind);
                        // Cache with empty paths for unknown types
                        let reflect_types = extract_reflect_types(&type_schema);
                        let supported_operations = determine_supported_operations(&reflect_types);
                        let type_category: TypeCategory = type_schema
                            .get("kind")
                            .and_then(Value::as_str)
                            .map_or(TypeCategory::Unknown, Into::into);

                        let cached_info = CachedTypeInfo {
                            mutation_paths: vec![],
                            registry_schema: type_schema,
                            reflect_types,
                            spawn_format: json!({}),
                            supported_operations,
                            type_category,
                            enum_variants: None,
                        };

                        global_cache().insert(type_name, cached_info);
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

/// Extract reflect types from a registry schema
#[allow(dead_code)]
fn extract_reflect_types(type_schema: &Value) -> Vec<String> {
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

/// Batch discovery of types by crate to minimize registry calls
async fn batch_discover_types_by_crate(types_to_discover: Vec<String>, port: Port) -> Result<()> {
    // Group types by crate
    let mut types_by_crate: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for type_name in types_to_discover {
        // Skip if already in cache
        if global_cache().get(&type_name.as_str().into()).is_some() {
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

                                global_cache().insert(type_name_key, cached_info);
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

                                global_cache().insert(type_name_key, cached_info);
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

                                global_cache().insert(type_name_key, cached_info);
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

/// Build enum spawn format from type schema
fn build_enum_spawn_format(type_schema: &Value) -> Value {
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

/// Build spawn format and mutation paths from registry schema properties
#[allow(dead_code)]
async fn build_spawn_format_and_mutation_paths(
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

                        if let Some(cached_info) = global_cache().get(&type_name_key) {
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

/// Determine which BRP operations are supported based on reflection types
#[allow(dead_code)]
fn determine_supported_operations(reflect_types: &[String]) -> Vec<BrpSupportedOperation> {
    use BrpSupportedOperation::{Get, Insert, Mutate, Query, Spawn};

    let mut ops = Vec::new();

    // Always supported if in registry
    ops.push(Query);
    ops.push(Get);

    let has_serialize = reflect_types.contains(&"Serialize".to_string());
    let has_deserialize = reflect_types.contains(&"Deserialize".to_string());
    let has_component = reflect_types.contains(&"Component".to_string());
    let has_resource = reflect_types.contains(&"Resource".to_string());

    // Component operations:
    // - spawn/insert require BOTH Serialize AND Deserialize
    // - mutate_component works even without SerDe
    if has_component {
        if has_serialize && has_deserialize {
            ops.push(Spawn);
            ops.push(Insert);
        }
        ops.push(Mutate); // Always available for components in registry
    }

    // Resource operations:
    // - insert_resource always works (no SerDe required)
    // - mutate_resource always works
    if has_resource {
        ops.push(Insert); // insert_resource always available
        ops.push(Mutate); // mutate_resource always available
    }

    ops
}

/// Build the complete type schema response matching extras format
#[allow(dead_code)]
fn build_type_schema_response(requested_types: &[String]) -> Value {
    let mut type_info = Map::new();
    let mut successful_discoveries = 0;
    let mut failed_discoveries = 0;

    for type_name in requested_types {
        if let Some(cached_info) = global_cache().get(&type_name.as_str().into()) {
            let type_entry = build_type_info_entry(type_name, &cached_info);
            type_info.insert(type_name.clone(), type_entry);
            successful_discoveries += 1;
        } else {
            // Type not found or failed to process - match extras format exactly
            let error_entry = json!({
                "error": "Type not found in registry",
                "in_registry": false,
                "type_name": type_name
            });
            type_info.insert(type_name.clone(), error_entry);
            failed_discoveries += 1;
        }
    }

    json!({
        "discovered_count": successful_discoveries,
        "requested_types": requested_types,
        "success": true,  // Always true, matching extras behavior
        "summary": {
            "failed_discoveries": failed_discoveries,
            "successful_discoveries": successful_discoveries,
            "total_requested": requested_types.len()
        },
        "type_info": type_info
    })
}

/// Build a single type info entry matching extras format
#[allow(dead_code)]
fn build_type_info_entry(type_name: &str, cached_info: &CachedTypeInfo) -> Value {
    // Use reflection flags directly from cached info
    let has_serialize = cached_info.reflect_types.contains(&"Serialize".to_string());
    let has_deserialize = cached_info
        .reflect_types
        .contains(&"Deserialize".to_string());

    // Convert mutation paths to extras format (object with path -> description mappings)
    let mut mutation_paths_obj = Map::new();

    // Group paths by base field to determine which are "entire" fields
    let mut component_fields = std::collections::HashSet::new();

    for mutation_path in &cached_info.mutation_paths {
        let path_parts: Vec<&str> = mutation_path
            .path
            .trim_start_matches('.')
            .split('.')
            .collect();
        if path_parts.len() == 2 {
            component_fields.insert(path_parts[0]);
        }
    }

    // Generate descriptions with example values
    for mutation_path in &cached_info.mutation_paths {
        let path_without_dot = mutation_path.path.trim_start_matches('.');

        // Handle array indices like custom_size[0]
        let description = if path_without_dot.contains('[') {
            // Array access pattern
            if path_without_dot.ends_with("[0]") {
                "Mutate the first element of the Vec".to_string()
            } else if path_without_dot.ends_with("[1]") {
                "Mutate the second element of the Vec".to_string()
            } else {
                format!("Mutate the {path_without_dot} field")
            }
        } else {
            let path_parts: Vec<&str> = path_without_dot.split('.').collect();

            if path_parts.len() == 1 {
                // Base field - check if it has components to determine "entire" vs just field name
                let field_name = path_parts[0];
                if component_fields.contains(field_name) {
                    format!("Mutate the entire {field_name} field")
                } else {
                    format!("Mutate the entire {field_name} field")
                }
            } else if path_parts.len() == 2 {
                // Component field like .rotation.x
                let component_name = path_parts[1];
                format!("Mutate the {component_name} component")
            } else {
                // Fallback for deeper nesting
                format!("Mutate the {path_without_dot} field")
            }
        };

        // Build path info with example, variants, and type
        let mut path_obj = Map::new();
        path_obj.insert("description".to_string(), json!(description));

        if !mutation_path.example_value.is_null() {
            path_obj.insert("example".to_string(), mutation_path.example_value.clone());
        }

        if let Some(variants) = &mutation_path.enum_variants {
            path_obj.insert("variants".to_string(), json!(variants));
        }

        if let Some(type_name) = &mutation_path.type_name {
            path_obj.insert("type".to_string(), json!(type_name));
        }

        // Use simple string if only description, otherwise use object
        let path_info = if path_obj.len() == 1 {
            json!(description)
        } else {
            Value::Object(path_obj)
        };

        mutation_paths_obj.insert(mutation_path.path.clone(), path_info);
    }

    // Convert enum variants to strings using strum
    let supported_ops: Vec<String> = cached_info
        .supported_operations
        .iter()
        .map(|op| op.as_ref().to_string())
        .collect();

    // Extract enum info if this is an enum type
    let enum_info = if cached_info.type_category == TypeCategory::Enum {
        // Get the variant information from the registry schema
        if let Some(one_of) = cached_info
            .registry_schema
            .get("oneOf")
            .and_then(Value::as_array)
        {
            let variants: Vec<Value> = one_of
                .iter()
                .filter_map(|v| {
                    v.get("shortPath").and_then(Value::as_str).map(|name| {
                        // Check if this is a unit variant, tuple variant, or struct variant
                        let variant_type = if v.get("prefixItems").is_some() {
                            "Tuple"
                        } else if v.get("properties").is_some() {
                            "Struct"
                        } else {
                            "Unit"
                        };

                        json!({
                            "name": name,
                            "type": variant_type
                        })
                    })
                })
                .collect();

            Some(json!({
                "variants": variants
            }))
        } else {
            None
        }
    } else {
        None
    };

    // Only include spawn examples if spawn/insert operations are supported
    let example_values = if cached_info
        .supported_operations
        .contains(&BrpSupportedOperation::Spawn)
        || cached_info
            .supported_operations
            .contains(&BrpSupportedOperation::Insert)
    {
        json!({
            "spawn": cached_info.spawn_format
        })
    } else {
        json!({})
    };

    json!({
        "type_name": type_name,
        "in_registry": true,
        "type_category": cached_info.type_category,
        "has_serialize": has_serialize,
        "has_deserialize": has_deserialize,
        "supported_operations": supported_ops,
        "mutation_paths": mutation_paths_obj,
        "example_values": example_values,
        "enum_info": enum_info,
        "error": null
    })
}

/// Count the number of types in the `type_info` object
#[allow(dead_code)]
fn count_type_info(result: &Value) -> usize {
    result
        .get("type_info")
        .and_then(Value::as_object)
        .map_or(0, serde_json::Map::len)
}

/// The `BrpTypeSchema` tool implementation
pub struct BrpTypeSchema;

impl ToolFn for BrpTypeSchema {
    type Output = TypeSchemaResult;
    type Params = TypeSchemaParams;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            // Extract typed parameters
            let params: TypeSchemaParams = ctx.extract_parameter_values()?;

            // Clone params for return value since execute() takes ownership
            let params_clone = TypeSchemaParams {
                types: params.types.clone(),
                port:  params.port,
            };

            // Execute the tool logic
            let result = params.execute().await;

            Ok(ToolResult {
                result,
                params: Some(params_clone),
            })
        })
    }
}
