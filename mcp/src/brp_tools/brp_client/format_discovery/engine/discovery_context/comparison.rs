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
    #[allow(dead_code)]
    pub extras_format: Option<Value>,
    /// Format derived from local registry + hardcoded knowledge
    #[allow(dead_code)]
    pub local_format:  Option<Value>,
}

/// Types of differences found during comparison
#[derive(Debug, Clone)]
pub enum FormatDifference {
    /// Field missing in one source
    MissingField {
        #[allow(dead_code)]
        path:   String,
        #[allow(dead_code)]
        source: ComparisonSource,
        #[allow(dead_code)]
        value:  Value,
    },
    /// Structure type mismatch (e.g., array vs object)
    StructureType {
        #[allow(dead_code)]
        extras: SerializationFormat,
        #[allow(dead_code)]
        local:  SerializationFormat,
        #[allow(dead_code)]
        path:   String,
    },
    /// Value mismatch - same structure but different JSON values
    ValueMismatch {
        #[allow(dead_code)]
        extras: Value,
        #[allow(dead_code)]
        local:  Value,
        #[allow(dead_code)]
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
        match (&self.extras_format, &self.local_format) {
            (Some(extras), Some(local)) => {
                // Both formats present - compare them
                self.differences = compare_json_values("", extras, local);
            }
            (Some(extras), None) => {
                // Phase 0.2: Local format not built yet - extract type_info and create missing field entries
                self.differences = create_missing_field_entries_from_extras(extras);
            }
            (None, Some(local)) => {
                // Extras missing (shouldn't happen in practice)
                self.differences = vec![FormatDifference::MissingField {
                    path:   String::new(),
                    source: ComparisonSource::Extras,
                    value:  local.clone(),
                }];
            }
            (None, None) => {
                // Both formats missing - equivalent but useless
                self.differences = Vec::new();
            }
        }
    }

    /// Check if formats are equivalent
    #[allow(dead_code)]
    pub const fn is_equivalent(&self) -> bool {
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
            for (key, value) in extras_obj {
                if !local_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Local,
                        value:  value.clone(),
                    });
                }
            }
            for (key, value) in local_obj {
                if !extras_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Extras,
                        value:  value.clone(),
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
            if extras_arr.len() == local_arr.len() {
                for (i, (extras_val, local_val)) in
                    extras_arr.iter().zip(local_arr.iter()).enumerate()
                {
                    let sub_path = format!("{path}[{i}]");
                    differences.extend(compare_json_values(&sub_path, extras_val, local_val));
                }
            } else {
                differences.push(FormatDifference::ValueMismatch {
                    path:   path.to_string(),
                    extras: extras.clone(),
                    local:  local.clone(),
                });
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

/// Convert a JSON value to its corresponding `SerializationFormat`
const fn value_to_serialization_format(val: &Value) -> SerializationFormat {
    match val {
        Value::Array(_) => SerializationFormat::Array,
        Value::Object(_) => SerializationFormat::Object,
        _ => SerializationFormat::Primitive, // null, bool, number, string
    }
}

/// Create missing field entries from extras response (Phase 0.2)
/// Extracts only the type_info portion and recursively creates MissingField entries
fn create_missing_field_entries_from_extras(extras_response: &Value) -> Vec<FormatDifference> {
    let mut differences = Vec::new();

    // Extract the type_info portion from the extras response
    if let Some(type_info) = extras_response.get("type_info") {
        // Recursively traverse the type_info structure
        add_missing_fields_recursive("", type_info, &mut differences);
    }

    differences
}

/// Recursively add MissingField entries for all fields in the JSON structure
fn add_missing_fields_recursive(path: &str, value: &Value, differences: &mut Vec<FormatDifference>) {
    // Add a MissingField entry for this path
    differences.push(FormatDifference::MissingField {
        path: path.to_string(),
        source: ComparisonSource::Local,
        value: value.clone(),
    });

    // Recursively process child fields
    match value {
        Value::Object(obj) => {
            for (key, child_value) in obj {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                add_missing_fields_recursive(&child_path, child_value, differences);
            }
        }
        Value::Array(arr) => {
            for (index, child_value) in arr.iter().enumerate() {
                let child_path = format!("{path}[{index}]");
                add_missing_fields_recursive(&child_path, child_value, differences);
            }
        }
        _ => {
            // Primitives (string, number, bool, null) don't have child fields
        }
    }
}
