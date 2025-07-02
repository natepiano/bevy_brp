//! Unified type system for format discovery
//!
//! Single coherent schema replacing fragmented type conversions. Contains all
//! discoverable type information in one place to prevent data loss.
//!
//! Core types: `UnifiedTypeInfo`, `FormatInfo`, `RegistryStatus`, `SerializationSupport`

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub enum_info:            Option<serde_json::Map<String, Value>>,
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
    #[allow(dead_code)]
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

    /// Check if this type can be used for BRP operations requiring serialization
    #[allow(dead_code)]
    pub const fn is_brp_serializable(&self) -> bool {
        self.serialization.brp_compatible
    }

    /// Check if this type supports mutation operations
    #[allow(dead_code)]
    pub fn supports_mutation(&self) -> bool {
        !self.format_info.mutation_paths.is_empty()
    }

    /// Get the mutation paths for this type
    #[allow(dead_code)]
    pub const fn get_mutation_paths(&self) -> &HashMap<String, String> {
        &self.format_info.mutation_paths
    }

    /// Get example value for a specific operation
    pub fn get_example(&self, operation: &str) -> Option<&Value> {
        self.format_info.examples.get(operation)
    }

    /// Check if type information is complete enough for format correction
    #[allow(dead_code)]
    pub fn is_correction_ready(&self) -> bool {
        self.format_info.corrected_format.is_some()
            || !self.format_info.examples.is_empty()
            || !self.format_info.mutation_paths.is_empty()
    }

    /// Merge information from another `UnifiedTypeInfo`, prioritizing more complete data
    #[allow(dead_code)]
    pub fn merge_with(&mut self, other: Self) {
        // Prefer registry information if the other source has it and we don't
        if !self.registry_status.in_registry && other.registry_status.in_registry {
            self.registry_status = other.registry_status;
        }

        // Merge serialization information (prefer true values)
        if !self.serialization.has_serialize && other.serialization.has_serialize {
            self.serialization.has_serialize = true;
        }
        if !self.serialization.has_deserialize && other.serialization.has_deserialize {
            self.serialization.has_deserialize = true;
        }
        if !self.serialization.brp_compatible && other.serialization.brp_compatible {
            self.serialization.brp_compatible = true;
        }

        // Merge format information (additive)
        for (key, value) in other.format_info.examples {
            self.format_info.examples.entry(key).or_insert(value);
        }
        for (key, value) in other.format_info.mutation_paths {
            self.format_info.mutation_paths.entry(key).or_insert(value);
        }

        // Prefer corrected format if other has it and we don't
        if self.format_info.corrected_format.is_none()
            && other.format_info.corrected_format.is_some()
        {
            self.format_info.corrected_format = other.format_info.corrected_format;
        }

        // Merge supported operations (union)
        for op in other.supported_operations {
            if !self.supported_operations.contains(&op) {
                self.supported_operations.push(op);
            }
        }

        // Prefer more specific type category
        if self.type_category == "Unknown" && other.type_category != "Unknown" {
            self.type_category = other.type_category;
        }

        // Merge child types and enum info
        for (key, value) in other.child_types {
            self.child_types.entry(key).or_insert(value);
        }
        if self.enum_info.is_none() && other.enum_info.is_some() {
            self.enum_info = other.enum_info;
        }

        // Prefer DirectDiscovery source over others
        if matches!(other.discovery_source, DiscoverySource::DirectDiscovery) {
            self.discovery_source = other.discovery_source;
        }
    }
}

impl CorrectionInfo {
    /// Create a new `CorrectionInfo` for a successful correction
    #[allow(dead_code)]
    pub const fn new(
        type_name: String,
        original_value: Value,
        corrected_value: Value,
        hint: String,
        correction_method: CorrectionMethod,
    ) -> Self {
        Self {
            type_name,
            original_value,
            corrected_value,
            hint,
            type_info: None,
            correction_method,
        }
    }

    /// Add type information to this correction
    #[allow(dead_code)]
    pub fn with_type_info(mut self, type_info: UnifiedTypeInfo) -> Self {
        self.type_info = Some(type_info);
        self
    }
}

impl RegistryStatus {
    /// Create registry status for a type not in the registry
    #[allow(dead_code)]
    pub const fn not_in_registry() -> Self {
        Self {
            in_registry: false,
            has_reflect: false,
            type_path:   None,
        }
    }

    /// Create registry status for a type in the registry
    #[allow(dead_code)]
    pub const fn in_registry(type_path: String, has_reflect: bool) -> Self {
        Self {
            in_registry: true,
            has_reflect,
            type_path: Some(type_path),
        }
    }
}

impl SerializationSupport {
    /// Create serialization support info indicating no serialization support
    #[allow(dead_code)]
    pub const fn no_support() -> Self {
        Self {
            has_serialize:   false,
            has_deserialize: false,
            brp_compatible:  false,
        }
    }

    /// Create serialization support info for full BRP compatibility
    #[allow(dead_code)]
    pub const fn full_support() -> Self {
        Self {
            has_serialize:   true,
            has_deserialize: true,
            brp_compatible:  true,
        }
    }

    /// Create serialization support info with specific trait support
    #[allow(dead_code)]
    pub const fn with_traits(has_serialize: bool, has_deserialize: bool) -> Self {
        Self {
            has_serialize,
            has_deserialize,
            brp_compatible: has_serialize && has_deserialize,
        }
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

    /// Create format info with a format correction
    #[allow(dead_code)]
    pub fn with_correction(original: Value, corrected: Value) -> Self {
        Self {
            examples:         HashMap::new(),
            mutation_paths:   HashMap::new(),
            original_format:  Some(original),
            corrected_format: Some(corrected),
        }
    }

    /// Add an example for a specific operation
    #[allow(dead_code)]
    pub fn add_example(&mut self, operation: String, example: Value) {
        self.examples.insert(operation, example);
    }

    /// Add a mutation path with description
    #[allow(dead_code)]
    pub fn add_mutation_path(&mut self, path: String, description: String) {
        self.mutation_paths.insert(path, description);
    }
}
