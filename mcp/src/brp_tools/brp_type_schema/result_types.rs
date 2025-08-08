//! Public API result types for the brp_type_schema tool
//!
//! This module contains the strongly-typed structures that form the public API
//! for type schema discovery results. These types are separate from the internal
//! processing types to provide a clean, stable API contract.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{CachedTypeInfo, MutationPath};

/// The main response structure for type schema discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaResponse {
    /// Number of types successfully discovered
    pub discovered_count: usize,
    /// List of type names that were requested
    pub requested_types:  Vec<String>,
    /// Whether the overall operation succeeded
    pub success:          bool,
    /// Summary statistics about the discovery process
    pub summary:          TypeSchemaSummary,
    /// Detailed information for each type, keyed by type name
    pub type_info:        HashMap<String, TypeInfo>,
}

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

/// Detailed information about a single type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Fully-qualified type name
    pub type_name:            String,
    /// Category of the type (Struct, Enum, TupleStruct, etc.)
    pub type_category:        String,
    /// Whether the type is registered in the Bevy registry
    pub in_registry:          bool,
    /// Whether the type has the Serialize trait
    pub has_serialize:        bool,
    /// Whether the type has the Deserialize trait
    pub has_deserialize:      bool,
    /// List of BRP operations supported by this type
    pub supported_operations: Vec<String>,
    /// Mutation paths available for this type
    pub mutation_paths:       HashMap<String, MutationPathInfo>,
    /// Example values for spawn/insert operations (currently empty)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub example_values:       HashMap<String, Value>,
    /// Information about enum variants if this is an enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_info:            Option<Vec<EnumVariantInfo>>,
    /// Error message if discovery failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:                Option<String>,
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
    pub variant_type: String,
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

// Builder/conversion implementations
impl TypeInfo {
    /// Create a TypeInfo from internal CachedTypeInfo
    pub fn from_cached_info(type_name: &str, cached: &CachedTypeInfo) -> Self {
        // Check for Serialize/Deserialize traits
        let has_serialize = cached.reflect_types.contains(&"Serialize".to_string());
        let has_deserialize = cached.reflect_types.contains(&"Deserialize".to_string());

        // Convert supported operations to strings
        let supported_operations: Vec<String> = cached
            .supported_operations
            .iter()
            .map(|op| op.as_ref().to_string())
            .collect();

        // Build mutation paths - this will be filled in by the caller
        // since it requires complex logic from tool.rs
        let mutation_paths = HashMap::new();

        // Currently we don't populate example_values
        let example_values = HashMap::new();

        Self {
            type_name: type_name.to_string(),
            type_category: cached.type_category.to_string(),
            in_registry: true,
            has_serialize,
            has_deserialize,
            supported_operations,
            mutation_paths,
            example_values,
            enum_info: None, // Will be populated by caller if needed
            error: None,
        }
    }

    /// Create an error TypeInfo for types not found in registry
    pub fn error(type_name: &str) -> Self {
        Self {
            type_name:            type_name.to_string(),
            type_category:        String::new(),
            in_registry:          false,
            has_serialize:        false,
            has_deserialize:      false,
            supported_operations: Vec::new(),
            mutation_paths:       HashMap::new(),
            example_values:       HashMap::new(),
            enum_info:            None,
            error:                Some("Type not found in registry".to_string()),
        }
    }
}

impl MutationPathInfo {
    /// Create from internal MutationPath with proper formatting logic
    pub fn from_mutation_path(path: &MutationPath, description: String, is_option: bool) -> Self {
        if is_option {
            // For Option types, check if we have the special format
            if let Some(examples_obj) = path.example_value.as_object() {
                if examples_obj.contains_key("some") && examples_obj.contains_key("none") {
                    return Self {
                        description,
                        type_name: path.type_name.clone().unwrap_or_default(),
                        example: None,
                        example_some: Some(examples_obj["some"].clone()),
                        example_none: Some(examples_obj["none"].clone()),
                        enum_variants: path.enum_variants.clone(),
                        note: Some("For Option fields: pass the value directly to set Some, null to set None".to_string()),
                    };
                }
            }
        }

        // Regular non-Option path
        Self {
            description,
            type_name: path.type_name.clone().unwrap_or_default(),
            example: if path.example_value.is_null() {
                None
            } else {
                Some(path.example_value.clone())
            },
            example_some: None,
            example_none: None,
            enum_variants: path.enum_variants.clone(),
            note: None,
        }
    }
}

impl TypeSchemaResponse {
    /// Create a new response with the given types
    pub fn new(requested_types: Vec<String>) -> Self {
        Self {
            discovered_count: 0,
            requested_types,
            success: true,
            summary: TypeSchemaSummary {
                failed_discoveries:     0,
                successful_discoveries: 0,
                total_requested:        0,
            },
            type_info: HashMap::new(),
        }
    }

    /// Add a successfully discovered type
    pub fn add_type(&mut self, type_info: TypeInfo) {
        self.type_info
            .insert(type_info.type_name.clone(), type_info);
        self.discovered_count += 1;
        self.summary.successful_discoveries += 1;
    }

    /// Add a failed type discovery
    pub fn add_error(&mut self, type_name: String) {
        self.type_info
            .insert(type_name.clone(), TypeInfo::error(&type_name));
        self.summary.failed_discoveries += 1;
    }

    /// Finalize the summary statistics
    pub fn finalize(&mut self) {
        self.summary.total_requested = self.requested_types.len();
        self.discovered_count = self.summary.successful_discoveries;
    }
}
