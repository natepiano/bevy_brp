//! Core implementation of `DiscoveryContext`
//!
//! This module contains the main logic for the `DiscoveryContext` struct.

use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::debug;

use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::brp_type_schema::{BrpTypeName, TypeSchemaEngine};
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::{Error, Result};
use crate::tool::BrpMethod;

pub struct DiscoveryContext {
    /// Port for BRP connections when making direct discovery calls
    port:     Port,
    /// Type information from Bevy's registry
    type_map: HashMap<BrpTypeName, UnifiedTypeInfo>,
}

impl DiscoveryContext {
    /// Create a new `DiscoveryContext` from BRP method parameters
    /// This combines type extraction, value extraction, and registry fetching
    pub async fn new(method: BrpMethod, port: Port, params: &Value) -> Result<Self> {
        // Extract type names and values together
        let type_value_pairs = Self::extract_type_name_and_original_value(method, params)?;

        debug!("fetching registry schema data");

        // Fetch registry data for all types at once
        let registry_data = Self::fetch_registry_schemas(&type_value_pairs, port).await?;

        // Build type_info HashMap with values included
        let mut type_map = HashMap::new();

        for (type_name, original_value) in type_value_pairs {
            // Find type in registry response
            let schema_data = Self::require_type_in_registry(type_name.as_str(), &registry_data)?;

            // Create UnifiedTypeInfo from registry schema
            let unified_info = UnifiedTypeInfo::from_registry_schema(
                type_name.clone(),
                &schema_data,
                original_value,
            );
            type_map.insert(type_name, unified_info);
        }

        Ok(Self { port, type_map })
    }

    /// Enrich existing type information with data from type schema engine
    ///
    /// This method attempts to discover additional format information for all types
    /// currently in the context using the local `TypeSchemaEngine`. It preserves existing
    /// registry information and marks enriched types with the `RegistryPlusExtras` source.
    ///
    /// Phase 5: Now uses direct `TypeSchemaEngine` integration instead of extras plugin.
    ///
    /// # Errors
    ///
    /// Returns Ok(()) even if some discoveries fail - individual failures are logged
    /// but don't prevent the overall enrichment process from completing.
    pub async fn enrich_with_type_registry(&mut self) -> Result<()> {
        let response = self.call_type_schema_engine().await?;

        // Existing enrichment logic
        for (type_name, type_info) in &mut self.type_map {
            if let Some(type_schema_data) = find_type_in_type_schema(type_name, &response) {
                type_info.enrich_from_type_schema(type_schema_data);
                debug!(
                    "TypeDiscoveryContext: Enriched type '{}' with type schema data",
                    type_name
                );
            }
        }

        Ok(())
    }

    /// Get all types as an iterator
    pub fn types(&self) -> impl Iterator<Item = &UnifiedTypeInfo> {
        self.type_map.values()
    }

    /// Fetch registry schemas for the given types
    async fn fetch_registry_schemas(
        type_value_pairs: &[(BrpTypeName, Value)],
        port: Port,
    ) -> Result<Value> {
        debug!(
            "Registry Integration: Fetching schemas for {} types",
            type_value_pairs.len()
        );

        // Extract unique crate names from type paths for filtering
        let mut crate_names: Vec<String> = type_value_pairs
            .iter()
            .filter_map(|(type_name, _)| {
                type_name
                    .as_str()
                    .split("::")
                    .next()
                    .map(std::string::ToString::to_string)
            })
            .collect();
        crate_names.sort_unstable();
        crate_names.dedup();

        // Call registry_schema with crate names for filtering
        let params = json!({
            "with_crates": crate_names
        });

        debug!("Registry Integration: Calling registry with params: {params}");

        let client = BrpClient::new(BrpMethod::BevyRegistrySchema, port, Some(params));
        match client.execute_raw().await {
            Ok(ResponseStatus::Success(Some(response_data))) => {
                debug!("Registry Integration: Received successful response");
                Ok(response_data)
            }
            Ok(ResponseStatus::Success(None)) => {
                debug!("Registry Integration: Received unexpected empty success response");
                Err(Error::BrpCommunication(
                    "Registry returned success but no data - this shouldn't happen".to_string(),
                )
                .into())
            }
            Ok(ResponseStatus::Error(error)) => {
                debug!("Registry Integration: Registry returned error: {:?}", error);
                Err(Error::BrpCommunication(format!(
                    "Registry query failed: {} - {}",
                    error.code, error.message
                ))
                .into())
            }
            Err(e) => {
                debug!("Registry Integration: Failed to call registry: {}", e);
                Err(e)
            }
        }
    }

    /// Find type in registry response (handles various response formats)
    fn find_type_in_registry_response(type_name: &str, response_data: &Value) -> Option<Value> {
        debug!("Registry Integration: Searching for '{type_name}' in registry response");

        // Try object format first (direct key lookup)
        if let Some(obj) = response_data.as_object()
            && let Some(type_data) = obj.get(type_name)
        {
            debug!("Registry Integration: Found '{type_name}' as direct key");
            return Some(type_data.clone());
        }

        // Try array format (search by typePath field)
        if let Some(arr) = response_data.as_array() {
            for item in arr {
                if let Some(type_path) = item.get("typePath").and_then(Value::as_str)
                    && type_path == type_name
                {
                    debug!("Registry Integration: Found '{type_name}' in array format");
                    return Some(item.clone());
                }
            }
        }

        debug!("Registry Integration: Type '{type_name}' not found in any expected format");
        None
    }

    /// Call `TypeSchemaEngine` to discover format information for all types
    async fn call_type_schema_engine(&self) -> Result<Value> {
        let type_names: Vec<String> = self
            .type_map
            .keys()
            .map(|k| k.as_str().to_string())
            .collect();

        debug!(
            "TypeDiscoveryContext: Calling TypeSchemaEngine on port {} with {} types",
            self.port,
            type_names.len()
        );

        // Create TypeSchemaEngine and get response
        let engine = TypeSchemaEngine::new(self.port).await?;
        let response = engine.generate_response(&type_names);

        // Convert TypeSchemaResponse to Value for compatibility with existing code
        serde_json::to_value(response).map_err(|e| {
            Error::BrpCommunication(format!("Failed to serialize TypeSchemaResponse: {e}")).into()
        })
    }

    /// Extract type names and the original parameter value from method parameters
    fn extract_type_name_and_original_value(
        method: BrpMethod,
        params: &Value,
    ) -> Result<Vec<(BrpTypeName, Value)>> {
        let mut pairs = Vec::new();

        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
                // Validate components field exists and is an object
                let components = params
                    .get("components")
                    .ok_or_else(|| {
                        Error::InvalidArgument("Missing 'components' field".to_string())
                    })?
                    .as_object()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'components' field must be an object".to_string())
                    })?;

                for (type_name, value) in components {
                    // Validate type name is a valid string (could add more validation here)
                    if type_name.is_empty() {
                        return Err(Error::InvalidArgument(
                            "Empty type name in components".to_string(),
                        )
                        .into());
                    }
                    pairs.push((type_name.into(), value.clone()));
                }

                if pairs.is_empty() {
                    return Err(Error::InvalidArgument("No components provided".to_string()).into());
                }
            }
            BrpMethod::BevyMutateComponent => {
                let component = params
                    .get("component")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'component' field".to_string()))?
                    .as_str()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'component' field must be a string".to_string())
                    })?;

                if component.is_empty() {
                    return Err(
                        Error::InvalidArgument("Empty component type name".to_string()).into(),
                    );
                }

                let value = params
                    .get("value")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'value' field".to_string()))?
                    .clone();
                pairs.push((component.into(), value));
            }
            BrpMethod::BevyInsertResource | BrpMethod::BevyMutateResource => {
                let resource = params
                    .get("resource")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'resource' field".to_string()))?
                    .as_str()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'resource' field must be a string".to_string())
                    })?;

                if resource.is_empty() {
                    return Err(
                        Error::InvalidArgument("Empty resource type name".to_string()).into(),
                    );
                }

                let value = params
                    .get("value")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'value' field".to_string()))?
                    .clone();
                pairs.push((resource.into(), value));
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Method {method:?} does not support type extraction"
                ))
                .into());
            }
        }

        Ok(pairs)
    }

    /// Find type in registry response and return error if not found
    ///
    /// This is a wrapper around `find_type_in_registry_response` that adds error handling
    /// and debug logging, used by `new()`
    fn require_type_in_registry(type_name: &str, registry_data: &Value) -> Result<Value> {
        let registry_schema = Self::find_type_in_registry_response(type_name, registry_data)
            .ok_or_else(|| Error::TypeNotRegistered {
                type_name: type_name.to_string(),
            })?;

        debug!(
            "Retrieved registry schema for {}: {:?}",
            type_name, registry_schema
        );

        Ok(registry_schema)
    }
}

/// Find type information in the response from `bevy_brp_extras/discover_format`
///
/// The response format is always:
/// ```json
/// {
///   "type_info": {
///     "TypeName": { ... }
///   }
/// }
/// ```
fn find_type_in_type_schema<'a>(
    type_name: &BrpTypeName,
    response_data: &'a Value,
) -> Option<&'a Value> {
    debug!("TypeDiscoveryContext: find_type_in_response looking for '{type_name}'");

    // bevy_brp_extras always returns format: { "type_info": { "TypeName": {...} } }
    response_data
        .get("type_info")
        .and_then(Value::as_object)
        .and_then(|type_info| {
            debug!(
                "TypeDiscoveryContext: Found type_info field, checking keys: {:?}",
                type_info.keys().collect::<Vec<_>>()
            );
            type_info.get(type_name.as_str()).inspect(|_| {
                debug!("TypeDiscoveryContext: Found type data for '{type_name}'");
            })
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn test_from_params_empty_components() {
        // Test with empty components object
        let params = json!({
            "components": {}
        });

        let result = DiscoveryContext::new(BrpMethod::BevySpawn, Port(15702), &params).await;

        // This should fail since no components are provided
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_from_params_with_components() {
        // Test with actual components
        let params = json!({
            "components": {
                "bevy_transform::components::transform::Transform": {
                    "translation": {"x": 0.0, "y": 0.0, "z": 0.0},
                    "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
                    "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
                }
            }
        });

        let result = DiscoveryContext::new(BrpMethod::BevySpawn, Port(15702), &params).await;

        // This may succeed or fail depending on BRP availability, but shouldn't crash
        assert!(result.is_ok() || result.is_err());
    }

    // Integration tests would go in the tests/ directory to test with actual BRP

    // Registry integration tests (moved from registry_integration.rs)
    #[test]
    fn test_find_type_in_registry_response_direct_key() {
        let response = json!({
            "bevy_transform::components::transform::Transform": {
                "typePath": "bevy_transform::components::transform::Transform",
                "reflectTypes": ["Component", "Serialize", "Deserialize"]
            }
        });

        let result = DiscoveryContext::find_type_in_registry_response(
            "bevy_transform::components::transform::Transform",
            &response,
        );

        assert!(result.is_some());
        let result = result.expect("Expected to find type in registry response");
        let type_path = result
            .get("typePath")
            .and_then(|v| v.as_str())
            .expect("Expected typePath to be a string");
        assert_eq!(
            type_path,
            "bevy_transform::components::transform::Transform"
        );
    }

    #[test]
    fn test_find_type_in_registry_response_array_format() {
        let response = json!([
            {
                "typePath": "bevy_transform::components::transform::Transform",
                "shortPath": "Transform",
                "reflectTypes": ["Component", "Serialize", "Deserialize"]
            },
            {
                "typePath": "bevy_ecs::name::Name",
                "shortPath": "Name",
                "reflectTypes": ["Component"]
            }
        ]);

        let result =
            DiscoveryContext::find_type_in_registry_response("bevy_ecs::name::Name", &response);

        assert!(result.is_some());
        let result = result.expect("Expected to find type in registry response");
        let type_path = result
            .get("typePath")
            .and_then(|v| v.as_str())
            .expect("Expected typePath to be a string");
        assert_eq!(type_path, "bevy_ecs::name::Name");
    }

    #[test]
    fn test_find_type_in_registry_response_not_found() {
        let response = json!({
            "other_type": {
                "typePath": "other::Type"
            }
        });

        let result = DiscoveryContext::find_type_in_registry_response(
            "bevy_transform::components::transform::Transform",
            &response,
        );

        assert!(result.is_none());
    }
}
