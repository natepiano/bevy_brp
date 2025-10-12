//! Category of type for quick identification and processing
//!
//! This enum represents the actual type kinds returned by Bevy's type registry.
//! These correspond to the "kind" field in registry schema responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

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
    /// Regular struct type
    Struct,
    /// Set type (`HashSet`, `BTreeSet`, etc.)
    Set,
    /// Tuple type
    Tuple,
    /// Tuple struct type
    TupleStruct,
    /// Value type (primitive types like i32, f32, bool, String)
    Value,
}

impl TypeKind {
    /// Extract `TypeKind` from a registry schema with fallback to `Value`
    ///
    /// Some types don't have a `kind` field in their schema because Bevy's reflection
    /// system doesn't provide full schema information for them. This includes:
    /// - External opaque types like `Uuid`, `Entity`
    /// - Standard library types like `String` that are referenced but not fully introspected
    /// - `NonZero*` types and other primitives without complete reflection data
    ///
    /// These types are safely treated as `TypeKind::Value` (leaf/primitive types).
    pub fn from_schema(schema: &Value) -> Self {
        schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| s.parse().ok())
            .unwrap_or(Self::Value)
    }

    /// Returns appropriate terminology for child elements of this type
    ///
    /// Used in descriptions to provide type-specific language instead of generic "descendants".
    /// For example, a Struct has "fields", an Array has "elements", a Map has "entries", etc.
    pub const fn child_terminology(&self) -> &'static str {
        match self {
            Self::Struct => "fields",
            Self::Enum => "variants",
            Self::Map => "entries",
            Self::Array | Self::List | Self::Set | Self::Tuple | Self::TupleStruct => "elements",
            Self::Value => "components",
        }
    }
}
