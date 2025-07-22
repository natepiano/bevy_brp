use serde_json::Value;

use super::extraction::{ResponseFieldType, extract_response_field};
use super::template_substitution::substitute_template_with_priority;
use super::{FieldPlacement, ResponseField};
use crate::brp_tools::{FormatCorrection, FormatCorrectionStatus};
use crate::constants::{RESPONSE_DEBUG_INFO, RESPONSE_METADATA};
use crate::tool::HandlerContext;

/// Encapsulates all extracted components from a response
pub struct ResponseComponents {
    /// Format corrections extracted from BRP response
    pub format_corrections: Option<Vec<FormatCorrection>>,
    /// Format correction status
    pub format_corrected:   Option<FormatCorrectionStatus>,
    /// Debug information from `brp_extras`
    pub debug_info:         Option<Value>,
    /// Fields configured for the response
    pub configured_fields:  Vec<ConfiguredField>,
    /// Final processed message with template substitution complete
    pub final_message:      String,
}

/// A field that has been extracted and configured for placement
pub struct ConfiguredField {
    pub name:               String,
    pub value:              Value,
    pub placement:          FieldPlacement,
    pub is_metadata_object: bool, // Special handling for metadata objects
}

impl ResponseComponents {
    /// Extract all components from response data and process final message
    pub fn from_response_data(
        response_def: &super::ResponseDef,
        handler_context: &HandlerContext,
        data: &Value,
    ) -> Self {
        // Extract format corrections
        let (format_corrections, format_corrected) = Self::extract_format_corrections(data);

        // Extract debug info and clean data
        let (clean_data, debug_info) = Self::extract_debug_and_clean_data(data);

        // Extract configured fields
        let configured_fields = Self::extract_configured_fields(
            &clean_data,
            handler_context,
            &response_def.response_fields,
        );

        // Process final message
        let final_message = Self::process_final_message(
            response_def.message_template,
            format_corrected.as_ref(),
            &configured_fields,
            handler_context,
            &clean_data,
        );

        Self {
            format_corrections,
            format_corrected,
            debug_info,
            configured_fields,
            final_message,
        }
    }

    /// Extract format correction information from BRP result JSON
    fn extract_format_corrections(
        value: &serde_json::Value,
    ) -> (
        Option<Vec<FormatCorrection>>,
        Option<FormatCorrectionStatus>,
    ) {
        let format_corrected = value
            .get("format_corrected")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let format_corrections = value
            .get("format_corrections")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|correction_json| {
                        // Convert JSON back to FormatCorrection struct
                        let component = correction_json
                            .get("component")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let original_format = correction_json
                            .get("original_format")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);

                        let corrected_format = correction_json
                            .get("corrected_format")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);

                        let hint = correction_json
                            .get("hint")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let supported_operations = correction_json
                            .get("supported_operations")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            });

                        let mutation_paths = correction_json
                            .get("mutation_paths")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            });

                        let type_category = correction_json
                            .get("type_category")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        FormatCorrection {
                            component,
                            original_format,
                            corrected_format,
                            hint,
                            supported_operations,
                            mutation_paths,
                            type_category,
                        }
                    })
                    .collect()
            });

        (format_corrections, format_corrected)
    }

    /// Extract debug info and clean data from incoming response
    fn extract_debug_and_clean_data(data: &Value) -> (Value, Option<Value>) {
        let mut clean_data = data.clone();
        let mut brp_extras_debug_info = None;

        if let Value::Object(data_map) = data {
            if let Some(debug_info) = data_map.get(RESPONSE_DEBUG_INFO) {
                if !debug_info.is_null() && (debug_info.is_array() || debug_info.is_string()) {
                    brp_extras_debug_info = Some(debug_info.clone());
                }
            }

            if let Value::Object(clean_map) = &mut clean_data {
                clean_map.remove(RESPONSE_DEBUG_INFO);
            }
        }

        (clean_data, brp_extras_debug_info)
    }

    /// Extract configured fields for JSON response structure.
    ///
    /// Processes each `ResponseField` definition to extract values from request parameters
    /// or response data and prepare them for placement in the JSON response structure
    /// (metadata/result sections).
    fn extract_configured_fields(
        clean_data: &Value,
        handler_context: &HandlerContext,
        response_fields: &[ResponseField],
    ) -> Vec<ConfiguredField> {
        let mut configured_fields = Vec::new();

        for field in response_fields {
            let field_name = field.name();
            let (value, placement) = Self::extract_field_value(field, clean_data, handler_context);

            let is_metadata_object =
                field_name == RESPONSE_METADATA && matches!(placement, FieldPlacement::Metadata);

            configured_fields.push(ConfiguredField {
                name: field_name.to_string(),
                value,
                placement,
                is_metadata_object,
            });
        }

        configured_fields
    }

    /// Extract field value based on `ResponseField` specification
    fn extract_field_value(
        field: &ResponseField,
        data: &Value,
        handler_context: &HandlerContext,
    ) -> (Value, FieldPlacement) {
        match field {
            ResponseField::FromRequest {
                parameter_name,
                placement,
                ..
            } => {
                // Extract from request parameters
                let value = handler_context
                    .extract_optional_named_field(parameter_name.into())
                    .cloned()
                    .unwrap_or(Value::Null);
                (value, placement.clone())
            }
            ResponseField::FromResponse {
                response_field_name,
                source_path,
                placement,
            } => {
                // Use unified extraction with source path override
                let value =
                    extract_response_field(data, source_path, response_field_name.field_type())
                        .map_or(Value::Null, std::convert::Into::into);
                (value, placement.clone())
            }
            ResponseField::DirectToMetadata => {
                // Return the entire data object
                (data.clone(), FieldPlacement::Metadata)
            }
            ResponseField::FromResponseNullableWithPlacement {
                response_field_name,
                source_path,
                placement,
            } => {
                // Use unified extraction with source path override
                let value =
                    extract_response_field(data, source_path, response_field_name.field_type())
                        .map_or(Value::Null, std::convert::Into::into);

                let result_value = if value.is_null() {
                    Value::String("__SKIP_NULL_FIELD__".to_string())
                } else {
                    value
                };
                (result_value, placement.clone())
            }
            ResponseField::BrpRawResultToResult => {
                // Extract raw result field using unified extraction
                let value = extract_response_field(data, "result", ResponseFieldType::Any)
                    .map_or(Value::Null, std::convert::Into::into);
                (value, FieldPlacement::Result)
            }
            ResponseField::FormatCorrection => {
                // Extract format correction fields
                let value = Self::extract_format_correction_fields(data);
                (value, FieldPlacement::Metadata)
            }
        }
    }

    /// Extract all format correction related fields from `BrpMethodResult`
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
                    Self::extract_correction_metadata(&mut format_data, obj);
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
                Self::extract_rich_guidance(format_data, corrected_obj);
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

    /// Process final message based on format correction status and template
    fn process_final_message(
        message_template: &str,
        format_corrected: Option<&FormatCorrectionStatus>,
        configured_fields: &[ConfiguredField],
        handler_context: &HandlerContext,
        clean_data: &Value,
    ) -> String {
        if format_corrected == Some(&FormatCorrectionStatus::Succeeded) {
            "Request succeeded with format correction applied".to_string()
        } else if !message_template.is_empty() {
            substitute_template_with_priority(
                message_template,
                configured_fields,
                handler_context,
                clean_data,
            )
        } else {
            String::new() // Default empty message
        }
    }
}
