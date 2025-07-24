use serde_json::Value;

use super::extraction::{ExtractedValue, ResponseFieldType, extract_response_field};
use super::field_placement_traits::FieldAccessor;
use super::{FieldPlacement, ResponseField, ResponseFieldName, template_substitution};
use crate::brp_tools::{FormatCorrection, FormatCorrectionField, FormatCorrectionStatus};
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
#[derive(Clone)]
pub struct ConfiguredField {
    pub name:               String,
    pub value:              Value,
    pub placement:          FieldPlacement,
    pub is_metadata_object: bool, // Special handling for metadata objects
}

impl ResponseComponents {
    /// Extract all components using field accessor instead of JSON
    pub fn from_field_accessor(
        response_def: &super::ResponseDef,
        handler_context: &HandlerContext,
        field_accessor: &dyn FieldAccessor,
    ) -> Self {
        // Extract format corrections using field accessor
        let (format_corrections, format_corrected) =
            Self::extract_format_corrections_from_accessor(field_accessor);

        // Extract debug info using field accessor
        let debug_info = Self::extract_debug_info_from_accessor(field_accessor);

        // Extract ALL fields automatically using field accessor
        let configured_fields = Self::extract_all_fields_from_accessor(field_accessor);

        // Process final message
        let final_message = Self::process_final_message(
            response_def.message_template,
            format_corrected.as_ref(),
            &configured_fields,
            handler_context,
            &Value::Null, // No JSON data needed
        );

        Self {
            format_corrections,
            format_corrected,
            debug_info,
            configured_fields,
            final_message,
        }
    }

    /// Extract format corrections from field accessor
    fn extract_format_corrections_from_accessor(
        field_accessor: &dyn FieldAccessor,
    ) -> (
        Option<Vec<FormatCorrection>>,
        Option<FormatCorrectionStatus>,
    ) {
        // Try to get format_corrected status
        let format_corrected = field_accessor
            .get_field("format_corrected")
            .and_then(|v| match v {
                ExtractedValue::Any(val) => serde_json::from_value(val).ok(),
                _ => None,
            });

        // Try to get format_corrections array
        let format_corrections =
            field_accessor
                .get_field("format_corrections")
                .and_then(|v| match v {
                    ExtractedValue::Any(val) => serde_json::from_value(val).ok(),
                    _ => None,
                });

        (format_corrections, format_corrected)
    }

    /// Extract debug info from field accessor
    fn extract_debug_info_from_accessor(field_accessor: &dyn FieldAccessor) -> Option<Value> {
        field_accessor.get_field("debug_info").map(|v| v.into())
    }

    /// Extract configured fields using field accessor
    fn extract_configured_fields_from_accessor(
        field_accessor: &dyn FieldAccessor,
        handler_context: &HandlerContext,
        response_fields: &[ResponseField],
    ) -> Vec<ConfiguredField> {
        let mut configured_fields = Vec::new();

        for field in response_fields {
            let field_name = field.name();
            let (value, placement) =
                Self::extract_field_value_from_accessor(field, field_accessor, handler_context);

            let is_metadata_object = field_name == (ResponseFieldName::Metadata.as_ref())
                && matches!(placement, FieldPlacement::Metadata);

            configured_fields.push(ConfiguredField {
                name: field_name.to_string(),
                value,
                placement,
                is_metadata_object,
            });
        }

        configured_fields
    }

    /// Extract field value based on ResponseField specification using field accessor
    fn extract_field_value_from_accessor(
        field: &ResponseField,
        field_accessor: &dyn FieldAccessor,
        handler_context: &HandlerContext,
    ) -> (Value, FieldPlacement) {
        match field {
            ResponseField::FromRequest {
                parameter_name,
                placement,
                ..
            } => {
                // Extract from request parameters (same as before)
                let value = handler_context
                    .extract_optional_named_field(parameter_name.into())
                    .cloned()
                    .unwrap_or(Value::Null);
                (value, placement.clone())
            }
            ResponseField::FromResponse {
                response_field_name: _,
                source_path,
                placement,
            } => {
                // Use field accessor to get the field
                let field_name = source_path.split('.').last().unwrap_or(source_path);
                let value = field_accessor
                    .get_field(field_name)
                    .map(|v| v.into())
                    .unwrap_or(Value::Null);
                (value, placement.clone())
            }
            ResponseField::DirectToMetadata => {
                // For DirectToMetadata, we can't get all fields from FieldAccessor
                // This should be handled by having all fields marked with #[to_metadata] instead
                (Value::Null, FieldPlacement::Metadata)
            }
            ResponseField::FromResponseNullableWithPlacement {
                response_field_name: _,
                source_path,
                placement,
            } => {
                // Use field accessor
                let field_name = source_path.split('.').last().unwrap_or(source_path);
                let value = field_accessor
                    .get_field(field_name)
                    .map(|v| v.into())
                    .unwrap_or(Value::Null);

                let result_value = if value.is_null() {
                    Value::String("__SKIP_NULL_FIELD__".to_string())
                } else {
                    value
                };
                (result_value, placement.clone())
            }
            ResponseField::BrpRawResultToResult => {
                // Get the result field
                let value = field_accessor
                    .get_field("result")
                    .map(|v| v.into())
                    .unwrap_or(Value::Null);
                (value, FieldPlacement::Result)
            }
            ResponseField::FormatCorrection => {
                // Extract format correction fields using field accessor
                let value = Self::extract_format_correction_fields_from_accessor(field_accessor);
                (value, FieldPlacement::Metadata)
            }
        }
    }

    /// Extract format correction fields from field accessor
    fn extract_format_correction_fields_from_accessor(field_accessor: &dyn FieldAccessor) -> Value {
        let mut format_data = serde_json::Map::new();

        // Extract format_corrected status
        if let Some(extracted) = field_accessor.get_field("format_corrected") {
            let value: Value = extracted.into();
            if !value.is_null() {
                format_data.insert("format_corrected".to_string(), value);
            }
        }

        // Extract format_corrections array
        if let Some(extracted) = field_accessor.get_field("format_corrections") {
            let value: Value = extracted.into();
            if !value.is_null() {
                format_data.insert("format_corrections".to_string(), value);
            }
        }

        // Note: Other metadata fields would need to be extracted from the first correction
        // This is complex with FieldAccessor, so we might need a different approach

        serde_json::Value::Object(format_data)
    }

    /// Extract ALL fields automatically from field accessor
    fn extract_all_fields_from_accessor(
        field_accessor: &dyn FieldAccessor,
    ) -> Vec<ConfiguredField> {
        let mut configured_fields = Vec::new();

        // Try to extract common field names that response types typically have
        let field_candidates = vec![
            ("result", FieldPlacement::Result),
            ("entities", FieldPlacement::Result),
            ("apps", FieldPlacement::Result),
            ("examples", FieldPlacement::Result),
            ("resources", FieldPlacement::Result),
            ("components", FieldPlacement::Result),
            ("watches", FieldPlacement::Result),
            ("logs", FieldPlacement::Result),
            ("content", FieldPlacement::Result),
            ("lines", FieldPlacement::Result),
            ("count", FieldPlacement::Result),
            ("log_path", FieldPlacement::Result),
            ("file_size_bytes", FieldPlacement::Result),
            ("exists", FieldPlacement::Result),
            ("status", FieldPlacement::Metadata),
            ("app_name", FieldPlacement::Metadata),
            ("port", FieldPlacement::Metadata),
            ("method", FieldPlacement::Metadata),
            ("watch_id", FieldPlacement::Metadata),
            ("message", FieldPlacement::Metadata),
            ("path", FieldPlacement::Metadata),
            ("level", FieldPlacement::Metadata),
            ("deleted_count", FieldPlacement::Metadata),
            // Launch-related fields
            ("target_name", FieldPlacement::Metadata),
            ("pid", FieldPlacement::Metadata),
            ("working_directory", FieldPlacement::Metadata),
            ("profile", FieldPlacement::Metadata),
            ("log_file", FieldPlacement::Metadata),
            ("binary_path", FieldPlacement::Metadata),
            ("launch_duration_ms", FieldPlacement::Metadata),
            ("launch_timestamp", FieldPlacement::Metadata),
            ("workspace", FieldPlacement::Metadata),
            ("package_name", FieldPlacement::Metadata),
            ("duplicate_paths", FieldPlacement::Metadata),
            ("note", FieldPlacement::Metadata),
        ];

        for (field_name, placement) in field_candidates {
            if let Some(extracted_value) = field_accessor.get_field(field_name) {
                let value: Value = extracted_value.into();
                // Skip null values to avoid cluttering the response
                if !value.is_null() {
                    configured_fields.push(ConfiguredField {
                        name: field_name.to_string(),
                        value,
                        placement,
                        is_metadata_object: false,
                    });
                }
            }
        }

        configured_fields
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
            template_substitution::substitute_template_with_priority(
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
