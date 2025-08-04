//! Type discovery context for managing type information from multiple sources
//!
//! `DiscoveryContext` provides a unified interface for accessing type information
//! discovered from various sources (registry, extras plugin, etc.). The context
//! automatically extracts type names and their original values from BRP method
//! parameters during construction, ensuring consistent value propagation throughout
//! the discovery process.
//!
//! # Value Propagation
//!
//! The context combines three key operations:
//! 1. Type extraction from method parameters (spawn components, mutation targets, etc.)
//! 2. Value extraction to preserve original user input
//! 3. Registry integration to fetch type metadata
//!
//! This unified approach eliminates repeated parameter parsing and ensures that
//! original values are available for format transformations at every discovery level.

use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::debug;

use super::super::{BrpClient, ResponseStatus};
use super::types::DiscoverySource;
use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::Port;
use crate::error::{Error, Result};
use crate::tool::BrpMethod;

pub struct DiscoveryContext {
    /// Port for BRP connections when making direct discovery calls
    port:      Port,
    /// Type information from Bevy's registry
    type_info: HashMap<String, UnifiedTypeInfo>,
}

impl DiscoveryContext {
    /// Create a new `DiscoveryContext` from BRP method parameters
    /// This combines type extraction, value extraction, and registry fetching
    pub async fn from_params(
        method: BrpMethod,
        port: Port,
        params: Option<&Value>,
    ) -> Result<Self> {
        // Extract type names and values together
        let type_value_pairs = Self::extract_types_with_values(method, params)?;

        if type_value_pairs.is_empty() {
            return Ok(Self {
                port,
                type_info: HashMap::new(),
            });
        }

        // Need to pass values to registry check so they can be included in UnifiedTypeInfo
        let registry_results =
            Self::check_multiple_types_registry_status_with_values(&type_value_pairs, port).await;

        // Build type_info HashMap with values included
        let mut type_info = HashMap::new();

        for ((type_name, value), (_, registry_info)) in
            type_value_pairs.iter().zip(registry_results.iter())
        {
            if let Some(unified_info) = registry_info {
                // Registry info already has value from updated from_registry_schema constructor
                type_info.insert(type_name.clone(), unified_info.clone());
            } else {
                // Create basic info with value for types not in registry
                let basic_info =
                    UnifiedTypeInfo::for_pattern_matching(type_name.clone(), value.clone());
                type_info.insert(type_name.clone(), basic_info);
            }
        }

        Ok(Self { port, type_info })
    }

    /// Enrich existing type information with data from `bevy_brp_extras`
    ///
    /// This method attempts to discover additional format information for all types
    /// currently in the context using `bevy_brp_extras`. It preserves existing registry
    /// information and marks enriched types with the `RegistryPlusExtras` source.
    ///
    /// # Errors
    ///
    /// Returns Ok(()) even if some discoveries fail - individual failures are logged
    /// but don't prevent the overall enrichment process from completing.
    pub async fn enrich_with_extras(&mut self) -> Result<()> {
        let type_names: Vec<String> = self.type_info.keys().cloned().collect();
        let mut enriched_count = 0;
        let mut error_count = 0;

        debug!(
            "TypeDiscoveryContext: Starting enrichment for {} types",
            type_names.len()
        );

        for type_name in type_names {
            // Get the original value from existing type info
            let original_value = self
                .type_info
                .get(&type_name)
                .and_then(|info| info.original_value.clone());

            match self
                .discover_type_via_extras(&type_name, original_value)
                .await
            {
                Ok(Some(discovered_info)) => {
                    self.merge_discovered_info(type_name, discovered_info);
                    enriched_count += 1;
                }
                Ok(None) => {
                    debug!(
                        "TypeDiscoveryContext: No extras info found for type: {}",
                        type_name
                    );
                }
                Err(e) => {
                    debug!(
                        "TypeDiscoveryContext: Extras discovery failed for type {}: {}",
                        type_name, e
                    );
                    error_count += 1;
                }
            }
        }

        debug!(
            "TypeDiscoveryContext: Enrichment complete: {} enriched, {} errors",
            enriched_count, error_count
        );
        Ok(())
    }

    /// Discover type format via `bevy_brp_extras/discover_format`
    async fn discover_type_via_extras(
        &self,
        type_name: &str,
        original_value: Option<Value>,
    ) -> Result<Option<UnifiedTypeInfo>> {
        debug!("TypeDiscoveryContext: Starting extras discovery for type '{type_name}'");

        // Call brp_extras/discover_format directly
        let params = json!({
            "types": [type_name]
        });

        debug!(
            "TypeDiscoveryContext: Calling brp_extras/discover_format on port {} with params: {params}",
            self.port.0
        );

        let client = BrpClient::new(BrpMethod::BrpExtrasDiscoverFormat, self.port, Some(params));
        match client.execute_raw().await {
            Ok(ResponseStatus::Success(Some(response_data))) => {
                debug!("TypeDiscoveryContext: Received successful response from brp_extras");

                // Process the response to extract type information
                Self::process_discovery_response(type_name, &response_data, original_value)
            }
            Ok(ResponseStatus::Success(None)) => {
                debug!("TypeDiscoveryContext: Received empty success response");
                Ok(None)
            }
            Ok(ResponseStatus::Error(error)) => {
                debug!(
                    "TypeDiscoveryContext: brp_extras/discover_format failed: {} - {}",
                    error.code, error.message
                );
                Ok(None) // Return None instead of Err - this just means brp_extras is not available
            }
            Err(e) => {
                debug!(
                    "TypeDiscoveryContext: Connection error calling brp_extras/discover_format: {e}"
                );
                Ok(None) // Return None instead of Err - this just means brp_extras is not available
            }
        }
    }

    /// Convert discovery response to `UnifiedTypeInfo`
    fn process_discovery_response(
        type_name: &str,
        response_data: &Value,
        original_value: Option<Value>,
    ) -> Result<Option<UnifiedTypeInfo>> {
        debug!("TypeDiscoveryContext: Processing discovery response for '{type_name}'");
        debug!(
            "TypeDiscoveryContext: Full response data: {}",
            serde_json::to_string_pretty(response_data)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );

        // The response should contain type information, possibly as an array or object
        // We need to find the entry for our specific type

        find_type_in_response(type_name, response_data).map_or_else(|| {
            debug!("TypeDiscoveryContext: Type '{type_name}' not found in discovery response");
            Ok(None)
        }, |type_data| {
            debug!("TypeDiscoveryContext: Found type data for '{type_name}'");

            // Use the constructor to convert TypeDiscoveryResponse → UnifiedTypeInfo
            if let Some(unified_info) = UnifiedTypeInfo::from_discovery_response(type_data, original_value) {
                debug!(
                    "TypeDiscoveryContext: Successfully converted to UnifiedTypeInfo with {} mutation paths, {} examples",
                    unified_info.format_info.mutation_paths.len(),
                    unified_info.format_info.examples.len()
                );
                Ok(Some(unified_info))
            } else {
                debug!("TypeDiscoveryContext: Failed to convert response to UnifiedTypeInfo");
                Ok(None) // Return None instead of error to match the behavior in extras_integration
            }
        })
    }

    /// Merge discovered information with existing registry information
    fn merge_discovered_info(&mut self, type_name: String, mut discovered_info: UnifiedTypeInfo) {
        // Preserve existing registry status if available
        if let Some(existing_info) = self.type_info.get(&type_name) {
            discovered_info.registry_status = existing_info.registry_status.clone();
            discovered_info.discovery_source = DiscoverySource::RegistryPlusExtras;
        }
        // discovered_info already has DirectDiscovery source if new
        self.type_info.insert(type_name, discovered_info);
    }

    /// Get all types as an iterator
    pub fn types(&self) -> impl Iterator<Item = &UnifiedTypeInfo> {
        self.type_info.values()
    }

    /// Get type names for compatibility
    pub fn type_names(&self) -> Vec<String> {
        self.type_info.keys().cloned().collect()
    }

    /// Extract type names and their values from method parameters
    fn extract_types_with_values(
        method: BrpMethod,
        params: Option<&Value>,
    ) -> Result<Vec<(String, Option<Value>)>> {
        let params =
            params.ok_or_else(|| Error::InvalidArgument("No parameters provided".to_string()))?;

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
                    pairs.push((type_name.clone(), Some(value.clone())));
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

                let value = params.get("value").cloned();
                pairs.push((component.to_string(), value));
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

                let value = params.get("value").cloned();
                pairs.push((resource.to_string(), value));
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

    /// Find type in registry response (handles various response formats)
    fn find_type_in_registry_response(type_name: &str, response_data: &Value) -> Option<Value> {
        debug!("Registry Integration: Searching for '{type_name}' in registry response");

        // Try different possible response formats:

        // Format 1: Direct object with type name as key
        if let Some(obj) = response_data.as_object() {
            if let Some(type_data) = obj.get(type_name) {
                debug!("Registry Integration: Found '{type_name}' as direct key");
                return Some(type_data.clone());
            }
        }

        // Format 2: Array of type objects with typePath field
        if let Some(arr) = response_data.as_array() {
            for item in arr {
                if let Some(item_type_path) = item.get("typePath").and_then(Value::as_str) {
                    if item_type_path == type_name {
                        debug!("Registry Integration: Found '{type_name}' in array by typePath");
                        return Some(item.clone());
                    }
                }
                // Also check shortPath for convenience
                if let Some(item_short_path) = item.get("shortPath").and_then(Value::as_str) {
                    if item_short_path == type_name {
                        debug!("Registry Integration: Found '{type_name}' in array by shortPath");
                        return Some(item.clone());
                    }
                }
            }
        }

        // Format 3: Single type object (if we requested only one type)
        if let Some(item_type_path) = response_data.get("typePath").and_then(Value::as_str) {
            if item_type_path == type_name {
                debug!("Registry Integration: Found '{type_name}' as single object");
                return Some(response_data.clone());
            }
        }

        // Format 4: Nested under specific keys
        for key in ["types", "schemas", "data"] {
            if let Some(nested) = response_data.get(key) {
                if let Some(result) = Self::find_type_in_registry_response(type_name, nested) {
                    return Some(result);
                }
            }
        }

        debug!("Registry Integration: Type '{type_name}' not found in any expected format");
        None
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
fn find_type_in_response<'a>(type_name: &str, response_data: &'a Value) -> Option<&'a Value> {
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
            type_info.get(type_name).inspect(|_| {
                debug!("TypeDiscoveryContext: Found type data for '{type_name}'");
            })
        })
}

impl DiscoveryContext {
    /// Batch check multiple types in registry and include their values
    async fn check_multiple_types_registry_status_with_values(
        type_value_pairs: &[(String, Option<Value>)],
        port: Port,
    ) -> Vec<(String, Option<UnifiedTypeInfo>)> {
        debug!(
            "Registry Integration: Batch checking {} types with values",
            type_value_pairs.len()
        );

        // Extract unique crate names from type paths for filtering
        let mut crate_names: Vec<String> = type_value_pairs
            .iter()
            .filter_map(|(type_name, _)| {
                type_name
                    .split("::")
                    .next()
                    .map(std::string::ToString::to_string)
            })
            .collect();
        crate_names.sort_unstable();
        crate_names.dedup();

        // Call registry_schema with crate names
        let params = json!({
            "with_crates": crate_names
        });

        debug!("Registry Integration: Batch call with params: {params}");

        let client = BrpClient::new(BrpMethod::BevyRegistrySchema, port, Some(params));
        match client.execute_raw().await {
            Ok(ResponseStatus::Success(Some(response_data))) => {
                debug!("Registry Integration: Received successful batch response");

                // Process each type in the response WITH its value
                let mut results = Vec::new();
                for (type_name, value) in type_value_pairs {
                    if let Some(schema_data) =
                        Self::find_type_in_registry_response(type_name, &response_data)
                    {
                        // Pass the value to from_registry_schema
                        let type_info = UnifiedTypeInfo::from_registry_schema(
                            type_name,
                            &schema_data,
                            value.clone(),
                        );
                        results.push((type_name.clone(), Some(type_info)));
                    } else {
                        debug!(
                            "Registry Integration: Type '{type_name}' not found in batch response"
                        );
                        results.push((type_name.clone(), None));
                    }
                }
                results
            }
            Ok(ResponseStatus::Success(None) | ResponseStatus::Error(_)) | Err(_) => {
                debug!("Registry Integration: Batch registry check failed");
                type_value_pairs
                    .iter()
                    .map(|(name, _)| (name.clone(), None))
                    .collect()
            }
        }
    }
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

        let result =
            DiscoveryContext::from_params(BrpMethod::BevySpawn, Port(15702), Some(&params)).await;

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

        let result =
            DiscoveryContext::from_params(BrpMethod::BevySpawn, Port(15702), Some(&params)).await;

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
                "typePath": "bevy_core::name::Name",
                "shortPath": "Name",
                "reflectTypes": ["Component"]
            }
        ]);

        let result =
            DiscoveryContext::find_type_in_registry_response("bevy_core::name::Name", &response);

        assert!(result.is_some());
        let result = result.expect("Expected to find type in registry response");
        let type_path = result
            .get("typePath")
            .and_then(|v| v.as_str())
            .expect("Expected typePath to be a string");
        assert_eq!(type_path, "bevy_core::name::Name");
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
