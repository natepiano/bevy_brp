//! Core extraction infrastructure for JSON field extraction.
//!
//! This module provides the unified extraction logic for both parameter
//! and response field extraction with type safety.

use serde_json::Value;

use crate::error::Result;

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
    pub fn into_string(self) -> Result<String> {
        match self {
            Self::String(s) => Ok(s),
            _ => Err(crate::error::Error::invalid("parameter", "Expected string value").into()),
        }
    }

    /// Convert to u64, returning error if wrong type
    pub fn into_u64(self) -> Result<u64> {
        match self {
            Self::Number(n) => Ok(n),
            _ => Err(crate::error::Error::invalid("parameter", "Expected number value").into()),
        }
    }

    /// Convert to u32, returning error if wrong type or out of range
    pub fn into_u32(self) -> Result<u32> {
        match self {
            Self::Number(n) => u32::try_from(n).map_err(|_| {
                crate::error::Error::invalid("parameter", "Number value too large for u32").into()
            }),
            _ => Err(crate::error::Error::invalid("parameter", "Expected number value").into()),
        }
    }

    /// Convert to bool, returning error if wrong type
    pub fn into_bool(self) -> Result<bool> {
        match self {
            Self::Boolean(b) => Ok(b),
            _ => Err(crate::error::Error::invalid("parameter", "Expected boolean value").into()),
        }
    }

    /// Convert to string array, returning error if wrong type
    pub fn into_string_array(self) -> Result<Vec<String>> {
        match self {
            Self::StringArray(arr) => Ok(arr),
            _ => {
                Err(crate::error::Error::invalid("parameter", "Expected string array value").into())
            }
        }
    }

    /// Convert to number array, returning error if wrong type
    pub fn into_number_array(self) -> Result<Vec<u64>> {
        match self {
            Self::NumberArray(arr) => Ok(arr),
            _ => {
                Err(crate::error::Error::invalid("parameter", "Expected number array value").into())
            }
        }
    }

    /// Convert to any JSON value, returning error if wrong type
    pub fn into_any(self) -> Result<Value> {
        match self {
            Self::Any(v) => Ok(v),
            _ => Err(crate::error::Error::invalid("parameter", "Expected any JSON value").into()),
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

impl From<ExtractedValue> for serde_json::Value {
    fn from(extracted: ExtractedValue) -> Self {
        match extracted {
            ExtractedValue::String(s) => Self::String(s),
            ExtractedValue::Number(n) => Self::Number(serde_json::Number::from(n)),
            ExtractedValue::Boolean(b) => Self::Bool(b),
            ExtractedValue::StringArray(arr) => {
                Self::Array(arr.into_iter().map(Self::String).collect())
            }
            ExtractedValue::NumberArray(arr) => Self::Array(
                arr.into_iter()
                    .map(|n| Self::Number(serde_json::Number::from(n)))
                    .collect(),
            ),
            ExtractedValue::Any(v) => v,
        }
    }
}

/// Response field types for extraction.
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

fn extract_query_component_count(value: &Value) -> ExtractedValue {
    // Count total components across query results
    // Expects an array of entities where each entity is an object with components
    let total = value.as_array().map_or(0, |entities| {
        entities
            .iter()
            .filter_map(|e| e.as_object())
            .map(serde_json::Map::len)
            .sum::<usize>()
    });
    ExtractedValue::Number(total as u64)
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
            Self::QueryComponentCount => Some(extract_query_component_count(value)),
        }
    }
}

/// Extract a response field value from a provider.
///
/// This function provides type-safe extraction for responses, allowing
/// all field types including Count and `LineSplit`.
pub fn extract_response_field<P>(
    provider: &P,
    field_name: &str,
    field_type: ResponseFieldType,
) -> Option<ExtractedValue>
where
    P: JsonFieldProvider,
{
    let value = provider.get_field(field_name)?;
    field_type.extract(&value)
}
