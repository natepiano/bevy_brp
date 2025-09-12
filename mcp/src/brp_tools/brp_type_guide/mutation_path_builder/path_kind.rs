//! Context for a mutation path describing what kind of mutation this is
use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::super::response_types::BrpTypeName;
use super::type_kind::TypeKind;

#[derive(Debug, Clone, Deserialize)]
pub enum PathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField {
        field_name:  String,
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

    /// Create a new `StructField`
    pub const fn new_struct_field(
        field_name: String,
        type_name: BrpTypeName,
        parent_type: BrpTypeName,
    ) -> Self {
        Self::StructField {
            field_name,
            type_name,
            parent_type,
        }
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

    /// Create a new `ArrayElement`
    pub const fn new_array_element(
        index: usize,
        type_name: BrpTypeName,
        parent_type: BrpTypeName,
    ) -> Self {
        Self::ArrayElement {
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

    /// Extract a key suitable for `HashMap<String, Value>` from this `PathKind`
    /// Used by `ProtocolEnforcer` to build `child_examples` `HashMap`
    pub fn to_child_key(&self) -> String {
        match self {
            Self::StructField { field_name, .. } => field_name.clone(),
            Self::IndexedElement { index, .. } | Self::ArrayElement { index, .. } => {
                index.to_string()
            }
            Self::RootValue { .. } => String::new(),
        }
    }

    /// Generate a human-readable description for this mutation
    pub fn description(&self, type_kind: &TypeKind) -> String {
        let type_kind_str = if matches!(type_kind, TypeKind::Value) {
            String::new()
        } else {
            format!(" {}", type_kind.as_ref().to_lowercase())
        };

        match self {
            Self::RootValue { type_name, .. } => {
                let short_name = type_name.short_name();
                format!("Replace the entire {short_name}{type_kind_str}")
            }
            Self::StructField {
                field_name,
                parent_type,
                ..
            } => {
                let short_parent = parent_type.short_name();
                format!("Mutate the {field_name} field of {short_parent}{type_kind_str}")
            }
            Self::IndexedElement {
                index, parent_type, ..
            } => {
                let short_parent = parent_type.short_name();
                format!("Mutate element {index} of {short_parent}{type_kind_str}")
            }
            Self::ArrayElement {
                index, parent_type, ..
            } => {
                let short_parent = parent_type.short_name();
                format!("Mutate element [{index}] of {short_parent}{type_kind_str}")
            }
        }
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
