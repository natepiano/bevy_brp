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

use tracing::debug;

use super::registry_integration;
use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::Port;
use crate::error::Result;

pub struct TypeDiscoveryContext {
    /// Type information from Bevy's registry
    type_info: HashMap<String, UnifiedTypeInfo>,
}

impl TypeDiscoveryContext {
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

        Ok(Self { type_info })
    }

    /// Get the underlying `HashMap` (temporary for compatibility)
    /// This method will be removed in Phase 4 when all discovery levels
    /// are updated to work directly with `TypeDiscoveryContext`
    pub const fn as_hashmap(&self) -> &HashMap<String, UnifiedTypeInfo> {
        &self.type_info
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_type_names() {
        let context = TypeDiscoveryContext::fetch_from_registry(Port(15702), vec![])
            .await
            .unwrap();
        assert!(context.as_hashmap().is_empty());
    }

    #[tokio::test]
    async fn test_as_hashmap() {
        let port = Port(15702);
        let context = TypeDiscoveryContext::fetch_from_registry(port, vec![])
            .await
            .unwrap();

        // Should return empty HashMap when no types provided
        assert!(context.as_hashmap().is_empty());
    }

    // Integration tests would go in the tests/ directory to test with actual BRP
}
