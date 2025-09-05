//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including mutation path structures and status types.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::response_types::BrpTypeName;
use super::TypeKind;
use super::path_kind::PathKind;

/// Status of whether a mutation path can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Path can be fully mutated
    Mutatable,
    /// Path cannot be mutated (missing traits, unsupported type, etc.)
    NotMutatable,
    /// Path is partially mutatable (some elements mutable, others not)
    PartiallyMutatable,
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path
    pub example:         Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:            String,
    /// For enum types, list of valid variant names
    pub enum_variants:   Option<Vec<String>>,
    /// Type information for this path
    pub type_name:       BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:       PathKind,
    /// Status of whether this path can be mutated
    pub mutation_status: MutationStatus,
    /// Error reason if mutation is not possible
    pub error_reason:    Option<String>,
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
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPath {
    /// Human-readable description of what this path mutates
    pub description:      String,
    /// Combined path navigation and type metadata
    pub path_info:        PathInfo,
    /// Status of whether this path can be mutated
    pub mutation_status:  MutationStatus,
    /// Error reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason:     Option<String>,
    /// Example value for mutations (for non-Option types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:          Option<Value>,
    /// Example value for setting Some variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_some:     Option<Value>,
    /// Example value for setting None variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_none:     Option<Value>,
    /// List of valid enum variants for this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_variants:    Option<Vec<String>>,
    /// Example values for enum variants (maps variant names to example JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_variants: Option<HashMap<String, Value>>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:             Option<String>,
}

impl MutationPath {
    /// Create from internal `MutationPath` with proper formatting logic
    pub fn from_mutation_path(
        path: &MutationPathInternal,
        description: String,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Regular non-Option path
        let example_variants = if path.enum_variants.is_some() {
            // This is an enum type - generate example variants using the new system
            let enum_type = Some(&path.type_name); // Extract enum type from path
            let examples = super::build_all_enum_examples(type_schema, registry, 0, enum_type); // Pass both
            if examples.is_empty() {
                None
            } else {
                Some(examples)
            }
        } else {
            None
        };

        // Compute enum_variants from example_variants keys (alphabetically sorted)
        let enum_variants = example_variants.as_ref().map(|variants| {
            let mut keys: Vec<String> = variants.keys().cloned().collect();
            keys.sort(); // Alphabetical sorting for consistency
            keys
        });

        // Get TypeKind for the field type
        let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema, &path.type_name);

        Self {
            description,
            path_info: PathInfo {
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
            },
            example: if path.example.is_null() {
                None
            } else {
                Some(path.example.clone())
            },
            example_some: None,
            example_none: None,
            enum_variants,
            example_variants,
            note: None,
            mutation_status: path.mutation_status,
            error_reason: path.error_reason.clone(),
        }
    }
}
