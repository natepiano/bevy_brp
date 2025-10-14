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
pub struct FullMutationPath(String);

impl Deref for FullMutationPath {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for FullMutationPath {
    fn from(path: String) -> Self {
        Self(path)
    }
}

impl From<&str> for FullMutationPath {
    fn from(path: &str) -> Self {
        Self(path.to_string())
    }
}

impl std::fmt::Display for FullMutationPath {
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
pub enum MutationStatus {
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

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path - now type-safe!
    pub example:                PathExample,
    /// Path for mutation, e.g., ".translation.x"
    pub full_mutation_path:     FullMutationPath,
    /// Type information for this path
    pub type_name:              BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:              PathKind,
    /// Status of whether this path can be mutated
    pub mutation_status:        MutationStatus,
    /// Reason if mutation is not possible
    pub mutation_status_reason: Option<Value>,
    /// Consolidated enum-specific data (new approach)
    pub enum_path_data:         Option<EnumPathData>,
    /// Depth level of this path in the recursion tree (0 = root, 1 = .field, etc.)
    /// Used to identify direct children vs grandchildren during assembly
    pub depth:                  usize,

    /// For enum root paths at each nesting level: Maps FULL variant chains to partial
    /// root examples built from this enum level down through all descendants.
    ///
    /// **Populated for paths where `matches!(example, PathExample::EnumRoot { .. })`** - meaning
    /// any path that is the root of an enum type at ANY nesting level:
    /// - Path `""` (`TestVariantChainEnum`) has this field
    /// - Path `".middle_struct.nested_enum"` (`BottomEnum`) has this field
    /// - Leaf paths like `".middle_struct.nested_enum.name"` have None
    ///
    /// Example at `BottomEnum` (path `".middle_struct.nested_enum"`):
    ///   `[WithMiddleStruct, VariantB]` => `{"VariantB": {"name": "...", "value": ...}}`
    ///   `[WithMiddleStruct, VariantA]` => `{"VariantA": 123}`
    ///
    /// Example for `TestVariantChainEnum` with chain `["WithMiddleStruct", "VariantA"]`:
    ///   `{"WithMiddleStruct": {"middle_struct": {"nested_enum": {"VariantA": 1000000}, ...}}}`
    ///
    /// Partial roots built during ascent using assembly approach by wrapping child partial roots
    /// as we ascend through recursion.
    ///
    /// None for non-enum paths (structs, primitives) and enum leaf paths.
    pub partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
}

impl MutationPathInternal {
    /// Convert to summary for reason reporting
    pub fn to_path_summary(&self) -> PathSummary {
        PathSummary {
            full_mutation_path: self.full_mutation_path.clone(),
            type_name:          self.type_name.clone(),
            status:             self.mutation_status,
            reason:             self.mutation_status_reason.clone(),
        }
    }

    /// Check if this path is a direct child at the given parent depth
    pub const fn is_direct_child_at_depth(&self, parent_depth: usize) -> bool {
        self.depth == parent_depth + 1
    }
}

/// Summary of a mutation path for reason reporting
///
/// Generic over the path type to support both `FullMutationPath` (for structs/lists)
/// and `VariantName` (for enums) without requiring early string conversion.
#[derive(Debug, Clone)]
pub struct PathSummary<T = FullMutationPath> {
    pub full_mutation_path: T,
    pub type_name:          BrpTypeName,
    pub status:             MutationStatus,
    pub reason:             Option<Value>,
}

/// Path information combining navigation and type metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind:              PathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name:              BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind:              TypeKind,
    /// Status of whether this path can be mutated
    pub mutation_status:        MutationStatus,
    /// Reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_status_reason: Option<Value>,
    /// Instructions for setting variants required for this mutation path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_instructions:      Option<String>,
    /// Example: `["BottomEnum::VariantB"]`
    /// `VariantName` serializes as a string in JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants:    Option<Vec<VariantName>>,
    /// Only present for paths nested in enums - built using assembly during ascent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example:           Option<Value>,
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<VariantName>,
    /// Example value for this group (omitted for `NotMutable` variants)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:             Option<Value>,
    /// The variant signature as a string
    pub signature:           String,
    /// Mutation status for this signature/variant group
    pub mutation_status:     MutationStatus,
}

/// Consolidated enum-specific data for mutation paths
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
pub struct MutationPath {
    /// Human-readable description of what this path mutates
    pub description: String,
    /// Combined path navigation and type metadata
    pub path_info:   PathInfo,
    /// Array of example groups with variants, signatures, and examples (for enums)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples:    Vec<ExampleGroup>,
    /// Single example value (for non-enum types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:     Option<Value>,
}

impl MutationPath {
    /// Create from `MutationPathInternal` with proper formatting logic
    pub fn from_mutation_path_internal(
        path: &MutationPathInternal,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Get TypeKind for the field type
        let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema);

        // Check for Default trait once at the top for root paths
        let has_default_for_root = if matches!(path.path_kind, PathKind::RootValue { .. }) {
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
        let description = match path.mutation_status {
            MutationStatus::PartiallyMutable => {
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
            _ => path.path_kind.description(&type_kind),
        };

        let (examples, example) = match path.mutation_status {
            MutationStatus::PartiallyMutable => {
                // PartiallyMutable enums: show examples array with per-variant status
                // PartiallyMutable non-enums: check for Default trait
                match &path.example {
                    PathExample::EnumRoot { groups, .. } => (groups.clone(), None),
                    PathExample::Simple(_) => {
                        let example = if has_default_for_root {
                            Some(json!({}))
                        } else {
                            None
                        };
                        (vec![], example)
                    }
                }
            }
            MutationStatus::NotMutable => {
                // NotMutable: no example at all (not even null)
                (vec![], None)
            }
            MutationStatus::Mutable => {
                match &path.example {
                    PathExample::EnumRoot { groups, .. } => {
                        // Enum root: use the examples array
                        (groups.clone(), None)
                    }
                    PathExample::Simple(val) => {
                        // Mutable paths: use the example value
                        // This includes enum children (with embedded `applicable_variants`) and
                        // regular values
                        (vec![], Some(val.clone()))
                    }
                }
            }
        };

        // Extract enum-specific fields (instructions, variants, examples) only for paths that
        // can actually be mutated. This prevents contradictory output where a `not_mutable`
        // path shows instructions on how to mutate it.
        //
        // For example, `.main_animation.0` might be `not_mutable` because `NodeIndex` has no
        // example value. Without this check, we'd show:
        //   - `mutation_status: "not_mutable"` (can't be mutated)
        //   - `enum_instructions: "First, set root to..."` (here's how to mutate it!)
        //
        // This is confusing. Instead, we only include enum metadata for mutable/partially
        // mutable paths where the instructions are actually useful.
        let (enum_instructions, applicable_variants, root_example) = if matches!(
            path.mutation_status,
            MutationStatus::Mutable | MutationStatus::PartiallyMutable
        ) {
            path.enum_path_data
                .as_ref()
                .map_or((None, None, None), |enum_data| {
                    let instructions = Some(format!(
                        "First, set the root mutation path to 'root_example', then you can mutate the '{}' path. See 'applicable_variants' for which variants support this field.", &path.full_mutation_path
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
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
                mutation_status: path.mutation_status,
                mutation_status_reason: path.mutation_status_reason.clone(),
                enum_instructions,
                applicable_variants,
                root_example,
            },
            examples,
            example,
        }
    }
}
