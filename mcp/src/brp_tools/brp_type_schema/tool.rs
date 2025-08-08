//! `brp_type_schema` tool - Local registry-based type schema discovery
//!
//! This tool provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide the same information as `brp_extras_discover_format`.

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::debug;

use super::TypeKind;
use super::registry_cache::REGISTRY_CACHE;
use super::result_types::{
    EnumFieldInfo, EnumVariantInfo, MutationPathInfo, TypeInfo, TypeSchemaResponse,
};
use super::type_discovery::{
    build_spawn_format_and_mutation_paths, determine_supported_operations, extract_reflect_types,
    require_type_in_registry,
};
use super::types::{BrpTypeName, CachedTypeInfo};
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, HandlerContext, HandlerResult, ToolFn, ToolResult};

/// Parameters for the `brp_type_schema` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct TypeSchemaParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_type_schema` tool
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct TypeSchemaResult {
    /// The type schema information containing format discovery results
    #[to_result]
    result: TypeSchemaResponse,

    /// Count of types discovered
    #[to_metadata]
    type_count: usize,

    /// Message template for formatting responses
    #[to_message]
    message_template: Option<String>,
}

/// The main tool struct for type schema discovery
pub struct TypeSchema;

impl ToolFn for TypeSchema {
    type Output = TypeSchemaResult;
    type Params = TypeSchemaParams;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            let params: TypeSchemaParams = ctx.extract_parameter_values()?;
            let result = handle_impl(params.clone()).await;
            Ok(ToolResult {
                result,
                params: Some(params),
            })
        })
    }
}

/// Implementation of the type schema discovery
async fn handle_impl(params: TypeSchemaParams) -> Result<TypeSchemaResult> {
    debug!("Executing brp_type_schema for {} types", params.types.len());

    // Convert types to BrpTypeName and prepare for registry calls
    let type_value_pairs: Vec<(BrpTypeName, Value)> = params
        .types
        .iter()
        .map(|type_str| (type_str.as_str().into(), json!({})))
        .collect();

    // Fetch registry schemas for all types at once
    let registry_data = fetch_registry_schemas(&type_value_pairs, params.port).await?;

    // Build local type info for each type (allow partial failures)
    for (type_name, _) in &type_value_pairs {
        // Continue processing even if individual types fail
        if let Err(e) = build_local_type_info_for_type(type_name, &registry_data, params.port).await
        {
            debug!("Failed to build type info for {}: {}", type_name, e);
        }
    }

    // Build the full response using typed structures
    let response = build_type_schema_response(&params.types);
    let type_count = response.discovered_count;

    Ok(TypeSchemaResult::new(response, type_count)
        .with_message_template(format!("Discovered {type_count} type(s)")))
}

/// Fetch registry schemas for all types at once
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
async fn build_local_type_info_for_type(
    type_name: &BrpTypeName,
    registry_data: &Value,
    port: Port,
) -> Result<()> {
    let type_name_str = type_name.as_str();
    debug!("Building local type info for {}", type_name_str);

    // Check if already cached
    if REGISTRY_CACHE.get(type_name).is_some() {
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
    let type_category: TypeKind = type_schema
        .get("kind")
        .and_then(Value::as_str)
        .map_or(TypeKind::Unknown, Into::into);

    // Create complete CachedTypeInfo
    let cached_info = CachedTypeInfo {
        mutation_paths,
        registry_schema: type_schema.clone(),
        reflect_types,
        spawn_format: Value::Object(spawn_format),
        supported_operations,
        type_category,
        enum_variants: None,
    };

    // Store in permanent cache
    REGISTRY_CACHE.insert(type_name.clone(), cached_info);

    debug!("Successfully cached type info for {}", type_name_str);
    Ok(())
}

// Moved require_type_in_registry to type_discovery module

// Moved build_spawn_format_and_mutation_paths to type_discovery module
// The function is now imported and used from there

// Moved determine_supported_operations to type_discovery module

/// Build the complete type schema response matching extras format
fn build_type_schema_response(requested_types: &[String]) -> TypeSchemaResponse {
    let mut response = TypeSchemaResponse::new(requested_types.to_vec());

    for type_name in requested_types {
        if let Some(cached_info) = REGISTRY_CACHE.get(&type_name.as_str().into()) {
            let type_info = build_type_info_entry(type_name, &cached_info);
            response.add_type(type_info);
        } else {
            // Type not found or failed to process
            response.add_error(type_name.clone());
        }
    }

    response.finalize();
    response
}

/// Build a single type info entry matching extras format
fn build_type_info_entry(type_name: &str, cached_info: &CachedTypeInfo) -> TypeInfo {
    // Start with the basic type info from cached data
    let mut type_info = TypeInfo::from_cached_info(type_name, cached_info);

    // Build mutation paths with proper formatting
    let mut mutation_paths = std::collections::HashMap::new();

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

        // Check if this is an Option type
        let is_option = mutation_path
            .type_name
            .as_ref()
            .map_or(false, |t| t.starts_with("core::option::Option<"));

        // Create the mutation path info
        let path_info = MutationPathInfo::from_mutation_path(mutation_path, description, is_option);

        mutation_paths.insert(mutation_path.path.clone(), path_info);
    }

    // Store the mutation paths in the type info
    type_info.mutation_paths = mutation_paths;

    // Extract enum info if this is an enum type
    if cached_info.type_category == TypeKind::Enum {
        // Get the variant information from the registry schema
        if let Some(one_of) = cached_info
            .registry_schema
            .get("oneOf")
            .and_then(Value::as_array)
        {
            let variants: Vec<EnumVariantInfo> = one_of
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

                        // Extract tuple types if present
                        let tuple_types = if variant_type == "Tuple" {
                            v.get("prefixItems").and_then(Value::as_array).map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| {
                                        item.get("type").and_then(Value::as_str).map(String::from)
                                    })
                                    .collect()
                            })
                        } else {
                            None
                        };

                        // Extract struct fields if present
                        let fields = if variant_type == "Struct" {
                            v.get("properties").and_then(Value::as_object).map(|props| {
                                props
                                    .iter()
                                    .map(|(field_name, field_value)| {
                                        let type_name = field_value
                                            .get("type")
                                            .and_then(Value::as_str)
                                            .unwrap_or("unknown")
                                            .to_string();
                                        EnumFieldInfo {
                                            name: field_name.clone(),
                                            type_name,
                                        }
                                    })
                                    .collect()
                            })
                        } else {
                            None
                        };

                        EnumVariantInfo {
                            name: name.to_string(),
                            variant_type: variant_type.to_string(),
                            fields,
                            tuple_types,
                        }
                    })
                })
                .collect();

            type_info.enum_info = Some(variants);
        }
    }

    type_info
}

// BrpTypeSchema is exported as an alias for backward compatibility
pub use TypeSchema as BrpTypeSchema;
