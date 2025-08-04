//! Unified type system for format discovery
//!
//! Single coherent schema replacing fragmented type conversions. Contains all
//! discoverable type information in one place to prevent data loss.
//!
//! Core types: `UnifiedTypeInfo`, `FormatInfo`, `RegistryStatus`, `SerializationSupport`

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{
    Correction, CorrectionInfo, CorrectionMethod, DiscoverySource, EnumInfo, FormatInfo,
    RegistryStatus, SerializationSupport, TypeCategory,
};

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
    pub type_category:        TypeCategory,
    /// Child type information for complex types (enums, generics)
    pub child_types:          HashMap<String, String>,
    /// Enum variant information (only populated for enum types)
    pub enum_info:            Option<EnumInfo>,
    /// Source of this type information for debugging
    pub discovery_source:     DiscoverySource,
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
            type_category: TypeCategory::Unknown,
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
        match self.type_category {
            TypeCategory::Struct => self.generate_struct_example(),
            TypeCategory::Enum => self.generate_enum_example(),
            TypeCategory::MathType => self.generate_math_type_example(),
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
        match self.type_category {
            TypeCategory::MathType => self.transform_math_value(value),
            TypeCategory::Struct => self.transform_struct_value(value),
            TypeCategory::Enum => self.transform_enum_value(value),
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

    /// Convert this type info to a correction result
    pub fn to_correction_result(&mut self, original_value: Option<Value>) -> Correction {
        use tracing::debug;

        use super::format_correction_fields::FormatCorrectionField;

        // Ensure examples are generated
        self.ensure_examples();

        // Check if this is an enum with variants - create enum-specific correction
        if let Some(enum_info) = &self.enum_info {
            let variant_names: Vec<String> =
                enum_info.variants.iter().map(|v| v.name.clone()).collect();

            let corrected_format = serde_json::json!({
                FormatCorrectionField::Hint.as_ref(): "Use empty path with variant name as value",
                FormatCorrectionField::ValidValues.as_ref(): variant_names,
                FormatCorrectionField::Examples.as_ref(): variant_names.iter().take(2).map(|variant| serde_json::json!({
                    FormatCorrectionField::Path.as_ref(): "",
                    FormatCorrectionField::Value.as_ref(): variant
                })).collect::<Vec<_>>()
            });

            let correction_info = CorrectionInfo {
                type_name:         self.type_name.clone(),
                original_value:    original_value.unwrap_or(serde_json::json!(null)),
                corrected_value:   corrected_format.clone(),
                corrected_format:  Some(corrected_format),
                hint:              format!(
                    "Enum '{}' requires empty path for unit variant mutation. Valid variants: {}",
                    self.type_name.split("::").last().unwrap_or(&self.type_name),
                    variant_names.join(", ")
                ),
                target_type:       self.type_name.clone(),
                type_info:         Some(self.clone()),
                correction_method: CorrectionMethod::DirectReplacement,
            };

            return Correction::Candidate { correction_info };
        }

        // Check if we can actually transform the original input
        if let Some(original_value) = original_value {
            debug!(
                "Extras Integration: Attempting to transform original value: {}",
                serde_json::to_string(&original_value)
                    .unwrap_or_else(|_| "invalid json".to_string())
            );
            if let Some(transformed_value) = self.transform_value(&original_value) {
                debug!(
                    "Extras Integration: Successfully transformed value to: {}",
                    serde_json::to_string(&transformed_value)
                        .unwrap_or_else(|_| "invalid json".to_string())
                );
                // We can transform the input - return Corrected with actual transformation
                let correction_info = CorrectionInfo {
                    type_name:         self.type_name.clone(),
                    original_value:    original_value.clone(),
                    corrected_value:   transformed_value,
                    hint:              format!(
                        "Transformed {} format for type '{}' (discovered via bevy_brp_extras)",
                        if original_value.is_object() {
                            "object"
                        } else {
                            "value"
                        },
                        self.type_name
                    ),
                    target_type:       self.type_name.clone(),
                    corrected_format:  None,
                    type_info:         Some(self.clone()),
                    correction_method: CorrectionMethod::ObjectToArray,
                };

                return Correction::Candidate { correction_info };
            }
            debug!("Extras Integration: transform_value() returned None - cannot transform input");
        } else {
            debug!("Extras Integration: No original value provided for transformation");
        }

        // Cannot transform input - provide guidance with examples
        let reason = if let Some(spawn_example) = self.get_example("spawn") {
            format!(
                "Cannot transform input for type '{}'. Use this format: {}",
                self.type_name,
                serde_json::to_string(spawn_example)
                    .unwrap_or_else(|_| "correct format".to_string())
            )
        } else {
            format!(
                "Cannot transform input for type '{}'. Type discovered but no format example available.",
                self.type_name
            )
        };

        Correction::Uncorrectable {
            type_info: self.clone(),
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::types::{Correction, CorrectionMethod, DiscoverySource, TypeCategory};
    use super::UnifiedTypeInfo;

    #[test]
    fn test_to_correction_result_metadata_only() {
        let mut type_info = UnifiedTypeInfo::new(
            "bevy_transform::components::transform::Transform".to_string(),
            DiscoverySource::DirectDiscovery,
        );

        let result = type_info.to_correction_result(None);

        match result {
            Correction::Uncorrectable { type_info, reason } => {
                assert_eq!(
                    type_info.type_name,
                    "bevy_transform::components::transform::Transform"
                );
                assert!(reason.contains("no format example"));
            }
            Correction::Candidate { .. } => {
                unreachable!("Expected MetadataOnly correction result")
            }
        }
    }

    #[test]
    fn test_to_correction_result_with_example() {
        let mut type_info = UnifiedTypeInfo::new(
            "bevy_transform::components::transform::Transform".to_string(),
            DiscoverySource::DirectDiscovery,
        );
        type_info.type_category = TypeCategory::Struct;
        type_info.format_info.examples.insert(
            "spawn".to_string(),
            json!({
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            }),
        );

        let original = json!({"translation": {"x": 0.0, "y": 0.0, "z": 0.0}});
        let result = type_info.to_correction_result(Some(original.clone()));

        match result {
            Correction::Candidate { correction_info } => {
                assert_eq!(correction_info.original_value, original);
                assert!(correction_info.corrected_value.get("translation").is_some());
                assert_eq!(
                    correction_info.correction_method,
                    CorrectionMethod::ObjectToArray
                );
            }
            Correction::Uncorrectable { .. } => {
                unreachable!("Expected Applied correction result")
            }
        }
    }
}
