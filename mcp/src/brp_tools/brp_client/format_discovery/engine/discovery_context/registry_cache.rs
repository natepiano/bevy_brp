//! Global registry cache for type information
//!
//! This module provides a thread-safe global cache for registry-derived
//! type information that persists across tool invocations.

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::brp_tools::brp_client::format_discovery::engine::discovery_context::types::CachedTypeInfo;
use crate::brp_tools::brp_client::format_discovery::engine::types::BrpTypeName;

/// Global registry cache shared across all tool invocations
static REGISTRY_CACHE: Lazy<RegistryCache> = Lazy::new(RegistryCache::new);

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

    /// Clear all cached entries
    pub fn clear(&self) {
        self.types.clear();
    }

    /// Get the number of cached types
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Check if the cache is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

/// Get the global registry cache instance
pub fn global_cache() -> &'static RegistryCache {
    &REGISTRY_CACHE
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use serde_json::json;

    use super::*;
    use crate::brp_tools::brp_client::format_discovery::engine::discovery_context::types::{
        BrpFormats, SerializationFormat,
    };

    #[test]
    fn test_cache_operations() {
        let cache = RegistryCache::new();
        let type_name: BrpTypeName = "test::Type".into();

        // Initially empty
        assert!(cache.get(&type_name).is_none());

        // Insert and retrieve
        let info = CachedTypeInfo {
            registry_schema: json!({"test": "data"}),
            brp_formats:     BrpFormats {
                spawn_format:         json!({}),
                mutation_paths:       vec![],
                serialization_format: SerializationFormat::Object,
            },
            cached_at:       Instant::now(),
        };

        cache.insert(type_name.clone(), info.clone());
        let retrieved = cache.get(&type_name);
        assert!(retrieved.is_some());

        // Clear cache
        cache.clear();
        assert!(cache.get(&type_name).is_none());
    }
}
