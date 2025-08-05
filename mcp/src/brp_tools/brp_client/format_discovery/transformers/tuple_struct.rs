//! Tuple struct transformer for tuple struct access patterns

use serde_json::Value;

use super::super::detection::ErrorPattern;
use super::super::engine::TransformationResult;
use super::super::field_mapper;
use super::super::types::{ComponentType, FieldAccess};
use super::FormatTransformer;
use super::common::{extract_single_field_value, extract_type_name_from_error};
use crate::brp_tools::BrpClientError;

/// Parses a path string like ".LinearRgba.red" into a `FieldAccess` struct
fn parse_path_to_field_access(path: &str) -> Option<FieldAccess> {
    // Simple field access (no component type) should be handled by the fallback logic
    // These remain as direct tuple indices (.0, .1, .2)
    if path.starts_with('.') && path.matches('.').count() == 1 {
        return None; // Let the fallback handle these
    }

    // Split the path into parts
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() < 3 || !parts[0].is_empty() {
        return None;
    }

    // Extract component type and field name
    let component_name = parts[1];
    let field_name = parts[2];

    // Parse component type
    let component_type = parse_component_type(component_name)?;

    // Parse field name
    let field = field_mapper::parse_field_name(field_name, component_type)?;

    Some(FieldAccess {
        component_type,
        field,
    })
}

/// Parses a component type name string into a `ComponentType` enum
fn parse_component_type(component_name: &str) -> Option<ComponentType> {
    match component_name {
        // Color types
        "LinearRgba" => Some(ComponentType::LinearRgba),
        "Srgba" => Some(ComponentType::Srgba),
        "Hsla" => Some(ComponentType::Hsla),
        "Hsva" => Some(ComponentType::Hsva),
        "Hwba" => Some(ComponentType::Hwba),
        "Laba" => Some(ComponentType::Laba),
        "Lcha" => Some(ComponentType::Lcha),
        "Oklaba" => Some(ComponentType::Oklaba),
        "Oklcha" => Some(ComponentType::Oklcha),
        "Xyza" => Some(ComponentType::Xyza),

        // Math types - floating point
        "Vec2" => Some(ComponentType::Vec2),
        "Vec3" => Some(ComponentType::Vec3),
        "Vec4" => Some(ComponentType::Vec4),
        "Quat" => Some(ComponentType::Quat),

        // Math types - signed integers
        "IVec2" => Some(ComponentType::IVec2),
        "IVec3" => Some(ComponentType::IVec3),
        "IVec4" => Some(ComponentType::IVec4),

        // Math types - unsigned integers
        "UVec2" => Some(ComponentType::UVec2),
        "UVec3" => Some(ComponentType::UVec3),
        "UVec4" => Some(ComponentType::UVec4),

        // Math types - double precision
        "DVec2" => Some(ComponentType::DVec2),
        "DVec3" => Some(ComponentType::DVec3),
        "DVec4" => Some(ComponentType::DVec4),

        _ => None,
    }
}

/// Checks if a variant name looks like an enum variant (starts with uppercase)
fn is_enum_variant(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Parses generic enum variant field access patterns
/// Handles cases where we don't have a specific component type mapping
fn parse_generic_enum_field_access(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() < 3 || !parts[0].is_empty() || parts[1].is_empty() || parts[2].is_empty() {
        return None;
    }

    let variant_name = parts[1];
    let field_name = parts[2];

    // Check if the second part looks like an enum variant (starts with uppercase)
    if !is_enum_variant(variant_name) {
        return None;
    }

    // For color enum variants, try to map common field names to indices
    match field_name {
        // Index 0: First position fields
        "red" | "r" | "hue" | "h" | "lightness" | "l" | "x" => Some(".0.0".to_string()),
        // Index 1: Second position fields (including special cases)
        "green" | "g" | "saturation" | "s" | "y" | "whiteness" | "chroma" | "c" => {
            Some(".0.1".to_string())
        }
        // Index 2: Third position fields
        "blue" | "b" | "value" | "v" | "z" | "blackness" => Some(".0.2".to_string()),
        // Index 3: Fourth position fields
        "alpha" | "w" => Some(".0.3".to_string()),
        // Special case for 'a' - could be alpha or Lab 'a' component
        "a" => {
            if variant_name.contains("Lab") {
                Some(".0.1".to_string()) // Lab 'a' component
            } else {
                Some(".0.3".to_string()) // Alpha component
            }
        }
        _ => {
            // Generic enum variant field access -> use tuple index 0 and preserve field path
            if parts.len() > 3 {
                let remaining = parts[2..].join(".");
                Some(format!(".0.{remaining}"))
            } else {
                Some(format!(".0.{field_name}"))
            }
        }
    }
}

/// Transformer for tuple struct access patterns
/// Handles field access to tuple index conversions and path corrections
pub struct TupleStructTransformer;

impl TupleStructTransformer {
    /// Create a new tuple struct transformer
    pub const fn new() -> Self {
        Self
    }

    /// Helper function to fix tuple struct paths for all enum tuple variants
    /// Uses the new type-safe system for better maintainability and correctness
    pub fn fix_tuple_struct_path(path: &str) -> String {
        // First, try the type-safe approach using our new parsing system
        if let Some(field_access) = parse_path_to_field_access(path) {
            return field_mapper::map_field_to_tuple_index(&field_access);
        }

        // Fallback: handle simple field access patterns
        match path {
            // Simple tuple struct field access (not nested) - these remain direct indices
            ".x" => ".0".to_string(),
            ".y" => ".1".to_string(),
            ".z" => ".2".to_string(),

            // Generic patterns for unknown enum variants
            _ => {
                // Try generic enum field access parsing as fallback
                if let Some(fixed_path) = parse_generic_enum_field_access(path) {
                    return fixed_path;
                }

                // Ultimate fallback: return original path
                path.to_string()
            }
        }
    }

    /// Fix tuple struct path access errors
    fn fix_tuple_struct_format(
        type_name: &str,
        original_value: &Value,
        field_path: &str,
    ) -> Option<TransformationResult> {
        // Tuple structs use numeric indices like .0, .1, etc.
        // If the error mentions a field path, it might be trying to access
        // a field using the wrong syntax

        // Common patterns:
        // - Trying to access .value on a tuple struct that should be .0
        // - Trying to use named fields on a tuple struct
        // - Enum tuple variants like LinearRgba with color field names

        // Apply enum-specific path fixes
        let fixed_path = Self::fix_tuple_struct_path(field_path);

        match original_value {
            Value::Object(obj) => {
                // If we have an object with a single field, try converting to tuple access
                if obj.len() == 1 {
                    if let Some((_, value)) = obj.iter().next() {
                        return Some(TransformationResult {
                            corrected_value: value.clone(),
                            hint:            format!(
                                "`{type_name}` is a tuple struct, use numeric indices like .0 instead of named fields"
                            ),
                        });
                    }
                }
            }
            Value::Array(arr) => {
                // If we have an array and the path suggests index access, extract the element
                // Use the fixed path which may have been transformed from enum variant field names
                if let Ok(index) = fixed_path.trim_start_matches('.').parse::<usize>() {
                    if let Some(element) = arr.get(index) {
                        let hint = if fixed_path == field_path {
                            format!("`{type_name}` tuple struct element at index {index} extracted")
                        } else {
                            format!(
                                "`{type_name}` tuple struct: converted '{field_path}' to '{fixed_path}' for element access"
                            )
                        };
                        return Some(TransformationResult {
                            corrected_value: element.clone(),
                            hint,
                        });
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Convert single-field object to value for tuple struct access
    fn convert_object_to_tuple_access(
        type_name: &str,
        obj: &serde_json::Map<String, Value>,
        context: &str,
    ) -> Option<TransformationResult> {
        extract_single_field_value(obj).map(|(field_name, value)| {
            let hint =
                format!("`{type_name}` {context}: converted field '{field_name}' to tuple access");
            TransformationResult {
                corrected_value: value.clone(),
                hint,
            }
        })
    }

    /// Try to convert field name to tuple index and extract element from array
    fn try_tuple_struct_field_access(
        type_name: &str,
        field_name: &str,
        original_value: &Value,
    ) -> Option<TransformationResult> {
        let fixed_path = Self::fix_tuple_struct_path(&format!(".{field_name}"));
        if fixed_path != format!(".{field_name}") {
            // The path was transformed, so it's likely a tuple struct
            match original_value {
                Value::Array(arr) => {
                    // Extract the correct index from the fixed path
                    if let Some(index_str) = fixed_path.strip_prefix('.') {
                        if let Ok(index) = index_str.parse::<usize>() {
                            if let Some(element) = arr.get(index) {
                                let hint = format!(
                                    "`{type_name}` MissingField '{field_name}': converted to tuple struct index {index}"
                                );
                                return Some(TransformationResult {
                                    corrected_value: element.clone(),
                                    hint,
                                });
                            }
                        }
                    }
                }
                Value::Object(obj) => {
                    let context = format!(
                        "MissingField '{field_name}': converted object to tuple struct access"
                    );
                    return Self::convert_object_to_tuple_access(type_name, obj, &context);
                }
                _ => {}
            }
        }
        None
    }

    /// Handle missing field scenarios for tuple structs
    fn handle_missing_field(
        type_name: &str,
        original_value: &Value,
        field_name: &str,
    ) -> Option<TransformationResult> {
        // Missing field errors often occur when:
        // 1. Trying to access a named field on a tuple struct
        // 2. Trying to access a field that doesn't exist
        // 3. Enum variant field access issues

        // Check if this is a tuple struct access issue
        if field_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
        {
            // Likely a field name like "red", "x", "y", etc.
            if let Some(result) =
                Self::try_tuple_struct_field_access(type_name, field_name, original_value)
            {
                return Some(result);
            }
        }

        // Generic fallback: try to extract any reasonable value
        match original_value {
            Value::Object(obj) => {
                if let Some((actual_field, value)) = extract_single_field_value(obj) {
                    let hint = format!(
                        "`{type_name}` MissingField '{field_name}': used available field '{actual_field}'"
                    );
                    return Some(TransformationResult {
                        corrected_value: value.clone(),
                        hint,
                    });
                }
            }
            Value::Array(arr) => {
                if let Some(element) = arr.first() {
                    let hint = format!(
                        "`{type_name}` MissingField '{field_name}': using first array element"
                    );
                    return Some(TransformationResult {
                        corrected_value: element.clone(),
                        hint,
                    });
                }
            }
            _ => {}
        }
        None
    }

    /// Check if the error indicates tuple struct access issues
    fn is_tuple_struct_error(error: &BrpClientError) -> bool {
        let message = &error.message;

        message.contains("tuple struct")
            || message.contains("tuple_struct")
            || message.contains("TupleIndex")
            || message.contains("found a tuple struct instead")
            || message.contains("AccessError")
    }
}

impl FormatTransformer for TupleStructTransformer {
    fn can_handle(&self, error_pattern: &ErrorPattern) -> bool {
        matches!(
            error_pattern,
            ErrorPattern::TupleStructAccess { .. }
                | ErrorPattern::AccessError { .. }
                | ErrorPattern::MissingField { .. }
        )
    }

    fn transform(&self, value: &Value) -> Option<TransformationResult> {
        // Generic tuple struct transformation
        match value {
            Value::Object(obj) if obj.len() == 1 => {
                if let Some((field_name, field_value)) = obj.iter().next() {
                    Some(TransformationResult {
                        corrected_value: field_value.clone(),
                        hint:            format!(
                            "Converted field '{field_name}' to tuple struct access"
                        ),
                    })
                } else {
                    None
                }
            }
            Value::Array(arr) if !arr.is_empty() => Some(TransformationResult {
                corrected_value: arr[0].clone(),
                hint:            "Using first array element for tuple struct access".to_string(),
            }),
            _ => None,
        }
    }

    fn transform_with_error(
        &self,
        value: &Value,
        error: &BrpClientError,
    ) -> Option<TransformationResult> {
        // Extract type name from error for better messaging
        let type_name =
            extract_type_name_from_error(error).unwrap_or_else(|| "unknown".to_string());

        // Check if this is a tuple struct related error
        if Self::is_tuple_struct_error(error) {
            // Try to extract path information and fix it
            let message = &error.message;

            // Look for path patterns in the message
            if let Some(path_start) = message.find("path ") {
                if let Some(path_quote_start) = message[path_start..].find('`') {
                    let search_start = path_start + path_quote_start + 1;
                    if let Some(path_quote_end) = message[search_start..].find('`') {
                        let path = &message[search_start..search_start + path_quote_end];
                        return Self::fix_tuple_struct_format(&type_name, value, path);
                    }
                }
            }

            // Look for field names in the message
            if message.contains("MissingField") {
                // Extract field name (this is a simple heuristic)
                if let Some(field_start) = message.find('\'') {
                    if let Some(field_end) = message[field_start + 1..].find('\'') {
                        let field_name = &message[field_start + 1..field_start + 1 + field_end];
                        return Self::handle_missing_field(&type_name, value, field_name);
                    }
                }
            }
        }

        // Fallback to generic transformation
        self.transform(value)
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "TupleStructTransformer"
    }
}

impl Default for TupleStructTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use serde_json::json;

    use super::*;

    #[test]
    fn test_can_handle_tuple_struct_access() {
        let transformer = TupleStructTransformer::new();
        let pattern = ErrorPattern::TupleStructAccess {
            field_path: ".x".to_string(),
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_can_handle_access_error() {
        let transformer = TupleStructTransformer::new();
        let pattern = ErrorPattern::AccessError {
            access:     "Field".to_string(),
            error_type: "some error".to_string(),
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_can_handle_missing_field() {
        let transformer = TupleStructTransformer::new();
        let pattern = ErrorPattern::MissingField {
            field_name: "x".to_string(),
            type_name:  "SomeType".to_string(),
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_cannot_handle_other_patterns() {
        let transformer = TupleStructTransformer::new();
        let pattern = ErrorPattern::MathTypeArray {
            math_type: "Vec3".to_string(),
        };
        assert!(!transformer.can_handle(&pattern));
    }

    #[test]
    fn test_fix_tuple_struct_path_simple() {
        assert_eq!(TupleStructTransformer::fix_tuple_struct_path(".x"), ".0");
        assert_eq!(TupleStructTransformer::fix_tuple_struct_path(".y"), ".1");
        assert_eq!(TupleStructTransformer::fix_tuple_struct_path(".z"), ".2");
    }

    #[test]
    fn test_fix_tuple_struct_path_unchanged() {
        // Unknown paths should remain unchanged
        assert_eq!(
            TupleStructTransformer::fix_tuple_struct_path(".unknown"),
            ".unknown"
        );
        assert_eq!(TupleStructTransformer::fix_tuple_struct_path(".0"), ".0");
    }

    #[test]
    fn test_transform_single_field_object() {
        let transformer = TupleStructTransformer::new();
        let value = json!({
            "field": "value"
        });

        let result = transformer.transform(&value);
        assert!(result.is_some(), "Failed to transform single field object");
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(transformation_result.corrected_value, json!("value"));
        assert!(
            transformation_result
                .hint
                .contains("Converted field 'field' to tuple struct access")
        );
    }

    #[test]
    fn test_transform_array() {
        let transformer = TupleStructTransformer::new();
        let value = json!(["first", "second", "third"]);

        let result = transformer.transform(&value);
        assert!(result.is_some(), "Failed to transform array");
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(transformation_result.corrected_value, json!("first"));
        assert!(
            transformation_result
                .hint
                .contains("Using first array element")
        );
    }

    #[test]
    fn test_transform_empty_array() {
        let transformer = TupleStructTransformer::new();
        let value = json!([]);

        let result = transformer.transform(&value);
        assert!(result.is_none());
    }

    #[test]
    fn test_transform_multi_field_object() {
        let transformer = TupleStructTransformer::new();
        let value = json!({
            "field1": "value1",
            "field2": "value2"
        });

        let result = transformer.transform(&value);
        assert!(result.is_none());
    }

    #[test]
    fn test_transformer_name() {
        let transformer = TupleStructTransformer::new();
        assert_eq!(transformer.name(), "TupleStructTransformer");
    }

    #[test]
    fn test_fix_tuple_struct_format_object() {
        let value = json!({
            "x": 1.0
        });

        let result = TupleStructTransformer::fix_tuple_struct_format("TestType", &value, ".x");
        assert!(
            result.is_some(),
            "Failed to fix tuple struct format for object"
        );
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(transformation_result.corrected_value, json!(1.0));
        assert!(transformation_result.hint.contains("TestType"));
        assert!(transformation_result.hint.contains("tuple struct"));
    }

    #[test]
    fn test_fix_tuple_struct_format_array() {
        let value = json!([1.0, 2.0, 3.0]);

        let result = TupleStructTransformer::fix_tuple_struct_format("TestType", &value, ".x");
        assert!(
            result.is_some(),
            "Failed to fix tuple struct format for array"
        );
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(transformation_result.corrected_value, json!(1.0)); // .x should map to .0, which is index 0
        assert!(transformation_result.hint.contains("TestType"));
        assert!(transformation_result.hint.contains("tuple struct"));
    }

    #[test]
    fn test_is_tuple_struct_error() {
        let _transformer = TupleStructTransformer::new();

        let error1 = BrpClientError {
            code:    -1,
            message: "tuple struct access error".to_string(),
            data:    None,
        };
        assert!(TupleStructTransformer::is_tuple_struct_error(&error1));

        let error2 = BrpClientError {
            code:    -1,
            message: "AccessError: Field not found".to_string(),
            data:    None,
        };
        assert!(TupleStructTransformer::is_tuple_struct_error(&error2));

        let error3 = BrpClientError {
            code:    -1,
            message: "some other error".to_string(),
            data:    None,
        };
        assert!(!TupleStructTransformer::is_tuple_struct_error(&error3));
    }

    // Tests for the moved path parser functions
    mod path_parser_tests {
        use super::*;
        use crate::brp_tools::brp_client::format_discovery::types::{ColorField, Field, MathField};

        #[test]
        fn test_parse_path_to_field_access() {
            // Test color path parsing
            let field_access = parse_path_to_field_access(".LinearRgba.red").unwrap();
            assert_eq!(field_access.component_type, ComponentType::LinearRgba);
            assert_eq!(field_access.field, Field::Color(ColorField::Red));

            let field_access = parse_path_to_field_access(".Hsla.saturation").unwrap();
            assert_eq!(field_access.component_type, ComponentType::Hsla);
            assert_eq!(field_access.field, Field::Color(ColorField::Saturation));

            // Test math path parsing
            let field_access = parse_path_to_field_access(".Vec3.x").unwrap();
            assert_eq!(field_access.component_type, ComponentType::Vec3);
            assert_eq!(field_access.field, Field::Math(MathField::X));

            // Test Lab 'a' disambiguation
            let field_access = parse_path_to_field_access(".Laba.a").unwrap();
            assert_eq!(field_access.component_type, ComponentType::Laba);
            assert_eq!(field_access.field, Field::Color(ColorField::A));

            let field_access = parse_path_to_field_access(".LinearRgba.a").unwrap();
            assert_eq!(field_access.component_type, ComponentType::LinearRgba);
            assert_eq!(field_access.field, Field::Color(ColorField::Alpha));
        }

        #[test]
        fn test_parse_component_type() {
            assert_eq!(
                parse_component_type("LinearRgba"),
                Some(ComponentType::LinearRgba)
            );
            assert_eq!(parse_component_type("Vec3"), Some(ComponentType::Vec3));
            assert_eq!(parse_component_type("Quat"), Some(ComponentType::Quat));
            assert_eq!(parse_component_type("InvalidType"), None);
        }

        #[test]
        fn test_is_enum_variant() {
            assert!(is_enum_variant("LinearRgba"));
            assert!(is_enum_variant("SomeVariant"));
            assert!(!is_enum_variant("lowercase"));
            assert!(!is_enum_variant(""));
        }

        #[test]
        fn test_parse_generic_enum_field_access() {
            // Test standard color field mappings
            assert_eq!(
                parse_generic_enum_field_access(".LinearRgba.red"),
                Some(".0.0".to_string())
            );
            assert_eq!(
                parse_generic_enum_field_access(".SomeColor.green"),
                Some(".0.1".to_string())
            );
            assert_eq!(
                parse_generic_enum_field_access(".AnyColor.alpha"),
                Some(".0.3".to_string())
            );

            // Test Lab 'a' disambiguation
            assert_eq!(
                parse_generic_enum_field_access(".SomeLabColor.a"),
                Some(".0.1".to_string())
            );
            assert_eq!(
                parse_generic_enum_field_access(".RegularColor.a"),
                Some(".0.3".to_string())
            );

            // Test generic field access
            assert_eq!(
                parse_generic_enum_field_access(".SomeEnum.custom_field"),
                Some(".0.custom_field".to_string())
            );

            // Test invalid paths
            assert_eq!(parse_generic_enum_field_access(".lowercase.field"), None);
            assert_eq!(parse_generic_enum_field_access(".SomeEnum"), None);
            assert_eq!(parse_generic_enum_field_access("no_dot_prefix"), None);
        }

        #[test]
        fn test_simple_field_paths() {
            use crate::brp_tools::brp_client::format_discovery::field_mapper::map_field_to_tuple_index;

            // Test simple field paths return None (handled by fallback)
            assert_eq!(parse_path_to_field_access(".x"), None);
            assert_eq!(parse_path_to_field_access(".y"), None);
            assert_eq!(parse_path_to_field_access(".z"), None);

            // Test the mapping result for compound paths
            let field_access = parse_path_to_field_access(".Vec3.x").unwrap();
            assert_eq!(map_field_to_tuple_index(&field_access), ".0.0");
        }
    }
}
