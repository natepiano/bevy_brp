//! Context for a mutation path describing what kind of mutation this is
use std::borrow::Borrow;
use std::fmt::Display;
use std::ops::Deref;

use serde::Deserialize;
use serde::Serialize;

use super::super::brp_type_name::BrpTypeName;
use super::super::type_kind::TypeKind;
use super::enum_builder::OptionClassification;
use super::new_types::StructFieldName;
use super::new_types::VariantName;
use super::types::EnumPathInfo;

/// A semantic identifier for mutation paths in the builder system
///
/// This newtype wraps the path descriptor strings used as keys in the
/// `HashMap` passed to `assemble_from_children`. The descriptor varies by `PathKind`:
/// - `StructField`: field name (e.g., "translation", "rotation")
/// - `IndexedElement`/`ArrayElement`: index as string (e.g., "0", "1")
/// - `RootValue`: empty string ""
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MutationPathDescriptor(String);

impl Deref for MutationPathDescriptor {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<str> for MutationPathDescriptor {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<String> for MutationPathDescriptor {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MutationPathDescriptor {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<StructFieldName> for MutationPathDescriptor {
    fn from(field_name: StructFieldName) -> Self {
        Self(field_name.to_string())
    }
}

impl From<&StructFieldName> for MutationPathDescriptor {
    fn from(field_name: &StructFieldName) -> Self {
        Self(field_name.to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum PathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField {
        field_name:  StructFieldName,
        type_name:   BrpTypeName,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in a tuple by index
    /// Applies to tuple elements, enums variants, including generics such as Option<T>
    IndexedElement {
        index:       usize,
        type_name:   BrpTypeName,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in an array
    ArrayElement {
        index:       usize,
        type_name:   BrpTypeName,
        parent_type: BrpTypeName,
    },
}

impl PathKind {
    /// Create a new `RootValue`
    pub const fn new_root_value(type_name: BrpTypeName) -> Self {
        Self::RootValue { type_name }
    }

    /// Create a new `IndexedElement`
    pub const fn new_indexed_element(
        index: usize,
        type_name: BrpTypeName,
        parent_type: BrpTypeName,
    ) -> Self {
        Self::IndexedElement {
            index,
            type_name,
            parent_type,
        }
    }

    /// Get the type name being processed (matches `PathLocation::type_name()` behavior)
    pub const fn type_name(&self) -> &BrpTypeName {
        match self {
            Self::RootValue { type_name }
            | Self::StructField { type_name, .. }
            | Self::IndexedElement { type_name, .. }
            | Self::ArrayElement { type_name, .. } => type_name,
        }
    }

    /// Returns the variant if there's exactly one applicable variant whose short name
    /// matches the parent type (indicating redundancy that should be eliminated in description)
    ///
    /// Example: For path `.0.z` where `parent_type` is "Xyza" and `applicable_variants` is
    /// [`Color::Xyza`], returns Some(&VariantName) to enable integrated description
    /// "Mutate the z field of `Color::Xyza` variant" instead of redundant
    /// "Mutate the z field of `Xyza` within `Color::Xyza` variant"
    fn single_variant_matching_parent(
        &self,
        enum_path_info: Option<&EnumPathInfo>,
    ) -> Option<VariantName> {
        let enum_path_info = enum_path_info?;

        // Must have exactly one variant
        if enum_path_info.applicable_variants.len() != 1 {
            return None;
        }

        let variant = &enum_path_info.applicable_variants[0];
        let variant_short = variant.short_name();

        // Check if parent_type matches variant short name
        let parent_short = match self {
            Self::StructField { parent_type, .. }
            | Self::IndexedElement { parent_type, .. }
            | Self::ArrayElement { parent_type, .. } => parent_type.short_name(),
            Self::RootValue { .. } => return None, // No parent_type
        };

        if parent_short == variant_short {
            Some(variant.clone())
        } else {
            None
        }
    }

    /// Extract a descriptor suitable for `HashMap<MutationPathDescriptor, Value>` from this
    /// `PathKind` Used by `MutationPathBuilder` to build `child_examples` `HashMap`
    pub fn to_mutation_path_descriptor(&self) -> MutationPathDescriptor {
        match self {
            Self::StructField { field_name, .. } => MutationPathDescriptor::from(field_name),
            Self::IndexedElement { index, .. } | Self::ArrayElement { index, .. } => {
                MutationPathDescriptor::from(index.to_string())
            }
            Self::RootValue { .. } => MutationPathDescriptor::from(String::new()),
        }
    }

    /// Generate a human-readable description for this mutation
    pub fn description(
        &self,
        type_kind: &TypeKind,
        enum_path_info: Option<&EnumPathInfo>,
    ) -> String {
        // Handle redundancy case: parent_type matches single variant's short name
        // Example: ".0.z" where parent="Xyza" and variants=["Color::Xyza"]
        // Generate: "Mutate the z field of Color::Xyza variant" (integrated, no suffix)
        if let Some(variant) = self.single_variant_matching_parent(enum_path_info) {
            return match self {
                Self::StructField { field_name, .. } => {
                    format!("Mutate the {field_name} field of {variant} variant")
                }
                Self::IndexedElement { index, .. } => {
                    format!("Mutate element {index} of {variant} variant")
                }
                Self::ArrayElement { index, .. } => {
                    format!("Mutate element [{index}] of {variant} variant")
                }
                Self::RootValue { .. } => {
                    unreachable!("single_variant_matching_parent returns None for RootValue")
                }
            };
        }

        // Normal case: generate base description with type_kind suffix
        let type_kind_str = if matches!(type_kind, TypeKind::Value) {
            String::new()
        } else {
            format!(" {}", type_kind.as_ref().to_lowercase())
        };

        let base_description = match self {
            Self::RootValue { type_name, .. } => {
                let short_name = type_name.short_name();
                format!("Replace the entire {short_name}{type_kind_str}")
            }
            Self::StructField {
                field_name,
                parent_type,
                type_name,
                ..
            } => {
                // Check if field type is Option<T>
                if let OptionClassification::Option { inner_type } =
                    OptionClassification::from_type_name(type_name)
                {
                    let inner_short = inner_type.short_name();
                    format!("Set {field_name} to None or Some({inner_short})")
                } else {
                    let short_parent = parent_type.short_name();
                    format!("Mutate the {field_name} field of {short_parent}{type_kind_str}")
                }
            }
            Self::IndexedElement {
                index,
                parent_type,
                type_name,
                ..
            } => {
                // Check if parent is Option<T> and index is 0
                if *index == 0
                    && let OptionClassification::Option { .. } =
                        OptionClassification::from_type_name(parent_type)
                {
                    let value_short = type_name.short_name();
                    return format!("Mutate the {value_short} value inside Some variant");
                }
                let short_parent = parent_type.short_name();
                format!("Mutate element {index} of {short_parent}{type_kind_str}")
            }
            Self::ArrayElement {
                index, parent_type, ..
            } => {
                let short_parent = parent_type.short_name();
                format!("Mutate element [{index}] of {short_parent}{type_kind_str}")
            }
        };

        // Add variant suffix if applicable
        let suffix = enum_path_info.map_or_else(String::new, |enum_data| {
            match &enum_data.applicable_variants {
                variants if variants.is_empty() => String::new(),
                variants if variants.len() == 1 => {
                    format!(" within {} variant", variants[0])
                }
                variants => {
                    let variant_list = variants
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(" within '{variant_list}' variants")
                }
            }
        });

        format!("{base_description}{suffix}")
    }

    /// Get just the variant name for serialization
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::RootValue { .. } => "RootValue",
            Self::StructField { .. } => "StructField",
            Self::IndexedElement { .. } => "IndexedElement",
            Self::ArrayElement { .. } => "ArrayElement",
        }
    }
}

impl Display for PathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.variant_name())
    }
}

impl Serialize for PathKind {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
