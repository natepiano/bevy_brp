//! Comparison logic for local vs extras formats
//!
//! This module provides functionality to compare type formats derived
//! from our local registry + hardcoded knowledge with those from the extras plugin.

use serde_json::Value;

use super::types::SerializationFormat;

/// Comparison results between extras and local formats
#[derive(Debug, Clone)]
pub struct RegistryComparison {
    /// Differences found between formats
    pub differences:   Vec<FormatDifference>,
    /// Format from extras plugin
    pub extras_format: Option<Value>,
    /// Format derived from local registry + hardcoded knowledge
    pub local_format:  Option<Value>,
}

/// Types of differences found during comparison
#[derive(Debug, Clone)]
pub enum FormatDifference {
    /// Field missing in one source
    MissingField {
        path:   String,
        source: ComparisonSource,
    },
    /// Structure type mismatch (e.g., array vs object)
    StructureType {
        extras: SerializationFormat,
        local:  SerializationFormat,
        path:   String,
    },
    /// Value mismatch - same structure but different JSON values
    ValueMismatch {
        extras: Value,
        local:  Value,
        path:   String,
    },
}

/// Source of comparison data
#[derive(Debug, Clone, Copy)]
pub enum ComparisonSource {
    /// From extras plugin
    Extras,
    /// From our local registry + hardcoded knowledge construction
    Local,
}

impl RegistryComparison {
    /// Create a new comparison result
    pub fn new(extras_format: Option<Value>, local_format: Option<Value>) -> Self {
        let mut comparison = Self {
            extras_format,
            local_format,
            differences: Vec::new(),
        };
        comparison.compute_differences();
        comparison
    }

    /// Compute differences between extras and local formats
    fn compute_differences(&mut self) {
        // Stub implementation - will be filled in Phase 2
        // This will compare the structure and values of the two formats
        if let (Some(extras), Some(local)) = (&self.extras_format, &self.local_format) {
            self.differences = compare_json_values("", extras, local);
        }
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
/// Uses priority logic: 1) Check existence 2) Check structure type 3) Check values
#[allow(dead_code)]
pub fn compare_json_values(path: &str, extras: &Value, local: &Value) -> Vec<FormatDifference> {
    let mut differences = Vec::new();

    // Check if structure types match (SerializationFormat)
    let extras_format = value_to_serialization_format(extras);
    let local_format = value_to_serialization_format(local);

    if extras_format != local_format {
        differences.push(FormatDifference::StructureType {
            path:   path.to_string(),
            extras: extras_format,
            local:  local_format,
        });
        return differences; // Short-circuit - don't also report ValueMismatch
    }

    // Compare based on type (same SerializationFormat, now check values)
    match (extras, local) {
        (Value::Object(extras_obj), Value::Object(local_obj)) => {
            // Check for missing fields
            for key in extras_obj.keys() {
                if !local_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Local,
                    });
                }
            }
            for key in local_obj.keys() {
                if !extras_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Extras,
                    });
                }
            }

            // Recursively compare common fields
            for (key, extras_val) in extras_obj {
                if let Some(local_val) = local_obj.get(key) {
                    let sub_path = format!("{path}.{key}");
                    differences.extend(compare_json_values(&sub_path, extras_val, local_val));
                }
            }
        }
        (Value::Array(extras_arr), Value::Array(local_arr)) => {
            if extras_arr.len() != local_arr.len() {
                differences.push(FormatDifference::ValueMismatch {
                    path:   path.to_string(),
                    extras: extras.clone(),
                    local:  local.clone(),
                });
            } else {
                for (i, (extras_val, local_val)) in
                    extras_arr.iter().zip(local_arr.iter()).enumerate()
                {
                    let sub_path = format!("{path}[{i}]");
                    differences.extend(compare_json_values(&sub_path, extras_val, local_val));
                }
            }
        }
        _ => {
            // For primitives, just check equality
            if extras != local {
                differences.push(FormatDifference::ValueMismatch {
                    path:   path.to_string(),
                    extras: extras.clone(),
                    local:  local.clone(),
                });
            }
        }
    }

    differences
}

/// Convert a JSON value to its corresponding SerializationFormat
fn value_to_serialization_format(val: &Value) -> SerializationFormat {
    match val {
        Value::Array(_) => SerializationFormat::Array,
        Value::Object(_) => SerializationFormat::Object,
        _ => SerializationFormat::Primitive, // null, bool, number, string
    }
}
