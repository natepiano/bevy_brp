//! Unified type system for format discovery
//!
//! Single coherent schema replacing fragmented type conversions. Contains all
//! discoverable type information in one place to prevent data loss.
//!
//! Core types: `UnifiedTypeInfo`, `FormatInfo`, `RegistryStatus`, `SerializationSupport`

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Information about an enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    /// The name of the variant
    pub name:         String,
    /// The type of the variant (Unit, Tuple, Struct)
    pub variant_type: String,
}

/// Information about an enum type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    /// List of enum variants
    pub variants: Vec<EnumVariant>,
}

/// Registry and reflection status for a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatus {
    /// Whether the type is registered in Bevy's type registry
    pub in_registry: bool,
    /// Whether the type has reflection support
    pub has_reflect: bool,
    /// Type path as registered in the registry
    pub type_path:   Option<String>,
}

/// Serialization trait support for a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationSupport {
    /// Whether the type implements Serialize
    pub has_serialize:   bool,
    /// Whether the type implements Deserialize
    pub has_deserialize: bool,
    /// Whether the type can be used in BRP operations requiring serialization
    pub brp_compatible:  bool,
}

/// Format-specific information and examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    /// Real example values for different BRP operations
    pub examples:         HashMap<String, Value>,
    /// Available mutation paths if the type supports mutation
    pub mutation_paths:   HashMap<String, String>,
    /// Original format that caused the error (if applicable)
    pub original_format:  Option<Value>,
    /// Corrected format to use instead
    pub corrected_format: Option<Value>,
}

/// Comprehensive type information unified across all discovery sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTypeInfo {
    /// The fully-qualified type name
    pub type_name:            String,
    /// Registry and reflection information
    pub registry_status:      RegistryStatus,
    /// Serialization support information
    pub serialization:        SerializationSupport,
    /// Format-specific data and examples
    pub format_info:          FormatInfo,
    /// List of supported BRP operations for this type
    pub supported_operations: Vec<String>,
    /// Type category for quick identification
    pub type_category:        String,
    /// Child type information for complex types (enums, generics)
    pub child_types:          HashMap<String, String>,
    /// Enum variant information (only populated for enum types)
    pub enum_info:            Option<EnumInfo>,
    /// Source of this type information for debugging
    pub discovery_source:     DiscoverySource,
}

/// Information about a format correction applied during recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionInfo {
    /// The type that was corrected
    pub type_name:         String,
    /// Original value that needed correction
    pub original_value:    Value,
    /// Corrected value to use
    pub corrected_value:   Value,
    /// Human-readable explanation of the correction
    pub hint:              String,
    /// Component or resource name for error reporting
    pub target_type:       String,
    /// Structured format information for error responses (usage, `valid_values`, examples)
    pub corrected_format:  Option<Value>,
    /// Type information discovered during correction (if available)
    pub type_info:         Option<UnifiedTypeInfo>,
    /// The correction method used
    pub correction_method: CorrectionMethod,
}

/// Method used to discover or correct a type format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoverySource {
    /// Information from `bevy_brp_extras` direct discovery
    DirectDiscovery,
    /// Information from Bevy's type registry
    TypeRegistry,
    /// Information inferred from error patterns
    PatternMatching,
    /// Information from built-in type knowledge
    BuiltinTypes,
    /// Manually provided or hardcoded information
    Manual,
}

/// Method used to correct a format error
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CorrectionMethod {
    /// Direct replacement based on exact type knowledge
    DirectReplacement,
    /// Object to array conversion for math types
    ObjectToArray,
    /// Array to object conversion
    ArrayToObject,
    /// String to enum variant conversion
    StringToEnum,
    /// Nested structure correction
    NestedCorrection,
    /// Field name mapping or aliasing
    FieldMapping,
}

impl UnifiedTypeInfo {
    /// Create a new `UnifiedTypeInfo` with minimal required information
    pub fn new(type_name: String, discovery_source: DiscoverySource) -> Self {
        Self {
            type_name,
            registry_status: RegistryStatus {
                in_registry: false,
                has_reflect: false,
                type_path:   None,
            },
            serialization: SerializationSupport {
                has_serialize:   false,
                has_deserialize: false,
                brp_compatible:  false,
            },
            format_info: FormatInfo {
                examples:         HashMap::new(),
                mutation_paths:   HashMap::new(),
                original_format:  None,
                corrected_format: None,
            },
            supported_operations: Vec::new(),
            type_category: "Unknown".to_string(),
            child_types: HashMap::new(),
            enum_info: None,
            discovery_source,
        }
    }

    /// Check if this type supports mutation operations
    pub fn supports_mutation(&self) -> bool {
        !self.format_info.mutation_paths.is_empty()
    }

    /// Get the mutation paths for this type
    pub const fn get_mutation_paths(&self) -> &HashMap<String, String> {
        &self.format_info.mutation_paths
    }

    /// Get example value for a specific operation
    pub fn get_example(&self, operation: &str) -> Option<&Value> {
        self.format_info.examples.get(operation)
    }
}

impl FormatInfo {
    /// Create empty format info
    pub fn empty() -> Self {
        Self {
            examples:         HashMap::new(),
            mutation_paths:   HashMap::new(),
            original_format:  None,
            corrected_format: None,
        }
    }
}
