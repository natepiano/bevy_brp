//! Field extraction functions for response formatting.
//!
//! This module provides the bridge between `ExtractorType` enum variants
//! and the actual field extraction functions used by the response formatter.

use serde_json::Value;

use crate::response::{FieldPlacement, ResponseExtractorType, ResponseField};
use crate::tool::HandlerContext;

/// Trait for contexts that can provide request parameters
pub trait RequestParameterProvider {
    fn get_request_parameter(&self, field: &str) -> Option<&Value>;
}

/// Implementation for `HandlerContext`
impl<Port, Method> RequestParameterProvider for HandlerContext<Port, Method> {
    fn get_request_parameter(&self, field: &str) -> Option<&Value> {
        self.request.arguments.as_ref()?.get(field)
    }
}

/// Function type for extracting field values from response data and context.
///
/// Takes:
/// - `&Value` - The response data (usually from BRP)
/// - `&dyn RequestParameterProvider` - Context that can provide request parameters
///
/// Returns: `Value` - The extracted field value
pub type FieldExtractor = Box<dyn Fn(&Value, &dyn RequestParameterProvider) -> Value + Send + Sync>;

/// Create a field accessor for already-extracted request parameters.
///
/// `FieldExtractor` defines a closure that will allow us to specify the field name
/// (captured in the closure) that we want to use to access arguments passed in
/// on the initial tool call.
///
/// Or put another way...
///
/// This function creates extractors that reference fields from the `ExtractedParams`,
/// which were already validated during the parameter extraction phase.
pub fn create_request_field_extractor(field: &'static str) -> FieldExtractor {
    Box::new(move |_data, context| {
        context
            .get_request_parameter(field)
            .cloned()
            .unwrap_or(Value::Null)
    })
}

/// Convert a `ResponseField` specification to a field extractor function with placement info.
///
/// This creates the actual closure that will extract data based on the
/// extraction strategy defined by the `ResponseField` variant, and returns
/// the placement information for where the field should be put in the response.
#[allow(clippy::too_many_lines)]
pub fn create_response_field_extractor(field: &ResponseField) -> (FieldExtractor, FieldPlacement) {
    match field {
        ResponseField::FromRequest {
            parameter_field_name: field,
            placement,
            ..
        } => (create_request_field_extractor(field), placement.clone()),
        ResponseField::FromResponse {
            response_extractor: extractor,
            placement,
            ..
        } => {
            let extractor = extractor.clone();
            (
                Box::new(move |data, _context| extractor.extract(data)),
                placement.clone(),
            )
        }
        ResponseField::DirectToMetadata => (
            Box::new(|data, _context| data.clone()),
            FieldPlacement::Metadata,
        ),
        ResponseField::FromResponseNullableWithPlacement {
            response_extractor: extractor,
            placement,
            ..
        } => {
            let extractor = extractor.clone();
            let placement = placement.clone();
            (
                Box::new(move |data, _context| {
                    // Return a special marker for null values that the formatter can detect
                    let value = extractor.extract(data);
                    if value.is_null() {
                        serde_json::Value::String("__SKIP_NULL_FIELD__".to_string())
                    } else {
                        value
                    }
                }),
                placement,
            )
        }
        ResponseField::BrpRawResultToResult => (
            Box::new(|data, _context| {
                // Extract raw BRP response data from "result" field for V2 handlers
                ResponseExtractorType::Field("result").extract(data)
            }),
            FieldPlacement::Result,
        ),
        ResponseField::FormatCorrection => (
            Box::new(|data, _context| extract_format_correction_fields(data)),
            FieldPlacement::Metadata,
        ),
    }
}

/// Extract all format correction related fields from V2 `BrpMethodResult`
fn extract_format_correction_fields(data: &Value) -> Value {
    let mut format_data = serde_json::Map::new();

    // Extract format_corrected status
    if let Some(format_corrected) = data.get("format_corrected") {
        if !format_corrected.is_null() {
            format_data.insert("format_corrected".to_string(), format_corrected.clone());
        }
    }

    // Extract original_error if present (when error message was enhanced)
    if let Some(error_data) = data.get("error_data") {
        if let Some(original_error) = error_data.get("original_error") {
            if !original_error.is_null() {
                format_data.insert("original_error".to_string(), original_error.clone());
            }
        }
    }

    // Extract format_corrections array
    if let Some(format_corrections) = data.get("format_corrections") {
        if !format_corrections.is_null() {
            format_data.insert("format_corrections".to_string(), format_corrections.clone());
        }
    }

    // Extract metadata from first correction if available
    if let Some(corrections_array) = data.get("format_corrections").and_then(|c| c.as_array()) {
        if let Some(first_correction) = corrections_array.first() {
            if let Some(obj) = first_correction.as_object() {
                extract_correction_metadata(&mut format_data, obj);
            }
        }
    }

    serde_json::Value::Object(format_data)
}

/// Extract metadata fields from a format correction object
fn extract_correction_metadata(
    format_data: &mut serde_json::Map<String, Value>,
    correction: &serde_json::Map<String, Value>,
) {
    // Extract common format correction metadata
    for field in [
        "hint",
        "mutation_paths",
        "supported_operations",
        "type_category",
    ] {
        if let Some(value) = correction.get(field) {
            if !value.is_null() {
                format_data.insert(field.to_string(), value.clone());
            }
        }
    }

    // Extract rich guidance from corrected_format if available
    if let Some(corrected_format) = correction.get("corrected_format") {
        if let Some(corrected_obj) = corrected_format.as_object() {
            extract_rich_guidance(format_data, corrected_obj);
        }
    }

    // Also check for examples and valid_values at correction level
    if !format_data.contains_key("examples") {
        if let Some(examples) = correction.get("examples") {
            if !examples.is_null() {
                format_data.insert("examples".to_string(), examples.clone());
            }
        }
    }

    if !format_data.contains_key("valid_values") {
        if let Some(valid_values) = correction.get("valid_values") {
            if !valid_values.is_null() {
                format_data.insert("valid_values".to_string(), valid_values.clone());
            }
        }
    }
}

/// Extract rich guidance fields from `corrected_format` object
fn extract_rich_guidance(
    format_data: &mut serde_json::Map<String, Value>,
    corrected_format: &serde_json::Map<String, Value>,
) {
    // Extract examples from corrected_format
    if let Some(examples) = corrected_format.get("examples") {
        if !examples.is_null() {
            format_data.insert("examples".to_string(), examples.clone());
        }
    }

    // Extract valid_values from corrected_format
    if let Some(valid_values) = corrected_format.get("valid_values") {
        if !valid_values.is_null() {
            format_data.insert("valid_values".to_string(), valid_values.clone());
        }
    }

    // Also check for hint in corrected_format as fallback
    if !format_data.contains_key("hint") {
        if let Some(hint) = corrected_format.get("hint") {
            if !hint.is_null() {
                format_data.insert("hint".to_string(), hint.clone());
            }
        }
    }
}
