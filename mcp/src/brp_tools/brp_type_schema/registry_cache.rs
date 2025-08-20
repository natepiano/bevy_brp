//! Global registry cache for type information
//!
//! This module provides a thread-safe global cache for registry-derived
//! type information that persists across tool invocations.

use std::collections::HashMap;
use std::sync::LazyLock;

use dashmap::DashMap;
use serde_json::{Value, json};

use super::types::{BrpTypeName, CachedTypeInfo};
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::Result;
use crate::tool::BrpMethod;

/// Global registry cache shared across all tool invocations
pub static REGISTRY_CACHE: LazyLock<RegistryCache> = LazyLock::new(RegistryCache::new);

/// Cache for complete registries by port
static FULL_REGISTRY_CACHE: LazyLock<DashMap<Port, HashMap<BrpTypeName, Value>>> =
    LazyLock::new(DashMap::new);

/// Thread-safe cache for registry type information
pub struct RegistryCache {
    /// Map of type names to cached information
    types: DashMap<BrpTypeName, CachedTypeInfo>,
}

impl RegistryCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            types: DashMap::new(),
        }
    }

    /// Get cached type information if available
    pub fn get(&self, type_name: &BrpTypeName) -> Option<CachedTypeInfo> {
        self.types.get(type_name).map(|entry| entry.clone())
    }

    /// Insert or update cached type information
    pub fn insert(&self, type_name: BrpTypeName, info: CachedTypeInfo) {
        self.types.insert(type_name, info);
    }
}

/// Get the complete registry for a port, with caching
pub async fn get_full_registry(port: Port) -> Result<HashMap<BrpTypeName, Value>> {
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
                    let type_name = BrpTypeName::from(key);
                    registry_map.insert(type_name, value.clone());
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
