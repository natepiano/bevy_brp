use super::ResponseFieldName;
use super::extraction::JsonFieldProvider;
use super::large_response::LargeResponseConfig;
use crate::tool::ParameterName;

/// Implement `JsonFieldProvider` for `serde_json::Value` to enable field extraction
impl JsonFieldProvider for serde_json::Value {
    fn get_root(&self) -> serde_json::Value {
        self.clone()
    }
}

/// Specifies where a response field should be placed in the output JSON
#[derive(Clone, Debug)]
pub enum FieldPlacement {
    /// Place field in the metadata object
    Metadata,
    /// Place field in the result object
    Result,
}

/// Response field specification for structured responses.
///
/// Defines how to extract and place fields in the response JSON structure.
#[derive(Clone, Debug)]
pub enum ResponseField {
    /// Reference a field from already-extracted request parameters with explicit placement.
    ///
    /// This variant references data that was already extracted and validated during
    /// the parameter extraction phase, with explicit control over where the field is placed.
    FromRequest {
        /// Name of the field to be output in the response
        response_field_name: ResponseFieldName,
        /// Parameter name from the tool call request parameters
        parameter_name:      ParameterName,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Extract a field from response data with explicit placement.
    ///
    /// This variant specifies extraction of data from the handler or BRP response payload
    /// with explicit control over where the field is placed.
    FromResponse {
        /// Name of the field in the response
        response_field_name: ResponseFieldName,
        /// Source path for extraction
        /// Supports dot notation for nested fields (e.g., "result.entity")
        source_path:         &'static str,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Pass all fields from the BRP response directly to the metadata field.
    ///
    /// This variant takes all top-level fields from the response and places them
    /// in metadata, useful for tools that return many fields that all belong in metadata.
    DirectToMetadata,
    /// Extract a field from response data that may be null - skip if null.
    ///
    /// This variant extracts a field and omits it from the response if the value is null.
    /// Use this for optional fields that should not appear in the response when missing.
    FromResponseNullableWithPlacement {
        /// Name of the field in the response
        response_field_name: ResponseFieldName,
        /// Source path for extraction
        /// Supports dot notation for nested fields (e.g., "result.entity")
        source_path:         &'static str,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Extract the raw BRP response data from the "result" field to the result field
    ///
    /// This is a convenience variant for BRP tools that need to extract the raw BRP response
    /// from the "result" field and place it in the JSON response result field.
    BrpRawResultToResult,
    /// Extract format correction metadata from handler responses
    ///
    /// This variant extracts all format correction fields (`format_corrected`,
    /// `format_corrections`, etc.) from `BrpMethodResult` and places them in metadata. Only
    /// used for V2 tools that support format correction.
    FormatCorrection,
}

impl ResponseField {
    /// Get the field name for this response field specification.
    pub fn name(&self) -> &'static str {
        match self {
            Self::FromRequest {
                response_field_name: name,
                ..
            }
            | Self::FromResponse {
                response_field_name: name,
                ..
            }
            | Self::FromResponseNullableWithPlacement {
                response_field_name: name,
                ..
            } => name.into(),
            Self::DirectToMetadata | Self::FormatCorrection => ResponseFieldName::Metadata.into(),
            Self::BrpRawResultToResult => ResponseFieldName::Result.into(),
        }
    }
}

/// Defines how to format the response for a tool.
///
/// Specifies the message template and fields to include in structured responses.
#[derive(Clone)]
pub struct ResponseSpecification {
    /// Template for success messages
    pub message_template: &'static str,
    /// Fields to include in the response
    pub response_fields:  Vec<ResponseField>,
}

impl ResponseSpecification {
    /// Build formatter configuration from this response specification
    pub fn build_response_formatter(&self) -> super::ResponseFormatter {
        super::ResponseFormatter {
            message_template:      self.message_template.to_string(),
            success_fields:        self.response_fields.clone(),
            large_response_config: LargeResponseConfig {
                file_prefix: "brp_response_".to_string(),
                ..Default::default()
            },
        }
    }
}
