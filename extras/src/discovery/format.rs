//! Format discovery structures for BRP component introspection
//!
//! This module provides structures for discovering and representing component
//! serialization information that can be used for proper BRP operation formatting.

use serde::{Deserialize, Serialize};

use super::types::{MutationInfo, SerializationFormat};

/// Complete format information for component and resource types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryInfo {
    /// The fully-qualified type name
    pub type_name:            String,
    /// Serialization format for types
    pub serialization_format: SerializationFormat,
    /// Format information for mutation operations
    pub mutation_info:        MutationInfo,
}
