use super::fields::ResponseFieldName;
use crate::extraction::{FieldType, JsonFieldProvider, ResponseFieldSpec, extract_field};
use crate::tool::ParameterName;

/// Implement `JsonFieldProvider` for `serde_json::Value` to enable field extraction
impl JsonFieldProvider for serde_json::Value {
    fn get_root(&self) -> serde_json::Value {
        self.clone()
    }
}

/// Extract a nested field from JSON data using dot notation
///
/// Supports paths like "result.entity" or "data.components.count"
/// Empty string means use the data as-is (root level access)
fn extract_nested_field(data: &serde_json::Value, field_path: &str) -> serde_json::Value {
    // Empty path means use the data as-is
    if field_path.is_empty() {
        return data.clone();
    }

    // Split the path by dots and traverse each part
    let path_parts: Vec<&str> = field_path.split('.').collect();

    let mut current_value = data;

    // Navigate through each part of the path
    for part in path_parts {
        if let Some(next_value) = current_value.get(part) {
            current_value = next_value;
        } else {
            // If any part of the path is not found, return null
            return serde_json::Value::Null;
        }
    }

    current_value.clone()
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
        /// Extractor type for response data
        response_extractor:  ResponseExtractorType,
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
        /// Extractor type for response data
        response_extractor:  ResponseExtractorType,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Extract the raw BRP response data from the "result" field to the result field (V2 handlers)
    ///
    /// This is a convenience variant for V2 BRP tools that need to extract the raw BRP response
    /// from the "result" field and place it in the JSON response result field.
    BrpRawResultToResult,
    /// Extract format correction metadata from V2 handler responses
    ///
    /// This variant extracts all format correction fields (`format_corrected`,
    /// `format_corrections`, etc.) from V2 `BrpMethodResult` and places them in metadata. Only
    /// used for V2 tools that support format correction.
    FormatCorrection,
}

/// Extraction strategies for response data only.
#[derive(Clone, Debug)]
pub enum ResponseExtractorType {
    /// Extract a specific field from the response data structure
    /// Supports dot notation for nested fields (e.g., "result.entity")
    Field(&'static str),
    /// Extract count field from response data
    Count,
    /// Count items in an array at the specified field path
    /// Supports dot notation for nested field access (e.g., "result" or "data.entities")
    ArrayCount(&'static str),
    /// Count keys in an object at the specified field path
    /// Supports dot notation for nested field access (e.g., "result.components")
    KeyCount(&'static str),
    /// Extract total component count from nested query results
    /// Supports dot notation for nested field access (e.g., "result" or "data.entities")
    QueryComponentCount(&'static str),
    /// Split content field into numbered lines
    SplitContentIntoLines,
}

impl ResponseExtractorType {
    /// Extract data based on the extraction strategy
    pub fn extract(&self, data: &serde_json::Value) -> serde_json::Value {
        match self {
            Self::Field(field_path) => {
                // Use unified extraction with Any type for generic field access
                let spec = ResponseFieldSpec {
                    field_name: (*field_path).to_string(),
                    field_type: FieldType::Any,
                };
                extract_field(data, spec).map_or(serde_json::Value::Null, |extracted| {
                    match extracted {
                        crate::extraction::ExtractedValue::Any(v) => v,
                        _ => serde_json::Value::Null,
                    }
                })
            }
            Self::Count => {
                // Extract count field as a number
                let spec = ResponseFieldSpec {
                    field_name: <ResponseFieldName as Into<&'static str>>::into(
                        ResponseFieldName::Count,
                    )
                    .to_string(),
                    field_type: FieldType::Number,
                };
                extract_field(data, spec).map_or(serde_json::Value::Null, |extracted| {
                    match extracted {
                        crate::extraction::ExtractedValue::Number(n) => {
                            serde_json::Value::Number(serde_json::Number::from(n))
                        }
                        _ => serde_json::Value::Null,
                    }
                })
            }
            Self::ArrayCount(field_path) => {
                // Use unified extraction with Count type
                let spec = ResponseFieldSpec {
                    field_name: (*field_path).to_string(),
                    field_type: FieldType::Count,
                };
                extract_field(data, spec).map_or(serde_json::Value::Null, |extracted| {
                    match extracted {
                        crate::extraction::ExtractedValue::Number(n) => {
                            serde_json::Value::Number(serde_json::Number::from(n))
                        }
                        _ => serde_json::Value::Null,
                    }
                })
            }
            Self::KeyCount(field_path) => {
                // Use unified extraction with Count type (works for objects too)
                let spec = ResponseFieldSpec {
                    field_name: (*field_path).to_string(),
                    field_type: FieldType::Count,
                };
                extract_field(data, spec).map_or(serde_json::Value::Null, |extracted| {
                    match extracted {
                        crate::extraction::ExtractedValue::Number(n) => {
                            serde_json::Value::Number(serde_json::Number::from(n))
                        }
                        _ => serde_json::Value::Null,
                    }
                })
            }
            Self::QueryComponentCount(field_path) => {
                // This is a special case that needs custom logic
                // Extract the field first, then count components
                let field_data = extract_nested_field(data, field_path);
                let total = field_data.as_array().map_or(0, |entities| {
                    entities
                        .iter()
                        .filter_map(|e| e.as_object())
                        .map(serde_json::Map::len)
                        .sum::<usize>()
                });
                serde_json::Value::Number(serde_json::Number::from(total))
            }
            Self::SplitContentIntoLines => {
                // Use unified extraction with LineSplit type
                let spec = ResponseFieldSpec {
                    field_name: <ResponseFieldName as Into<&'static str>>::into(
                        ResponseFieldName::Content,
                    )
                    .to_string(),
                    field_type: FieldType::LineSplit,
                };
                extract_field(data, spec).map_or_else(
                    || serde_json::Value::Array(vec![]),
                    |extracted| match extracted {
                        crate::extraction::ExtractedValue::StringArray(lines) => {
                            let json_lines: Vec<serde_json::Value> =
                                lines.into_iter().map(serde_json::Value::String).collect();
                            serde_json::Value::Array(json_lines)
                        }
                        _ => serde_json::Value::Array(vec![]),
                    },
                )
            }
        }
    }
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
    pub fn build_formatter_config(&self) -> super::FormatterConfig {
        use super::large_response::LargeResponseConfig;

        // Set the template if provided
        let success_template = if self.message_template.is_empty() {
            None
        } else {
            Some(self.message_template.to_string())
        };

        super::FormatterConfig {
            success_template,
            success_fields: self.response_fields.clone(),
            large_response_config: LargeResponseConfig {
                file_prefix: "brp_response_".to_string(),
                ..Default::default()
            },
        }
    }
}
