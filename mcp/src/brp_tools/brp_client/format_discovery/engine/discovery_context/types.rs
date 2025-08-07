//! Type definitions for registry-based type discovery
//!
//! This module contains type structures used for caching and comparing
//! registry-derived type information with extras-based discovery.

use serde_json::Value;
use strum::{AsRefStr, Display};

use super::super::types::TypeCategory;

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub struct BrpFormatKnowledge {
    /// Example value in the correct BRP format
    pub example_value:  Value,
    /// Subfield paths for types that support subfield mutation (e.g., Vec3 has x,y,z)
    /// Each tuple is (`subfield_name`, `example_value`)
    pub subfield_paths: Option<Vec<(&'static str, Value)>>,
}

/// Cached type information from registry
#[derive(Debug, Clone)]
pub struct CachedTypeInfo {
    /// Mutation paths available for this type
    pub mutation_paths:       Vec<MutationPath>,
    /// Raw registry schema response
    #[allow(dead_code)]
    pub registry_schema:      Value,
    /// Reflection types from registry (e.g., `["Component", "Serialize", "Deserialize"]`)
    pub reflect_types:        Vec<String>,
    /// Full object format for spawn/insert
    pub spawn_format:         Value,
    /// Operations supported by this type in BRP
    pub supported_operations: Vec<BrpSupportedOperation>,
    /// Category of this type (Struct, Enum, etc.)
    pub type_category:        TypeCategory,
}

/// Enum for BRP supported operations
/// Each operation has specific requirements based on type registration and traits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum BrpSupportedOperation {
    /// Get operation - requires type in registry
    Get,
    /// Insert operation - requires Serialize + Deserialize traits
    Insert,
    /// Mutate operation - requires mutable type (struct/tuple)
    Mutate,
    /// Query operation - requires type in registry
    Query,
    /// Spawn operation - requires Serialize + Deserialize traits
    Spawn,
}

/// Mutation path information
#[derive(Debug, Clone)]
pub struct MutationPath {
    /// Example value for this path
    pub example_value: Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:          String,
}
