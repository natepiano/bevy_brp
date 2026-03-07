//! Newtype wrappers for mutation path building
//!
//! This module contains simple newtype wrappers that provide type safety and domain-specific
//! behavior for strings used throughout the mutation path building system.

use std::ops::Deref;

use serde::Deserialize;
use serde::Serialize;

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
