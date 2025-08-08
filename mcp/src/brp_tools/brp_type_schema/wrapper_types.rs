//! Well-known wrapper types that have trivial handling
//!
//! These types wrap inner values in predictable ways and don't need
//! special mutation paths or enum variant listings.

use serde_json::{Value, json};
use strum::{AsRefStr, Display, EnumIter, IntoEnumIterator};

/// Wrapper type variant names for serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum WrapperVariant {
    Some,
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
    fn wrap_none() -> Value {
        json!(WrapperVariant::None.to_string())
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
    pub fn detect(type_name: &str) -> Option<(Self, &str)> {
        for wrapper in Self::iter() {
            if let Some(inner) = wrapper.extract_inner_type(type_name) {
                return Some((wrapper, inner));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_option() {
        assert_eq!(
            WrapperType::detect("core::option::Option<glam::Vec2>"),
            Some((WrapperType::Option, "glam::Vec2"))
        );
        assert_eq!(
            WrapperType::detect("core::option::Option<bevy_math::rects::rect::Rect>"),
            Some((WrapperType::Option, "bevy_math::rects::rect::Rect"))
        );
    }

    #[test]
    fn test_detect_handle() {
        assert_eq!(
            WrapperType::detect("bevy_asset::handle::Handle<bevy_image::image::Image>"),
            Some((WrapperType::Handle, "bevy_image::image::Image"))
        );
        assert_eq!(
            WrapperType::detect("bevy_asset::handle::Handle<bevy_scene::scene::Scene>"),
            Some((WrapperType::Handle, "bevy_scene::scene::Scene"))
        );
    }

    #[test]
    fn test_detect_non_wrapper() {
        assert_eq!(WrapperType::detect("glam::Vec2"), None);
        assert_eq!(WrapperType::detect("bevy_transform::Transform"), None);
    }

    #[test]
    fn test_nested_wrappers() {
        assert_eq!(
            WrapperType::detect("core::option::Option<core::option::Option<f32>>"),
            Some((WrapperType::Option, "core::option::Option<f32>"))
        );
    }

    #[test]
    fn test_wrap_example() {
        let inner = json!([1.0, 2.0]);
        assert_eq!(
            WrapperType::Option.wrap_example(inner.clone()),
            json!({"Some": [1.0, 2.0]})
        );
        assert_eq!(
            WrapperType::Handle.wrap_example(inner),
            json!({"Strong": [[1.0, 2.0]]})
        );
    }

    #[test]
    fn test_default_example() {
        assert_eq!(WrapperType::Option.default_example(), json!("None"));
        assert_eq!(WrapperType::Handle.default_example(), json!({"Weak": [{}]}));
    }

    #[test]
    fn test_mutation_examples() {
        let inner = json!([1.0, 2.0]);
        assert_eq!(
            WrapperType::Option.mutation_examples(inner.clone()),
            json!({"some": [1.0, 2.0], "none": null})
        );
        assert_eq!(
            WrapperType::Handle.mutation_examples(inner),
            json!({
                "strong": {"Strong": [[1.0, 2.0]]},
                "weak_placeholder": {"Weak": [{}]}
            })
        );
    }
}
