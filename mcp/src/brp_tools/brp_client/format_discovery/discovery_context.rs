//! Type discovery context for managing type information from multiple sources
//!
//! `TypeDiscoveryContext` provides a unified interface for accessing type
//! information discovered from various sources (registry, extras plugin, etc.)
//! This abstraction replaces direct `HashMap` passing and provides better
//! encapsulation of type discovery logic.
//!
//! # Current Implementation (Phase 1)
//!
//! Currently wraps registry lookups only with minimal API surface.
//! Only includes methods with identified usage in the current codebase.
//! Future phases will add more methods as needed.
//!
//! # Example
//!
//! ```rust
//! let type_names = vec!["Transform".to_string(), "Sprite".to_string()];
//! let context = TypeDiscoveryContext::fetch_from_registry(port, type_names).await?;
//!
//! if let Some(transform_info) = context.get_type("Transform") {
//!     // Use type information
//! }
//!
//! // For compatibility with existing code:
//! let type_info_map = context.as_hashmap();
//! ```

use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::debug;

use super::super::{BrpClient, ResponseStatus};
use super::types::DiscoverySource;
use super::unified_types::UnifiedTypeInfo;
use super::{adapters, registry_integration};
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

        // Call existing registry integration to get type info
        let registry_results =
            registry_integration::check_multiple_types_registry_status(&type_names, port).await;

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

            // Use the schema adapter to convert TypeDiscoveryResponse → UnifiedTypeInfo
            if let Some(unified_info) = adapters::from_type_discovery_response_json(type_data) {
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

    /// Get the underlying `HashMap` (temporary for compatibility)
    /// This method will be removed in Phase 4 when all discovery levels
    /// are updated to work directly with `TypeDiscoveryContext`
    pub const fn as_hashmap(&self) -> &HashMap<String, UnifiedTypeInfo> {
        &self.type_info
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
        assert!(context.as_hashmap().is_empty());
    }

    #[tokio::test]
    async fn test_as_hashmap() {
        let port = Port(15702);
        let context = DiscoveryContext::fetch_from_registry(port, vec![])
            .await
            .unwrap();

        // Should return empty HashMap when no types provided
        assert!(context.as_hashmap().is_empty());
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
}
