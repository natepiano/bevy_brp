//! JSON schema type definitions for MCP tools
//!
//! This module provides standardized JSON schema type names used across
//! the MCP tool ecosystem for parameter and response schema generation.

use serde::Serialize;
use serde_json::Value;
use strum::AsRefStr;
use strum::Display;
use strum::EnumString;

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

impl From<JsonSchemaType> for Value {
    fn from(schema_type: JsonSchemaType) -> Self { Self::String(schema_type.as_ref().to_string()) }
}

/// Registry schema field names
///
/// This enum provides type-safe field names for JSON schema structures.
/// It's used throughout the codebase to avoid hardcoded strings when
/// accessing schema fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr)]
#[strum(serialize_all = "camelCase")]
pub enum SchemaField {
    /// The additionalProperties field for `HashMap` types
    AdditionalProperties,
    /// The anyOf field for union types
    #[strum(serialize = "anyOf")]
    AnyOf,
    /// The const field for constant values
    Const,
    /// The crate name field
    CrateName,
    /// The $defs field for schema definitions
    #[strum(serialize = "$defs")]
    Defs,
    /// The description field
    Description,
    /// The items field for array types
    Items,
    /// Map Key
    Key,
    /// The keyType field for map types
    KeyType,
    /// The kind field for type categories
    Kind,
    /// The module path field
    ModulePath,
    /// The oneOf field for enum variants
    OneOf,
    /// The prefixItems field for tuple types
    PrefixItems,
    /// The properties field for object types
    Properties,
    /// The $ref field for type references
    #[strum(serialize = "$ref")]
    Ref,
    /// The reflect types field
    ReflectTypes,
    /// The required field for object types
    Required,
    /// The short path field
    ShortPath,
    /// The type field
    Type,
    /// The type path field (e.g., "`bevy_color::color::Color::Srgba`")
    TypePath,
    /// Map Value
    Value,
    /// The valueType field for map types
    ValueType,
}
