//! Request-scoped engine for type schema discovery
//!
//! This module contains `TypeSchemaEngine`, which owns per-request state
//! (BRP `port` and fetched `registry_data`) and implements the workflow for
//! building cached type info and assembling a `TypeSchemaResponse`.
//!
//! The engine intentionally mirrors the logic previously in `tool.rs`, but is
//! moved here so the `tool.rs` can remain a thin delegating wrapper.

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
use super::types::{BrpTypeName, CachedTypeInfo, MutationPath};
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::{Error, Result};
use crate::tool::BrpMethod;

/// Engine that owns request-scoped state and performs discovery.
///
/// Construct with `TypeSchemaEngine::new(...)` which will fetch registry schemas
/// for the given types. Then call `run(...)` to build cached info and produce a
/// `TypeSchemaResponse`.
pub struct TypeSchemaEngine {
    port:          Port,
    registry_data: Value,
}

impl TypeSchemaEngine {
    /// Create a new engine by fetching registry schemas for the given types.
    pub async fn new(type_value_pairs: &[(BrpTypeName, Value)], port: Port) -> Result<Self> {
        debug!(
            "TypeSchemaEngine::new - fetching registry schemas for {} types",
            type_value_pairs.len()
        );
        let registry_data = Self::fetch_registry_schemas(type_value_pairs, port).await?;
        Ok(Self {
            port,
            registry_data,
        })
    }

    /// Run the discovery workflow for the requested types.
    /// This will build local cached info for each type (allowing partial failures),
    /// then assemble and return a `TypeSchemaResponse`.
    pub async fn run(&self, requested_types: &[String]) -> Result<TypeSchemaResponse> {
        // Build local type info for each type (allow partial failures)
        for type_name in requested_types {
            let brp_name: BrpTypeName = type_name.as_str().into();
            if let Err(e) = self.build_local_type_info_for_type(&brp_name).await {
                debug!("Failed to build type info for {}: {}", type_name, e);
            }
        }

        // Build the typed response
        Ok(Self::build_type_schema_response(requested_types))
    }

    /// Fetch registry schemas for all types at once
    async fn fetch_registry_schemas(
        type_value_pairs: &[(BrpTypeName, Value)],
        port: Port,
    ) -> Result<Value> {
        debug!(
            "TypeSchemaEngine::fetch_registry_schemas: Fetching registry schemas for {} types",
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

    /// Build local type info for a single type and store in the permanent cache.
    async fn build_local_type_info_for_type(&self, type_name: &BrpTypeName) -> Result<()> {
        let type_name_str = type_name.as_str();
        debug!(
            "TypeSchemaEngine: Building local type info for {}",
            type_name_str
        );

        // Check if already cached
        if REGISTRY_CACHE.get(type_name).is_some() {
            debug!("Type {} already in cache", type_name_str);
            return Ok(());
        }

        // Find this type in the registry response
        let type_schema = require_type_in_registry(type_name_str, &self.registry_data)?;

        // Extract serialization flags from registry schema directly
        let reflect_types = extract_reflect_types(type_schema);

        // Build spawn format and mutation paths from properties using hardcoded knowledge
        let (spawn_format, mutation_paths) =
            build_spawn_format_and_mutation_paths(type_schema, type_name_str, self.port).await;

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

    /// Build the complete type schema response matching extras format
    fn build_type_schema_response(requested_types: &[String]) -> TypeSchemaResponse {
        let mut response = TypeSchemaResponse::new(requested_types.to_vec());

        for type_name in requested_types {
            if let Some(cached_info) = REGISTRY_CACHE.get(&type_name.as_str().into()) {
                let type_info = Self::build_type_info_entry(type_name, &cached_info);
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
        type_info.mutation_paths = Self::build_mutation_paths(cached_info);

        // Extract enum info if this is an enum type
        if cached_info.type_category == TypeKind::Enum {
            type_info.enum_info = Self::extract_enum_info(&cached_info.registry_schema);
        }

        type_info
    }

    /// Build mutation paths from cached info
    fn build_mutation_paths(
        cached_info: &CachedTypeInfo,
    ) -> std::collections::HashMap<String, MutationPathInfo> {
        let mut mutation_paths = std::collections::HashMap::new();

        // Group paths by base field to determine which are "entire" fields
        let component_fields = Self::get_component_fields(&cached_info.mutation_paths);

        // Generate descriptions with example values
        for mutation_path in &cached_info.mutation_paths {
            let path_without_dot = mutation_path.path.trim_start_matches('.');
            let description =
                Self::generate_mutation_description(path_without_dot, &component_fields);

            // Check if this is an Option type
            let is_option = mutation_path
                .type_name
                .as_ref()
                .is_some_and(|t| t.starts_with("core::option::Option<"));

            // Create the mutation path info
            let path_info =
                MutationPathInfo::from_mutation_path(mutation_path, description, is_option);

            mutation_paths.insert(mutation_path.path.clone(), path_info);
        }

        mutation_paths
    }

    /// Get component fields from mutation paths
    fn get_component_fields(mutation_paths: &[MutationPath]) -> std::collections::HashSet<&str> {
        let mut component_fields = std::collections::HashSet::new();

        for mutation_path in mutation_paths {
            let path_parts: Vec<&str> = mutation_path
                .path
                .trim_start_matches('.')
                .split('.')
                .collect();
            if path_parts.len() == 2 {
                component_fields.insert(path_parts[0]);
            }
        }

        component_fields
    }

    /// Generate a description for a mutation path
    fn generate_mutation_description(
        path_without_dot: &str,
        _component_fields: &std::collections::HashSet<&str>,
    ) -> String {
        if path_without_dot.contains('[') {
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
                let field_name = path_parts[0];
                format!("Mutate the entire {field_name} field")
            } else if path_parts.len() == 2 {
                // Component field like .rotation.x
                let component_name = path_parts[1];
                format!("Mutate the {component_name} component")
            } else {
                // Fallback for deeper nesting
                format!("Mutate the {path_without_dot} field")
            }
        }
    }

    /// Extract enum info from registry schema
    fn extract_enum_info(registry_schema: &Value) -> Option<Vec<EnumVariantInfo>> {
        let one_of = registry_schema.get("oneOf").and_then(Value::as_array)?;

        let variants: Vec<EnumVariantInfo> = one_of
            .iter()
            .filter_map(Self::extract_enum_variant)
            .collect();

        Some(variants)
    }

    /// Extract a single enum variant from schema
    fn extract_enum_variant(v: &Value) -> Option<EnumVariantInfo> {
        let name = v.get("shortPath").and_then(Value::as_str)?;

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
                    .filter_map(|item| item.get("type").and_then(Value::as_str).map(String::from))
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

        Some(EnumVariantInfo {
            name: name.to_string(),
            variant_type: variant_type.to_string(),
            fields,
            tuple_types,
        })
    }
}
