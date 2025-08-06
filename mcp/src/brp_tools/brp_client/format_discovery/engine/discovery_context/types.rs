//! Type definitions for registry-based type discovery
//!
//! This module contains type structures used for caching and comparing
//! registry-derived type information with extras-based discovery.

use std::time::Instant;

use serde_json::Value;

use crate::brp_tools::brp_client::format_discovery::engine::types::BrpTypeName;

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub struct BrpFormatKnowledge {
    /// How this type should be serialized for BRP
    pub serialization_format: SerializationFormat,
    /// Example value in the correct BRP format
    pub example_value:        Value,
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

/// Enum for BRP operation context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrpOperationContext {
    /// Spawn operation
    Spawn,
    /// Insert operation
    Insert,
    /// Mutate operation
    Mutate,
    /// Resource operation
    Resource,
}

/// Cached type information from registry
#[derive(Debug, Clone)]
pub struct CachedTypeInfo {
    /// Raw registry schema response
    pub registry_schema: Value,
    /// BRP-formatted examples for different operations
    pub brp_formats:     BrpFormats,
    /// When this was cached
    pub cached_at:       Instant,
}

/// BRP-specific format information
#[derive(Debug, Clone)]
pub struct BrpFormats {
    /// Full object format for spawn/insert
    pub spawn_format:         Value,
    /// Mutation paths available for this type
    pub mutation_paths:       Vec<MutationPath>,
    /// The serialization format for this type
    pub serialization_format: SerializationFormat,
}

/// Mutation path information
#[derive(Debug, Clone)]
pub struct MutationPath {
    /// Path for mutation, e.g., ".translation.x"
    pub path:          String,
    /// Example value for this path
    pub example_value: Value,
    /// Type of the value at this path
    pub value_type:    BrpTypeName,
}

/// Comparison results between extras and registry formats
#[derive(Debug, Clone)]
pub struct RegistryComparison {
    /// Format from extras plugin
    pub extras_format:   Option<Value>,
    /// Format derived from registry
    pub registry_format: Option<Value>,
    /// Differences found between formats
    pub differences:     Vec<FormatDifference>,
}

/// Types of differences found during comparison
#[derive(Debug, Clone)]
pub enum FormatDifference {
    /// Structure type mismatch (e.g., array vs object)
    StructureType {
        path:     String,
        extras:   String,
        registry: String,
    },
    /// Field missing in one source
    MissingField {
        path:   String,
        source: ComparisonSource,
    },
    /// Value type mismatch
    ValueType {
        path:     String,
        extras:   String,
        registry: String,
    },
}

/// Source of comparison data
#[derive(Debug, Clone, Copy)]
pub enum ComparisonSource {
    /// From extras plugin
    Extras,
    /// From registry
    Registry,
}
