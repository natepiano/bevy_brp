//! Math type transformer for Vec2, Vec3, Vec4, and Quat conversions

use serde_json::{Map, Value};

use super::super::detection::ErrorPattern;
use super::FormatTransformer;
use super::common::extract_type_name_from_error;
use super::constants::TRANSFORM_SEQUENCE_F32_COUNT;
use crate::brp_tools::support::brp_client::BrpError;

/// Transformer for math types (Vec2, Vec3, Vec4, Quat)
/// Converts object format {x: 1.0, y: 2.0} to array format [1.0, 2.0]
pub struct MathTypeTransformer;

/// Helper function to format array expectation messages
fn type_expects_array(type_name: &str, array_type: &str) -> String {
    format!("`{type_name}` {array_type} expects array format")
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
                let field_value = obj.get(*field_name)?.as_f64()? as f32;
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
    ) -> Option<(Value, String)> {
        match math_type {
            "Vec3" => Self::convert_to_math_type_array(original_value, "Vec3")
                .map(|arr| (arr, type_expects_array(type_name, "Vec3") + " [x, y, z]")),
            "Vec2" => Self::convert_to_math_type_array(original_value, "Vec2")
                .map(|arr| (arr, type_expects_array(type_name, "Vec2") + " [x, y]")),
            "Vec4" => Self::convert_to_math_type_array(original_value, "Vec4")
                .map(|arr| (arr, type_expects_array(type_name, "Vec4") + " [x, y, z, w]")),
            "Quat" => Self::convert_to_math_type_array(original_value, "Quat")
                .map(|arr| (arr, type_expects_array(type_name, "Quat") + " [x, y, z, w]")),
            _ => None,
        }
    }

    /// Fix Transform component expecting sequence of f32 values
    fn apply_transform_sequence_fix(
        type_name: &str,
        original_value: &Value,
        expected_count: usize,
    ) -> Option<(Value, String)> {
        // Early return for non-object values
        let Value::Object(obj) = original_value else {
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
                    hint_parts.push(format!("`{field}` converted to Vec3 array format"));
                } else {
                    corrected.insert(field.to_string(), field_value.clone());
                }
            }
        }

        // Convert Quat field (rotation)
        if let Some(rotation_value) = obj.get("rotation") {
            if let Some(quat_array) = Self::convert_to_math_type_array(rotation_value, "Quat") {
                corrected.insert("rotation".to_string(), quat_array);
                hint_parts.push("`rotation` converted to Quat array format".to_string());
            } else {
                corrected.insert("rotation".to_string(), rotation_value.clone());
            }
        }

        if corrected.is_empty() {
            None
        } else {
            let hint = format!(
                "`{type_name}` Transform expected {expected_count} f32 values in sequence - {}",
                hint_parts.join(", ")
            );
            Some((Value::Object(corrected), hint))
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

    fn transform(&self, value: &Value) -> Option<(Value, String)> {
        // Try different math type conversions
        for math_type in ["Vec2", "Vec3", "Vec4", "Quat"] {
            if let Some(converted) = Self::convert_to_math_type_array(value, math_type) {
                let hint = format!("Converted to {math_type} array format");
                return Some((converted, hint));
            }
        }
        None
    }

    fn transform_with_error(&self, value: &Value, error: &BrpError) -> Option<(Value, String)> {
        // Extract type name from error for better messaging
        let type_name =
            extract_type_name_from_error(error).unwrap_or_else(|| "unknown".to_string());

        // Try specific math type conversions based on error content
        let message = &error.message;

        if message.contains("Vec2") {
            return Self::apply_math_type_array_fix(&type_name, value, "Vec2");
        }
        if message.contains("Vec3") {
            return Self::apply_math_type_array_fix(&type_name, value, "Vec3");
        }
        if message.contains("Vec4") {
            return Self::apply_math_type_array_fix(&type_name, value, "Vec4");
        }
        if message.contains("Quat") {
            return Self::apply_math_type_array_fix(&type_name, value, "Quat");
        }
        if message.contains("Transform") {
            // Try transform sequence fix with the defined constant
            return Self::apply_transform_sequence_fix(
                &type_name,
                value,
                TRANSFORM_SEQUENCE_F32_COUNT,
            );
        }

        // Fallback to generic transformation
        self.transform(value)
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
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!([1.0, 2.0])); // Vec2 is checked first, so only x,y are extracted
        assert!(hint.contains("Vec2")); // Should find Vec2 first in the loop
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
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert!(converted.is_object(), "Expected object result");
        let obj = converted.as_object().unwrap(); // Safe after assertion

        assert_eq!(obj.get("translation"), Some(&json!([1.0, 2.0, 3.0])));
        assert_eq!(obj.get("rotation"), Some(&json!([0.0, 0.0, 0.0, 1.0])));
        assert_eq!(obj.get("scale"), Some(&json!([1.0, 1.0, 1.0])));
        assert!(hint.contains("Transform expected 12 f32 values"));
    }
}
