//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::TypeKind;
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariantSignature {
    /// Unit variant (no data)
    Unit,
    /// Tuple variant with ordered types
    Tuple(Vec<BrpTypeName>),
    /// Struct variant with named fields and types
    Struct(Vec<(String, BrpTypeName)>),
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
    pub path:                         String,
    /// Type information for this path
    pub type_name:                    BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:                    PathKind,
    /// Status of whether this path can be mutated
    pub mutation_status:              MutationStatus,
    /// Reason if mutation is not possible
    pub mutation_status_reason:       Option<Value>,
    /// Requirement information for paths needing specific enum variants
    pub path_requirement:             Option<PathRequirement>,
}

impl MutationPathInternal {
    /// Convert to summary for reason reporting
    pub fn to_path_summary(&self) -> PathSummary {
        PathSummary {
            path:      self.path.clone(),
            type_name: self.type_name.clone(),
            status:    self.mutation_status,
            reason:    self.mutation_status_reason.clone(),
        }
    }
}

/// Summary of a mutation path for reason reporting
#[derive(Debug, Clone)]
pub struct PathSummary {
    pub path:      String,
    pub type_name: BrpTypeName,
    pub status:    MutationStatus,
    pub reason:    Option<Value>,
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
    /// Requirement information for paths needing specific enum variants
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_requirement:       Option<PathRequirement>,
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<String>,
    /// Example value for this group
    pub example:             Value,
    /// The variant signature as a string
    pub signature:           String,
}

/// Entry describing a variant requirement at a specific path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPathEntry {
    /// The mutation path where this variant is required (e.g., `""`, `".nested_config"`)
    pub path:    String,
    /// The variant name including enum type (e.g., `"TestEnumWithSerDe::Nested"`)
    pub variant: String,
}

/// Requirement information for paths that need specific enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRequirement {
    /// Human-readable description of the variant requirements
    pub description:  String,
    /// Example value showing the complete structure needed
    pub example:      Value,
    /// Ordered list of variant requirements from root to this path
    pub variant_path: Vec<VariantPathEntry>,
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
        let type_kind = TypeKind::from_schema(field_schema, &path.type_name);

        // Generate description using the context
        let description = path.path_kind.description(&type_kind);

        let (examples, example) = path.enum_root_examples.as_ref().map_or_else(
            || {
                // Everything else: use the example value
                // This includes enum children (with embedded `applicable_variants`) and regular
                // values
                (vec![], Some(path.example.clone()))
            },
            |enum_examples| {
                // Enum root: use the examples array
                (enum_examples.clone(), None)
            },
        );

        Self {
            description,
            path_info: PathInfo {
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
                mutation_status: path.mutation_status,
                mutation_status_reason: path.mutation_status_reason.clone(),
                path_requirement: path.path_requirement.clone(),
            },
            examples,
            example,
        }
    }
}
