//! Option type classification and transformation for BRP mutations
//!
//! This module handles special processing for `Option<T>` types in BRP mutations.
//! The Bevy Remote Protocol expects Option values in a specific format:
//! - `None` -> `null`
//! - `Some(value)` -> `value` (unwrapped)

use super::constants::OPTION_PREFIX;
use super::constants::OPTION_SUFFIX;
use super::path_example::Example;
use super::variant_name::VariantName;
use crate::brp_tools::brp_type_guide::BrpTypeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OptionClassification {
    Option { inner_type: BrpTypeName },
    Regular(BrpTypeName),
}

impl OptionClassification {
    pub(super) const fn is_option(&self) -> bool { matches!(self, Self::Option { .. }) }

    fn extract_option_inner(type_name: &BrpTypeName) -> Option<BrpTypeName> {
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

impl From<&BrpTypeName> for OptionClassification {
    fn from(type_name: &BrpTypeName) -> Self {
        Self::extract_option_inner(type_name).map_or_else(
            || Self::Regular(type_name.clone()),
            |inner_type| Self::Option { inner_type },
        )
    }
}

/// Apply `Option<T>` transformation if needed: `{"Some": value}` -> `value`, `"None"` -> `null`
pub(super) fn apply_option_transformation(
    example: Example,
    variant_name: &VariantName,
    enum_type: &BrpTypeName,
) -> Example {
    let type_category: OptionClassification = enum_type.into();

    if !type_category.is_option() {
        return example;
    }

    // Transform Option variants for BRP mutations
    match variant_name.short_name() {
        "None" => Example::OptionNone,
        "Some" => {
            // Extract the inner value from {"Some": value}
            if let Example::Json(val) = &example
                && let Some(obj) = val.as_object()
                && let Some(value) = obj.get("Some")
            {
                return Example::Json(value.clone());
            }
            example
        },
        _ => example,
    }
}
