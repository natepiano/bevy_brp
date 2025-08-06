//! Type definitions for registry-based type discovery
//!
//! This module contains type structures used for caching and comparing
//! registry-derived type information with extras-based discovery.

use std::time::Instant;

use serde_json::Value;

use super::super::types::BrpTypeName;

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub struct BrpFormatKnowledge {
    /// Example value in the correct BRP format
    pub example_value:        Value,
    /// How this type should be serialized for BRP
    pub serialization_format: SerializationFormat,
}

/// Cached type information from registry
#[derive(Debug, Clone)]
pub struct CachedTypeInfo {
    /// When this was cached
    pub cached_at:            Instant,
    /// Mutation paths available for this type
    pub mutation_paths:       Vec<MutationPath>,
    /// Raw registry schema response
    pub registry_schema:      Value,
    /// The serialization format for this type
    pub serialization_format: SerializationFormat,
    /// Full object format for spawn/insert
    pub spawn_format:         Value,
    /// Operations supported by this type in BRP
    pub supported_operations: Vec<BrpSupportedOperation>,
}

/// Enum for serialization format (not bare strings)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SerializationFormat {
    /// Array format, e.g., Vec3 as [1.0, 2.0, 3.0]
    Array,
    /// Object format, e.g., Transform as {translation: {...}}
    Object,
    /// Primitive value, e.g., f32, bool
    Primitive,
    /// Enum with special handling
    Enum,
}

/// Enum for BRP supported operations
/// Each operation has specific requirements based on type registration and traits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Type of the value at this path
    pub value_type:    BrpTypeName,
}

