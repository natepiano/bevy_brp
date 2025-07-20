//! Core extraction infrastructure for JSON field extraction.
//!
//! This module provides the unified extraction logic for both parameter
//! and response field extraction with type safety.

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

/// Generic trait for field specifications.
///
/// This trait provides a uniform interface for both tool parameters
/// and response fields to specify their names and expected types.
pub trait FieldSpec<T> {
    /// Get the field name as a string.
    fn field_name(&self) -> &str;

    /// Get the expected field type.
    fn field_type(&self) -> T;
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

/// Parameter field types (no Count or `LineSplit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterFieldType {
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
    /// Dynamic parameters for BRP methods - the value becomes the method parameters directly
    DynamicParams,
}

/// Response field types (includes Count and `LineSplit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseFieldType {
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
    /// Count total components across query results (entities with their components)
    QueryComponentCount,
}

// Private extraction helper functions used by both field type enums

fn extract_string(value: &Value) -> Option<ExtractedValue> {
    value
        .as_str()
        .map(|s| ExtractedValue::String(s.to_string()))
}

fn extract_number(value: &Value) -> Option<ExtractedValue> {
    value.as_u64().map(ExtractedValue::Number)
}

fn extract_boolean(value: &Value) -> Option<ExtractedValue> {
    value.as_bool().map(ExtractedValue::Boolean)
}

fn extract_string_array(value: &Value) -> Option<ExtractedValue> {
    value.as_array().and_then(|arr| {
        let strings: Option<Vec<String>> =
            arr.iter().map(|v| v.as_str().map(String::from)).collect();
        strings.map(ExtractedValue::StringArray)
    })
}

fn extract_number_array(value: &Value) -> Option<ExtractedValue> {
    value.as_array().and_then(|arr| {
        let numbers: Option<Vec<u64>> = arr.iter().map(Value::as_u64).collect();
        numbers.map(ExtractedValue::NumberArray)
    })
}

fn extract_any(value: &Value) -> ExtractedValue {
    ExtractedValue::Any(value.clone())
}

fn extract_count(value: &Value) -> Option<ExtractedValue> {
    // Count items in arrays or object keys
    match value {
        Value::Array(arr) => Some(ExtractedValue::Number(arr.len() as u64)),
        Value::Object(obj) => Some(ExtractedValue::Number(obj.len() as u64)),
        _ => None,
    }
}

fn extract_line_split(value: &Value) -> Option<ExtractedValue> {
    // Split a string value into lines
    value.as_str().map(|s| {
        let lines: Vec<String> = s.lines().map(String::from).collect();
        ExtractedValue::StringArray(lines)
    })
}

fn extract_query_component_count(value: &Value) -> Option<ExtractedValue> {
    // Count total components across query results
    // Expects an array of entities where each entity is an object with components
    let total = value.as_array().map_or(0, |entities| {
        entities
            .iter()
            .filter_map(|e| e.as_object())
            .map(serde_json::Map::len)
            .sum::<usize>()
    });
    Some(ExtractedValue::Number(total as u64))
}

impl ParameterFieldType {
    /// Extract a value based on the parameter field type.
    pub(crate) fn extract(self, value: &Value) -> Option<ExtractedValue> {
        match self {
            Self::String => extract_string(value),
            Self::Number => extract_number(value),
            Self::Boolean => extract_boolean(value),
            Self::StringArray => extract_string_array(value),
            Self::NumberArray => extract_number_array(value),
            Self::Any | Self::DynamicParams => Some(extract_any(value)),
        }
    }
}

impl ResponseFieldType {
    /// Extract a value based on the response field type.
    pub(crate) fn extract(self, value: &Value) -> Option<ExtractedValue> {
        match self {
            Self::String => extract_string(value),
            Self::Number => extract_number(value),
            Self::Boolean => extract_boolean(value),
            Self::StringArray => extract_string_array(value),
            Self::NumberArray => extract_number_array(value),
            Self::Any => Some(extract_any(value)),
            Self::Count => extract_count(value),
            Self::LineSplit => extract_line_split(value),
            Self::QueryComponentCount => extract_query_component_count(value),
        }
    }
}

/// Extract a parameter field value from a provider based on field specification.
///
/// This function provides type-safe extraction for parameters, ensuring
/// that only valid parameter field types can be used.
pub fn extract_parameter_field<P, F>(provider: &P, field: F) -> Option<ExtractedValue>
where
    P: JsonFieldProvider,
    F: FieldSpec<ParameterFieldType>,
{
    let value = provider.get_field(field.field_name())?;
    field.field_type().extract(&value)
}

/// Extract a response field value from a provider based on field specification.
///
/// This function provides type-safe extraction for responses, allowing
/// all field types including Count and `LineSplit`.
pub fn extract_response_field<P, F>(provider: &P, field: F) -> Option<ExtractedValue>
where
    P: JsonFieldProvider,
    F: FieldSpec<ResponseFieldType>,
{
    let value = provider.get_field(field.field_name())?;
    field.field_type().extract(&value)
}
