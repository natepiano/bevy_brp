//! A newtype wrapper for BRP type names used throughout the system
//!
//! This module provides the `BrpTypeName` type which represents fully-qualified
//! Rust type names (e.g., `"bevy_transform::components::transform::Transform"`)
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

    /// Shorten an enum type name while preserving generic parameters
    /// e.g., `"core::option::Option<alloc::string::String>"` → `"Option<String>"`
    /// e.g., `"extras_plugin::TestEnumWithSerDe"` → `"TestEnumWithSerDe"`
    pub fn short_enum_type_name(&self) -> String {
        let type_str = &self.0;

        // Find generic bracket if present
        type_str.find('<').map_or_else(
            || {
                // No generics, just shorten the type name
                type_str.rsplit("::").next().unwrap_or(type_str).to_string()
            },
            |angle_pos| {
                // Split into base type and generic params
                let base_type = &type_str[..angle_pos];
                let generic_part = &type_str[angle_pos..];

                // Shorten the base type
                let short_base = base_type.rsplit("::").next().unwrap_or(base_type);

                // Process generic parameters recursively
                let mut result = String::from(short_base);
                result.push('<');

                // Simple approach: shorten each :: separated segment within generics
                let inner = &generic_part[1..generic_part.len() - 1]; // Remove < >
                let parts: Vec<String> = inner
                    .split(',')
                    .map(|part| {
                        let trimmed = part.trim();
                        // For each type in the generic params, take the last component
                        if trimmed.contains("::") {
                            trimmed.rsplit("::").next().unwrap_or(trimmed).to_string()
                        } else {
                            trimmed.to_string()
                        }
                    })
                    .collect();

                result.push_str(&parts.join(", "));
                result.push('>');
                result
            },
        )
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

// this one is because in `mutation_knowledge` we allow
// passing in an impl Into<BrpTypeName>
// useful for passing in &str and String
// but we also want to be able to psas in a `&BrpTypeName`
// hence this odd beast
impl From<&Self> for BrpTypeName {
    fn from(type_name: &Self) -> Self {
        type_name.clone()
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
