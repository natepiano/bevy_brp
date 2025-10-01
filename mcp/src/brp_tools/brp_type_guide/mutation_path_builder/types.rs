//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.
use std::collections::HashMap;
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::super::type_kind::TypeKind;
use super::path_kind::PathKind;
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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// A signature for grouping `PathKinds` that have similar structure
/// Used as a `HashMap` key for deduplication in output stage grouping
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Will be used in Plan 1 implementation
pub enum PathSignature {
    Root { type_name: BrpTypeName },
    Field { type_name: BrpTypeName },
    Index { type_name: BrpTypeName },
    Array { type_name: BrpTypeName },
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path
    pub example:                      Value,
    /// For enum roots only: the examples array with all variant groups
    /// None for all other paths (including enum children and regular types)
    pub enum_root_examples:           Option<Vec<ExampleGroup>>,
    /// For enum roots only: simple example for parent assembly
    /// None for all other paths (including enum children and regular types)
    pub enum_root_example_for_parent: Option<Value>,
    /// Path for mutation, e.g., ".translation.x"
    pub full_mutation_path:           FullMutationPath,
    /// Type information for this path
    pub type_name:                    BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:                    PathKind,
    /// Status of whether this path can be mutated
    pub mutation_status:              MutationStatus,
    /// Reason if mutation is not possible
    pub mutation_status_reason:       Option<Value>,
    /// Consolidated enum-specific data (new approach)
    pub enum_data:                    Option<EnumPathData>,
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

    /// Get the signature of this path for grouping purposes
    #[allow(dead_code)] // Will be used in Plan 1 implementation
    pub fn signature(&self) -> PathSignature {
        self.path_kind.to_signature()
    }
}

/// Summary of a mutation path for reason reporting
#[derive(Debug, Clone)]
pub struct PathSummary {
    pub full_mutation_path: FullMutationPath,
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
    /// Ordered list of variant requirements from root to this path (optional)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enum_variant_path:      Vec<VariantPath>,
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<VariantName>,
    /// Example value for this group
    pub example:             Value,
    /// The variant signature as a string
    pub signature:           String,
}

/// Entry describing a variant requirement at a specific path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPath {
    /// The mutation path where this variant is required (e.g., `""`, `".nested_config"`)
    pub full_mutation_path: FullMutationPath,
    /// The variant name including enum type (e.g., `"TestEnumWithSerDe::Nested"`)
    #[serde(skip)]
    pub variant:            VariantName,
    /// Clear instruction for this step (e.g., `"Set root to TestEnumWithSerDe::Nested"`)
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub instructions:       String,
    /// The exact mutation value needed for this step
    #[serde(skip_serializing_if = "Value::is_null", default)]
    pub variant_example:    Value,
}

/// Consolidated enum-specific data for mutation paths
#[derive(Debug, Clone)]
pub struct EnumPathData {
    /// Chain of enum variants from root to this path with full metadata
    pub variant_chain: Vec<VariantPath>,

    /// All variants that share the same signature and support this path
    pub applicable_variants: Vec<VariantName>,

    /// Complete root example for this specific variant chain
    pub variant_chain_root_example: Option<Value>,

    /// Human-readable instructions for using this enum path
    pub enum_instructions: Option<String>,
}

impl EnumPathData {
    pub fn new(variant_chain: Vec<VariantPath>, enum_instructions: Option<String>) -> Self {
        Self {
            variant_chain,
            applicable_variants: Vec::new(),
            variant_chain_root_example: None,
            enum_instructions,
        }
    }

    pub fn with_applicable_variants(mut self, variants: Vec<VariantName>) -> Self {
        self.applicable_variants = variants;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.variant_chain.is_empty()
    }
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

        // Generate description - override for partially_mutable paths
        let description = match path.mutation_status {
            MutationStatus::PartiallyMutable => {
                "This path is not mutable due to some of its descendants not being mutable"
                    .to_string()
            }
            _ => path.path_kind.description(&type_kind),
        };

        let (examples, example) = match path.mutation_status {
            MutationStatus::PartiallyMutable | MutationStatus::NotMutable => {
                // PartiallyMutable and NotMutable: no example at all (not even null)
                (vec![], None)
            }
            MutationStatus::Mutable => {
                path.enum_root_examples.as_ref().map_or_else(
                    || {
                        // Mutable paths: use the example value
                        // This includes enum children (with embedded `applicable_variants`) and
                        // regular values
                        (vec![], Some(path.example.clone()))
                    },
                    |enum_examples| {
                        // Enum root: use the examples array
                        (enum_examples.clone(), None)
                    },
                )
            }
        };

        Self {
            description,
            path_info: PathInfo {
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
                mutation_status: path.mutation_status,
                mutation_status_reason: path.mutation_status_reason.clone(),
                enum_instructions: path
                    .enum_data
                    .as_ref()
                    .and_then(|ed| ed.enum_instructions.clone()),
                enum_variant_path: path
                    .enum_data
                    .as_ref()
                    .map(|ed| ed.variant_chain.clone())
                    .unwrap_or_default(),
            },
            examples,
            example,
        }
    }
}
