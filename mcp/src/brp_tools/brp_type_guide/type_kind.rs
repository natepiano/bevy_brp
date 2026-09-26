//! Category of type for quick identification and processing
//!
//! This enum represents the actual type kinds returned by Bevy's type registry.
//! These correspond to the "kind" field in registry schema responses.

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use strum::AsRefStr;
use strum::Display;
use strum::EnumString;

use crate::support::JsonObjectAccess;
use crate::support::SchemaField;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRefStr, EnumString)]
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
pub(crate) enum TypeKind {
    /// Array type
    Array,
    /// Enum type
    Enum,
    /// List type
    List,
    /// Map type (`HashMap`, `BTreeMap`, etc.)
    Map,
    /// Regular struct type
    Struct,
    /// Set type (`HashSet`, `BTreeSet`, etc.)
    Set,
    /// Tuple type
    Tuple,
    /// Tuple struct type
    TupleStruct,
    /// Value type (primitive types like `i32`, `f32`, `bool`, `String`)
    Value,
}

impl TypeKind {
    /// Returns appropriate terminology for child elements of this type
    ///
    /// Used in descriptions to provide type-specific language instead of generic "descendants".
    /// For example, a Struct has "fields", an Array has "elements", a Map has "entries", etc.
    pub(super) const fn child_terminology(&self) -> &'static str {
        match self {
            Self::Struct => "fields",
            Self::Enum => "variants",
            Self::Map => "entries",
            Self::Array | Self::List | Self::Set | Self::Tuple | Self::TupleStruct => "elements",
            Self::Value => "components",
        }
    }

    /// Returns the lowercase noun naming this kind in mutation path descriptions
    ///
    /// `TupleStruct` reads as "tuple struct", which `AsRef<str>` plus `to_lowercase` would
    /// render as "tuplestruct".
    pub(super) const fn description_label(&self) -> &'static str {
        match self {
            Self::Array => "array",
            Self::Enum => "enum",
            Self::List => "list",
            Self::Map => "map",
            Self::Struct => "struct",
            Self::Set => "set",
            Self::Tuple => "tuple",
            Self::TupleStruct => "tuple struct",
            Self::Value => "value",
        }
    }
}

impl From<&Value> for TypeKind {
    /// Extract `TypeKind` from a registry schema with fallback to `Value`
    ///
    /// Some types don't have a `kind` field in their schema because Bevy's reflection
    /// system doesn't provide full schema information for them. This includes:
    /// - External opaque types like `Uuid`, `Entity`
    /// - Standard library types like `String` that are referenced but not fully introspected
    /// - `NonZero*` types and other primitives without complete reflection data
    ///
    /// These types are safely treated as `TypeKind::Value` (leaf/primitive types).
    fn from(schema: &Value) -> Self {
        schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| s.parse().ok())
            .unwrap_or(Self::Value)
    }
}
