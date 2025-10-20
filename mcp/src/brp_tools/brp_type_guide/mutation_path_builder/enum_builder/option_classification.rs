//! Option type classification and transformation for BRP mutations
//!
//! This module handles special processing for `Option<T>` types in BRP mutations.
//! The Bevy Remote Protocol expects Option values in a specific format:
//! - `None` → `null`
//! - `Some(value)` → `value` (unwrapped)

use serde_json::Value;
use serde_json::json;

use crate::brp_tools::brp_type_guide::BrpTypeName;
use crate::brp_tools::brp_type_guide::mutation_path_builder::new_types::VariantName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionClassification {
    Option { inner_type: BrpTypeName },
    Regular(BrpTypeName),
}

impl OptionClassification {
    pub fn from_type_name(type_name: &BrpTypeName) -> Self {
        Self::extract_option_inner(type_name).map_or_else(
            || Self::Regular(type_name.clone()),
            |inner_type| Self::Option { inner_type },
        )
    }

    pub const fn is_option(&self) -> bool {
        matches!(self, Self::Option { .. })
    }

    /// Recursively unwrap nested Option types to find the leaf type
    ///
    /// BRP collapses nested Options in JSON representation:
    /// - `Option<Option<Option<f32>>>` serializes as either `null` or `5.0` (the leaf value)
    /// - Intermediate `Some(None)` or `Some(Some(None))` all collapse to `null`
    ///
    /// This means intermediate Option layers don't have accessible mutation paths,
    /// so we skip directly to the innermost non-Option type.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let nested = BrpTypeName::from("core::option::Option<core::option::Option<f32>>");
    /// let leaf = OptionClassification::extract_leaf_type(&nested);
    /// assert_eq!(leaf.as_str(), "f32");
    /// ```
    pub fn extract_leaf_type(type_name: &BrpTypeName) -> BrpTypeName {
        match Self::from_type_name(type_name) {
            Self::Option { inner_type } => Self::extract_leaf_type(&inner_type),
            Self::Regular(leaf_type) => leaf_type,
        }
    }

    fn extract_option_inner(type_name: &BrpTypeName) -> Option<BrpTypeName> {
        const OPTION_PREFIX: &str = "core::option::Option<";
        const OPTION_SUFFIX: char = '>';

        let type_str = type_name.as_str();
        type_str
            .strip_prefix(OPTION_PREFIX)
            .and_then(|inner_with_suffix| {
                inner_with_suffix
                    .strip_suffix(OPTION_SUFFIX)
                    .map(|inner| BrpTypeName::from(inner.to_string()))
            })
    }
}

/// Apply `Option<T>` transformation if needed: `{"Some": value}` → `value`, `"None"` → `null`
pub fn apply_option_transformation(
    example: Value,
    variant_name: &VariantName,
    enum_type: &BrpTypeName,
) -> Value {
    let type_category = OptionClassification::from_type_name(enum_type);

    if !type_category.is_option() {
        return example;
    }

    // Transform Option variants for BRP mutations
    match variant_name.short_name() {
        "None" => {
            json!(null)
        }
        "Some" => {
            // Extract the inner value from {"Some": value}
            if let Some(obj) = example.as_object()
                && let Some(value) = obj.get("Some")
            {
                return value.clone();
            }
            example
        }
        _ => example,
    }
}
