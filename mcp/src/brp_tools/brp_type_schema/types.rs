//! Type definitions for registry-based type discovery
//!
//! This module contains type structures used for caching and comparing
//! registry-derived type information with extras-based discovery.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display};

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub struct BrpFormatKnowledge {
    /// Example value in the correct BRP format
    pub example_value:  Value,
    /// Subfield paths for types that support subfield mutation (e.g., Vec3 has x,y,z)
    /// Each tuple is (`subfield_name`, `example_value`)
    pub subfield_paths: Option<Vec<(&'static str, Value)>>,
}

/// A newtype wrapper for BRP type names used as `HashMap` keys
///
/// This type provides documentation and type safety for strings that represent
/// fully-qualified Rust type names (e.g., "`bevy_transform::components::transform::Transform`")
/// when used as keys in type information maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrpTypeName(String);

impl BrpTypeName {
    /// Get the underlying string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrpTypeName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrpTypeName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&String> for BrpTypeName {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl std::fmt::Display for BrpTypeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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
    pub type_category:        TypeKind,
    /// For enums, list of variant names
    pub enum_variants:        Option<Vec<String>>,
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
    #[allow(dead_code)] // Used in response building when tool is called
    pub example_value: Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:          String,
    /// For enum types, list of valid variant names
    pub enum_variants: Option<Vec<String>>,
    /// Type information for this path
    pub type_name:     Option<String>,
}

/// Category of type for quick identification and processing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "PascalCase")]
pub enum TypeKind {
    /// Unknown or unclassified type
    Unknown,
    /// Regular struct type
    Struct,
    /// Tuple struct type
    TupleStruct,
    /// Enum type
    Enum,
    /// Math type (Vec2, Vec3, Quat, etc.)
    MathType,
    /// Component type
    Component,
}

impl From<&str> for TypeKind {
    fn from(s: &str) -> Self {
        match s {
            "Struct" => Self::Struct,
            "TupleStruct" => Self::TupleStruct,
            "Enum" => Self::Enum,
            _ => Self::Unknown,
        }
    }
}
