//! Public API response types for the `brp_type_schema` tool
//!
//! This module contains the strongly-typed structures that form the public API
//! for type schema discovery results. These types are separate from the internal
//! processing types to provide a clean, stable API contract.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

use super::constants::SCHEMA_REF_PREFIX;
use super::mutation_path_builder::TypeKind;
use super::type_info::TypeInfo;
use crate::string_traits::JsonFieldAccess;

/// Enum for BRP supported operations
/// Each operation has specific requirements based on type registration and traits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, AsRefStr, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
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

impl From<BrpSupportedOperation> for String {
    fn from(op: BrpSupportedOperation) -> Self {
        op.as_ref().to_string()
    }
}

/// A newtype wrapper for BRP type names used as `HashMap` keys
///
/// This type provides documentation and type safety for strings that represent
/// fully-qualified Rust type names (e.g., "`bevy_transform::components::transform::Transform`")
/// when used as keys in type information maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct BrpTypeName(String);

impl BrpTypeName {
    /// Create a `BrpTypeName` representing an unknown type
    ///
    /// This is commonly used as a fallback when type information
    /// is not available or cannot be determined.
    pub fn unknown() -> Self {
        Self("unknown".to_string())
    }

    /// Get the underlying string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the type string for comparison
    /// This is an alias for `as_str()` but with clearer intent
    pub fn type_string(&self) -> &str {
        &self.0
    }

    /// Extract the base type name by stripping generic parameters
    /// For example: `Vec<String>` returns `Some("Vec")`
    pub fn base_type(&self) -> Option<&str> {
        self.0.split('<').next()
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

impl From<BrpTypeName> for String {
    fn from(type_name: BrpTypeName) -> Self {
        type_name.0
    }
}

impl From<&BrpTypeName> for String {
    fn from(type_name: &BrpTypeName) -> Self {
        type_name.0.clone()
    }
}

impl std::fmt::Display for BrpTypeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<BrpTypeName> for Value {
    fn from(type_name: BrpTypeName) -> Self {
        Self::String(type_name.0)
    }
}

impl From<&BrpTypeName> for Value {
    fn from(type_name: &BrpTypeName) -> Self {
        Self::String(type_name.0.clone())
    }
}

/// Schema information extracted from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Category of the type (Struct, Enum, etc.) from registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_kind:   Option<TypeKind>,
    /// Field definitions from the registry schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties:  Option<Value>,
    /// Required fields list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required:    Option<Vec<String>>,
    /// Module path of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
    /// Crate name of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crate_name:  Option<String>,
}

/// Math type component names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum MathComponent {
    X,
    Y,
    Z,
    W,
}

impl From<MathComponent> for String {
    fn from(component: MathComponent) -> Self {
        component.as_ref().to_string()
    }
}

impl TryFrom<&str> for MathComponent {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "x" => Ok(Self::X),
            "y" => Ok(Self::Y),
            "z" => Ok(Self::Z),
            "w" => Ok(Self::W),
            _ => Err(()),
        }
    }
}

/// Bevy reflection trait names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum ReflectTrait {
    Component,
    Resource,
    Serialize,
    Deserialize,
}

/// Registry schema field names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr)]
#[strum(serialize_all = "camelCase")]
pub enum SchemaField {
    CrateName,
    Items,
    Kind,
    ModulePath,
    OneOf,
    PrefixItems,
    Properties,
    #[strum(serialize = "$ref")]
    Ref,
    ReflectTypes,
    Required,
    ShortPath,
    Type,
}

impl SchemaField {
    /// Extract field type from field info JSON
    ///
    /// This extracts the type reference from a field definition in the schema,
    /// handling the standard pattern of type.$ref with #/$defs/ prefix.
    pub fn extract_field_type(field_info: &Value) -> Option<BrpTypeName> {
        field_info
            .get_field(Self::Type)
            .and_then(|t| t.get_field(Self::Ref))
            .and_then(Value::as_str)
            .and_then(|ref_str| ref_str.strip_prefix(SCHEMA_REF_PREFIX))
            .map(BrpTypeName::from)
    }
}

/// response structure
#[derive(Debug, Clone, Serialize)]
pub struct TypeSchemaResponse {
    /// Number of types successfully discovered
    pub discovered_count: usize,
    /// List of type names that were requested
    pub requested_types:  Vec<String>,
    /// Summary statistics for the discovery operation
    pub summary:          TypeSchemaSummary,
    /// Detailed information for each type, keyed by type name
    pub type_info:        HashMap<BrpTypeName, TypeInfo>,
}

/// Summary statistics for the discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaSummary {
    /// Number of types that failed discovery
    pub failed_discoveries:     usize,
    /// Number of types successfully discovered
    pub successful_discoveries: usize,
    /// Total number of types requested
    pub total_requested:        usize,
}
