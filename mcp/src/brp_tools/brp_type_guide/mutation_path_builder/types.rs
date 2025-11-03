//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.

use serde::Deserialize;
use serde::Serialize;
use serde::ser::SerializeMap;
use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::super::type_kind::TypeKind;
use super::enum_builder::VariantSignature;
use super::new_types::MutationPath;
use super::new_types::VariantName;
use super::path_example::PathExample;
use super::path_kind::PathKind;

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
    fn from(value: Value) -> Self {
        Self::Json(value)
    }
}

impl From<Example> for Value {
    fn from(example: Example) -> Self {
        example.to_value()
    }
}

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
    pub target:    MutabilityIssueTarget,
    pub type_name: BrpTypeName,
    pub status:    Mutability,
    pub reason:    Option<Value>,
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

/// User facing path information
///
/// This is serialized into the output json, and as such, it intentionally does not
/// match up with the types used to construct it
#[derive(Debug, Clone, Serialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind:           PathKind,
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
    /// either the `root_example` or the `root_example_unavailable_reason`
    /// depending on which is available on this path
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub root_example:        Option<RootExample>,
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for `NotMutable` variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:             Option<Value>,
    /// The variant signature (Unit, Tuple, or Struct)
    pub signature:           VariantSignature,
    /// Mutation status for this signature/variant group
    pub mutability:          Mutability,
}

/// Consolidated enum-specific data for mutation paths
/// Added to a `MutationPathInternal` whenever that path is nested in an enum
/// i.e. `!ctx.variant_chain.is_empty()` - whenever we have a variant chain
#[derive(Debug, Clone)]
pub struct EnumPathInfo {
    /// Chain of enum variants from root to this path
    pub variant_chain: Vec<VariantName>,

    /// All variants that share the same signature and support this path
    pub applicable_variants: Vec<VariantName>,

    /// root example enum - handles mutual exclusivity
    ///
    /// Available: Complete root example for this specific variant chain
    /// Unavailable: Explanation for why `root_example` cannot be used to construct this variant
    /// via BRP.
    pub root_example: Option<RootExample>,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize)]
pub struct MutationPathExternal {
    /// The mutation path (e.g., ".translation.x" or "" for root)
    pub path:         MutationPath,
    /// Human-readable description of what this path mutates
    pub description:  String,
    /// Combined path navigation and type metadata
    pub path_info:    PathInfo,
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

/// Spawn/insert example with educational guidance for AI agents
///
/// Serializes differently based on variant:
/// - `SpawnExample` → `{"spawn_example": {"agent_guidance": "...", "example": <value>}}`
/// - `ResourceExample` → `{"resource_example": {"agent_guidance": "...", "example": <value>}}`
///
/// When `example` is `Example::NotApplicable`, only `agent_guidance` is included.
///
/// Note: Only derives Debug and Clone (NOT Deserialize) because we implement
/// Deserialize manually below with a stub that returns an error.
#[derive(Debug, Clone)]
pub enum SpawnInsertExample {
    SpawnExample {
        agent_guidance: String,
        example:        Example,
    },
    ResourceExample {
        agent_guidance: String,
        example:        Example,
    },
}

impl Serialize for SpawnInsertExample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::SpawnExample {
                agent_guidance,
                example,
            } => {
                // Check if example is NotApplicable (null-equivalent)
                if example.is_null_equivalent() {
                    // Only serialize agent_guidance field
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(
                        "spawn_example",
                        &serde_json::json!({
                            "agent_guidance": agent_guidance
                        }),
                    )?;
                    map.end()
                } else {
                    // Serialize both fields
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(
                        "spawn_example",
                        &serde_json::json!({
                            "agent_guidance": agent_guidance,
                            "example": example.to_value()
                        }),
                    )?;
                    map.end()
                }
            }
            Self::ResourceExample {
                agent_guidance,
                example,
            } => {
                // Check if example is NotApplicable (null-equivalent)
                if example.is_null_equivalent() {
                    // Only serialize agent_guidance field
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(
                        "resource_example",
                        &serde_json::json!({
                            "agent_guidance": agent_guidance
                        }),
                    )?;
                    map.end()
                } else {
                    // Serialize both fields
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(
                        "resource_example",
                        &serde_json::json!({
                            "agent_guidance": agent_guidance,
                            "example": example.to_value()
                        }),
                    )?;
                    map.end()
                }
            }
        }
    }
}

/// Stub `Deserialize` implementation for `SpawnInsertExample`
///
/// Required by serde's flatten attribute but never actually used.
impl<'de> Deserialize<'de> for SpawnInsertExample {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "SpawnInsertExample deserialization not implemented - this type is write-only",
        ))
    }
}
