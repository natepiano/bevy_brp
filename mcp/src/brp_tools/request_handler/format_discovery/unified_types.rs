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

    /// Ensure examples are generated for all operations
    pub fn ensure_examples(&mut self) {
        // Generate examples for spawn operation if not present
        if !self.format_info.examples.contains_key("spawn") {
            if let Some(example) = self.generate_spawn_example() {
                self.format_info
                    .examples
                    .insert("spawn".to_string(), example);
            }
        }

        // Generate examples for insert operation if not present
        if !self.format_info.examples.contains_key("insert") {
            if let Some(example) = self.generate_insert_example() {
                self.format_info
                    .examples
                    .insert("insert".to_string(), example);
            }
        }

        // Generate examples for mutation if paths exist
        if self.supports_mutation() && !self.format_info.examples.contains_key("mutate") {
            if let Some(example) = self.generate_mutation_example() {
                self.format_info
                    .examples
                    .insert("mutate".to_string(), example);
            }
        }
    }

    /// Generate spawn example based on type structure
    fn generate_spawn_example(&self) -> Option<Value> {
        match self.type_category.as_str() {
            "Struct" => self.generate_struct_example(),
            "Enum" => self.generate_enum_example(),
            "MathType" => self.generate_math_type_example(),
            _ => None,
        }
    }

    /// Generate insert example (similar to spawn)
    fn generate_insert_example(&self) -> Option<Value> {
        self.generate_spawn_example()
    }

    /// Generate mutation example with paths
    fn generate_mutation_example(&self) -> Option<Value> {
        if let Some((path, description)) = self.format_info.mutation_paths.iter().next() {
            Some(serde_json::json!({
                "path": path,
                "value": Self::generate_value_for_type(description),
                "description": description
            }))
        } else {
            None
        }
    }

    /// Generate example for struct types
    fn generate_struct_example(&self) -> Option<Value> {
        // For now, return corrected format if available
        self.format_info.corrected_format.clone()
    }

    /// Generate example for enum types
    fn generate_enum_example(&self) -> Option<Value> {
        self.enum_info.as_ref().and_then(|enum_info| {
            enum_info
                .variants
                .first()
                .map(|variant| match variant.variant_type.as_str() {
                    "Unit" => Value::String(variant.name.clone()),
                    _ => serde_json::json!({
                        variant.name.clone(): {}
                    }),
                })
        })
    }

    /// Generate example for math types (Vec2, Vec3, etc.)
    fn generate_math_type_example(&self) -> Option<Value> {
        match self.type_name.as_str() {
            name if name.contains("Vec2") => Some(serde_json::json!([0.0, 0.0])),
            name if name.contains("Vec3") => Some(serde_json::json!([0.0, 0.0, 0.0])),
            name if name.contains("Vec4") => Some(serde_json::json!([0.0, 0.0, 0.0, 0.0])),
            name if name.contains("Quat") => Some(serde_json::json!([0.0, 0.0, 0.0, 1.0])),
            _ => None,
        }
    }

    /// Generate appropriate value for a type description
    fn generate_value_for_type(type_desc: &str) -> Value {
        match type_desc {
            desc if desc.contains("f32") || desc.contains("float") => Value::from(0.0),
            desc if desc.contains("i32") || desc.contains("int") => Value::from(0),
            desc if desc.contains("bool") => Value::from(false),
            desc if desc.contains("String") => Value::from(""),
            _ => Value::Null,
        }
    }

    /// Transform an incorrect value to the correct format
    pub fn transform_value(&self, value: &Value) -> Option<Value> {
        match self.type_category.as_str() {
            "MathType" => self.transform_math_value(value),
            "Struct" => self.transform_struct_value(value),
            "Enum" => self.transform_enum_value(value),
            _ => None,
        }
    }

    /// Transform math type values (Vec2, Vec3, Quat, etc.)
    fn transform_math_value(&self, value: &Value) -> Option<Value> {
        // Handle object to array conversion for math types
        value
            .as_object()
            .and_then(|obj| match self.type_name.as_str() {
                name if name.contains("Vec2") => Self::extract_vec2_from_object(obj),
                name if name.contains("Vec3") => Self::extract_vec3_from_object(obj),
                name if name.contains("Vec4") => Self::extract_vec4_from_object(obj),
                name if name.contains("Quat") => Self::extract_quat_from_object(obj),
                _ => None,
            })
    }

    /// Extract Vec2 array from object
    fn extract_vec2_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        let x = obj.get("x").and_then(Value::as_f64)?;
        let y = obj.get("y").and_then(Value::as_f64)?;
        Some(serde_json::json!([x, y]))
    }

    /// Extract Vec3 array from object
    fn extract_vec3_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        let x = obj.get("x").and_then(Value::as_f64)?;
        let y = obj.get("y").and_then(Value::as_f64)?;
        let z = obj.get("z").and_then(Value::as_f64)?;
        Some(serde_json::json!([x, y, z]))
    }

    /// Extract Vec4 array from object
    fn extract_vec4_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        let x = obj.get("x").and_then(Value::as_f64)?;
        let y = obj.get("y").and_then(Value::as_f64)?;
        let z = obj.get("z").and_then(Value::as_f64)?;
        let w = obj.get("w").and_then(Value::as_f64)?;
        Some(serde_json::json!([x, y, z, w]))
    }

    /// Extract Quaternion array from object
    fn extract_quat_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        // Same as Vec4 for quaternions
        Self::extract_vec4_from_object(obj)
    }

    /// Transform struct values - only transform if input is valid and transformable
    fn transform_struct_value(&self, value: &Value) -> Option<Value> {
        // Check if this is a Transform type with child math types that can be transformed
        if self.type_name.contains("Transform") {
            // Try to transform object format to array format for math fields
            if let Some(obj) = value.as_object() {
                let mut result = serde_json::Map::new();
                let mut transformed_any = false;

                for (field_name, field_value) in obj {
                    match field_name.as_str() {
                        "translation" | "scale" => {
                            // Try to transform Vec3 object to array
                            if let Some(field_obj) = field_value.as_object() {
                                if let Some(vec3_array) = Self::extract_vec3_from_object(field_obj)
                                {
                                    result.insert(field_name.clone(), vec3_array);
                                    transformed_any = true;
                                } else {
                                    // Cannot transform this field - copy as-is
                                    result.insert(field_name.clone(), field_value.clone());
                                }
                            } else {
                                // Field is not an object, copy as-is
                                result.insert(field_name.clone(), field_value.clone());
                            }
                        }
                        "rotation" => {
                            // Try to transform Quat object to array
                            if let Some(field_obj) = field_value.as_object() {
                                if let Some(quat_array) = Self::extract_quat_from_object(field_obj)
                                {
                                    result.insert(field_name.clone(), quat_array);
                                    transformed_any = true;
                                } else {
                                    // Cannot transform this field - copy as-is
                                    result.insert(field_name.clone(), field_value.clone());
                                }
                            } else {
                                // Field is not an object, copy as-is
                                result.insert(field_name.clone(), field_value.clone());
                            }
                        }
                        _ => {
                            // Copy other fields as-is
                            result.insert(field_name.clone(), field_value.clone());
                        }
                    }
                }

                if transformed_any {
                    return Some(Value::Object(result));
                }
            }
        }

        // For other struct types, return None - no transformation possible
        None
    }

    /// Transform enum values
    fn transform_enum_value(&self, value: &Value) -> Option<Value> {
        if let Some(enum_info) = &self.enum_info {
            // Handle string to enum variant conversion
            if let Some(str_val) = value.as_str() {
                // Check if string matches a variant name
                if enum_info.variants.iter().any(|v| v.name == str_val) {
                    // For unit variants, just return the string
                    return Some(Value::String(str_val.to_string()));
                }
            }
        }
        None
    }
}

impl FormatInfo {}
