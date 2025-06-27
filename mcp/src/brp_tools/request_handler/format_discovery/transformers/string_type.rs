//! String type transformer for Name component and string conversions

use serde_json::Value;

use super::super::detection::ErrorPattern;
use super::FormatTransformer;
use super::common::extract_type_name_from_error;
use crate::brp_tools::request_handler::constants::{
    FIELD_LABEL, FIELD_NAME, FIELD_TEXT, FIELD_VALUE,
};
use crate::brp_tools::support::brp_client::BrpError;

/// Transformer for string types, especially the Name component
/// Extracts strings from objects and arrays to convert to string format
pub struct StringTypeTransformer;

impl StringTypeTransformer {
    /// Create a new string type transformer
    pub const fn new() -> Self {
        Self
    }

    /// Extract string value from various input formats
    /// Returns (`extracted_string`, `source_description`)
    pub fn extract_string_value(value: &Value) -> Option<(String, String)> {
        match value {
            Value::Object(obj) => {
                // Try common field names that might contain the string value
                for field in [FIELD_NAME, FIELD_VALUE, FIELD_TEXT, FIELD_LABEL] {
                    if let Some(Value::String(s)) = obj.get(field) {
                        return Some((s.clone(), format!("from `{field}` field")));
                    }
                }
                // For single-field objects, use the value
                if obj.len() == 1 {
                    if let Some((field_name, Value::String(s))) = obj.iter().next() {
                        return Some((s.clone(), format!("from `{field_name}` field")));
                    }
                }
            }
            Value::Array(arr) => {
                // If it's an array with one string, extract it
                if arr.len() == 1 {
                    if let Value::String(s) = &arr[0] {
                        return Some((s.clone(), "from single-element array".to_string()));
                    }
                }
            }
            Value::String(s) => {
                // Already a string
                return Some((s.clone(), "already string format".to_string()));
            }
            _ => {}
        }
        None
    }

    /// Fix Name component format
    fn apply_name_component_fix(
        type_name: &str,
        original_value: &Value,
    ) -> Option<(Value, String)> {
        if let Some((extracted_string, source_description)) =
            Self::extract_string_value(original_value)
        {
            let format_type = match original_value {
                Value::Object(_) => "object",
                Value::Array(_) => "array",
                _ => "other",
            };

            Some((
                Value::String(extracted_string),
                format!(
                    "`{type_name} Name component` expects string format, extracted {source_description} (was {format_type})"
                ),
            ))
        } else {
            None
        }
    }

    /// Convert value to string format
    fn convert_to_string_format(
        type_name: &str,
        original_value: &Value,
    ) -> Option<(Value, String)> {
        if let Some((extracted_string, source_description)) =
            Self::extract_string_value(original_value)
        {
            Some((
                Value::String(extracted_string),
                format!("`{type_name}` expects string format, extracted {source_description}"),
            ))
        } else {
            None
        }
    }

    /// Apply expected type fix for string-related types
    #[allow(dead_code)]
    fn apply_expected_type_fix(
        type_name: &str,
        original_value: &Value,
        expected_type: &str,
    ) -> Option<(Value, String)> {
        // Handle Name component specifically
        if expected_type.contains("::Name") || expected_type.contains("::name::Name") {
            return Self::apply_name_component_fix(type_name, original_value);
        }

        // Handle other known type patterns
        if expected_type.contains("String") {
            return Self::convert_to_string_format(type_name, original_value);
        }

        None
    }

    /// Check if the error mentions string-related expectations
    fn is_string_expectation_error(error: &BrpError) -> bool {
        let message = &error.message;

        message.contains("expected string")
            || message.contains("String")
            || message.contains("Name")
            || message.contains("expected `bevy_ecs::name::Name`")
    }
}

impl FormatTransformer for StringTypeTransformer {
    fn can_handle(&self, error_pattern: &ErrorPattern) -> bool {
        match error_pattern {
            ErrorPattern::ExpectedType { expected_type } => {
                expected_type.contains("String")
                    || expected_type.contains("::Name")
                    || expected_type.contains("::name::Name")
            }
            _ => false,
        }
    }

    fn transform(&self, value: &Value) -> Option<(Value, String)> {
        // Try to extract a string from the value
        if let Some((extracted_string, source_description)) = Self::extract_string_value(value) {
            Some((
                Value::String(extracted_string),
                format!("String extracted {source_description}"),
            ))
        } else {
            None
        }
    }

    fn transform_with_error(&self, value: &Value, error: &BrpError) -> Option<(Value, String)> {
        // Extract type name from error for better messaging
        let type_name =
            extract_type_name_from_error(error).unwrap_or_else(|| "unknown".to_string());

        // Check if this is a string expectation error
        if Self::is_string_expectation_error(error) {
            return Self::convert_to_string_format(&type_name, value);
        }

        // Check for specific Name component errors
        if error.message.contains("Name") {
            return Self::apply_name_component_fix(&type_name, value);
        }

        // Fallback to generic transformation
        self.transform(value)
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "StringTypeTransformer"
    }
}

impl Default for StringTypeTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use serde_json::json;

    use super::*;

    #[test]
    fn test_can_handle_expected_string_type() {
        let transformer = StringTypeTransformer::new();
        let pattern = ErrorPattern::ExpectedType {
            expected_type: "String".to_string(),
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_can_handle_expected_name_type() {
        let transformer = StringTypeTransformer::new();
        let pattern = ErrorPattern::ExpectedType {
            expected_type: "bevy_ecs::name::Name".to_string(),
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_cannot_handle_other_patterns() {
        let transformer = StringTypeTransformer::new();
        let pattern = ErrorPattern::MathTypeArray {
            math_type: "Vec3".to_string(),
        };
        assert!(!transformer.can_handle(&pattern));
    }

    #[test]
    fn test_extract_string_from_object_name_field() {
        let value = json!({
            "name": "test_entity"
        });

        let result = StringTypeTransformer::extract_string_value(&value);
        assert!(result.is_some(), "Failed to extract string from name field");
        let (extracted, description) = result.unwrap(); // Safe after assertion
        assert_eq!(extracted, "test_entity");
        assert!(description.contains("from `name` field"));
    }

    #[test]
    fn test_extract_string_from_object_value_field() {
        let value = json!({
            "value": "test_value"
        });

        let result = StringTypeTransformer::extract_string_value(&value);
        assert!(
            result.is_some(),
            "Failed to extract string from value field"
        );
        let (extracted, description) = result.unwrap(); // Safe after assertion
        assert_eq!(extracted, "test_value");
        assert!(description.contains("from `value` field"));
    }

    #[test]
    fn test_extract_string_from_single_field_object() {
        let value = json!({
            "custom_field": "test_custom"
        });

        let result = StringTypeTransformer::extract_string_value(&value);
        assert!(
            result.is_some(),
            "Failed to extract string from single field object"
        );
        let (extracted, description) = result.unwrap(); // Safe after assertion
        assert_eq!(extracted, "test_custom");
        assert!(description.contains("from `custom_field` field"));
    }

    #[test]
    fn test_extract_string_from_single_element_array() {
        let value = json!(["test_array_string"]);

        let result = StringTypeTransformer::extract_string_value(&value);
        assert!(
            result.is_some(),
            "Failed to extract string from single-element array"
        );
        let (extracted, description) = result.unwrap(); // Safe after assertion
        assert_eq!(extracted, "test_array_string");
        assert!(description.contains("from single-element array"));
    }

    #[test]
    fn test_extract_string_already_string() {
        let value = json!("already_string");

        let result = StringTypeTransformer::extract_string_value(&value);
        assert!(result.is_some(), "Failed to handle already string value");
        let (extracted, description) = result.unwrap(); // Safe after assertion
        assert_eq!(extracted, "already_string");
        assert!(description.contains("already string format"));
    }

    #[test]
    fn test_extract_string_fails_for_multi_field_object() {
        let value = json!({
            "field1": "value1",
            "field2": "value2"
        });

        // Should not extract from multi-field objects unless they have standard field names
        let result = StringTypeTransformer::extract_string_value(&value);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_string_fails_for_multi_element_array() {
        let value = json!(["value1", "value2"]);

        let result = StringTypeTransformer::extract_string_value(&value);
        assert!(result.is_none());
    }

    #[test]
    fn test_transform_generic() {
        let transformer = StringTypeTransformer::new();
        let value = json!({
            "name": "test_entity"
        });

        let result = transformer.transform(&value);
        assert!(result.is_some(), "Failed to transform object to string");
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!("test_entity"));
        assert!(hint.contains("String extracted"));
    }

    #[test]
    fn test_transformer_name() {
        let transformer = StringTypeTransformer::new();
        assert_eq!(transformer.name(), "StringTypeTransformer");
    }

    #[test]
    fn test_apply_name_component_fix() {
        let value = json!({
            "name": "entity_name"
        });

        let result = StringTypeTransformer::apply_name_component_fix("TestType", &value);
        assert!(result.is_some(), "Failed to apply name component fix");
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!("entity_name"));
        assert!(hint.contains("TestType Name component"));
        assert!(hint.contains("expects string format"));
        assert!(hint.contains("was object"));
    }

    #[test]
    fn test_convert_to_string_format() {
        let value = json!({
            "value": "test_string"
        });

        let result = StringTypeTransformer::convert_to_string_format("TestType", &value);
        assert!(result.is_some(), "Failed to convert to string format");
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!("test_string"));
        assert!(hint.contains("TestType"));
        assert!(hint.contains("expects string format"));
    }

    #[test]
    fn test_is_string_expectation_error() {
        let error1 = BrpError {
            code:    -1,
            message: "expected string but found object".to_string(),
            data:    None,
        };
        assert!(StringTypeTransformer::is_string_expectation_error(&error1));

        let error2 = BrpError {
            code:    -1,
            message: "expected `bevy_ecs::name::Name`".to_string(),
            data:    None,
        };
        assert!(StringTypeTransformer::is_string_expectation_error(&error2));

        let error3 = BrpError {
            code:    -1,
            message: "some other error".to_string(),
            data:    None,
        };
        assert!(!StringTypeTransformer::is_string_expectation_error(&error3));
    }
}
