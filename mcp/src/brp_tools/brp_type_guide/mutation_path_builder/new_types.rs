//! Newtype wrappers for mutation path building
//!
//! This module contains simple newtype wrappers that provide type safety and domain-specific
//! behavior for strings used throughout the mutation path building system.

use std::ops::Deref;

use serde::Deserialize;
use serde::Serialize;

use crate::json_schema::SchemaField;

/// Newtype for a mutation path used in BRP operations (e.g., ".translation.x")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationPath(String);

impl Deref for MutationPath {
    type Target = String;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl From<String> for MutationPath {
    fn from(path: String) -> Self { Self(path) }
}

impl From<&str> for MutationPath {
    fn from(path: &str) -> Self { Self(path.to_string()) }
}

impl std::fmt::Display for MutationPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

/// Newtype for a struct field name used in mutation paths and variant signatures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct StructFieldName(String);

impl StructFieldName {
    /// Get the field name as a string slice
    pub fn as_str(&self) -> &str { &self.0 }
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

/// Newtype for variant name from a Bevy enum type (e.g., "`Option<String>::Some`",
/// "`Color::Srgba`")
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct VariantName(String);

impl From<String> for VariantName {
    fn from(name: String) -> Self { Self(name) }
}

impl std::fmt::Display for VariantName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl VariantName {
    /// Get just the short variant name without the enum prefix (e.g., "Srgba" from
    /// "`Color::Srgba`")
    pub fn short_name(&self) -> &str { self.0.rsplit_once("::").map_or(&self.0, |(_, name)| name) }
}
