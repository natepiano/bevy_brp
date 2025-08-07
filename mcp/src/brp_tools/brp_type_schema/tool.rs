//! `brp_type_schema` tool - Local registry-based type schema discovery
//!
//! This tool provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide the same information as `brp_extras_discover_format`.

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tracing::{debug, warn};

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

        // Build local type info for each type
        for (type_name, _) in &type_value_pairs {
            build_local_type_info_for_type(type_name, &registry_data, self.port)?;
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
fn build_local_type_info_for_type(
    type_name: &BrpTypeName,
    registry_data: &Value,
    _port: Port,
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
        build_spawn_format_and_mutation_paths(&type_schema, type_name_str);

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

/// Build spawn format and mutation paths from registry schema properties
#[allow(dead_code)]
fn build_spawn_format_and_mutation_paths(
    type_schema: &Value,
    type_name: &str,
) -> (Map<String, Value>, Vec<MutationPath>) {
    let mut spawn_format = Map::new();
    let mut mutation_paths = Vec::new();

    let properties = type_schema.get("properties").and_then(Value::as_object);

    if let Some(props) = properties {
        for (field_name, field_info) in props {
            // Extract type from {"type": {"$ref": "#/$defs/glam::Vec3"}} structure
            let field_type = field_info
                .get("type")
                .and_then(|t| t.get("$ref"))
                .and_then(Value::as_str)
                .and_then(|ref_str| ref_str.strip_prefix("#/$defs/"));

            match field_type {
                Some(ft) => {
                    if let Some(hardcoded) = BRP_FORMAT_KNOWLEDGE.get(&ft.into()) {
                        spawn_format.insert(field_name.clone(), hardcoded.example_value.clone());
                        debug!("Added field '{}' from hardcoded knowledge", field_name);

                        // Always generate base field mutation path
                        let base_path = format!(".{field_name}");
                        mutation_paths.push(MutationPath {
                            path:          base_path,
                            example_value: hardcoded.example_value.clone(),
                        });

                        // Generate component mutation paths if available
                        if let Some(component_paths) = &hardcoded.subfield_paths {
                            for (component_name, example_value) in component_paths {
                                let component_path = format!(".{field_name}.{component_name}");

                                mutation_paths.push(MutationPath {
                                    path:          component_path,
                                    example_value: example_value.clone(),
                                });
                            }
                        }
                    } else {
                        warn!(
                            "Skipping unknown field type '{}' for field '{}' in '{}' - not in hardcoded knowledge",
                            ft, field_name, type_name
                        );
                    }
                }
                None => {
                    debug!(
                        "Skipping field '{}' in '{}' - missing or invalid $ref format",
                        field_name, type_name
                    );
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
            // Type not found or failed to process
            let error_entry = json!({
                "type_name": type_name,
                "in_registry": false,
                "error": format!("Type '{}' not found in registry or failed to process", type_name),
                "enum_info": null,
                "example_values": null,
                "has_deserialize": false,
                "has_serialize": false,
                "mutation_paths": {},
                "supported_operations": [],
                "type_category": "Unknown"
            });
            type_info.insert(type_name.clone(), error_entry);
            failed_discoveries += 1;
        }
    }

    json!({
        "discovered_count": successful_discoveries,
        "requested_types": requested_types,
        "success": failed_discoveries == 0,
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
        let path_parts: Vec<&str> = path_without_dot.split('.').collect();

        let description = if path_parts.len() == 1 {
            // Base field - check if it has components to determine "entire" vs just field name
            let field_name = path_parts[0];
            if component_fields.contains(field_name) {
                format!("Mutate the entire {field_name} field")
            } else {
                format!("Mutate the {field_name} field")
            }
        } else if path_parts.len() == 2 {
            // Component field like .rotation.x
            let component_name = path_parts[1];
            format!("Mutate the {component_name} component")
        } else {
            // Fallback for deeper nesting
            format!("Mutate the {path_without_dot} field")
        };

        mutation_paths_obj.insert(mutation_path.path.clone(), json!(description));
    }

    // Convert enum variants to strings using strum
    let supported_ops: Vec<String> = cached_info
        .supported_operations
        .iter()
        .map(|op| op.as_ref().to_string())
        .collect();

    json!({
        "type_name": type_name,
        "in_registry": true,
        "type_category": cached_info.type_category,
        "has_serialize": has_serialize,
        "has_deserialize": has_deserialize,
        "supported_operations": supported_ops,
        "mutation_paths": mutation_paths_obj,
        "example_values": {
            "spawn": cached_info.spawn_format
        },
        "enum_info": null,
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
