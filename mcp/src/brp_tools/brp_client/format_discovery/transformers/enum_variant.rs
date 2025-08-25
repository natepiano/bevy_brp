//! Enum variant transformer for enum variant conversions and mismatches

use serde_json::{Value, json};

use super::super::detection::{self, ErrorPattern};
use super::super::engine::{TransformationResult, UnifiedTypeInfo};
use super::super::format_correction_fields::FormatCorrectionField;
use super::FormatTransformer;
use super::common::{extract_single_field_value, extract_type_name_from_error};
use crate::brp_tools::BrpClientError;

/// Transformer for enum variant patterns
/// Handles enum variant mismatches and conversions between different variant types
pub struct EnumVariantTransformer;

impl EnumVariantTransformer {
    /// Create a new enum variant transformer
    pub const fn new() -> Self {
        Self
    }

    /// Convert single-field object to value for enum variant access
    fn convert_object_to_variant_access(
        type_name: &str,
        obj: &serde_json::Map<String, Value>,
        context: &str,
    ) -> Option<TransformationResult> {
        extract_single_field_value(obj).map(|(field_name, value)| {
            let hint = format!(
                "`{type_name}` {context}: converted field '{field_name}' to variant access"
            );
            TransformationResult {
                corrected_value: value.clone(),
                hint,
            }
        })
    }

    /// Convert array to single element for variant access
    fn convert_array_to_variant_access(
        type_name: &str,
        arr: &[Value],
        context: &str,
    ) -> Option<TransformationResult> {
        arr.first().map(|element| {
            let hint = format!("`{type_name}` {context}: using first array element");
            TransformationResult {
                corrected_value: element.clone(),
                hint,
            }
        })
    }

    /// Try to extract enum variant value from object
    fn try_enum_variant_extraction(
        type_name: &str,
        field_name: &str,
        obj: &serde_json::Map<String, Value>,
    ) -> Option<TransformationResult> {
        // Try to find the variant field
        obj.get(field_name).map_or_else(
            || {
                // Fallback: try single field extraction
                extract_single_field_value(obj).map(|(actual_field, value)| {
                    let hint = format!(
                        "`{type_name}` MissingField '{field_name}': used field '{actual_field}' instead"
                    );
                    TransformationResult {
                        corrected_value: value.clone(),
                        hint,
                    }
                })
            },
            |variant_value| {
                let hint =
                    format!("`{type_name}` MissingField '{field_name}': extracted enum variant value");
                Some(TransformationResult {
                    corrected_value: variant_value.clone(),
                    hint,
                })
            },
        )
    }

    /// Handle type mismatch scenarios for enum variants
    fn handle_type_mismatch(
        type_name: &str,
        original_value: &Value,
        expected: &str,
        actual: &str,
        access: &str,
    ) -> Option<TransformationResult> {
        // Common type mismatches and their fixes
        match (expected, actual) {
            // Trying to access a struct field on a tuple struct
            ("struct", "tuple_struct") => {
                if let Value::Object(obj) = original_value {
                    let context =
                        format!("TypeMismatch: Expected {expected} access to access a {actual}");
                    return Self::convert_object_to_variant_access(type_name, obj, &context);
                }
            }
            // Trying to access a tuple index on a struct
            ("tuple_struct", "struct") => {
                if let Value::Array(arr) = original_value {
                    let context =
                        format!("TypeMismatch: Expected {expected} access to access a {actual}");
                    return Self::convert_array_to_variant_access(type_name, arr, &context);
                }
            }
            // Enum variant mismatches
            ("variant", "tuple_struct") | ("tuple_struct", "variant") => {
                // Try to convert between variant and tuple struct formats
                match original_value {
                    Value::Object(obj) => {
                        let context = format!(
                            "TypeMismatch: Expected {expected}, found {actual}, extracting inner value"
                        );
                        return Self::convert_object_to_variant_access(type_name, obj, &context);
                    }
                    Value::Array(arr) => {
                        let context = format!("TypeMismatch: Expected {expected}, found {actual}");
                        return Self::convert_array_to_variant_access(type_name, arr, &context);
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Use access type as additional context
        match access {
            "Field" | "FieldMut" => {
                // Field access mismatch, try extracting single field
                if let Value::Object(obj) = original_value {
                    let context = format!("TypeMismatch with {access} access");
                    if let Some((field_name, value)) = extract_single_field_value(obj) {
                        let hint = format!("`{type_name}` {context}: using field '{field_name}'");
                        return Some(TransformationResult {
                            corrected_value: value.clone(),
                            hint,
                        });
                    }
                }
            }
            "TupleIndex" => {
                // Tuple index access mismatch
                if let Value::Array(arr) = original_value {
                    let context = format!("TypeMismatch with {access} access");
                    return Self::convert_array_to_variant_access(type_name, arr, &context);
                }
            }
            _ => {}
        }
        None
    }

    /// Handle variant type mismatch scenarios
    fn handle_variant_type_mismatch(
        type_name: &str,
        original_value: &Value,
        expected: &str,
        actual: &str,
        access: &str,
    ) -> Option<TransformationResult> {
        // Common enum variant mismatches
        match (expected, actual) {
            // Tuple variant vs struct variant
            ("tuple", "struct") => {
                if let Value::Object(obj) = original_value
                    && let Some((variant_name, value)) = extract_single_field_value(obj)
                {
                    let hint = format!(
                        "`{type_name}` VariantTypeMismatch: Expected {expected} variant access to access a {actual} variant, \
                                    converted '{variant_name}' to tuple variant format"
                    );
                    return Some(TransformationResult {
                        corrected_value: value.clone(),
                        hint,
                    });
                }
            }
            // Struct variant vs tuple variant
            ("struct", "tuple") => {
                if let Value::Array(arr) = original_value {
                    let context = format!(
                        "VariantTypeMismatch: Expected {expected} variant access to access a {actual} variant, converted array to struct variant format"
                    );
                    return Self::convert_array_to_variant_access(type_name, arr, &context);
                }
            }
            _ => {}
        }

        // Use access type to determine conversion
        match access {
            "Field" | "FieldMut" => {
                // Field access on enum variant, likely needs tuple conversion
                if let Value::Object(obj) = original_value {
                    let context = format!(
                        "VariantTypeMismatch with {access} access: converted to variant element"
                    );
                    return Self::convert_object_to_variant_access(type_name, obj, &context);
                }
            }
            "TupleIndex" => {
                // Tuple index access on enum variant
                if let Value::Array(arr) = original_value {
                    let context =
                        format!("VariantTypeMismatch with {access} access: using variant element");
                    return Self::convert_array_to_variant_access(type_name, arr, &context);
                }
            }
            _ => {}
        }
        None
    }

    /// Enhanced handler for enum unit variant errors with type information
    fn handle_enum_unit_variant_error_with_type_info(
        type_name: &str,
        expected_variant_type: &str,
        actual_variant_type: &str,
        enum_info: &[crate::brp_tools::brp_type_schema::EnumVariantInfo],
    ) -> TransformationResult {
        // Use actual enum variants from type information
        let valid_values: Vec<String> = enum_info.iter().map(|v| v.variant_name.clone()).collect();

        // Return format correction that explains empty path usage
        let format_info = json!({
            FormatCorrectionField::Hint: "Use empty path with variant name as value",
            FormatCorrectionField::Path: "",
            FormatCorrectionField::ValidValues: valid_values,
            FormatCorrectionField::Examples: valid_values.iter().take(2).map(|v| json!({FormatCorrectionField::Path: "", FormatCorrectionField::Value: v})).collect::<Vec<_>>()
        });

        let hint = format!(
            "Enum '{type_name}' requires empty path for unit variant mutation. Expected {expected_variant_type} variant, found {actual_variant_type} variant. Valid variants: {}",
            valid_values.join(", ")
        );

        TransformationResult {
            corrected_value: format_info,
            hint,
        }
    }

    /// Handle missing field scenarios for enum variants
    fn handle_missing_field(
        type_name: &str,
        original_value: &Value,
        field_name: &str,
    ) -> Option<TransformationResult> {
        // Missing field errors often occur when:
        // 1. Trying to access a named field on a tuple struct
        // 2. Trying to access a field that doesn't exist
        // 3. Enum variant field access issues

        // Check if this is an enum variant field access issue
        if field_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        {
            // Likely an enum variant name like "LinearRgba"
            if let Value::Object(obj) = original_value
                && let Some(result) = Self::try_enum_variant_extraction(type_name, field_name, obj)
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

    /// Check if the error indicates enum variant issues
    fn is_enum_variant_error(error: &BrpClientError) -> bool {
        let message = &error.message;

        message.contains("variant")
            || message.contains("Variant")
            || message.contains("enum")
            || message.contains("Enum")
            || message.contains("VariantTypeMismatch")
    }

    /// Transform enum with discovered type information
    ///
    /// This method uses comprehensive enum information from direct discovery
    /// to provide more accurate variant transformations.
    fn transform_enum_with_discovered_info(
        value: &Value,
        error: &BrpClientError,
        type_name: &str,
        enum_info: &Vec<crate::brp_tools::brp_type_schema::EnumVariantInfo>,
    ) -> Option<TransformationResult> {
        // For now, fall back to basic pattern matching
        // This can be enhanced in the future to use the rich enum_info data
        // to provide more sophisticated variant transformations

        // Example of how enum_info could be used:
        // - Check available variants from enum_info
        // - Suggest closest matching variant names
        // - Use variant structure information for conversions

        // Check if we have variant names in enum_info
        let error_message = &error.message;
        for variant in enum_info {
            if error_message.contains(&variant.variant_name) {
                // Found a variant reference, could provide targeted transformation
                let hint = format!(
                    "Enum '{type_name}' variant '{}' transformation based on discovered schema",
                    variant.variant_name
                );
                // For now, return the original value with an informative hint
                // Real transformations would analyze the variant structure
                return Some(TransformationResult {
                    corrected_value: value.clone(),
                    hint,
                });
            }
        }

        None
    }
}

impl FormatTransformer for EnumVariantTransformer {
    fn can_handle(&self, error_pattern: &ErrorPattern) -> bool {
        match error_pattern {
            ErrorPattern::TypeMismatch { is_variant, .. } => *is_variant,
            ErrorPattern::MissingField { field_name, .. } => {
                // Can handle missing fields that look like enum variant names (start with
                // uppercase)
                field_name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
            }
            ErrorPattern::EnumUnitVariantMutation { .. }
            | ErrorPattern::EnumUnitVariantAccessError { .. } => true,
            _ => false,
        }
    }

    fn transform(&self, value: &Value) -> Option<TransformationResult> {
        // Generic enum variant transformation
        match value {
            Value::Object(obj) if obj.len() == 1 => {
                if let Some((field_name, field_value)) = obj.iter().next() {
                    Some(TransformationResult {
                        corrected_value: field_value.clone(),
                        hint:            format!(
                            "Converted enum variant field '{field_name}' to variant access"
                        ),
                    })
                } else {
                    None
                }
            }
            Value::Array(arr) if !arr.is_empty() => Some(TransformationResult {
                corrected_value: arr[0].clone(),
                hint:            "Using first array element for enum variant access".to_string(),
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

        // Analyze the error pattern
        let pattern = detection::analyze_error_pattern(error).pattern;

        // Handle specific error patterns
        match pattern {
            Some(
                ErrorPattern::EnumUnitVariantMutation {
                    expected_variant_type: _,
                    actual_variant_type: _,
                }
                | ErrorPattern::EnumUnitVariantAccessError {
                    access: _,
                    expected_variant_type: _,
                    actual_variant_type: _,
                },
            ) => {
                // This should only be called with registry data Return None to indicate this
                // transformer cannot handle the error without type info
                None
            }
            Some(ErrorPattern::TypeMismatch {
                expected,
                actual,
                access,
                is_variant,
            }) => {
                if is_variant {
                    Self::handle_variant_type_mismatch(
                        &type_name, value, &expected, &actual, &access,
                    )
                } else {
                    Self::handle_type_mismatch(&type_name, value, &expected, &actual, &access)
                }
            }
            Some(ErrorPattern::MissingField { field_name, .. }) => {
                Self::handle_missing_field(&type_name, value, &field_name)
            }
            _ => {
                // Check if this is still an enum variant related error
                if Self::is_enum_variant_error(error) {
                    // Fallback to generic transformation
                    self.transform(value)
                } else {
                    None
                }
            }
        }
    }

    fn transform_with_type_info(
        &self,
        value: &Value,
        error: &BrpClientError,
        type_info: &UnifiedTypeInfo,
    ) -> Option<TransformationResult> {
        // Extract type name from error for better messaging
        let type_name = type_info.type_name();

        // If type_info has enum information, use it for more accurate transformations
        if let Some(enum_info) = type_info.enum_info() {
            // Analyze the error pattern to check for enum unit variant errors
            let pattern = super::super::detection::analyze_error_pattern(error).pattern;

            // Handle enum unit variant errors with actual enum variants
            if let Some(
                ErrorPattern::EnumUnitVariantMutation {
                    expected_variant_type,
                    actual_variant_type,
                }
                | ErrorPattern::EnumUnitVariantAccessError {
                    access: _,
                    expected_variant_type,
                    actual_variant_type,
                },
            ) = pattern
            {
                return Some(Self::handle_enum_unit_variant_error_with_type_info(
                    type_name.as_str(),
                    &expected_variant_type,
                    &actual_variant_type,
                    enum_info,
                ));
            }

            // Use enum information from type discovery for other variant transformations
            if let Some(result) = Self::transform_enum_with_discovered_info(
                value,
                error,
                type_name.as_str(),
                enum_info,
            ) {
                return Some(result);
            }
        }

        // Fall back to basic transformation if no enum info available
        self.transform_with_error(value, error)
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "EnumVariantTransformer"
    }
}

impl Default for EnumVariantTransformer {
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
    fn test_can_handle_variant_type_mismatch() {
        let transformer = EnumVariantTransformer::new();
        let pattern = ErrorPattern::TypeMismatch {
            expected:   "tuple".to_string(),
            actual:     "struct".to_string(),
            access:     "Field".to_string(),
            is_variant: true,
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_can_handle_missing_field_uppercase() {
        let transformer = EnumVariantTransformer::new();
        let pattern = ErrorPattern::MissingField {
            field_name: "LinearRgba".to_string(),
            type_name:  "SomeType".to_string(),
        };
        assert!(transformer.can_handle(&pattern));
    }

    #[test]
    fn test_cannot_handle_missing_field_lowercase() {
        let transformer = EnumVariantTransformer::new();
        let pattern = ErrorPattern::MissingField {
            field_name: "x".to_string(),
            type_name:  "SomeType".to_string(),
        };
        assert!(!transformer.can_handle(&pattern));
    }

    #[test]
    fn test_cannot_handle_non_variant_type_mismatch() {
        let transformer = EnumVariantTransformer::new();
        let pattern = ErrorPattern::TypeMismatch {
            expected:   "tuple".to_string(),
            actual:     "struct".to_string(),
            access:     "Field".to_string(),
            is_variant: false,
        };
        assert!(!transformer.can_handle(&pattern));
    }

    #[test]
    fn test_cannot_handle_other_patterns() {
        let transformer = EnumVariantTransformer::new();
        let pattern = ErrorPattern::MathTypeArray {
            math_type: "Vec3".to_string(),
        };
        assert!(!transformer.can_handle(&pattern));
    }

    #[test]
    fn test_transform_single_field_object() {
        let transformer = EnumVariantTransformer::new();
        let value = json!({
            "LinearRgba": {
                "red": 1.0,
                "green": 0.5,
                "blue": 0.0,
                "alpha": 1.0
            }
        });

        let result = transformer.transform(&value);
        assert!(
            result.is_some(),
            "Expected transform to succeed for single field object"
        );
        let transformation_result = result.unwrap(); // Safe after assertion
        let expected = json!({
            "red": 1.0,
            "green": 0.5,
            "blue": 0.0,
            "alpha": 1.0
        });
        assert_eq!(transformation_result.corrected_value, expected);
    }

    #[test]
    fn test_transform_array() {
        let transformer = EnumVariantTransformer::new();
        let value = json!(["first", "second", "third"]);

        let result = transformer.transform(&value);
        assert!(result.is_some(), "Expected transform to succeed for array");
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(transformation_result.corrected_value, json!("first"));
    }

    #[test]
    fn test_transform_empty_array() {
        let transformer = EnumVariantTransformer::new();
        let value = json!([]);

        let result = transformer.transform(&value);
        assert!(result.is_none());
    }

    #[test]
    fn test_transform_multi_field_object() {
        let transformer = EnumVariantTransformer::new();
        let value = json!({
            "field1": "value1",
            "field2": "value2"
        });

        let result = transformer.transform(&value);
        assert!(result.is_none());
    }

    #[test]
    fn test_transformer_name() {
        let transformer = EnumVariantTransformer::new();
        assert_eq!(transformer.name(), "EnumVariantTransformer");
    }

    #[test]
    fn test_try_enum_variant_extraction() {
        let obj = json!({
            "LinearRgba": {
                "red": 1.0,
                "green": 0.5,
                "blue": 0.0,
                "alpha": 1.0
            }
        });

        assert!(obj.is_object(), "Expected object value");
        let map = obj.as_object().unwrap(); // Safe after assertion
        let result =
            EnumVariantTransformer::try_enum_variant_extraction("TestType", "LinearRgba", map);
        assert!(
            result.is_some(),
            "Expected enum variant extraction to succeed"
        );
        let transformation_result = result.unwrap(); // Safe after assertion
        let expected = json!({
            "red": 1.0,
            "green": 0.5,
            "blue": 0.0,
            "alpha": 1.0
        });
        assert_eq!(transformation_result.corrected_value, expected);
    }

    #[test]
    fn test_try_enum_variant_extraction_fallback() {
        let obj = json!({
            "SomeOtherField": "value"
        });

        assert!(obj.is_object(), "Expected object value");
        let map = obj.as_object().unwrap(); // Safe after assertion
        let result = EnumVariantTransformer::try_enum_variant_extraction(
            "TestType",
            "NonExistentField",
            map,
        );
        assert!(
            result.is_some(),
            "Expected fallback field extraction to succeed"
        );
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(transformation_result.corrected_value, json!("value"));
    }

    #[test]
    fn test_is_enum_variant_error() {
        let error1 = BrpClientError {
            code:    -1,
            message: "VariantTypeMismatch: expected tuple variant".to_string(),
            data:    None,
        };
        assert!(EnumVariantTransformer::is_enum_variant_error(&error1));

        let error2 = BrpClientError {
            code:    -1,
            message: "enum variant access error".to_string(),
            data:    None,
        };
        assert!(EnumVariantTransformer::is_enum_variant_error(&error2));

        let error3 = BrpClientError {
            code:    -1,
            message: "some other error".to_string(),
            data:    None,
        };
        assert!(!EnumVariantTransformer::is_enum_variant_error(&error3));
    }

    #[test]
    fn test_handle_variant_type_mismatch_tuple_to_struct() {
        let value = json!({
            "LinearRgba": [1.0, 0.5, 0.0, 1.0]
        });

        let result = EnumVariantTransformer::handle_variant_type_mismatch(
            "TestType", &value, "tuple", "struct", "Field",
        );
        assert!(
            result.is_some(),
            "Expected variant type mismatch handling to succeed"
        );
        let transformation_result = result.unwrap(); // Safe after assertion
        assert_eq!(
            transformation_result.corrected_value,
            json!([1.0, 0.5, 0.0, 1.0])
        );
    }

    #[test]
    fn test_handle_missing_field_enum_variant() {
        let value = json!({
            "LinearRgba": {
                "red": 1.0,
                "green": 0.5,
                "blue": 0.0,
                "alpha": 1.0
            }
        });

        let result = EnumVariantTransformer::handle_missing_field("TestType", &value, "LinearRgba");
        assert!(
            result.is_some(),
            "Expected missing field handling to succeed"
        );
        let transformation_result = result.unwrap(); // Safe after assertion
        let expected = json!({
            "red": 1.0,
            "green": 0.5,
            "blue": 0.0,
            "alpha": 1.0
        });
        assert_eq!(transformation_result.corrected_value, expected);
        assert!(
            transformation_result
                .hint
                .contains("extracted enum variant value")
        );
    }
}
