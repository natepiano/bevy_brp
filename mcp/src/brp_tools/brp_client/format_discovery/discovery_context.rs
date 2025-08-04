//! Type discovery context for managing type information from multiple sources
//!
//! `TypeDiscoveryContext` provides a unified interface for accessing type
//! information discovered from various sources (registry, extras plugin, etc.)

use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::debug;

use super::super::{BrpClient, ResponseStatus};
use super::types::DiscoverySource;
use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::Port;
use crate::error::Result;
use crate::tool::BrpMethod;

pub struct DiscoveryContext {
    /// Port for BRP connections when making direct discovery calls
    port:      Port,
    /// Type information from Bevy's registry
    type_info: HashMap<String, UnifiedTypeInfo>,
}

impl DiscoveryContext {
    /// Create context and fetch registry information for the given types
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The BRP connection to the specified port fails
    /// - The registry query times out or returns invalid data
    ///
    /// Note: Types that couldn't be found in the registry are silently filtered
    /// rather than causing errors.
    pub async fn fetch_from_registry(port: Port, type_names: Vec<String>) -> Result<Self> {
        debug!(
            "TypeDiscoveryContext: Fetching registry info for {} types on port {}",
            type_names.len(),
            port.0
        );

        if type_names.is_empty() {
            return Ok(Self {
                port,
                type_info: HashMap::new(),
            });
        }

        // Call registry to get type info
        let registry_results = Self::check_multiple_types_registry_status(&type_names, port).await;

        // Convert to HashMap, filtering out failed lookups with detailed logging
        let mut type_info = HashMap::new();
        let mut missing_types = Vec::new();

        for (name, info) in registry_results {
            match info {
                Some(type_info_data) => {
                    type_info.insert(name, type_info_data);
                }
                None => {
                    missing_types.push(name);
                }
            }
        }

        if missing_types.is_empty() {
            debug!(
                "TypeDiscoveryContext: Successfully fetched all {} requested types",
                type_info.len()
            );
        } else {
            debug!(
                "TypeDiscoveryContext: Fetched {} types, {} types not found in registry: {:?}",
                type_info.len(),
                missing_types.len(),
                missing_types
            );
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
            match self.discover_type_via_extras(&type_name).await {
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
    async fn discover_type_via_extras(&self, type_name: &str) -> Result<Option<UnifiedTypeInfo>> {
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
                Self::process_discovery_response(type_name, &response_data)
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
            if let Some(unified_info) = UnifiedTypeInfo::from_discovery_response(type_data) {
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

    /// Get type information for a specific type name
    ///
    /// Returns `None` if the type is not found in the context.
    /// This replaces direct `HashMap` access and provides controlled
    /// access to type information.
    pub fn get_type(&self, type_name: &str) -> Option<&UnifiedTypeInfo> {
        self.type_info.get(type_name)
    }

    /// Count how many of the given type names have registry information
    pub fn count_types_with_info(&self, type_names: &[String]) -> usize {
        type_names
            .iter()
            .filter(|name| self.type_info.contains_key(name.as_str()))
            .count()
    }

    /// Batch check multiple types in a single registry call
    async fn check_multiple_types_registry_status(
        type_names: &[String],
        port: Port,
    ) -> Vec<(String, Option<UnifiedTypeInfo>)> {
        debug!(
            "Registry Integration: Batch checking {} types",
            type_names.len()
        );

        // Extract unique crate names from type paths for filtering
        let mut crate_names: Vec<String> = type_names
            .iter()
            .filter_map(|type_name| {
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

                // Process each type in the response
                let mut results = Vec::new();
                for type_name in type_names {
                    if let Some(schema_data) =
                        Self::find_type_in_registry_response(type_name, &response_data)
                    {
                        let type_info =
                            UnifiedTypeInfo::from_registry_schema(type_name, &schema_data);
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
                type_names.iter().map(|name| (name.clone(), None)).collect()
            }
        }
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_type_names() {
        let context = DiscoveryContext::fetch_from_registry(Port(15702), vec![])
            .await
            .unwrap();
        assert!(context.type_info.is_empty());
    }

    #[tokio::test]
    async fn test_get_type() {
        let port = Port(15702);
        let context = DiscoveryContext::fetch_from_registry(port, vec![])
            .await
            .unwrap();

        // Should return None for any type when no types provided
        assert!(context.get_type("SomeType").is_none());
    }

    #[tokio::test]
    async fn test_existing_constructor_with_port() {
        // Test that updated fetch_from_registry stores port correctly
        let port = Port(15702);

        // Note: This will make an actual BRP call and likely fail with connection error,
        // but we can still verify the port is stored correctly in the resulting context
        let result = DiscoveryContext::fetch_from_registry(
            port,
            vec!["NonExistentType".to_string()], // Use a type that won't be found
        )
        .await;

        // The call should succeed even if the type isn't found (types are filtered)
        if let Ok(context) = result {
            // Verify the port was stored correctly
            assert_eq!(context.port.0, port.0);
            // Should have empty type_info since type doesn't exist
            assert_eq!(context.type_info.len(), 0);
        } else {
            // If BRP connection fails, that's okay for this unit test
            // We're primarily testing the constructor logic, not the BRP connection
        }
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
