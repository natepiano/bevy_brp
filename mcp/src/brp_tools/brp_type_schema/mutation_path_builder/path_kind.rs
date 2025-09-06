//! Context for a mutation path describing what kind of mutation this is
use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::super::response_types::BrpTypeName;

#[derive(Debug, Clone, Deserialize)]
pub enum PathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField {
        field_name:  String,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in a tuple by index
    /// Applies to tuple elements, enums variants, including generics such as Option<T>
    IndexedElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in an array
    ArrayElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
}

impl PathKind {
    /// Create a new `RootValue`
    pub const fn new_root_value(type_name: BrpTypeName) -> Self {
        Self::RootValue { type_name }
    }

    /// Create a new `StructField`
    pub const fn new_struct_field(field_name: String, parent_type: BrpTypeName) -> Self {
        Self::StructField {
            field_name,
            parent_type,
        }
    }

    /// Create a new `IndexedElement`
    pub const fn new_indexed_element(index: usize, parent_type: BrpTypeName) -> Self {
        Self::IndexedElement { index, parent_type }
    }

    /// Create a new `ArrayElement`
    pub const fn new_array_element(index: usize, parent_type: BrpTypeName) -> Self {
        Self::ArrayElement { index, parent_type }
    }

    /// Generate a human-readable description for this mutation
    pub fn description(&self) -> String {
        match self {
            Self::RootValue { type_name, .. } => {
                format!("Replace the entire {type_name} value")
            }
            Self::StructField {
                field_name,
                parent_type,
                ..
            } => {
                format!("Mutate the {field_name} field of {parent_type}")
            }
            Self::IndexedElement {
                index, parent_type, ..
            } => {
                format!("Mutate element {index} of {parent_type}")
            }
            Self::ArrayElement {
                index, parent_type, ..
            } => {
                format!("Mutate element [{index}] of {parent_type}")
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

    /// Generate the path segment string for this mutation
    pub fn to_path_segment(&self) -> String {
        match self {
            Self::RootValue { .. } => String::new(),
            Self::StructField { field_name, .. } => format!(".{field_name}"),
            Self::IndexedElement { index, .. } => format!(".{index}"),
            Self::ArrayElement { index, .. } => format!("[{index}]"),
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
