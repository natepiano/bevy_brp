//! Type definitions for registry-based type discovery
//!
//! This module contains type structures used for caching and comparing
//! registry-derived type information with extras-based discovery.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub struct BrpFormatKnowledge {
    /// Example value in the correct BRP format
    pub example_value:  Value,
    /// Subfield paths for types that support subfield mutation (e.g., Vec3 has x,y,z)
    /// Each tuple is (`component_name`, `example_value`)
    pub subfield_paths: Option<Vec<(MathComponent, Value)>>,
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

/// Enum variant classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EnumVariantKind {
    Tuple,
    Struct,
    Unit,
}

/// Math type component names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum MathComponent {
    X,
    Y,
    Z,
    W,
}

/// Mutation path information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPath {
    /// Example value for this path
    #[allow(dead_code)] // Used in response building when tool is called
    pub example: Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:          String,
    /// For enum types, list of valid variant names
    pub enum_variants: Option<Vec<String>>,
    /// Type information for this path
    pub type_name:     Option<String>,
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
    Kind,
    OneOf,
    ShortPath,
    PrefixItems,
    Properties,
    Type,
    #[strum(serialize = "$ref")]
    Ref,
    ReflectTypes,
}

/// JSON schema type names for type schema generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, Serialize, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JsonSchemaType {
    Object,
    Array,
    String,
    Number,
    Integer,
    Boolean,
    Null,
}

/// Category of type for quick identification and processing
///
/// This enum represents the actual type kinds returned by Bevy's type registry.
/// These correspond to the "kind" field in registry schema responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRefStr, EnumString)]
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
pub enum TypeKind {
    /// Array type
    Array,
    /// Enum type
    Enum,
    /// List type
    List,
    /// Map type (`HashMap`, `BTreeMap`, etc.)
    Map,
    /// Option type
    Option,
    /// Regular struct type
    Struct,
    /// Tuple type
    Tuple,
    /// Tuple struct type
    TupleStruct,
    /// Value type (primitive types like i32, f32, bool, String)
    Value,
}

impl From<BrpSupportedOperation> for String {
    fn from(op: BrpSupportedOperation) -> Self {
        op.as_ref().to_string()
    }
}

impl From<MathComponent> for String {
    fn from(component: MathComponent) -> Self {
        component.as_ref().to_string()
    }
}
