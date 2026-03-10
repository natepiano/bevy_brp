use serde::Deserialize;
use serde::Serialize;

use crate::json_schema::SchemaField;

/// Newtype for a struct field name used in mutation paths and variant signatures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub(super) struct StructFieldName(String);

impl StructFieldName {
    /// Get the field name as a string slice
    pub(super) fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for StructFieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl std::borrow::Borrow<str> for StructFieldName {
    fn borrow(&self) -> &str { &self.0 }
}

impl From<String> for StructFieldName {
    fn from(s: String) -> Self { Self(s) }
}

impl From<&str> for StructFieldName {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

impl From<SchemaField> for StructFieldName {
    fn from(field: SchemaField) -> Self { Self(field.to_string()) }
}
