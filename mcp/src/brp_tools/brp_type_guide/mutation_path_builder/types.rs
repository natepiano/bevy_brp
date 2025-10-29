//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::super::type_kind::TypeKind;
use super::enum_builder::VariantSignature;
use super::new_types::MutationPath;
use super::new_types::VariantName;
use super::path_example::PathExample;
use super::path_kind::PathKind;

/// Action to take regarding path creation during recursion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAction {
    /// Create mutation paths during recursion
    Create,
    /// Skip path creation during recursion
    Skip,
}

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
pub enum MutabilityIssueTarget {
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
pub struct MutabilityIssue {
    pub target: MutabilityIssueTarget,
    pub type_name: BrpTypeName,
    pub status: Mutability,
    pub reason: Option<Value>,
}

impl MutabilityIssue {
    /// Create from an enum variant name (for enum types)
    pub const fn from_variant_name(
        variant: VariantName,
        type_name: BrpTypeName,
        status: Mutability,
    ) -> Self {
        Self {
            target: MutabilityIssueTarget::Variant(variant),
            type_name,
            status,
            reason: None,
        }
    }
}

/// Path information combining navigation and type metadata
#[derive(Debug, Clone, Serialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind: PathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name: BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind: TypeKind,
    /// Status of whether this path can be mutated
    pub mutability: Mutability,
    /// Reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability_reason: Option<Value>,
    /// Instructions for setting variants required for this mutation path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_instructions: Option<String>,
    /// Example: `["BottomEnum::VariantB"]`
    /// `VariantName` serializes as a string in JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants: Option<Vec<VariantName>>,
    /// Only present for paths nested in enums - built using assembly during ascent
    #[serde(skip_serializing_if = "Option::is_none", skip_serializing)]
    pub old_root_example: Option<Value>,
    /// Explanation for why root_example cannot be used to construct the required variant
    #[serde(skip_serializing_if = "Option::is_none", skip_serializing)]
    pub old_root_example_unavailable_reason: Option<String>,
    /// either the root_example or the root_example_unavailable_reason
    /// depending on which is available on this path
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub root_example: Option<RootExample>,
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for `NotMutable` variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,
    /// The variant signature (Unit, Tuple, or Struct)
    pub signature: VariantSignature,
    /// Mutation status for this signature/variant group
    pub mutability: Mutability,
}

/// Consolidated enum-specific data for mutation paths
/// Added to a `MutationPathInternal` whenever that path is nested in an enum
/// i.e. `!ctx.variant_chain.is_empty()` - whenever we have a variant chain
#[derive(Debug, Clone)]
pub struct EnumPathData {
    /// Chain of enum variants from root to this path
    pub variant_chain: Vec<VariantName>,

    /// All variants that share the same signature and support this path
    pub applicable_variants: Vec<VariantName>,

    /// Complete root example for this specific variant chain
    pub old_root_example: Option<Value>,

    /// Explanation for why root_example cannot be used to construct this variant via BRP.
    /// Only populated for PartiallyMutable/NotMutable variants.
    pub root_example_unavailable_reason: Option<String>,

    /// new root example
    /// will replace current root_example and root-root_example_unavailable_reason with an enum
    /// as they are mutually exclusive
    ///
    /// Available: Complete root example for this specific variant chain
    /// Unavailable: Explanation for why root_example cannot be used to construct this variant via BRP.
    pub root_example: Option<RootExample>,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize)]
pub struct MutationPathExternal {
    /// Human-readable description of what this path mutates
    pub description: String,
    /// Combined path navigation and type metadata
    pub path_info: PathInfo,
    /// Example data (either single value or enum variant groups)
    #[serde(flatten)]
    pub path_example: PathExample,
}

/// Root example for an enum variant, either available for construction or unavailable with reason
///
/// Serializes to JSON as either:
/// - `{"root_example": <value>}` for Available variant
/// - `{"root_example_unavailable_reason": "<reason>"}` for Unavailable variant
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum RootExample {
    /// Variant can be constructed via BRP spawn/insert operations
    Available { root_example: Value },
    /// Variant cannot be constructed via BRP, with explanation
    Unavailable {
        root_example_unavailable_reason: String,
    },
}
