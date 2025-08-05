//! Common utilities shared across format transformers

use serde_json::Value;

use crate::brp_tools::BrpClientError;

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
pub fn extract_type_name_from_error(error: &BrpClientError) -> Option<String> {
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
