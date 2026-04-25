//! Response-facing types for mutation path building.

use serde::Serialize;
use serde_json::Value;

use super::mutability::Mutability;
use super::mutation_path::MutationPath;
use super::path_example::Example;
use super::path_example::PathExample;
use super::path_kind::PathKind;
use super::variant_name::VariantName;
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::brp_tools::brp_type_guide::type_kind::TypeKind;

/// User facing path information
///
/// This is serialized into the output json, and as such, it intentionally does not
/// match up with the types used to construct it.
#[derive(Debug, Clone, Serialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub(super) path_kind:    PathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name:           BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind:           TypeKind,
    /// Status of whether this path can be mutated
    pub mutability:          Mutability,
    /// Reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability_reason:   Option<Value>,
    /// Example: `["BottomEnum::VariantB"]`
    /// `VariantName` serializes as a string in JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants: Option<Vec<VariantName>>,
    /// Instructions for setting variants required for this mutation path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_instructions:   Option<String>,
    /// Either the `root_example` or the `root_example_unavailable_reason`
    /// depending on which is available on this path
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub root_example:        Option<RootExample>,
}

/// Information about a mutation path that we serialize to our response.
#[derive(Debug, Clone, Serialize)]
pub struct MutationPathExternal {
    /// The mutation path (e.g., ".translation.x" or "" for root)
    pub path:        MutationPath,
    /// Human-readable description of what this path mutates
    pub description: String,
    /// Combined path navigation and type metadata
    pub path_info:   PathInfo,
    /// Example data (either single value or enum variant groups)
    #[serde(flatten)]
    path_example:    PathExample,
}

impl MutationPathExternal {
    pub(super) const fn new(
        path: MutationPath,
        description: String,
        path_info: PathInfo,
        path_example: PathExample,
    ) -> Self {
        Self {
            path,
            description,
            path_info,
            path_example,
        }
    }

    pub(super) fn preferred_example(&self) -> Example { self.path_example.preferred_example() }
}

/// Root example for an enum variant, either available for construction or unavailable with reason
///
/// Serializes to JSON as either:
/// - `{"example": <value>}` for Available variant
/// - `{"unavailable_reason": "<reason>"}` for Unavailable variant
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum RootExample {
    /// Variant can be constructed via BRP spawn/insert operations
    Available { example: Value },
    /// Variant cannot be constructed via BRP, with explanation
    Unavailable { unavailable_reason: String },
}
