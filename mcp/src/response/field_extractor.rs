//! Field extraction functions for response formatting.
//!
//! This module provides the bridge between `ExtractorType` enum variants
//! and the actual field extraction functions used by the response formatter.

use serde_json::Value;

use super::FormatterContext;
use crate::response::{FieldPlacement, ResponseField};

/// Function type for extracting field values from response data and context.
///
/// Takes:
/// - `&Value` - The response data (usually from BRP)
/// - `&FormatterContext` - Context including request parameters
///
/// Returns: `Value` - The extracted field value
pub type FieldExtractor = Box<dyn Fn(&Value, &FormatterContext) -> Value + Send + Sync>;

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
pub fn create_request_field_accessor(field: &'static str) -> FieldExtractor {
    Box::new(move |_data, context| {
        match field {
            "entity" => {
                // Extract entity ID from request parameters
                context
                    .params
                    .as_ref()
                    .and_then(|params| {
                        // Look for entity field in params
                        params.get("entity").cloned()
                    })
                    .unwrap_or(Value::Null)
            }

            // For all other fields, perform a generic lookup in the params
            _ => context
                .params
                .as_ref()
                .and_then(|params| params.get(field))
                .cloned()
                .unwrap_or(Value::Null),
        }
    })
}

/// Convert a `ResponseField` specification to a field extractor function with placement info.
///
/// This creates the actual closure that will extract data based on the
/// extraction strategy defined by the `ResponseField` variant, and returns
/// the placement information for where the field should be put in the response.
pub fn convert_response_field(field: &ResponseField) -> (FieldExtractor, FieldPlacement) {
    match field {
        ResponseField::FromRequest {
            parameter_field_name: field,
            placement,
            ..
        } => (create_request_field_accessor(field), placement.clone()),
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
        ResponseField::DirectToResult => (
            Box::new(|data, _context| data.clone()),
            FieldPlacement::Result,
        ),
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
    }
}
