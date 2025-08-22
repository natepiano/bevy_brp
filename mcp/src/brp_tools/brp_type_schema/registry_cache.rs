//! Global registry cache for complete type registries
//!
//! This module provides a thread-safe global cache for complete type registries
//! that persists across tool invocations.

use std::collections::HashMap;
use std::sync::LazyLock;

use dashmap::DashMap;
use serde_json::{Value, json};

use super::types::BrpTypeName;
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::Result;
use crate::tool::BrpMethod;

/// Cache for complete registries by port
static FULL_REGISTRY_CACHE: LazyLock<DashMap<Port, HashMap<BrpTypeName, Value>>> =
    LazyLock::new(DashMap::new);

/// Get the complete registry for a port, with caching
///
/// If `force_refresh` is true, the cache for this port will be cleared before fetching,
/// ensuring fresh data is retrieved from the BRP server.
pub async fn get_full_registry(
    port: Port,
    force_refresh: bool,
) -> Result<HashMap<BrpTypeName, Value>> {
    // If force_refresh is true, remove the cached entry first
    if force_refresh {
        FULL_REGISTRY_CACHE.remove(&port);
    }

    // Check cache first
    if let Some(cached_registry) = FULL_REGISTRY_CACHE.get(&port) {
        return Ok(cached_registry.clone());
    }

    // Fetch full registry from BRP
    let client = BrpClient::new(BrpMethod::BevyRegistrySchema, port, Some(json!({})));

    match client.execute_raw().await {
        Ok(ResponseStatus::Success(Some(registry_data))) => {
            // Convert to HashMap with BrpTypeName keys
            let mut registry_map = HashMap::new();

            if let Some(obj) = registry_data.as_object() {
                for (key, value) in obj {
                    let brp_type_name = BrpTypeName::from(key);
                    registry_map.insert(brp_type_name, value.clone());
                }
            }

            // Cache the result
            FULL_REGISTRY_CACHE.insert(port, registry_map.clone());
            Ok(registry_map)
        }
        Ok(_) => Err(crate::error::Error::BrpCommunication(
            "Registry call returned no data".to_string(),
        )
        .into()),
        Err(e) => Err(e),
    }
}
