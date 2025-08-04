//! Math type transformer for Vec2, Vec3, Vec4, and Quat conversions

use serde_json::{Map, Value};
use tracing::debug;

use super::super::constants::TRANSFORM_SEQUENCE_F32_COUNT;
use super::super::detection::ErrorPattern;
use super::super::types::TransformationResult;
use super::super::unified_types::UnifiedTypeInfo;
use super::FormatTransformer;
use super::common::{extract_single_field_value, extract_type_name_from_error, messages};
use crate::brp_tools::BrpClientError;

/// Transformer for math types (Vec2, Vec3, Vec4, Quat)
/// Converts object format {x: 1.0, y: 2.0} to array format [1.0, 2.0]
pub struct MathTypeTransformer;

/// Helper function to format array expectation messages
fn type_expects_array(type_name: &str, array_type: &str) -> String {
    messages::expects_array_format(type_name, array_type)
}

/// Helper function to extract numeric value from JSON, handling both integers and floats
fn extract_numeric_value(value: &Value) -> Option<f64> {
    #[allow(clippy::cast_precision_loss)]
    {
        value.as_f64().or_else(|| value.as_i64().map(|i| i as f64))
    }
}

/// Generic function to convert object values to array format
/// Handles Vec2 [x, y], Vec3 [x, y, z], Vec4/Quat [x, y, z, w]
fn convert_to_array_format(value: &Value, field_names: &[&str]) -> Option<Value> {
    match value {
        Value::Object(obj) => {
            // Extract fields in order and convert to f32
            let mut values = Vec::new();
            for field_name in field_names {
                #[allow(clippy::cast_possible_truncation)]
                let field_value = extract_numeric_value(obj.get(*field_name)?)? as f32;
                values.push(serde_json::json!(field_value));
            }
            Some(Value::Array(values))
        }
        Value::Array(arr) if arr.len() == field_names.len() => {
            // Already in array format, validate all are numbers
            if arr.iter().all(Value::is_number) {
                Some(value.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

impl MathTypeTransformer {
    /// Create a new math type transformer
    pub const fn new() -> Self {
        Self
    }

    /// Use `UnifiedTypeInfo` to transform math types
    fn try_unified_transform(type_name: &str, value: &Value) -> Option<TransformationResult> {
        // Create a UnifiedTypeInfo for the math type
        let type_info = UnifiedTypeInfo::for_math_type(type_name.to_string());

        // Try transformation using UnifiedTypeInfo
        type_info.transform_value(value).map(|transformed| {
            let hint = format!("Transformed {type_name} using UnifiedTypeInfo");
            TransformationResult {
                corrected_value: transformed,
                hint,
            }
        })
    }

    /// Convert object values to array format for math types
    /// Handles Vec2 [x, y], Vec3 [x, y, z], Vec4/Quat [x, y, z, w]
    fn convert_to_math_type_array(value: &Value, math_type: &str) -> Option<Value> {
        let field_names = match math_type {
            "Vec2" => &["x", "y"][..],
            "Vec3" => &["x", "y", "z"][..],
            "Vec4" | "Quat" => &["x", "y", "z", "w"][..],
            _ => return None,
        };
        convert_to_array_format(value, field_names)
    }

    /// Apply math type array fix with appropriate error message
    fn apply_math_type_array_fix(
        type_name: &str,
        original_value: &Value,
        math_type: &str,
    ) -> Option<TransformationResult> {
        // First try UnifiedTypeInfo transformation
        if let Some(result) =
            Self::try_unified_transform(&format!("glam::{math_type}"), original_value)
        {
            return Some(result);
        }

        // Fallback to legacy implementation for compatibility
        match math_type {
            "Vec3" => Self::convert_to_math_type_array(original_value, "Vec3").map(|arr| {
                TransformationResult {
                    corrected_value: arr,
                    hint:            type_expects_array(type_name, "Vec3") + " [x, y, z]",
                }
            }),
            "Vec2" => Self::convert_to_math_type_array(original_value, "Vec2").map(|arr| {
                TransformationResult {
                    corrected_value: arr,
                    hint:            type_expects_array(type_name, "Vec2") + " [x, y]",
                }
            }),
            "Vec4" => Self::convert_to_math_type_array(original_value, "Vec4").map(|arr| {
                TransformationResult {
                    corrected_value: arr,
                    hint:            type_expects_array(type_name, "Vec4") + " [x, y, z, w]",
                }
            }),
            "Quat" => Self::convert_to_math_type_array(original_value, "Quat").map(|arr| {
                TransformationResult {
                    corrected_value: arr,
                    hint:            type_expects_array(type_name, "Quat") + " [x, y, z, w]",
                }
            }),
            _ => None,
        }
    }

    /// Fix Transform component expecting sequence of f32 values
    fn apply_transform_sequence_fix(
        type_name: &str,
        original_value: &Value,
        expected_count: usize,
    ) -> Option<TransformationResult> {
        debug!(
            "apply_transform_sequence_fix: Processing input value: {}",
            original_value
        );

        // Extract the actual Transform data from the component map if needed
        let (actual_type_name, transform_data) = if let Value::Object(obj) = original_value {
            // Check if this is a component map with a single component
            if let Some((component_type, component_data)) = extract_single_field_value(obj) {
                debug!(
                    "apply_transform_sequence_fix: Found component '{}' in component map",
                    component_type
                );
                (component_type, component_data)
            } else {
                // This is already the Transform data object
                debug!("apply_transform_sequence_fix: Input is direct Transform data");
                (type_name, original_value)
            }
        } else {
            debug!("apply_transform_sequence_fix: Input is not an object, using as-is");
            (type_name, original_value)
        };

        debug!(
            "apply_transform_sequence_fix: Working with Transform data: {}",
            transform_data
        );

        // First try using UnifiedTypeInfo for Transform
        let type_info = UnifiedTypeInfo::for_transform_type(actual_type_name.to_string());

        if let Some(transformed) = type_info.transform_value(transform_data) {
            let hint = format!("`{actual_type_name}` Transform converted to proper array format");
            debug!("apply_transform_sequence_fix: UnifiedTypeInfo transformation succeeded");
            return Some(TransformationResult {
                corrected_value: transformed,
                hint,
            });
        }

        // Fallback to legacy implementation if UnifiedTypeInfo doesn't work
        let Value::Object(obj) = transform_data else {
            debug!("apply_transform_sequence_fix: Transform data is not an object, cannot process");
            return None;
        };

        // Transform typically expects Vec3 arrays for translation/scale and Quat array for rotation
        let mut corrected = Map::new();
        let mut hint_parts = Vec::new();

        // Convert Vec3 fields (translation, scale)
        for field in ["translation", "scale"] {
            if let Some(field_value) = obj.get(field) {
                if let Some(vec3_array) = Self::convert_to_math_type_array(field_value, "Vec3") {
                    corrected.insert(field.to_string(), vec3_array);
                    hint_parts.push(format!(
                        "{} {}",
                        messages::extracted_from_field(field),
                        messages::converted_to_format("Vec3 array")
                    ));
                } else {
                    corrected.insert(field.to_string(), field_value.clone());
                }
            }
        }

        // Convert Quat field (rotation)
        if let Some(rotation_value) = obj.get("rotation") {
            if let Some(quat_array) = Self::convert_to_math_type_array(rotation_value, "Quat") {
                corrected.insert("rotation".to_string(), quat_array);
                hint_parts.push(format!(
                    "{} {}",
                    messages::extracted_from_field("rotation"),
                    messages::converted_to_format("Quat array")
                ));
            } else {
                corrected.insert("rotation".to_string(), rotation_value.clone());
            }
        }

        if corrected.is_empty() {
            None
        } else {
            let hint = format!(
                "`{actual_type_name}` Transform expected {expected_count} f32 values in sequence - {}",
                hint_parts.join(", ")
            );
            Some(TransformationResult {
                corrected_value: Value::Object(corrected),
                hint,
            })
        }
    }
}

impl FormatTransformer for MathTypeTransformer {
    fn can_handle(&self, error_pattern: &ErrorPattern) -> bool {
        matches!(
            error_pattern,
            ErrorPattern::MathTypeArray { .. } | ErrorPattern::TransformSequence { .. }
        )
    }

    fn transform(&self, value: &Value) -> Option<TransformationResult> {
        // Try different math type conversions using UnifiedTypeInfo first
        for math_type in ["Vec2", "Vec3", "Vec4", "Quat"] {
            if let Some(result) = Self::try_unified_transform(&format!("glam::{math_type}"), value)
            {
                return Some(result);
            }
        }

        // Fallback to legacy conversion
        for math_type in ["Vec2", "Vec3", "Vec4", "Quat"] {
            if let Some(converted) = Self::convert_to_math_type_array(value, math_type) {
                let hint = messages::converted_to_format(&format!("{math_type} array"));
                return Some(TransformationResult {
                    corrected_value: converted,
                    hint,
                });
            }
        }
        None
    }

    fn transform_with_error(
        &self,
        value: &Value,
        error: &BrpClientError,
    ) -> Option<TransformationResult> {
        debug!("MathTypeTransformer: transform_with_error called");
        debug!("MathTypeTransformer: Input value: {}", value);
        debug!("MathTypeTransformer: Error message: {}", error.message);

        // Extract type name from error for better messaging
        let type_name =
            extract_type_name_from_error(error).unwrap_or_else(|| "unknown".to_string());
        debug!("MathTypeTransformer: Extracted type name: '{}'", type_name);

        // Try specific math type conversions based on error content
        let message = &error.message;

        if message.contains("Vec2") {
            debug!("MathTypeTransformer: Attempting Vec2 transformation");
            let result = Self::apply_math_type_array_fix(&type_name, value, "Vec2");
            debug!(
                "MathTypeTransformer: Vec2 transformation result: {:?}",
                result.is_some()
            );
            return result;
        }
        if message.contains("Vec3") {
            debug!("MathTypeTransformer: Attempting Vec3 transformation");
            let result = Self::apply_math_type_array_fix(&type_name, value, "Vec3");
            debug!(
                "MathTypeTransformer: Vec3 transformation result: {:?}",
                result.is_some()
            );
            return result;
        }
        if message.contains("Vec4") {
            debug!("MathTypeTransformer: Attempting Vec4 transformation");
            let result = Self::apply_math_type_array_fix(&type_name, value, "Vec4");
            debug!(
                "MathTypeTransformer: Vec4 transformation result: {:?}",
                result.is_some()
            );
            return result;
        }
        if message.contains("Quat") {
            debug!("MathTypeTransformer: Attempting Quat transformation");
            let result = Self::apply_math_type_array_fix(&type_name, value, "Quat");
            debug!(
                "MathTypeTransformer: Quat transformation result: {:?}",
                result.is_some()
            );
            return result;
        }
        if message.contains("Transform") {
            debug!("MathTypeTransformer: Attempting Transform sequence transformation");
            // Try transform sequence fix with the defined constant
            let result =
                Self::apply_transform_sequence_fix(&type_name, value, TRANSFORM_SEQUENCE_F32_COUNT);
            debug!(
                "MathTypeTransformer: Transform transformation result: {:?}",
                result.is_some()
            );
            return result;
        }

        debug!(
            "MathTypeTransformer: No specific pattern matched, falling back to generic transformation"
        );
        // Fallback to generic transformation
        let result = self.transform(value);
        debug!(
            "MathTypeTransformer: Generic transformation result: {:?}",
            result.is_some()
        );
        result
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "MathTypeTransformer"
    }
}

impl Default for MathTypeTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use serde_json::json;

    use super::*;

    fn create_vec3_object() -> Value {
        json!({
            "x": 1.0,
            "y": 2.0,
            "z": 3.0
        })
    }

    fn create_vec2_object() -> Value {
        json!({
            "x": 1.0,
            "y": 2.0
        })
    }

    fn create_quat_object() -> Value {
        json!({
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "w": 1.0
        })
    }

    #[test]
    fn test_can_handle_math_type_array() {
        let transformer = MathTypeTransformer::new();
        let pattern = ErrorPattern::MathTypeArray {
            math_type: "Vec3".to_string(),
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_can_handle_transform_sequence() {
        let transformer = MathTypeTransformer::new();
        let pattern = ErrorPattern::TransformSequence { expected_count: 12 };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_cannot_handle_other_patterns() {
        let transformer = MathTypeTransformer::new();
        let pattern = ErrorPattern::ExpectedType {
            expected_type: "String".to_string(),
        };
        assert!(!transformer.can_handle(&pattern));
    }

    #[test]
    fn test_convert_vec3_object_to_array() {
        let value = create_vec3_object();

        let result = MathTypeTransformer::convert_to_math_type_array(&value, "Vec3");
        assert!(result.is_some(), "Failed to convert Vec3 object to array");
        let converted = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!([1.0, 2.0, 3.0]));
    }

    #[test]
    fn test_convert_vec2_object_to_array() {
        let value = create_vec2_object();

        let result = MathTypeTransformer::convert_to_math_type_array(&value, "Vec2");
        assert!(result.is_some(), "Failed to convert Vec2 object to array");
        let converted = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!([1.0, 2.0]));
    }

    #[test]
    fn test_convert_quat_object_to_array() {
        let value = create_quat_object();

        let result = MathTypeTransformer::convert_to_math_type_array(&value, "Quat");
        assert!(result.is_some(), "Failed to convert Quat object to array");
        let converted = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!([0.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn test_transform_generic() {
        let transformer = MathTypeTransformer::new();
        let value = create_vec3_object();

        let result = transformer.transform(&value);
        assert!(result.is_some(), "Failed to transform Vec3 object");
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(transformation_result.corrected_value, json!([1.0, 2.0])); // Vec2 is checked first, so only x,y are extracted
        assert!(transformation_result.hint.contains("Vec2")); // Should find Vec2 first in the loop
    }

    #[test]
    fn test_transform_already_array() {
        let value = json!([1.0, 2.0, 3.0]);

        // Should still work with arrays
        let result = MathTypeTransformer::convert_to_math_type_array(&value, "Vec3");
        assert!(result.is_some(), "Failed to handle array input");
        let converted = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!([1.0, 2.0, 3.0]));
    }

    #[test]
    fn test_transformer_name() {
        let transformer = MathTypeTransformer::new();
        assert_eq!(transformer.name(), "MathTypeTransformer");
    }

    #[test]
    fn test_transform_sequence_fix() {
        let transform_obj = json!({
            "translation": {"x": 1.0, "y": 2.0, "z": 3.0},
            "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
            "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
        });

        let result =
            MathTypeTransformer::apply_transform_sequence_fix("Transform", &transform_obj, 12);
        assert!(result.is_some(), "Failed to apply transform sequence fix");
        let transformation_result = result.unwrap(); // Safe after assertion
        assert!(
            transformation_result.corrected_value.is_object(),
            "Expected object result"
        );
        let obj = transformation_result.corrected_value.as_object().unwrap(); // Safe after assertion

        assert_eq!(obj.get("translation"), Some(&json!([1.0, 2.0, 3.0])));
        assert_eq!(obj.get("rotation"), Some(&json!([0.0, 0.0, 0.0, 1.0])));
        assert_eq!(obj.get("scale"), Some(&json!([1.0, 1.0, 1.0])));
    }
}
