//! Response-facing types for mutation path building

use serde::Deserialize;
use serde::Serialize;
use serde::ser::SerializeMap;
use serde_json::Value;

use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::brp_tools::brp_type_guide::type_kind::TypeKind;
use super::new_types::MutationPath;
use super::new_types::VariantName;
use super::path_example::PathExample;
use super::path_kind::PathKind;
use super::types_internal::Example;
use super::types_internal::Mutability;

/// User facing path information
///
/// This is serialized into the output json, and as such, it intentionally does not
/// match up with the types used to construct it
#[derive(Debug, Clone, Serialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    path_kind:               PathKind,
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

/// Parameters for constructing a `PathInfo`
pub(super) struct PathInfoParams {
    pub(super) path_kind:           PathKind,
    pub(super) type_name:           BrpTypeName,
    pub(super) type_kind:           TypeKind,
    pub(super) mutability:          Mutability,
    pub(super) mutability_reason:   Option<Value>,
    pub(super) applicable_variants: Option<Vec<VariantName>>,
    pub(super) enum_instructions:   Option<String>,
    pub(super) root_example:        Option<RootExample>,
}

impl From<PathInfoParams> for PathInfo {
    fn from(params: PathInfoParams) -> Self {
        Self {
            path_kind:           params.path_kind,
            type_name:           params.type_name,
            type_kind:           params.type_kind,
            mutability:          params.mutability,
            mutability_reason:   params.mutability_reason,
            applicable_variants: params.applicable_variants,
            enum_instructions:   params.enum_instructions,
            root_example:        params.root_example,
        }
    }
}

/// Information about a mutation path that we serialize to our response
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
                if example.is_null_equivalent() {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(
                        "spawn_example",
                        &serde_json::json!({
                            "agent_guidance": agent_guidance
                        }),
                    )?;
                    map.end()
                } else {
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
            },
            Self::ResourceExample {
                agent_guidance,
                example,
            } => {
                if example.is_null_equivalent() {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(
                        "resource_example",
                        &serde_json::json!({
                            "agent_guidance": agent_guidance
                        }),
                    )?;
                    map.end()
                } else {
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
            },
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
