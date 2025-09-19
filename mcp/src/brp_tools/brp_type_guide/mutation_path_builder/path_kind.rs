//! Context for a mutation path describing what kind of mutation this is
use std::borrow::Borrow;
use std::fmt::Display;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use super::super::brp_type_name::BrpTypeName;
use super::path_builder::MaybeVariants;
use super::type_kind::TypeKind;

/// A semantic identifier for mutation paths in the builder system
///
/// This newtype wraps the path descriptor strings used as keys in the
/// HashMap passed to `assemble_from_children`. The descriptor varies by `PathKind`:
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

    /// Extract a descriptor suitable for `HashMap<MutationPathDescriptor, Value>` from this
    /// `PathKind` Used by `ProtocolEnforcer` to build `child_examples` HashMap
    pub fn to_mutation_path_descriptor(&self) -> MutationPathDescriptor {
        match self {
            Self::StructField { field_name, .. } => {
                MutationPathDescriptor::from(field_name.clone())
            }
            Self::IndexedElement { index, .. } | Self::ArrayElement { index, .. } => {
                MutationPathDescriptor::from(index.to_string())
            }
            Self::RootValue { .. } => MutationPathDescriptor::from(String::new()),
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

impl MaybeVariants for PathKind {
    fn applicable_variants(&self) -> Option<&[String]> {
        None // Regular paths have no variant information
    }

    fn into_path_kind(self) -> Option<PathKind> {
        Some(self)
    }
}
