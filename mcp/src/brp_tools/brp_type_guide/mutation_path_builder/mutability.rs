//! Mutability and diagnostic types used during mutation-path construction.

use serde::Deserialize;
use serde::Serialize;

use super::mutation_path::MutationPath;
use super::variant_name::VariantName;

/// Status of whether a mutation path can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mutability {
    /// Path can be fully mutated
    Mutable,
    /// Path cannot be mutated (missing traits, unsupported type, etc.)
    NotMutable,
    /// Path is partially mutable (some elements mutable, others not)
    PartiallyMutable,
}

/// Identifies what component has a mutability issue
#[derive(Debug, Clone)]
pub(super) enum MutabilityIssueTarget {
    /// A mutation path within a type (e.g., ".translation.x")
    Path(MutationPath),
    /// An enum variant name (e.g., "`Color::Srgba`")
    Variant(VariantName),
}

impl std::fmt::Display for MutabilityIssueTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(path) => write!(f, "{path}"),
            Self::Variant(name) => write!(f, "{name}"),
        }
    }
}

/// Summary of a mutation issue for diagnostic reporting
#[derive(Debug, Clone)]
pub(super) struct MutabilityIssue {
    pub(super) target: MutabilityIssueTarget,
    pub(super) status: Mutability,
}

impl MutabilityIssue {
    /// Create from an enum variant name (for enum types)
    pub(super) const fn from_variant_name(variant: VariantName, status: Mutability) -> Self {
        Self {
            target: MutabilityIssueTarget::Variant(variant),
            status,
        }
    }
}
