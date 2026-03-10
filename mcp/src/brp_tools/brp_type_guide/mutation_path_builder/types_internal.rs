//! Internal types for mutation path building

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::super::variant_signature::VariantSignature;
use super::new_types::MutationPath;
use super::new_types::VariantName;
use super::types_response::RootExample;

/// Self-documenting wrapper for example values in mutation paths
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Example {
    /// A regular JSON value
    Json(Value),

    /// Explicit `Option::None` (serializes to null)
    OptionNone,

    /// No example available (for `NotMutable` paths)
    NotApplicable,
}

impl Example {
    /// Convert to Value for JSON operations (assembly, serialization)
    pub fn to_value(&self) -> Value {
        match self {
            Self::Json(v) => v.clone(),
            Self::OptionNone | Self::NotApplicable => Value::Null,
        }
    }

    /// Returns true if this Example represents a null-equivalent value
    /// (`OptionNone` or `NotApplicable`)
    pub const fn is_null_equivalent(&self) -> bool {
        matches!(self, Self::OptionNone | Self::NotApplicable)
    }
}

impl From<Value> for Example {
    fn from(value: Value) -> Self { Self::Json(value) }
}

impl From<Example> for Value {
    fn from(example: Example) -> Self { example.to_value() }
}

/// Action to take regarding path creation during recursion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PathAction {
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

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ExampleGroup {
    /// List of variants that share this signature
    pub(super) applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for `NotMutable` variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) example:             Option<Value>,
    /// The variant signature (Unit, Tuple, or Struct)
    pub(super) signature:           VariantSignature,
    /// Mutation status for this signature/variant group
    pub(super) mutability:          Mutability,
}

/// Consolidated enum-specific data for mutation paths
/// Added to a `MutationPathInternal` whenever that path is nested in an enum
/// i.e. `!ctx.variant_chain.is_empty()` - whenever we have a variant chain
#[derive(Debug, Clone)]
pub(super) struct EnumPathInfo {
    /// Chain of enum variants from root to this path
    pub(super) variant_chain: Vec<VariantName>,

    /// All variants that share the same signature and support this path
    pub(super) applicable_variants: Vec<VariantName>,

    /// root example enum - handles mutual exclusivity
    ///
    /// Available: Complete root example for this specific variant chain
    /// Unavailable: Explanation for why `root_example` cannot be used to construct this variant
    /// via BRP.
    pub(super) root_example: Option<RootExample>,
}
