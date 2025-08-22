//! Public API result types for the `brp_type_schema` tool
//!
//! This module contains the strongly-typed structures that form the public API
//! for type schema discovery results. These types are separate from the internal
//! processing types to provide a clean, stable API contract.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{EnumVariantKind, MutationPath};

/// Summary statistics for the discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaSummary {
    /// Number of types that failed discovery
    pub failed_discoveries:     usize,
    /// Number of types successfully discovered
    pub successful_discoveries: usize,
    /// Total number of types requested
    pub total_requested:        usize,
}

/// Information about a mutation path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathInfo {
    /// Human-readable description of what this path mutates
    pub description:   String,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name:     String,
    /// Example value for mutations (for non-Option types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:       Option<Value>,
    /// Example value for setting Some variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_some:  Option<Value>,
    /// Example value for setting None variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_none:  Option<Value>,
    /// List of valid enum variants for this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_variants: Option<Vec<String>>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:          Option<String>,
}

/// Information about an enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariantInfo {
    /// Name of the variant
    pub name:         String,
    /// Type of the variant (Unit, Tuple, Struct)
    pub variant_type: EnumVariantKind,
    /// Fields for struct variants
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields:       Option<Vec<EnumFieldInfo>>,
    /// Types for tuple variants
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuple_types:  Option<Vec<String>>,
}

/// Information about a field in an enum struct variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumFieldInfo {
    /// Field name
    pub name:      String,
    /// Field type
    #[serde(rename = "type")]
    pub type_name: String,
}

impl MutationPathInfo {
    /// Create from internal `MutationPath` with proper formatting logic
    pub fn from_mutation_path(path: &MutationPath, description: String, is_option: bool) -> Self {
        if is_option {
            // For Option types, check if we have the special format
            if let Some(examples_obj) = path.example.as_object()
                && examples_obj.contains_key("some")
                && examples_obj.contains_key("none")
            {
                return Self {
                    description,
                    type_name: path.type_name.clone().unwrap_or_default(),
                    example: None,
                    example_some: Some(examples_obj["some"].clone()),
                    example_none: Some(examples_obj["none"].clone()),
                    enum_variants: path.enum_variants.clone(),
                    note: Some(
                        "For Option fields: pass the value directly to set Some, null to set None"
                            .to_string(),
                    ),
                };
            }
        }

        // Regular non-Option path
        Self {
            description,
            type_name: path.type_name.clone().unwrap_or_default(),
            example: if path.example.is_null() {
                None
            } else {
                Some(path.example.clone())
            },
            example_some: None,
            example_none: None,
            enum_variants: path.enum_variants.clone(),
            note: None,
        }
    }
}

// V2 Response Types for parallel implementation

/// V2 response structure - same as V1 but uses `TypeInfoV2`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaResponse {
    /// Number of types successfully discovered
    pub discovered_count: usize,
    /// List of type names that were requested
    pub requested_types:  Vec<String>,
    /// Whether the discovery operation succeeded overall
    pub success:          bool,
    /// Summary statistics for the discovery operation
    pub summary:          TypeSchemaSummary,
    /// Detailed information for each type, keyed by type name
    pub type_info:        HashMap<String, TypeInfo>,
}

/// V2 version of `TypeInfo` - same structure as V1 but without `registry_schema` field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Fully-qualified type name
    pub type_name:            String,
    /// Category of the type (Struct, Enum, etc.)
    pub type_category:        String,
    /// Whether the type is registered in the Bevy registry
    pub in_registry:          bool,
    /// Whether the type has the Serialize trait
    pub has_serialize:        bool,
    /// Whether the type has the Deserialize trait
    pub has_deserialize:      bool,
    /// List of BRP operations supported by this type
    pub supported_operations: Vec<String>,
    /// Mutation paths available for this type - using same format as V1
    pub mutation_paths:       HashMap<String, MutationPathInfo>,
    /// Example values for spawn/insert operations (currently empty to match V1)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub example_values:       HashMap<String, Value>,
    /// Information about enum variants if this is an enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_info:            Option<Vec<EnumVariantInfo>>,
    /// Error message if discovery failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:                Option<String>,
}
