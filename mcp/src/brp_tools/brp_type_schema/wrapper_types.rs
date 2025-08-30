//! Well-known wrapper types that have trivial handling
//!
//! These types wrap inner values in predictable ways and don't need
//! special mutation paths or enum variant listings.

use serde_json::{Value, json};
use strum::{AsRefStr, Display, EnumIter, IntoEnumIterator};

use super::response_types::BrpTypeName;

/// Wrapper type variant names for serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum WrapperVariant {
    Some,
    #[allow(dead_code)]
    None,
    Strong,
    Weak,
}

/// Well-known wrapper types that we handle specially
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, EnumIter)]
pub enum WrapperType {
    #[strum(serialize = "core::option::Option")]
    Option,
    #[strum(serialize = "bevy_asset::handle::Handle")]
    Handle,
}

impl WrapperType {
    // Helper methods for building JSON with WrapperVariant

    /// Build JSON for `Option::Some` variant
    fn wrap_some(value: Value) -> Value {
        json!({WrapperVariant::Some.to_string(): value})
    }

    /// Build JSON for `Option::None` variant
    const fn wrap_none() -> Value {
        Value::Null
    }

    /// Build JSON for `Handle::Strong` variant
    fn wrap_strong(value: Value) -> Value {
        json!({WrapperVariant::Strong.to_string(): [value]})
    }

    /// Build JSON for `Handle::Weak` variant
    fn wrap_weak(value: Value) -> Value {
        json!({WrapperVariant::Weak.to_string(): [value]})
    }

    /// Try to detect a wrapper type from a full type name
    pub fn detect(type_name: &str) -> Option<(Self, BrpTypeName)> {
        for wrapper in Self::iter() {
            if let Some(inner) = wrapper.extract_inner_type(type_name) {
                return Some((wrapper, BrpTypeName::from(inner)));
            }
        }
        None
    }

    /// Extract the inner type from a wrapper type string
    pub fn extract_inner_type(self, type_name: &str) -> Option<&str> {
        let prefix = self.as_ref();
        let expected_start = format!("{prefix}<");

        if type_name.starts_with(&expected_start) && type_name.ends_with('>') {
            // Extract the inner type between < and >
            let inner = &type_name[expected_start.len()..type_name.len() - 1];
            Some(inner)
        } else {
            None
        }
    }

    /// Wrap an example value in the appropriate wrapper format
    pub fn wrap_example(self, inner_value: Value) -> Value {
        match self {
            Self::Option => {
                // Option wraps as {"Some": inner_value}
                Self::wrap_some(inner_value)
            }
            Self::Handle => {
                // Handle wraps as {"Strong": [inner_value]}
                // Note: We use Strong for examples, though Handle also supports Weak
                Self::wrap_strong(inner_value)
            }
        }
    }

    /// Get the default/empty example for this wrapper type
    pub fn default_example(self) -> Value {
        match self {
            Self::Option => Self::wrap_none(),
            // Handle::default() returns Weak(AssetId::default()) in Bevy
            Self::Handle => Self::wrap_weak(json!({})),
        }
    }

    /// Get mutation examples showing how to set the value
    /// For Option: shows both Some and None examples
    /// For Handle: shows the wrapped format
    pub fn mutation_examples(self, inner_value: Value) -> Value {
        match self {
            Self::Option => {
                // For Option mutation, you pass the value directly or null
                json!({
                    "some": inner_value,
                    "none": null
                })
            }
            Self::Handle => {
                // Handle still needs the wrapper in mutations
                // Using Strong for actual asset references (Weak would be for placeholders)
                json!({
                    "strong": Self::wrap_strong(inner_value),
                    "weak_placeholder": Self::wrap_weak(json!({}))
                })
            }
        }
    }
}

impl From<WrapperType> for String {
    fn from(wrapper: WrapperType) -> Self {
        wrapper.as_ref().to_string()
    }
}
