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

/// Example value for a mutation path
///
/// This enum ensures we cannot accidentally use the wrong example format for a path.
/// Enum roots MUST use `EnumRoot` variant, non-enum paths MUST use `Simple` variant.
#[derive(Debug, Clone)]
pub enum PathExample {
    /// Simple value example used by non-enum types
    ///
    /// Examples:
    /// - Structs: `{"field1": value1, "field2": value2}`
    /// - Primitives: `42`, `"text"`, `true`
    /// - Arrays: `[1, 2, 3]`
    /// - `Option::None`: `null` (special case for Option enum)
    Simple(Value),

    /// Enum root with variant groups and parent assembly value
    ///
    /// Only used for enum root paths.
    /// The `for_parent` field provides the simplified example that parent types
    /// use when assembling their own examples.
    EnumRoot {
        /// All variant groups for this enum (the `examples` array in JSON output)
        groups:     Vec<ExampleGroup>,
        /// Simplified example for parent assembly
        for_parent: Value,
    },
}

impl PathExample {
    /// Get the value to use for parent assembly
    ///
    /// For `Simple`, returns the value directly.
    /// For `EnumRoot`, returns the `for_parent` field.
    ///
    /// This is the ONLY helper method provided. All other usage should use explicit
    /// pattern matching to maintain type safety and force exhaustive handling of both cases.
    pub const fn for_parent(&self) -> &Value {
        match self {
            Self::Simple(val) => val,
            Self::EnumRoot { for_parent, .. } => for_parent,
        }
    }
}

/// Custom serialization for `PathExample` that flattens into parent struct
///
/// This produces the correct JSON format for `MutationPathExternal`:
/// - `Simple(value)` → `"example": <value>` (skipped if value is null)
/// - `EnumRoot { groups, .. }` → `"examples": <groups>`
impl Serialize for PathExample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Simple(value) => {
                // Skip serializing null examples to match V1 behavior
                if value.is_null() {
                    serializer.serialize_map(Some(0))?.end()
                } else {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("example", value)?;
                    map.end()
                }
            }
            Self::EnumRoot { groups, .. } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("examples", groups)?;
                map.end()
            }
        }
    }
}

/// Stub `Deserialize` implementation for `PathExample`
///
/// This is required by serde's flatten attribute but never actually used
/// since we only serialize `MutationPathExternal`, never deserialize it.
impl<'de> Deserialize<'de> for PathExample {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "PathExample deserialization not implemented - this type is write-only",
        ))
    }
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

/// Path information combining navigation and type metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind:                       PathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name:                       BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind:                       TypeKind,
    /// Status of whether this path can be mutated
    pub mutability:                      Mutability,
    /// Reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability_reason:               Option<Value>,
    /// Instructions for setting variants required for this mutation path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_instructions:               Option<String>,
    /// Example: `["BottomEnum::VariantB"]`
    /// `VariantName` serializes as a string in JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants:             Option<Vec<VariantName>>,
    /// Only present for paths nested in enums - built using assembly during ascent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example:                    Option<Value>,
    /// Explanation for why root_example cannot be used to construct the required variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example_unavailable_reason: Option<String>,
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
pub struct EnumPathData {
    /// Chain of enum variants from root to this path
    pub variant_chain: Vec<VariantName>,

    /// All variants that share the same signature and support this path
    pub applicable_variants: Vec<VariantName>,

    /// Complete root example for this specific variant chain
    pub root_example: Option<Value>,

    /// Explanation for why root_example cannot be used to construct this variant via BRP.
    /// Only populated for PartiallyMutable/NotMutable variants.
    pub root_example_unavailable_reason: Option<String>,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    /// Human-readable description of what this path mutates
    pub description:  String,
    /// Combined path navigation and type metadata
    pub path_info:    PathInfo,
    /// Example data (either single value or enum variant groups)
    #[serde(flatten)]
    pub path_example: PathExample,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_path_example_simple_non_null_serialization() {
        let example = PathExample::Simple(json!({"x": 1.0, "y": 2.0}));
        let serialized = serde_json::to_value(&example).unwrap();

        // Should serialize as {"example": <value>}
        assert_eq!(
            serialized,
            json!({
                "example": {"x": 1.0, "y": 2.0}
            })
        );
    }

    #[test]
    fn test_path_example_simple_null_serialization() {
        let example = PathExample::Simple(Value::Null);
        let serialized = serde_json::to_value(&example).unwrap();

        // Should serialize as empty object (no "example" field)
        assert_eq!(serialized, json!({}));
    }

    #[test]
    fn test_path_example_enum_root_serialization() {
        let groups = vec![ExampleGroup {
            applicable_variants: vec![VariantName::from("SomeVariant".to_string())],
            example:             Some(json!(42)),
            signature:           VariantSignature::Tuple(vec![BrpTypeName::from("i32")]),
            mutability:          Mutability::Mutable,
        }];

        let example = PathExample::EnumRoot {
            groups:     groups.clone(),
            for_parent: json!(42),
        };

        let serialized = serde_json::to_value(&example).unwrap();

        // Should serialize as {"examples": <groups>}
        assert_eq!(
            serialized,
            json!({
                "examples": groups
            })
        );
    }

    #[test]
    fn test_mutation_path_external_with_null_example() {
        // Create a minimal MutationPathExternal with null example
        let path_external = MutationPathExternal {
            description:  "Test path".to_string(),
            path_info:    PathInfo {
                path_kind:           PathKind::RootValue {
                    type_name: BrpTypeName::from("test::Type"),
                },
                type_name:           BrpTypeName::from("test::Type"),
                type_kind:           TypeKind::Struct,
                mutability:          Mutability::NotMutable,
                mutability_reason:   Some(json!("missing trait")),
                enum_instructions:   None,
                applicable_variants: None,
                root_example:        None,
            },
            path_example: PathExample::Simple(Value::Null),
        };

        let serialized = serde_json::to_value(&path_external).unwrap();

        // Verify that "example" field is NOT present in the serialized output
        assert!(
            !serialized.as_object().unwrap().contains_key("example"),
            "NotMutable path should not have 'example' field"
        );
    }

    #[test]
    fn test_mutation_path_external_with_non_null_example() {
        // Create a minimal MutationPathExternal with non-null example
        let path_external = MutationPathExternal {
            description:  "Test path".to_string(),
            path_info:    PathInfo {
                path_kind:           PathKind::RootValue {
                    type_name: BrpTypeName::from("test::Type"),
                },
                type_name:           BrpTypeName::from("test::Type"),
                type_kind:           TypeKind::Struct,
                mutability:          Mutability::Mutable,
                mutability_reason:   None,
                enum_instructions:   None,
                applicable_variants: None,
                root_example:        None,
            },
            path_example: PathExample::Simple(json!({"field": "value"})),
        };

        let serialized = serde_json::to_value(&path_external).unwrap();

        // Verify that "example" field IS present with the correct value
        assert_eq!(
            serialized.get("example").unwrap(),
            &json!({"field": "value"}),
            "Mutable path should have 'example' field with correct value"
        );
    }
}
