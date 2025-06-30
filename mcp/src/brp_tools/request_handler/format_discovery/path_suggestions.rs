//! Path suggestion generator for mutation errors
//! Provides actionable error messages by suggesting valid mutation paths

use std::collections::HashSet;

use serde_json::Value;

use crate::brp_tools::support::brp_client::{BrpError, BrpResult, execute_brp_method};
use crate::error::Result;
use crate::tools::BRP_METHOD_REGISTRY_SCHEMA;

/// Generate valid mutation paths for a component type
pub async fn generate_valid_paths(
    component_type: &str,
    port: Option<u16>,
    max_depth: usize,
) -> Result<Vec<String>> {
    // Get schema for the component type
    let schema_params = serde_json::json!({
        "with_types": ["Component", "Resource"],
        "with_crates": [extract_crate_name(component_type)]
    });

    let schema_result =
        execute_brp_method(BRP_METHOD_REGISTRY_SCHEMA, Some(schema_params), port).await?;

    match schema_result {
        BrpResult::Success(Some(schema_data)) => {
            let paths = extract_paths_from_schema(component_type, &schema_data, max_depth);
            Ok(paths)
        }
        _ => Ok(Vec::new()),
    }
}

/// Enhanced error message with path suggestions
pub async fn enhance_type_mismatch_error(
    original_error: &BrpError,
    component_type: Option<&str>,
    port: Option<u16>,
) -> BrpError {
    // Only enhance if we have a component type and it's a type mismatch error
    if let Some(comp_type) = component_type {
        if is_type_mismatch_error(&original_error.message) {
            if let Ok(valid_paths) = generate_valid_paths(comp_type, port, 2).await {
                if !valid_paths.is_empty() {
                    let short_type_name = comp_type.split("::").last().unwrap_or(comp_type);
                    let enhanced_message = format!(
                        "{}\nValid paths for {} include: {}",
                        original_error.message,
                        short_type_name,
                        format_path_suggestions(&valid_paths)
                    );

                    return BrpError {
                        code:    original_error.code,
                        message: enhanced_message,
                        data:    original_error.data.clone(),
                    };
                }
            }
        }
    }

    original_error.clone()
}

/// Check if an error message indicates a type mismatch suitable for path suggestions
fn is_type_mismatch_error(message: &str) -> bool {
    message.contains("Expected")
        && message.contains("access")
        && message.contains("found")
        && (message.contains("struct") || message.contains("tuple"))
}

/// Extract paths from schema data recursively
fn extract_paths_from_schema(
    type_name: &str,
    schema_data: &Value,
    max_depth: usize,
) -> Vec<String> {
    let mut paths = Vec::new();
    let mut visited = HashSet::new();

    if let Some(schema_obj) = schema_data.as_object() {
        if let Some(type_schema) = schema_obj.get(type_name) {
            traverse_schema(
                "".to_string(),
                type_schema,
                schema_obj,
                &mut paths,
                &mut visited,
                0,
                max_depth,
            );
        }
    }

    // Sort paths: shorter first, then alphabetically
    paths.sort_by(|a, b| {
        let depth_a = a.matches('.').count();
        let depth_b = b.matches('.').count();
        depth_a.cmp(&depth_b).then(a.cmp(b))
    });

    paths
}

/// Recursively traverse schema to generate all valid paths
fn traverse_schema(
    current_path: String,
    type_schema: &Value,
    full_schema: &serde_json::Map<String, Value>,
    paths: &mut Vec<String>,
    visited: &mut HashSet<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth >= max_depth {
        return;
    }

    if let Some(kind) = type_schema.get("kind").and_then(Value::as_str) {
        match kind {
            "Struct" => {
                if let Some(properties) = type_schema.get("properties").and_then(Value::as_object) {
                    for (field_name, field_info) in properties {
                        let field_path = if current_path.is_empty() {
                            format!(".{field_name}")
                        } else {
                            format!("{current_path}.{field_name}")
                        };

                        // Always add intermediate paths (can set entire structures)
                        paths.push(field_path.clone());

                        // Recursively add deeper paths if not a terminal type
                        if let Some(field_type_ref) = extract_type_ref(field_info) {
                            if !visited.contains(&field_type_ref)
                                && !is_terminal_type(&field_type_ref)
                            {
                                visited.insert(field_type_ref.clone());
                                if let Some(field_type_schema) = full_schema.get(&field_type_ref) {
                                    traverse_schema(
                                        field_path,
                                        field_type_schema,
                                        full_schema,
                                        paths,
                                        visited,
                                        depth + 1,
                                        max_depth,
                                    );
                                }
                                visited.remove(&field_type_ref);
                            }
                        }
                    }
                }
            }
            "TupleStruct" => {
                if let Some(prefix_items) = type_schema.get("prefixItems").and_then(Value::as_array)
                {
                    for (index, item_info) in prefix_items.iter().enumerate() {
                        let index_path = if current_path.is_empty() {
                            format!(".{index}")
                        } else {
                            format!("{current_path}.{index}")
                        };

                        // Always add intermediate paths
                        paths.push(index_path.clone());

                        // Recursively add deeper paths
                        if let Some(item_type_ref) = extract_type_ref(item_info) {
                            if !visited.contains(&item_type_ref)
                                && !is_terminal_type(&item_type_ref)
                            {
                                visited.insert(item_type_ref.clone());
                                if let Some(item_type_schema) = full_schema.get(&item_type_ref) {
                                    traverse_schema(
                                        index_path,
                                        item_type_schema,
                                        full_schema,
                                        paths,
                                        visited,
                                        depth + 1,
                                        max_depth,
                                    );
                                }
                                visited.remove(&item_type_ref);
                            }
                        }
                    }
                }
            }
            "Tuple" => {
                // Similar to TupleStruct
                if let Some(prefix_items) = type_schema.get("prefixItems").and_then(Value::as_array)
                {
                    for (index, item_info) in prefix_items.iter().enumerate() {
                        let index_path = if current_path.is_empty() {
                            format!(".{index}")
                        } else {
                            format!("{current_path}.{index}")
                        };

                        paths.push(index_path.clone());

                        if let Some(item_type_ref) = extract_type_ref(item_info) {
                            if !visited.contains(&item_type_ref)
                                && !is_terminal_type(&item_type_ref)
                            {
                                visited.insert(item_type_ref.clone());
                                if let Some(item_type_schema) = full_schema.get(&item_type_ref) {
                                    traverse_schema(
                                        index_path,
                                        item_type_schema,
                                        full_schema,
                                        paths,
                                        visited,
                                        depth + 1,
                                        max_depth,
                                    );
                                }
                                visited.remove(&item_type_ref);
                            }
                        }
                    }
                }
            }
            "Array" => {
                // For arrays, suggest index access
                let array_path = if current_path.is_empty() {
                    "[0]".to_string()
                } else {
                    format!("{current_path}[0]")
                };
                paths.push(array_path);
            }
            _ => {
                // Other types (Enum, Value) - handled differently or skipped
            }
        }
    }
}

/// Extract type reference from a field/item definition
fn extract_type_ref(field_info: &Value) -> Option<String> {
    if let Some(type_info) = field_info.get("type") {
        if let Some(ref_path) = type_info.get("$ref").and_then(Value::as_str) {
            // Extract type name from "$ref": "#/$defs/glam::Vec3"
            return ref_path.strip_prefix("#/$defs/").map(String::from);
        }
    }
    None
}

/// Check if a type is terminal (doesn't need further traversal)
fn is_terminal_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "f32"
            | "f64"
            | "i32"
            | "i64"
            | "u32"
            | "u64"
            | "bool"
            | "String"
            | "str"
            | "i8"
            | "i16"
            | "i128"
            | "u8"
            | "u16"
            | "u128"
            | "usize"
            | "isize"
            | "char"
    )
}

/// Extract crate name from a fully-qualified type name
fn extract_crate_name(type_name: &str) -> &str {
    type_name.split("::").next().unwrap_or(type_name)
}

/// Format path suggestions for display, limiting count and grouping by depth
fn format_path_suggestions(paths: &[String]) -> String {
    const MAX_SUGGESTIONS: usize = 12;

    if paths.len() <= MAX_SUGGESTIONS {
        paths.join(", ")
    } else {
        let mut result = paths
            .iter()
            .take(MAX_SUGGESTIONS)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        result.push_str(&format!(" (and {} more)", paths.len() - MAX_SUGGESTIONS));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_type_mismatch_error() {
        let cases = [
            (
                "Expected index access to access a tuple, found a struct instead.",
                true,
            ),
            (
                "Expected struct access to access a struct, found a tuple instead.",
                true,
            ),
            ("Some other error message", false),
            ("Expected array access but found string", false),
        ];

        for (message, expected) in cases {
            assert_eq!(
                is_type_mismatch_error(message),
                expected,
                "Failed for: {message}"
            );
        }
    }

    #[test]
    fn test_extract_crate_name() {
        assert_eq!(
            extract_crate_name("bevy_transform::components::transform::Transform"),
            "bevy_transform"
        );
        assert_eq!(extract_crate_name("simple_name"), "simple_name");
        assert_eq!(extract_crate_name(""), "");
    }

    #[test]
    fn test_is_terminal_type() {
        assert!(is_terminal_type("f32"));
        assert!(is_terminal_type("bool"));
        assert!(is_terminal_type("String"));
        assert!(!is_terminal_type(
            "bevy_transform::components::transform::Transform"
        ));
        assert!(!is_terminal_type("glam::Vec3"));
    }

    #[test]
    fn test_format_path_suggestions() {
        let short_paths = vec![".x".to_string(), ".y".to_string(), ".z".to_string()];
        assert_eq!(format_path_suggestions(&short_paths), ".x, .y, .z");

        let long_paths: Vec<String> = (0..20).map(|i| format!(".field{i}")).collect();
        let result = format_path_suggestions(&long_paths);
        assert!(result.contains("(and 8 more)"));
        assert!(result.len() < long_paths.iter().map(|s| s.len()).sum::<usize>());
    }
}
