//! Common utilities shared across format transformers

use serde_json::Value;

use crate::brp_tools::BrpError;

/// Standard error message templates for consistency across transformers
pub mod messages {
    /// Format expectation message for array types
    pub fn expects_array_format(type_name: &str, array_type: &str) -> String {
        format!("`{type_name}` expects {array_type} array format")
    }

    /// Format expectation message for string types
    pub fn expects_string_format(type_name: &str) -> String {
        format!("`{type_name}` expects string format")
    }

    /// Format conversion success message
    pub fn converted_to_format(format_type: &str) -> String {
        format!("Converted to {format_type} format")
    }

    /// Field extraction message
    pub fn extracted_from_field(field_name: &str) -> String {
        format!("Extracted from `{field_name}` field")
    }
}

/// Extract type name from error message by looking for text between backticks
/// Returns `Some(type_name)` if found, `None` otherwise
pub fn extract_type_name_from_error(error: &BrpError) -> Option<String> {
    let message = &error.message;

    // Look for common patterns that indicate the type name
    if let Some(start) = message.find('`') {
        if let Some(end) = message[start + 1..].find('`') {
            return Some(message[start + 1..start + 1 + end].to_string());
        }
    }

    None
}

/// Extract single field value from a single-field JSON object
/// Returns `Some((field_name, field_value))` if the object has exactly one field,
/// `None` otherwise
pub fn extract_single_field_value(obj: &serde_json::Map<String, Value>) -> Option<(&str, &Value)> {
    if obj.len() == 1 {
        obj.iter().next().map(|(k, v)| (k.as_str(), v))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_type_name_from_error_success() {
        let error = BrpError {
            code:    -1,
            message: "Invalid type `bevy_transform::components::transform::Transform` found"
                .to_string(),
            data:    None,
        };

        let result = extract_type_name_from_error(&error);
        assert_eq!(
            result,
            Some("bevy_transform::components::transform::Transform".to_string())
        );
    }

    #[test]
    fn test_extract_type_name_from_error_no_backticks() {
        let error = BrpError {
            code:    -1,
            message: "Invalid type found with no backticks".to_string(),
            data:    None,
        };

        let result = extract_type_name_from_error(&error);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_type_name_from_error_incomplete_backticks() {
        let error = BrpError {
            code:    -1,
            message: "Invalid type `Transform with no closing backtick".to_string(),
            data:    None,
        };

        let result = extract_type_name_from_error(&error);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_single_field_value_success() {
        let mut obj = serde_json::Map::new();
        obj.insert("test_field".to_string(), serde_json::json!("test_value"));

        let result = extract_single_field_value(&obj);
        assert_eq!(
            result,
            Some(("test_field", &serde_json::json!("test_value")))
        );
    }

    #[test]
    fn test_extract_single_field_value_empty_object() {
        let obj = serde_json::Map::new();

        let result = extract_single_field_value(&obj);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_single_field_value_multi_field() {
        let mut obj = serde_json::Map::new();
        obj.insert("field1".to_string(), serde_json::json!("value1"));
        obj.insert("field2".to_string(), serde_json::json!("value2"));

        let result = extract_single_field_value(&obj);
        assert_eq!(result, None);
    }
}
