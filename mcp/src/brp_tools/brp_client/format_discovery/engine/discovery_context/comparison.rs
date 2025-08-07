//! Comparison logic for local vs extras formats
//!
//! This module provides functionality to compare type formats derived
//! from our local registry + hardcoded knowledge with those from the extras plugin.

use serde_json::{Value, json};
use tracing::{debug, info};

use super::registry_cache::global_cache;
use crate::brp_tools::Port;
use crate::brp_tools::brp_client::format_discovery::engine::types::BrpTypeName;
use crate::error::Result;

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
    /// Value mismatch - different JSON values
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

    /// Compare local format with extras response
    ///
    /// Phase 1: Build local format using registry + hardcoded knowledge and compare
    /// with extras response to measure progress.
    pub async fn compare_with_local(
        extras_response: &Value,
        port: Port,
        type_name: &str,
    ) -> Result<Self> {
        debug!("Building local format for comparison with extras");

        // Build local format - checks cache first, builds if needed
        let local_format = Self::build_extras_equivalent_response(port, type_name).await?;

        debug!("Local format built: {:?}", local_format.is_some());

        // Compare with actual local format
        let comparison = Self::new(Some(extras_response.clone()), local_format);

        debug!(
            "Created comparison between extras and local formats - {} differences found",
            comparison.differences.len()
        );

        Ok(comparison)
    }

    /// Log structured comparison results for debugging and analysis
    ///
    /// This function provides detailed tracing output showing comparison results
    /// between local and extras formats. Essential for monitoring progress as
    /// we implement local format building in future phases.
    pub fn log_comparison_results(&self, type_name: &BrpTypeName) {
        // Categorize differences
        let missing_in_local: Vec<_> = self
            .differences
            .iter()
            .filter_map(|d| match d {
                FormatDifference::MissingField {
                    path,
                    source: ComparisonSource::Local,
                    ..
                } => Some(path.as_str()),
                _ => None,
            })
            .collect();

        let missing_in_extras: Vec<_> = self
            .differences
            .iter()
            .filter_map(|d| match d {
                FormatDifference::MissingField {
                    path,
                    source: ComparisonSource::Extras,
                    ..
                } => Some(path.as_str()),
                _ => None,
            })
            .collect();

        let value_mismatches: Vec<_> = self
            .differences
            .iter()
            .filter_map(|d| match d {
                FormatDifference::ValueMismatch {
                    path,
                    extras,
                    local,
                } => Some(json!({
                    "path": path,
                    "extras": extras,
                    "local": local
                })),
                _ => None,
            })
            .collect();

        // Extract spawn formats for comparison
        let extras_spawn = self
            .extras_format
            .as_ref()
            .and_then(|ef| ef.pointer(&format!("/type_info/{}/example_values/spawn", type_name)));

        let local_spawn = self
            .local_format
            .as_ref()
            .and_then(|lf| lf.pointer(&format!("/type_info/{}/example_values/spawn", type_name)));

        let spawn_match = match (extras_spawn, local_spawn) {
            (Some(e), Some(l)) => e == l,
            _ => false,
        };

        // Log structured comparison result (machine-readable)
        info!(
            "COMPARISON_RESULT: {}",
            json!({
                "type": type_name.as_str(),
                "phase": 1,
                "is_equivalent": self.differences.is_empty(),
                "total_differences": self.differences.len(),
                "missing_in_local": missing_in_local,
                "missing_in_extras": missing_in_extras,
                "value_mismatches": value_mismatches,
                "spawn_formats_match": spawn_match,
                "extras_format": self.extras_format,
                "local_format": self.local_format,
            })
        );

        // Phase 1 success criteria
        let has_type_info = self
            .local_format
            .as_ref()
            .and_then(|lf| lf.get("type_info"))
            .is_some();

        let has_example_values = self
            .local_format
            .as_ref()
            .and_then(|lf| lf.pointer(&format!("/type_info/{}/example_values", type_name)))
            .is_some();

        let phase1_success = spawn_match && has_type_info && has_example_values;

        // Log phase-specific status (machine-readable)
        info!(
            "PHASE_1_STATUS: {}",
            json!({
                "success": phase1_success,
                "type": type_name.as_str(),
                "spawn_formats_match": spawn_match,
                "missing_fields_count": missing_in_local.len(),
                "has_core_structure": has_type_info && has_example_values,
            })
        );

        // Human-friendly summary
        if phase1_success {
            info!(
                "✅ Phase 1 SUCCESS for {}: Spawn formats match, core structure built",
                type_name
            );
        } else {
            let preview_missing: Vec<_> = missing_in_local.iter().take(5).map(|s| *s).collect();
            info!(
                "❌ Phase 1 INCOMPLETE for {}: {} differences, missing: {:?}",
                type_name,
                self.differences.len(),
                preview_missing
            );
        }

        // Keep original debug logging for detailed troubleshooting
        debug!(
            type_name = %type_name,
            comparison_result = ?self,
            differences_count = self.differences.len(),
            is_equivalent = self.is_equivalent(),
            "Local vs Extras format comparison (detailed)"
        );
    }

    /// Build extras-equivalent response for comparison
    ///
    /// This method assembles a temporary comparison format from cached data that matches
    /// the structure returned by `bevy_brp_extras/discover_format`. It:
    /// 1. Checks cache first for existing `CachedTypeInfo`
    /// 2. Calls `build_local_type_info()` if cache miss
    /// 3. Assembles full extras-equivalent response with `type_info` wrapper
    ///
    /// Phase 1: Returns spawn format with metadata for Transform component
    async fn build_extras_equivalent_response(
        _port: Port,
        type_name: &str,
    ) -> Result<Option<Value>> {
        debug!("Building extras-equivalent response for comparison");

        // Check cache first
        let cached_info = if let Some(info) = global_cache().get(&type_name.into()) {
            debug!("Found cached type info for {}", type_name);
            info
        } else {
            // Cache miss - for now return None since build_local_type_info is in context.rs
            // In Phase 1, context.rs will call build_local_type_info before comparison
            debug!(
                "Cache miss for {} - no local format available yet",
                type_name
            );
            return Ok(None);
        };

        // Use reflection flags directly from cached info
        let has_serialize = cached_info.reflect_types.contains(&"Serialize".to_string());
        let has_deserialize = cached_info
            .reflect_types
            .contains(&"Deserialize".to_string());

        debug!(
            "Extracted reflection flags: serialize={}, deserialize={}",
            has_serialize, has_deserialize
        );

        // Convert mutation paths to extras format (object with path -> description mappings)
        let mut mutation_paths_obj = serde_json::Map::new();

        // Group paths by base field to determine which are "entire" fields
        let mut base_fields = std::collections::HashSet::new();
        let mut component_fields = std::collections::HashSet::new();

        for mutation_path in &cached_info.mutation_paths {
            let path_parts: Vec<&str> = mutation_path
                .path
                .trim_start_matches('.')
                .split('.')
                .collect();
            if path_parts.len() == 1 {
                base_fields.insert(path_parts[0]);
            } else if path_parts.len() == 2 {
                component_fields.insert(path_parts[0]);
            }
        }

        // Generate descriptions with example values
        for mutation_path in &cached_info.mutation_paths {
            let path_without_dot = mutation_path.path.trim_start_matches('.');
            let path_parts: Vec<&str> = path_without_dot.split('.').collect();

            let description = if path_parts.len() == 1 {
                // Base field - check if it has components to determine "entire" vs just field name
                let field_name = path_parts[0];
                if component_fields.contains(field_name) {
                    format!(
                        "Mutate the entire {} field, e.g.: {}",
                        field_name, mutation_path.example_value
                    )
                } else {
                    format!(
                        "Mutate the {} field, e.g.: {}",
                        field_name, mutation_path.example_value
                    )
                }
            } else if path_parts.len() == 2 {
                // Component field like .rotation.x
                let component_name = path_parts[1];
                format!(
                    "Mutate the {} component, e.g.: {}",
                    component_name, mutation_path.example_value
                )
            } else {
                // Fallback for deeper nesting
                format!(
                    "Mutate the {} field, e.g.: {}",
                    path_without_dot, mutation_path.example_value
                )
            };

            mutation_paths_obj.insert(mutation_path.path.clone(), json!(description));
        }

        // Log Phase 2 status for test validation
        let phase_2_status = json!({
            "success": !cached_info.mutation_paths.is_empty(),
            "has_mutation_paths": !cached_info.mutation_paths.is_empty(),
            "mutation_paths_count": cached_info.mutation_paths.len()
        });
        debug!("PHASE_2_STATUS: {}", phase_2_status);

        // Build full extras-equivalent response
        let local_response = json!({
            "type_info": {
                type_name: {
                    "example_values": {
                        "spawn": cached_info.spawn_format
                    },
                    "mutation_paths": mutation_paths_obj,
                    "type_category": "Struct",
                    "has_serialize": has_serialize,
                    "has_deserialize": has_deserialize,
                    "in_registry": true
                    // TODO Phase 2: Still missing supported_operations, enum_info, error, type_name
                }
            }
        });

        debug!("Built extras-equivalent response: {:?}", local_response);

        Ok(Some(local_response))
    }

    /// Compute differences between extras and local formats
    fn compute_differences(&mut self) {
        match (&self.extras_format, &self.local_format) {
            (Some(extras), Some(local)) => {
                // Both formats present - compare them
                self.differences = compare_json_values("", extras, local);
            }
            (Some(extras), None) => {
                // Phase 0.2: Local format not built yet - extract type_info and create missing
                // field entries
                self.differences = create_missing_field_entries_from_extras(extras);
            }
            (None, Some(_)) => {
                // Extras missing (shouldn't happen in practice)
                self.differences = vec![FormatDifference::MissingField {
                    path:   String::new(),
                    source: ComparisonSource::Extras,
                }];
            }
            (None, None) => {
                // Both formats missing - equivalent but useless
                self.differences = Vec::new();
            }
        }
    }

    /// Check if formats are equivalent
    pub const fn is_equivalent(&self) -> bool {
        self.differences.is_empty()
    }
}

/// Compare two JSON values and return differences
/// Checks for missing fields and value mismatches
pub fn compare_json_values(path: &str, extras: &Value, local: &Value) -> Vec<FormatDifference> {
    let mut differences = Vec::new();

    // Compare based on type
    match (extras, local) {
        (Value::Object(extras_obj), Value::Object(local_obj)) => {
            // Check for missing fields
            for (key, _) in extras_obj {
                if !local_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Local,
                    });
                }
            }
            for (key, _) in local_obj {
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

                    // Special handling for mutation_paths - only compare keys, not values
                    if key == "mutation_paths" {
                        differences
                            .extend(compare_mutation_paths(&sub_path, extras_val, local_val));
                    } else {
                        differences.extend(compare_json_values(&sub_path, extras_val, local_val));
                    }
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

/// Compare mutation paths objects - only check that keys match, allow different descriptions
fn compare_mutation_paths(path: &str, extras: &Value, local: &Value) -> Vec<FormatDifference> {
    let mut differences = Vec::new();

    match (extras, local) {
        (Value::Object(extras_obj), Value::Object(local_obj)) => {
            // Check for missing keys in local
            for key in extras_obj.keys() {
                if !local_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Local,
                    });
                }
            }

            // Check for extra keys in local (shouldn't happen but let's be thorough)
            for key in local_obj.keys() {
                if !extras_obj.contains_key(key) {
                    differences.push(FormatDifference::MissingField {
                        path:   format!("{path}.{key}"),
                        source: ComparisonSource::Extras,
                    });
                }
            }

            // Note: We intentionally do NOT compare the values (descriptions)
            // since our descriptions will be different from extras
        }
        _ => {
            // If either isn't an object, fall back to value mismatch
            differences.push(FormatDifference::ValueMismatch {
                path:   path.to_string(),
                extras: extras.clone(),
                local:  local.clone(),
            });
        }
    }

    differences
}

/// Create missing field entries from extras response (Phase 0.2)
/// Extracts only the `type_info` portion and recursively creates `MissingField` entries
fn create_missing_field_entries_from_extras(extras_response: &Value) -> Vec<FormatDifference> {
    let mut differences = Vec::new();

    // Extract the type_info portion from the extras response
    if let Some(type_info) = extras_response.get("type_info") {
        // Recursively traverse the type_info structure
        add_missing_fields_recursive("", type_info, &mut differences);
    }

    differences
}

/// Recursively add `MissingField` entries for all fields in the JSON structure
fn add_missing_fields_recursive(
    path: &str,
    value: &Value,
    differences: &mut Vec<FormatDifference>,
) {
    // Add a MissingField entry for this path
    differences.push(FormatDifference::MissingField {
        path:   path.to_string(),
        source: ComparisonSource::Local,
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
