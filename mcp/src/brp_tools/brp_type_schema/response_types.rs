//! Public API response types for the `brp_type_schema` tool
//!
//! This module contains the strongly-typed structures that form the public API
//! for type schema discovery results. These types are separate from the internal
//! processing types to provide a clean, stable API contract.

use std::collections::HashMap;
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

use super::constants::SCHEMA_REF_PREFIX;
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

/// Information about a field in an enum struct variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumFieldInfo {
    /// Field name
    pub field_name: String,
    /// Field type
    #[serde(rename = "type")]
    pub type_name:  BrpTypeName,
}

/// Information about an enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariantInfo {
    /// Name of the variant
    pub variant_name: String,
    /// Type of the variant (Unit, Tuple, Struct)
    pub variant_kind: EnumVariantKind,
    /// Fields for struct variants
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields:       Option<Vec<EnumFieldInfo>>,
    /// Types for tuple variants
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuple_types:  Option<Vec<BrpTypeName>>,
}

/// Enum variant classification
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, EnumString, Serialize, Deserialize,
)]
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
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

impl From<MathComponent> for String {
    fn from(component: MathComponent) -> Self {
        component.as_ref().to_string()
    }
}

/// Context for a mutation path describing what kind of mutation this is
#[derive(Debug, Clone)]
pub enum MutationPathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField {
        field_name:  String,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in a tuple by index
    TupleElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in an array
    ArrayElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
    /// Complex nested path (fallback for complicated paths)
    NestedPath {
        components: Vec<String>,
        final_type: BrpTypeName,
    },
}

impl MutationPathKind {
    /// Generate a human-readable description for this mutation
    pub fn description(&self) -> String {
        match self {
            Self::RootValue { type_name } => {
                format!("Replace the entire {type_name} value")
            }
            Self::StructField {
                field_name,
                parent_type,
            } => {
                format!("Mutate the {field_name} field of {parent_type}")
            }
            Self::TupleElement { index, parent_type } => {
                format!("Mutate element {index} of {parent_type}")
            }
            Self::ArrayElement { index, parent_type } => {
                format!("Mutate element [{index}] of {parent_type}")
            }
            Self::NestedPath {
                components,
                final_type,
            } => {
                if components.len() == 2 {
                    format!(
                        "Mutate the {} component of {} (type: {})",
                        components[1], components[0], final_type
                    )
                } else {
                    format!(
                        "Mutate nested path: {} (type: {})",
                        components.join("."),
                        final_type
                    )
                }
            }
        }
    }

    /// Get just the variant name for serialization
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::RootValue { .. } => "RootValue",
            Self::StructField { .. } => "StructField",
            Self::TupleElement { .. } => "TupleElement",
            Self::ArrayElement { .. } => "ArrayElement",
            Self::NestedPath { .. } => "NestedPath",
        }
    }
}

impl Display for MutationPathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.variant_name())
    }
}

impl Serialize for MutationPathKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path
    pub example:       Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:          String,
    /// For enum types, list of valid variant names
    pub enum_variants: Option<Vec<String>>,
    /// Type information for this path
    pub type_name:     BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:     MutationPathKind,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize)]
pub struct MutationPath {
    /// Human-readable description of what this path mutates
    pub description:   String,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name:     BrpTypeName,
    /// Example value for mutations (for non-Option types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:       Option<Value>,
    /// Example value for setting Some variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_some:  Option<Value>,
    /// Example value for setting None variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_none:  Option<Value>,
    /// List of valid enum variants for this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_variants: Option<Vec<String>>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:          Option<String>,
    /// Context metadata describing the kind of mutation this path represents
    pub path_kind:     MutationPathKind,
}

impl MutationPath {
    /// Create from internal `MutationPath` with proper formatting logic
    pub fn from_mutation_path(
        path: &MutationPathInternal,
        description: String,
        is_option: bool,
    ) -> Self {
        if is_option {
            // For Option types, check if we have the special format
            if let Some(some_val) = path.example.get_field(OptionField::Some)
                && let Some(none_val) = path.example.get_field(OptionField::None)
            {
                return Self {
                    description,
                    type_name: path.type_name.clone(),
                    example: None,
                    example_some: Some(some_val.clone()),
                    example_none: Some(none_val.clone()),
                    enum_variants: path.enum_variants.clone(),
                    note: Some(
                        "For Option fields: pass the value directly to set Some, null to set None"
                            .to_string(),
                    ),
                    path_kind: path.path_kind.clone(),
                };
            }
        }

        // Regular non-Option path
        Self {
            description,
            type_name: path.type_name.clone(),
            example: if path.example.is_null() {
                None
            } else {
                Some(path.example.clone())
            },
            example_some: None,
            example_none: None,
            enum_variants: path.enum_variants.clone(),
            note: None,
            path_kind: path.path_kind.clone(),
        }
    }
}

/// Option field keys for JSON representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum OptionField {
    Some,
    None,
}

impl From<OptionField> for String {
    fn from(field: OptionField) -> Self {
        field.as_ref().to_string()
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

impl TypeKind {
    /// Extract `TypeKind` from a registry schema with fallback to `Value`
    pub fn from_schema(schema: &Value, type_name: &BrpTypeName) -> Self {
        schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Type '{}' has missing or invalid 'kind' field in registry schema, defaulting to TypeKind::Value",
                    type_name
                );
                Self::Value
            })
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
