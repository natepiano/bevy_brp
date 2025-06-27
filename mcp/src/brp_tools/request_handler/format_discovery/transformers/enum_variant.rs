//! Enum variant transformer for enum variant conversions and mismatches

use serde_json::Value;

use super::super::detection::ErrorPattern;
use super::FormatTransformer;
use super::common::{extract_single_field_value, extract_type_name_from_error};
use crate::brp_tools::support::brp_client::BrpError;

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
    ) -> Option<(Value, String)> {
        extract_single_field_value(obj).map(|(field_name, value)| {
            let hint = format!(
                "`{type_name}` {context}: converted field '{field_name}' to variant access"
            );
            (value.clone(), hint)
        })
    }

    /// Convert array to single element for variant access
    fn convert_array_to_variant_access(
        type_name: &str,
        arr: &[Value],
        context: &str,
    ) -> Option<(Value, String)> {
        arr.first().map(|element| {
            let hint = format!("`{type_name}` {context}: using first array element");
            (element.clone(), hint)
        })
    }

    /// Try to extract enum variant value from object
    fn try_enum_variant_extraction(
        type_name: &str,
        field_name: &str,
        obj: &serde_json::Map<String, Value>,
    ) -> Option<(Value, String)> {
        // Try to find the variant field
        obj.get(field_name).map_or_else(
            || {
                // Fallback: try single field extraction
                extract_single_field_value(obj).map(|(actual_field, value)| {
                    let hint = format!(
                        "`{type_name}` MissingField '{field_name}': used field '{actual_field}' instead"
                    );
                    (value.clone(), hint)
                })
            },
            |variant_value| {
                let hint =
                    format!("`{type_name}` MissingField '{field_name}': extracted enum variant value");
                Some((variant_value.clone(), hint))
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
    ) -> Option<(Value, String)> {
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
                        return Some((value.clone(), hint));
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
    ) -> Option<(Value, String)> {
        // Common enum variant mismatches
        match (expected, actual) {
            // Tuple variant vs struct variant
            ("tuple", "struct") => {
                if let Value::Object(obj) = original_value {
                    if let Some((variant_name, value)) = extract_single_field_value(obj) {
                        let hint = format!(
                            "`{type_name}` VariantTypeMismatch: Expected {expected} variant access to access a {actual} variant, \
                                        converted '{variant_name}' to tuple variant format"
                        );
                        return Some((value.clone(), hint));
                    }
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

    /// Handle missing field scenarios for enum variants
    fn handle_missing_field(
        type_name: &str,
        original_value: &Value,
        field_name: &str,
    ) -> Option<(Value, String)> {
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
            if let Value::Object(obj) = original_value {
                if let Some(result) = Self::try_enum_variant_extraction(type_name, field_name, obj)
                {
                    return Some(result);
                }
            }
        }

        // Generic fallback: try to extract any reasonable value
        match original_value {
            Value::Object(obj) => {
                if let Some((actual_field, value)) = extract_single_field_value(obj) {
                    let hint = format!(
                        "`{type_name}` MissingField '{field_name}': used available field '{actual_field}'"
                    );
                    return Some((value.clone(), hint));
                }
            }
            Value::Array(arr) => {
                if let Some(element) = arr.first() {
                    let hint = format!(
                        "`{type_name}` MissingField '{field_name}': using first array element"
                    );
                    return Some((element.clone(), hint));
                }
            }
            _ => {}
        }
        None
    }

    /// Check if the error indicates enum variant issues
    fn is_enum_variant_error(error: &BrpError) -> bool {
        let message = &error.message;

        message.contains("variant")
            || message.contains("Variant")
            || message.contains("enum")
            || message.contains("Enum")
            || message.contains("VariantTypeMismatch")
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
            _ => false,
        }
    }

    fn transform(&self, value: &Value) -> Option<(Value, String)> {
        // Generic enum variant transformation
        match value {
            Value::Object(obj) if obj.len() == 1 => {
                if let Some((field_name, field_value)) = obj.iter().next() {
                    Some((
                        field_value.clone(),
                        format!("Converted enum variant field '{field_name}' to variant access"),
                    ))
                } else {
                    None
                }
            }
            Value::Array(arr) if !arr.is_empty() => Some((
                arr[0].clone(),
                "Using first array element for enum variant access".to_string(),
            )),
            _ => None,
        }
    }

    fn transform_with_error(&self, value: &Value, error: &BrpError) -> Option<(Value, String)> {
        // Extract type name from error for better messaging
        let type_name =
            extract_type_name_from_error(error).unwrap_or_else(|| "unknown".to_string());

        // Check if this is an enum variant related error
        if Self::is_enum_variant_error(error) {
            let message = &error.message;

            // Look for specific variant type mismatch patterns
            if message.contains("VariantTypeMismatch") {
                // Try to extract expected and actual types
                // This is a simple heuristic - in a real implementation, you might want more
                // sophisticated parsing
                if message.contains("tuple") && message.contains("struct") {
                    if message.contains("Expected tuple") {
                        return Self::handle_variant_type_mismatch(
                            &type_name, value, "tuple", "struct", "Field",
                        );
                    } else if message.contains("Expected struct") {
                        return Self::handle_variant_type_mismatch(
                            &type_name,
                            value,
                            "struct",
                            "tuple",
                            "TupleIndex",
                        );
                    }
                }
            }

            // Look for type mismatch patterns
            if message.contains("TypeMismatch")
                && message.contains("variant")
                && message.contains("tuple_struct")
            {
                if message.contains("Expected variant") {
                    return Self::handle_type_mismatch(
                        &type_name,
                        value,
                        "variant",
                        "tuple_struct",
                        "Field",
                    );
                } else if message.contains("Expected tuple_struct") {
                    return Self::handle_type_mismatch(
                        &type_name,
                        value,
                        "tuple_struct",
                        "variant",
                        "TupleIndex",
                    );
                }
            }

            // Look for missing field patterns
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
        let (converted, hint) = result.unwrap(); // Safe after assertion
        let expected = json!({
            "red": 1.0,
            "green": 0.5,
            "blue": 0.0,
            "alpha": 1.0
        });
        assert_eq!(converted, expected);
        assert!(hint.contains("Converted enum variant field 'LinearRgba'"));
    }

    #[test]
    fn test_transform_array() {
        let transformer = EnumVariantTransformer::new();
        let value = json!(["first", "second", "third"]);

        let result = transformer.transform(&value);
        assert!(result.is_some(), "Expected transform to succeed for array");
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!("first"));
        assert!(hint.contains("Using first array element"));
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
        let (converted, hint) = result.unwrap(); // Safe after assertion
        let expected = json!({
            "red": 1.0,
            "green": 0.5,
            "blue": 0.0,
            "alpha": 1.0
        });
        assert_eq!(converted, expected);
        assert!(hint.contains("TestType"));
        assert!(hint.contains("extracted enum variant value"));
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
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!("value"));
        assert!(hint.contains("used field 'SomeOtherField' instead"));
    }

    #[test]
    fn test_is_enum_variant_error() {
        let error1 = BrpError {
            code:    -1,
            message: "VariantTypeMismatch: expected tuple variant".to_string(),
            data:    None,
        };
        assert!(EnumVariantTransformer::is_enum_variant_error(&error1));

        let error2 = BrpError {
            code:    -1,
            message: "enum variant access error".to_string(),
            data:    None,
        };
        assert!(EnumVariantTransformer::is_enum_variant_error(&error2));

        let error3 = BrpError {
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
        let (converted, hint) = result.unwrap(); // Safe after assertion
        assert_eq!(converted, json!([1.0, 0.5, 0.0, 1.0]));
        assert!(hint.contains("VariantTypeMismatch"));
        assert!(hint.contains("tuple variant format"));
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
        let (converted, hint) = result.unwrap(); // Safe after assertion
        let expected = json!({
            "red": 1.0,
            "green": 0.5,
            "blue": 0.0,
            "alpha": 1.0
        });
        assert_eq!(converted, expected);
        assert!(hint.contains("extracted enum variant value"));
    }
}
