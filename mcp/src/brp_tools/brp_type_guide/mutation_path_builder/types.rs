//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::brp_type_name::BrpTypeName;
use super::super::constants::{DEFAULT_SPAWN_GUIDANCE, REFLECT_TRAIT_DEFAULT};
use super::super::type_kind::TypeKind;
use super::path_kind::PathKind;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Full mutation path for BRP operations (e.g., ".translation.x")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationPath(String);

impl Deref for MutationPath {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for MutationPath {
    fn from(path: String) -> Self {
        Self(path)
    }
}

impl From<&str> for MutationPath {
    fn from(path: &str) -> Self {
        Self(path.to_string())
    }
}

impl std::fmt::Display for MutationPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A struct field name used in mutation paths and variant signatures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct StructFieldName(String);

impl StructFieldName {
    /// Get the field name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StructFieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::borrow::Borrow<str> for StructFieldName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<String> for StructFieldName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for StructFieldName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<SchemaField> for StructFieldName {
    fn from(field: SchemaField) -> Self {
        Self(field.to_string())
    }
}

/// A variant name from a Bevy enum type (e.g., "`Option<String>::Some`", "`Color::Srgba`")
///
/// This newtype wrapper provides type safety and documentation for variant names
/// discovered through Bevy's reflection system at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct VariantName(String);

impl From<String> for VariantName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl std::fmt::Display for VariantName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl VariantName {
    /// Get the variant name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
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

/// Variant signature types for enum variants - used for grouping similar structures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum VariantSignature {
    /// Unit variant (no data)
    Unit,
    /// Tuple variant with ordered types
    Tuple(Vec<BrpTypeName>),
    /// Struct variant with named fields and types
    Struct(Vec<(StructFieldName, BrpTypeName)>),
}

impl std::fmt::Display for VariantSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Tuple(types) => {
                let type_names: Vec<String> =
                    types.iter().map(|t| t.display_name().to_string()).collect();
                write!(f, "tuple({})", type_names.join(", "))
            }
            Self::Struct(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, type_name)| format!("{}: {}", name, type_name.display_name()))
                    .collect();
                write!(f, "struct{{{}}}", field_strs.join(", "))
            }
        }
    }
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
        groups: Vec<ExampleGroup>,
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

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path - now type-safe!
    pub example: PathExample,
    /// Path for mutation, e.g., ".translation.x"
    pub mutation_path: MutationPath,
    /// Type information for this path
    pub type_name: BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind: PathKind,
    /// Whether this path can be mutated
    pub mutability: Mutability,
    /// Reason if mutation is not possible
    pub mutability_reason: Option<Value>,
    /// Consolidated enum-specific data (new approach)
    pub enum_path_data: Option<EnumPathData>,
    /// Depth level of this path in the recursion tree (0 = root, 1 = .field, etc.)
    /// Used to identify direct children vs grandchildren during assembly
    pub depth: usize,
    /// Maps variant chains to complete root examples for reaching nested enum paths.
    /// Populated during enum processing for paths where `matches!(example, PathExample::EnumRoot {
    /// .. })`. Built by `build_partial_root_examples()` in `enum_path_builder.rs` during
    /// ascent phase. None for non-enum paths and enum leaf paths.
    pub partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
}

impl MutationPathInternal {
    /// Check if this path is a direct child at the given parent depth
    pub const fn is_direct_child_at_depth(&self, parent_depth: usize) -> bool {
        self.depth == parent_depth + 1
    }
}

/// Identifies what component has a mutability issue
#[derive(Debug, Clone)]
pub enum MutabilityTarget {
    /// A mutation path within a type (e.g., ".translation.x")
    Path(MutationPath),
    /// An enum variant name (e.g., "`Color::Srgba`")
    Variant(VariantName),
}

impl std::fmt::Display for MutabilityTarget {
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
    pub target: MutabilityTarget,
    pub type_name: BrpTypeName,
    pub status: Mutability,
    pub reason: Option<Value>,
}

impl MutabilityIssue {
    /// Create from a mutation path (for non-enum types)
    pub fn from_mutation_path(path: &MutationPathInternal) -> Self {
        Self {
            target: MutabilityTarget::Path(path.mutation_path.clone()),
            type_name: path.type_name.clone(),
            status: path.mutability,
            reason: path.mutability_reason.clone(),
        }
    }

    /// Create from an enum variant name (for enum types)
    pub const fn from_variant_name(
        variant: VariantName,
        type_name: BrpTypeName,
        status: Mutability,
    ) -> Self {
        Self {
            target: MutabilityTarget::Variant(variant),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example: Option<Value>,
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for `NotMutable` variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,
    /// The variant signature as a string
    pub signature: String,
    /// Mutation status for this signature/variant group
    pub mutability: Mutability,
}

/// Consolidated enum-specific data for mutation paths
/// Added to a 'MutationPathInternal' whenever that path is nested in an enum
/// i.e. `!ctx.variant_chain.is_empty()` - whenever we have a variant chain
#[derive(Debug, Clone)]
pub struct EnumPathData {
    /// Chain of enum variants from root to this path
    pub variant_chain: Vec<VariantName>,

    /// All variants that share the same signature and support this path
    pub applicable_variants: Vec<VariantName>,

    /// Complete root example for this specific variant chain
    pub root_example: Option<Value>,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    /// Human-readable description of what this path mutates
    pub description: String,
    /// Combined path navigation and type metadata
    pub path_info: PathInfo,
    /// Example data (either single value or enum variant groups)
    #[serde(flatten)]
    pub path_example: PathExample,
}

impl MutationPathExternal {
    /// Create from `MutationPathInternal` with proper formatting logic
    pub fn from_mutation_path_internal(
        mutation_path_internal: &MutationPathInternal,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Get TypeKind for the field type
        let field_schema = registry
            .get(&mutation_path_internal.type_name)
            .unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema);

        // Check for Default trait once at the top for root paths
        let has_default_for_root =
            if matches!(mutation_path_internal.path_kind, PathKind::RootValue { .. }) {
                field_schema
                    .get_field_array(SchemaField::ReflectTypes)
                    .is_some_and(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .any(|t| t == REFLECT_TRAIT_DEFAULT)
                    })
            } else {
                false
            };

        // Generate description - override for partially_mutable paths
        // Use type-specific terminology (fields, elements, entries, variants) instead of generic
        // "descendants"
        let description = match mutation_path_internal.mutability {
            Mutability::PartiallyMutable => {
                let base_msg = format!(
                    "This {} path is partially mutable due to some of its {} not being mutable",
                    type_kind.as_ref().to_lowercase(),
                    type_kind.child_terminology()
                );
                if has_default_for_root {
                    format!("{base_msg}.{DEFAULT_SPAWN_GUIDANCE}")
                } else {
                    base_msg
                }
            }
            _ => mutation_path_internal.path_kind.description(&type_kind),
        };

        // Handle PathExample based on mutability status
        let path_example = match mutation_path_internal.mutability {
            Mutability::NotMutable => {
                // NotMutable: no example at all (use Simple with null)
                PathExample::Simple(Value::Null)
            }
            Mutability::PartiallyMutable => {
                // PartiallyMutable: check for Default trait on non-enum types
                match &mutation_path_internal.example {
                    PathExample::EnumRoot { .. } => mutation_path_internal.example.clone(),
                    PathExample::Simple(_) => {
                        if has_default_for_root {
                            PathExample::Simple(json!({}))
                        } else {
                            PathExample::Simple(Value::Null)
                        }
                    }
                }
            }
            Mutability::Mutable => mutation_path_internal.example.clone(),
        };

        // Extract enum-specific fields (instructions, variants, examples) only for paths that
        // can actually be mutated. This prevents contradictory output where a `not_mutable`
        // path shows instructions on how to mutate it.
        //
        // For example, `.main_animation.0` might be `not_mutable` because `NodeIndex` has no
        // example value. Without this check, we'd show:
        //   - `mutability: "not_mutable"` (can't be mutated)
        //   - `enum_instructions: "First, set root to..."` (here's how to mutate it!)
        //
        // This is confusing. Instead, we only include enum metadata for mutable/partially
        // mutable paths where the instructions are actually useful.
        let (enum_instructions, applicable_variants, root_example) = if matches!(
            mutation_path_internal.mutability,
            Mutability::Mutable | Mutability::PartiallyMutable
        ) {
            mutation_path_internal.enum_path_data
                .as_ref()
                .map_or((None, None, None), |enum_data| {
                    let instructions = Some(format!(
                        "First, set the root mutation path to 'root_example', then you can mutate the '{}' path. See 'applicable_variants' for which variants support this field.", &mutation_path_internal.mutation_path
                    ));
                    let variants = if enum_data.applicable_variants.is_empty() {
                        None
                    } else {
                        Some(enum_data.applicable_variants.clone())
                    };
                    (instructions, variants, enum_data.root_example.clone())
                })
        } else {
            // NotMutable paths: omit enum instructions, variants, and examples
            (None, None, None)
        };

        Self {
            description,
            path_info: PathInfo {
                path_kind: mutation_path_internal.path_kind.clone(),
                type_name: mutation_path_internal.type_name.clone(),
                type_kind,
                mutability: mutation_path_internal.mutability,
                mutability_reason: mutation_path_internal.mutability_reason.clone(),
                enum_instructions,
                applicable_variants,
                root_example,
            },
            path_example,
        }
    }
}

#[cfg(test)]
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
            example: Some(json!(42)),
            signature: "tuple(i32)".to_string(),
            mutability: Mutability::Mutable,
        }];

        let example = PathExample::EnumRoot {
            groups: groups.clone(),
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
            description: "Test path".to_string(),
            path_info: PathInfo {
                path_kind: PathKind::RootValue {
                    type_name: BrpTypeName::from("test::Type"),
                },
                type_name: BrpTypeName::from("test::Type"),
                type_kind: TypeKind::Struct,
                mutability: Mutability::NotMutable,
                mutability_reason: Some(json!("missing trait")),
                enum_instructions: None,
                applicable_variants: None,
                root_example: None,
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
            description: "Test path".to_string(),
            path_info: PathInfo {
                path_kind: PathKind::RootValue {
                    type_name: BrpTypeName::from("test::Type"),
                },
                type_name: BrpTypeName::from("test::Type"),
                type_kind: TypeKind::Struct,
                mutability: Mutability::Mutable,
                mutability_reason: None,
                enum_instructions: None,
                applicable_variants: None,
                root_example: None,
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
