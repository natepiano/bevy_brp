//! Global registry cache for type information
//!
//! This module provides a thread-safe global cache for registry-derived
//! type information that persists across tool invocations.

use std::sync::LazyLock;

use dashmap::DashMap;

use super::types::{BrpTypeName, CachedTypeInfo};

/// Global registry cache shared across all tool invocations
pub static REGISTRY_CACHE: LazyLock<RegistryCache> = LazyLock::new(RegistryCache::new);

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
