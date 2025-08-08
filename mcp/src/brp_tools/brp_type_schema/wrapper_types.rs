//! Well-known wrapper types that have trivial handling
//!
//! These types wrap inner values in predictable ways and don't need
//! special mutation paths or enum variant listings.

use serde_json::{Value, json};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

/// Well-known wrapper types that we handle specially
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, EnumIter)]
pub enum WrapperType {
    #[strum(serialize = "core::option::Option")]
    Option,
    #[strum(serialize = "bevy_asset::handle::Handle")]
    Handle,
}

impl WrapperType {
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
                json!({"Some": inner_value})
            }
            Self::Handle => {
                // Handle wraps as {"Strong": [inner_value]}
                json!({"Strong": [inner_value]})
            }
        }
    }

    /// Get the default/empty example for this wrapper type
    pub fn default_example(self) -> Value {
        match self {
            Self::Option => json!("None"),
            Self::Handle => json!({"Strong": [{}]}),
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
                json!({
                    "example": {"Strong": [inner_value]}
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
        assert_eq!(
            WrapperType::Handle.default_example(),
            json!({"Strong": [{}]})
        );
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
            json!({"example": {"Strong": [[1.0, 2.0]]}})
        );
    }
}
