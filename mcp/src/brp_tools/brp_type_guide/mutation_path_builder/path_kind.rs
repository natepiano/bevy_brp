//! Context for a mutation path describing what kind of mutation this is
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::enum_path_info::EnumPathInfo;
use super::option_classification::OptionClassification;
use super::variant_name::VariantName;
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::brp_tools::brp_type_guide::struct_field_name::StructFieldName;
use crate::brp_tools::brp_type_guide::type_kind::TypeKind;

/// A semantic identifier for mutation paths in the builder system
///
/// This newtype wraps the path descriptor strings used as keys in the
/// `HashMap` passed to `assemble_from_children`. The descriptor varies by `PathKind`:
/// - `StructField`: field name (e.g., "translation", "rotation")
/// - `IndexedElement`/`ArrayElement`: index as string (e.g., "0", "1")
/// - `RootValue`: empty string ""
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct MutationPathDescriptor(String);

impl Deref for MutationPathDescriptor {
    type Target = str;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl Borrow<str> for MutationPathDescriptor {
    fn borrow(&self) -> &str { &self.0 }
}

impl From<String> for MutationPathDescriptor {
    fn from(s: String) -> Self { Self(s) }
}

impl From<&str> for MutationPathDescriptor {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

impl From<StructFieldName> for MutationPathDescriptor {
    fn from(field_name: StructFieldName) -> Self { Self(field_name.to_string()) }
}

impl From<&StructFieldName> for MutationPathDescriptor {
    fn from(field_name: &StructFieldName) -> Self { Self(field_name.to_string()) }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) enum PathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField {
        field_name:  StructFieldName,
        type_name:   BrpTypeName,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in a tuple by index
    /// Applies to tuple elements, enums variants, including generics such as `Option<T>`
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
    pub(super) const fn new_root_value(type_name: BrpTypeName) -> Self {
        Self::RootValue { type_name }
    }

    /// Create a new `IndexedElement`
    pub(super) const fn new_indexed_element(
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
    pub(super) const fn type_name(&self) -> &BrpTypeName {
        match self {
            Self::RootValue { type_name }
            | Self::StructField { type_name, .. }
            | Self::IndexedElement { type_name, .. }
            | Self::ArrayElement { type_name, .. } => type_name,
        }
    }

    /// The type a `describe_normal` description names: `type_name` for `RootValue`
    /// ("Replace the entire `ClearColor` tuple struct"), `parent_type` for every child path
    /// ("Mutate element 0 of `ClearColor` tuple struct")
    const fn described_type(&self) -> &BrpTypeName {
        match self {
            Self::RootValue { type_name } => type_name,
            Self::StructField { parent_type, .. }
            | Self::IndexedElement { parent_type, .. }
            | Self::ArrayElement { parent_type, .. } => parent_type,
        }
    }

    /// Returns the variant if there's exactly one applicable variant whose short name
    /// matches the parent type (indicating redundancy that should be eliminated in description)
    ///
    /// Example: For path `.0.z` where `parent_type` is "Xyza" and `applicable_variants` is
    /// `Color::Xyza`, returns `Some(&VariantName)` to enable integrated description
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
    pub(super) fn to_mutation_path_descriptor(&self) -> MutationPathDescriptor {
        match self {
            Self::StructField { field_name, .. } => MutationPathDescriptor::from(field_name),
            Self::IndexedElement { index, .. } | Self::ArrayElement { index, .. } => {
                MutationPathDescriptor::from(index.to_string())
            },
            Self::RootValue { .. } => MutationPathDescriptor::from(String::new()),
        }
    }

    /// Generate a human-readable description for this mutation
    ///
    /// `registry` supplies the `TypeKind` of `described_type()`, so the kind label always
    /// belongs to the type whose short name the description prints.
    pub(super) fn description(
        &self,
        registry: &HashMap<BrpTypeName, Value>,
        enum_path_info: Option<&EnumPathInfo>,
    ) -> String {
        // Integrated description (no suffix): parent_type matches single variant's short name.
        // Example: ".0.z" where parent="Xyza" and variants=["Color::Xyza"]
        // → "Mutate the z field of Color::Xyza variant"
        if let Some(variant) = self.single_variant_matching_parent(enum_path_info) {
            match self {
                Self::StructField { field_name, .. } => {
                    format!("Mutate the {field_name} field of {variant} variant")
                },
                Self::IndexedElement { index, .. } => {
                    format!("Mutate element {index} of {variant} variant")
                },
                Self::ArrayElement { index, .. } => {
                    format!("Mutate element [{index}] of {variant} variant")
                },
                Self::RootValue { .. } => {
                    format!("Mutate the root value of {variant} variant")
                },
            }
        } else if let Self::IndexedElement {
            index: 0,
            parent_type,
            type_name,
            ..
        } = self
            && let OptionClassification::Wrapped { .. } = parent_type.into()
        {
            // Integrated description (no suffix): Option<T> element at index 0.
            let value_short = type_name.short_name();
            format!("Mutate the {value_short} value inside Some variant")
        } else {
            self.describe_normal(registry, enum_path_info)
        }
    }

    /// Normal case: `described_type()` short name and its own `TypeKind` label, plus enum
    /// variant suffix.
    fn describe_normal(
        &self,
        registry: &HashMap<BrpTypeName, Value>,
        enum_path_info: Option<&EnumPathInfo>,
    ) -> String {
        let described_type = self.described_type();
        let described_name = described_type.short_name();
        let described_kind = registry
            .get(described_type)
            .map_or(TypeKind::Value, TypeKind::from);
        let kind_suffix = if matches!(described_kind, TypeKind::Value) {
            String::new()
        } else {
            format!(" {}", described_kind.description_label())
        };

        let base_description = match self {
            Self::RootValue { .. } => {
                format!("Replace the entire {described_name}{kind_suffix}")
            },
            Self::StructField {
                field_name,
                type_name,
                ..
            } => {
                if let OptionClassification::Wrapped { inner_type } = type_name.into() {
                    let inner_short = inner_type.short_name();
                    format!("Set {field_name} to None or Some({inner_short})")
                } else {
                    format!("Mutate the {field_name} field of {described_name}{kind_suffix}")
                }
            },
            Self::IndexedElement { index, .. } => {
                format!("Mutate element {index} of {described_name}{kind_suffix}")
            },
            Self::ArrayElement { index, .. } => {
                format!("Mutate element [{index}] of {described_name}{kind_suffix}")
            },
        };

        let suffix = enum_path_info.map_or_else(String::new, |enum_data| {
            match &enum_data.applicable_variants {
                variants if variants.is_empty() => String::new(),
                variants if variants.len() == 1 => {
                    format!(" within {} variant", variants[0])
                },
                variants => {
                    let variant_list = variants
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(" within '{variant_list}' variants")
                },
            }
        });

        format!("{base_description}{suffix}")
    }

    /// Get just the variant name for serialization
    const fn variant_name(&self) -> &'static str {
        match self {
            Self::RootValue { .. } => "RootValue",
            Self::StructField { .. } => "StructField",
            Self::IndexedElement { .. } => "IndexedElement",
            Self::ArrayElement { .. } => "ArrayElement",
        }
    }
}

impl Display for PathKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::Map;
    use serde_json::Value;

    use super::EnumPathInfo;
    use super::PathKind;
    use super::VariantName;
    use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
    use crate::brp_tools::brp_type_guide::constants::TYPE_BEVY_CAMERA;
    use crate::brp_tools::brp_type_guide::constants::TYPE_BOOL;
    use crate::brp_tools::brp_type_guide::constants::TYPE_F32;
    use crate::brp_tools::brp_type_guide::type_kind::TypeKind;
    use crate::support::JsonObjectAccess;
    use crate::support::SchemaField;

    const CLEAR_COLOR: &str = "bevy_camera::clear_color::ClearColor";
    const COLOR: &str = "bevy_color::color::Color";
    const COLOR_SRGBA_VARIANT: &str = "Color::Srgba";
    const SRGBA: &str = "bevy_color::srgba::Srgba";

    /// Registry holding only the `kind` of each type, mirroring `ClearColor(Color)` where
    /// `Color::Srgba(Srgba)` and `Srgba { red: f32, .. }`, plus `Camera { is_active: bool }`
    fn registry() -> HashMap<BrpTypeName, Value> {
        [
            (CLEAR_COLOR, TypeKind::TupleStruct),
            (COLOR, TypeKind::Enum),
            (SRGBA, TypeKind::Struct),
            (TYPE_BEVY_CAMERA, TypeKind::Struct),
            (TYPE_BOOL, TypeKind::Value),
            (TYPE_F32, TypeKind::Value),
        ]
        .into_iter()
        .map(|(type_name, type_kind)| {
            let mut schema = Value::Object(Map::new());
            schema.insert_field(SchemaField::Kind.as_ref(), type_kind.as_ref());
            (BrpTypeName::from(type_name), schema)
        })
        .collect()
    }

    fn srgba_variant_info() -> EnumPathInfo {
        let variant = VariantName::from(COLOR_SRGBA_VARIANT.to_string());
        EnumPathInfo {
            variant_chain:       vec![variant.clone()],
            applicable_variants: vec![variant],
            root_example:        None,
        }
    }

    #[test]
    fn root_value_names_type_with_its_own_kind() {
        let path_kind = PathKind::new_root_value(BrpTypeName::from(CLEAR_COLOR));

        assert_eq!(
            path_kind.description(&registry(), None),
            "Replace the entire ClearColor tuple struct"
        );
    }

    #[test]
    fn tuple_struct_element_names_parent_with_parent_kind() {
        // `.0` of `ClearColor`: the element's own type is the `Color` enum.
        let path_kind = PathKind::new_indexed_element(
            0,
            BrpTypeName::from(COLOR),
            BrpTypeName::from(CLEAR_COLOR),
        );

        assert_eq!(
            path_kind.description(&registry(), None),
            "Mutate element 0 of ClearColor tuple struct"
        );
    }

    #[test]
    fn enum_variant_element_names_enum_with_enum_kind() {
        // `.0.0` of `ClearColor`: the element's own type is the `Srgba` struct.
        let path_kind =
            PathKind::new_indexed_element(0, BrpTypeName::from(SRGBA), BrpTypeName::from(COLOR));

        assert_eq!(
            path_kind.description(&registry(), Some(&srgba_variant_info())),
            "Mutate element 0 of Color enum within Color::Srgba variant"
        );
    }

    #[test]
    fn struct_field_names_parent_with_parent_kind() {
        // `.is_active` of `Camera`: the field's own type is `bool`, a `TypeKind::Value`.
        let path_kind = PathKind::StructField {
            field_name:  "is_active".into(),
            type_name:   BrpTypeName::from(TYPE_BOOL),
            parent_type: BrpTypeName::from(TYPE_BEVY_CAMERA),
        };

        assert_eq!(
            path_kind.description(&registry(), None),
            "Mutate the is_active field of Camera struct"
        );
    }

    #[test]
    fn field_of_struct_matching_variant_names_variant() {
        // `.0.0.red` of `ClearColor`: `Srgba` matches the single `Color::Srgba` variant.
        let path_kind = PathKind::StructField {
            field_name:  "red".into(),
            type_name:   BrpTypeName::from(TYPE_F32),
            parent_type: BrpTypeName::from(SRGBA),
        };

        assert_eq!(
            path_kind.description(&registry(), Some(&srgba_variant_info())),
            "Mutate the red field of Color::Srgba variant"
        );
    }
}
