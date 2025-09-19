//! A newtype wrapper for BRP type names used throughout the system
//!
//! This module provides the `BrpTypeName` type which represents fully-qualified
//! Rust type names (e.g., "bevy_transform::components::transform::Transform")
//! with various utility methods for working with these names.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::mutation_path_builder::MutationKnowledge;

/// A newtype wrapper for BRP type names used as `HashMap` keys
///
/// This type provides documentation and type safety for strings that represent
/// fully-qualified Rust type names (e.g., "`bevy_transform::components::transform::Transform`")
/// when used as keys in type information maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
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

    /// Check if this type is a Handle wrapper type
    /// Returns true for types like `bevy_asset::handle::Handle<...>`
    pub fn is_handle(&self) -> bool {
        self.0.starts_with("bevy_asset::handle::Handle<")
    }

    /// Get the short name (last segment after ::)
    /// For example: `bevy_transform::components::transform::Transform` returns `Transform`
    /// For generic types: `HashMap<String, i32>` returns `HashMap<String, i32>`
    /// For arrays: `[glam::Vec3; 2]` returns `[Vec3; 2]`
    pub fn short_name(&self) -> String {
        // Handle generic types like HashMap<K, V, H> - return just the base type for descriptions
        if let Some(angle_pos) = self.0.find('<') {
            let base_type = &self.0[..angle_pos];
            let short_base = base_type.rsplit("::").next().unwrap_or(base_type);
            return short_base.to_string(); // Just return "Axis", not "Axis<...>"
        }

        // Special handling for array types like [Type; size]
        if self.0.starts_with('[') && self.0.ends_with(']') {
            // For arrays, we need to shorten the inner type but keep the array syntax
            if let Some(semicolon_pos) = self.0.rfind(';')
                && let Some(bracket_pos) = self.0.find('[')
            {
                let inner_type = &self.0[bracket_pos + 1..semicolon_pos];
                let size_part = &self.0[semicolon_pos..];
                let short_inner = inner_type.rsplit("::").next().unwrap_or(inner_type);
                return format!("[{short_inner}{size_part}");
            }
        }

        // Find the last :: and take everything after it
        // If no :: found, return the whole name (handles primitives and generics)
        self.0.rsplit("::").next().unwrap_or(&self.0).to_string()
    }

    /// Get the display name for this type, using simplified name from knowledge if available
    pub fn display_name(&self) -> Self {
        MutationKnowledge::get_simplified_name(self).unwrap_or_else(|| self.clone())
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
