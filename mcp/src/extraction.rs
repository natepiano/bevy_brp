//! Unified JSON field extraction system for tool parameters and response fields.
//!
//! This module provides a common infrastructure for extracting values from JSON
//! structures, shared between the tool parameter system and response field system.

use rmcp::ErrorData as McpError;
use serde_json::Value;

/// Trait for anything that provides JSON fields for extraction.
///
/// This trait allows different sources (request arguments, response data, etc.)
/// to provide a uniform interface for field access.
pub trait JsonFieldProvider {
    /// Get the root JSON value.
    ///
    /// This is used for dot notation field access.
    fn get_root(&self) -> Value;

    /// Get a field value by name.
    ///
    /// Returns `None` if the field doesn't exist.
    /// The default implementation supports dot notation for nested field access.
    fn get_field(&self, field_name: &str) -> Option<Value> {
        // Check if field name contains dots for nested access
        if field_name.contains('.') {
            let parts: Vec<&str> = field_name.split('.').collect();
            let mut current = self.get_root();

            for part in parts {
                match current {
                    Value::Object(ref map) => {
                        current = map.get(part)?.clone();
                    }
                    Value::Array(ref arr) => {
                        // Support array indexing like "items.0"
                        if let Ok(index) = part.parse::<usize>() {
                            current = arr.get(index)?.clone();
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }

            Some(current)
        } else {
            // Simple field access from root
            match self.get_root() {
                Value::Object(map) => map.get(field_name).cloned(),
                _ => None,
            }
        }
    }
}

/// Trait for field specifications (name + type).
///
/// This trait provides a uniform interface for both tool parameters
/// and response fields to specify their names and expected types.
pub trait FieldSpec {
    /// Get the field name as a string.
    fn field_name(&self) -> &str;

    /// Get the expected field type.
    fn field_type(&self) -> FieldType;
}

/// Types of fields that can be extracted from JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldType {
    /// A string field
    String,
    /// A numeric field (typically u64)
    Number,
    /// A boolean field
    Boolean,
    /// An array of strings
    StringArray,
    /// An array of numbers
    NumberArray,
    /// Any JSON value (object, array, etc.)
    Any,
    /// Count of items in an array or object
    Count,
    /// String split into lines
    LineSplit,
    /// Dynamic parameters for BRP methods - the value becomes the method parameters directly
    DynamicParams,
}

/// Extracted field values with their types.
///
/// This enum represents the different types of values that can be
/// extracted from JSON fields.
#[derive(Debug, Clone)]
pub enum ExtractedValue {
    /// String value
    String(String),
    /// Numeric value (u64)
    Number(u64),
    /// Boolean value
    Boolean(bool),
    /// Array of strings
    StringArray(Vec<String>),
    /// Array of numbers
    NumberArray(Vec<u64>),
    /// Any JSON value
    Any(Value),
}

impl ExtractedValue {
    /// Convert to string, returning error if wrong type
    pub fn into_string(self) -> Result<String, McpError> {
        match self {
            Self::String(s) => Ok(s),
            _ => Err(McpError::invalid_params("Expected string value", None)),
        }
    }

    /// Convert to u64, returning error if wrong type
    pub fn into_u64(self) -> Result<u64, McpError> {
        match self {
            Self::Number(n) => Ok(n),
            _ => Err(McpError::invalid_params("Expected number value", None)),
        }
    }

    /// Convert to u32, returning error if wrong type or out of range
    pub fn into_u32(self) -> Result<u32, McpError> {
        match self {
            Self::Number(n) => u32::try_from(n)
                .map_err(|_| McpError::invalid_params("Number value too large for u32", None)),
            _ => Err(McpError::invalid_params("Expected number value", None)),
        }
    }

    /// Convert to bool, returning error if wrong type
    pub fn into_bool(self) -> Result<bool, McpError> {
        match self {
            Self::Boolean(b) => Ok(b),
            _ => Err(McpError::invalid_params("Expected boolean value", None)),
        }
    }

    /// Convert to string array, returning error if wrong type
    pub fn into_string_array(self) -> Result<Vec<String>, McpError> {
        match self {
            Self::StringArray(arr) => Ok(arr),
            _ => Err(McpError::invalid_params(
                "Expected string array value",
                None,
            )),
        }
    }

    /// Convert to number array, returning error if wrong type
    pub fn into_number_array(self) -> Result<Vec<u64>, McpError> {
        match self {
            Self::NumberArray(arr) => Ok(arr),
            _ => Err(McpError::invalid_params(
                "Expected number array value",
                None,
            )),
        }
    }

    /// Convert to any JSON value, returning error if wrong type
    pub fn into_any(self) -> Result<Value, McpError> {
        match self {
            Self::Any(v) => Ok(v),
            _ => Err(McpError::invalid_params("Expected any JSON value", None)),
        }
    }
}

// Implement Into<ExtractedValue> for common types to support defaults
impl From<String> for ExtractedValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for ExtractedValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<u64> for ExtractedValue {
    fn from(n: u64) -> Self {
        Self::Number(n)
    }
}

impl From<u32> for ExtractedValue {
    fn from(n: u32) -> Self {
        Self::Number(u64::from(n))
    }
}

impl From<u16> for ExtractedValue {
    fn from(n: u16) -> Self {
        Self::Number(u64::from(n))
    }
}

impl From<bool> for ExtractedValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<Vec<String>> for ExtractedValue {
    fn from(arr: Vec<String>) -> Self {
        Self::StringArray(arr)
    }
}

impl From<Vec<u64>> for ExtractedValue {
    fn from(arr: Vec<u64>) -> Self {
        Self::NumberArray(arr)
    }
}

impl From<Value> for ExtractedValue {
    fn from(v: Value) -> Self {
        Self::Any(v)
    }
}

/// Response field specification for unified extraction.
///
/// This struct bridges the response field system with the unified extraction infrastructure.
pub struct ResponseFieldSpec {
    /// The field name (can include dot notation)
    pub field_name: String,
    /// The expected field type
    pub field_type: FieldType,
}

impl FieldSpec for ResponseFieldSpec {
    fn field_name(&self) -> &str {
        &self.field_name
    }

    fn field_type(&self) -> FieldType {
        self.field_type
    }
}

/// Extract a field value from a provider based on field specification.
///
/// This function provides a unified way to extract values from JSON fields,
/// handling type conversion based on the field specification.
///
/// # Arguments
/// * `provider` - Something that implements `JsonFieldProvider`
/// * `field` - A field specification with name and expected type
///
/// # Returns
/// * `Some(ExtractedValue)` if the field exists and can be converted to the expected type
/// * `None` if the field doesn't exist or can't be converted
pub fn extract_field<F: FieldSpec>(
    provider: &impl JsonFieldProvider,
    field: F,
) -> Option<ExtractedValue> {
    let field_name = field.field_name();
    let field_type = field.field_type();

    // Get the raw JSON value
    let value = provider.get_field(field_name)?;

    // Extract based on the field's type
    match field_type {
        FieldType::String => value
            .as_str()
            .map(|s| ExtractedValue::String(s.to_string())),
        FieldType::Number => value.as_u64().map(ExtractedValue::Number),
        FieldType::Boolean => value.as_bool().map(ExtractedValue::Boolean),
        FieldType::StringArray => value.as_array().and_then(|arr| {
            let strings: Option<Vec<String>> =
                arr.iter().map(|v| v.as_str().map(String::from)).collect();
            strings.map(ExtractedValue::StringArray)
        }),
        FieldType::NumberArray => value.as_array().and_then(|arr| {
            let numbers: Option<Vec<u64>> = arr.iter().map(serde_json::Value::as_u64).collect();
            numbers.map(ExtractedValue::NumberArray)
        }),
        FieldType::Any => Some(ExtractedValue::Any(value)),
        FieldType::Count => {
            // Count items in arrays or object keys
            match &value {
                Value::Array(arr) => Some(ExtractedValue::Number(arr.len() as u64)),
                Value::Object(obj) => Some(ExtractedValue::Number(obj.len() as u64)),
                _ => None,
            }
        }
        FieldType::LineSplit => {
            // Split a string value into lines
            value.as_str().map(|s| {
                let lines: Vec<String> = s.lines().map(String::from).collect();
                ExtractedValue::StringArray(lines)
            })
        }
        FieldType::DynamicParams => {
            // For dynamic params, return the value as-is (same as Any)
            Some(ExtractedValue::Any(value))
        }
    }
}
