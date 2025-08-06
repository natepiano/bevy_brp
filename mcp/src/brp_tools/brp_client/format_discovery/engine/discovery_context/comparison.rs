//! Comparison logic for registry vs extras formats
//!
//! This module provides functionality to compare type formats derived
//! from the registry with those from the extras plugin.

use serde_json::Value;

use crate::brp_tools::brp_client::format_discovery::engine::discovery_context::types::{
    ComparisonSource, FormatDifference, RegistryComparison,
};

impl RegistryComparison {
    /// Create a new comparison result
    pub fn new(extras_format: Option<Value>, registry_format: Option<Value>) -> Self {
        let mut comparison = Self {
            extras_format,
            registry_format,
            differences: Vec::new(),
        };
        comparison.compute_differences();
        comparison
    }

    /// Compute differences between extras and registry formats
    fn compute_differences(&mut self) {
        // Stub implementation - will be filled in Phase 2
        // This will compare the structure and values of the two formats
    }

    /// Check if formats are equivalent
    #[allow(dead_code)]
    pub fn is_equivalent(&self) -> bool {
        self.differences.is_empty()
    }

    /// Get a summary of differences
    #[allow(dead_code)]
    pub fn difference_summary(&self) -> String {
        if self.differences.is_empty() {
            "Formats are equivalent".to_string()
        } else {
            format!("Found {} difference(s)", self.differences.len())
        }
    }
}

/// Compare two JSON values and return differences
#[allow(dead_code)]
pub fn compare_json_values(path: &str, extras: &Value, registry: &Value) -> Vec<FormatDifference> {
    let mut differences = Vec::new();

    // Check if types match
    if !values_have_same_type(extras, registry) {
        differences.push(FormatDifference::StructureType {
            path:     path.to_string(),
            extras:   value_type_name(extras),
            registry: value_type_name(registry),
        });
        return differences;
    }

    // Compare based on type
    match (extras, registry) {
        (Value::Object(extras_obj), Value::Object(registry_obj)) => {
            // Check for missing fields
            for key in extras_obj.keys() {
                if !registry_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Registry,
                    });
                }
            }
            for key in registry_obj.keys() {
                if !extras_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Extras,
                    });
                }
            }

            // Recursively compare common fields
            for (key, extras_val) in extras_obj {
                if let Some(registry_val) = registry_obj.get(key) {
                    let sub_path = format!("{path}.{key}");
                    differences.extend(compare_json_values(&sub_path, extras_val, registry_val));
                }
            }
        }
        (Value::Array(extras_arr), Value::Array(registry_arr)) => {
            if extras_arr.len() != registry_arr.len() {
                differences.push(FormatDifference::ValueType {
                    path:     path.to_string(),
                    extras:   format!("array[{}]", extras_arr.len()),
                    registry: format!("array[{}]", registry_arr.len()),
                });
            } else {
                for (i, (extras_val, registry_val)) in
                    extras_arr.iter().zip(registry_arr.iter()).enumerate()
                {
                    let sub_path = format!("{path}[{i}]");
                    differences.extend(compare_json_values(&sub_path, extras_val, registry_val));
                }
            }
        }
        _ => {
            // For primitives, just check equality
            if extras != registry {
                differences.push(FormatDifference::ValueType {
                    path:     path.to_string(),
                    extras:   format!("{extras:?}"),
                    registry: format!("{registry:?}"),
                });
            }
        }
    }

    differences
}

/// Check if two JSON values have the same type
fn values_have_same_type(a: &Value, b: &Value) -> bool {
    matches!(
        (a, b),
        (Value::Null, Value::Null)
            | (Value::Bool(_), Value::Bool(_))
            | (Value::Number(_), Value::Number(_))
            | (Value::String(_), Value::String(_))
            | (Value::Array(_), Value::Array(_))
            | (Value::Object(_), Value::Object(_))
    )
}

/// Get a string representation of a value's type
fn value_type_name(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "bool".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_compare_identical_values() {
        let val1 = json!({"x": 1.0, "y": 2.0});
        let val2 = json!({"x": 1.0, "y": 2.0});
        let differences = compare_json_values("root", &val1, &val2);
        assert!(differences.is_empty());
    }

    #[test]
    fn test_compare_different_types() {
        let val1 = json!([1.0, 2.0, 3.0]);
        let val2 = json!({"x": 1.0, "y": 2.0, "z": 3.0});
        let differences = compare_json_values("root", &val1, &val2);
        assert_eq!(differences.len(), 1);
        assert!(matches!(
            &differences[0],
            FormatDifference::StructureType { .. }
        ));
    }

    #[test]
    fn test_compare_missing_fields() {
        let val1 = json!({"x": 1.0, "y": 2.0});
        let val2 = json!({"x": 1.0, "y": 2.0, "z": 3.0});
        let differences = compare_json_values("root", &val1, &val2);
        assert_eq!(differences.len(), 1);
        assert!(matches!(
            &differences[0],
            FormatDifference::MissingField { .. }
        ));
    }
}
