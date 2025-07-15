use crate::constants::{
    JSON_FIELD_CONTENT, JSON_FIELD_COUNT, JSON_FIELD_METADATA, JSON_FIELD_RESULT,
};

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
        response_field_name:  &'static str,
        /// Field name in the tool call request parameters
        parameter_field_name: &'static str,
        /// Where to place this field in the response
        placement:            FieldPlacement,
    },
    /// Extract a field from response data with explicit placement.
    ///
    /// This variant specifies extraction of data from the handler or BRP response payload
    /// with explicit control over where the field is placed.
    FromResponse {
        /// Name of the field in the response
        response_field_name: &'static str,
        /// Extractor type for response data
        response_extractor:  ResponseExtractorType,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Pass the entire BRP response data directly to the result field.
    ///
    /// This variant is specifically for Raw-to-Structured migrations where the
    /// entire BRP response becomes the result field content.
    DirectToResult,
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
        response_field_name: &'static str,
        /// Extractor type for response data
        response_extractor:  ResponseExtractorType,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
}

/// Extraction strategies for response data only.
#[derive(Clone, Debug)]
pub enum ResponseExtractorType {
    /// Extract a specific field from the response data structure
    Field(&'static str),
    /// Extract count field from response data
    Count,
    /// Count items (entities, components, resources, etc.) in an array
    ItemCount,
    /// Extract total component count from nested query results
    QueryComponentCount,
    /// Split content field into numbered lines
    SplitContentIntoLines,
}

impl ResponseExtractorType {
    /// Extract data based on the extraction strategy
    pub fn extract(&self, data: &serde_json::Value) -> serde_json::Value {
        match self {
            Self::Field(field_name) => data
                .get(field_name)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            Self::Count => {
                // Extract count field as a number, return Null if not found or not a number
                data.as_object()
                    .and_then(|obj| obj.get(JSON_FIELD_COUNT))
                    .and_then(serde_json::Value::as_u64)
                    .map_or(serde_json::Value::Null, |count| {
                        serde_json::Value::Number(serde_json::Number::from(count))
                    })
            }
            Self::ItemCount => {
                // Count items in an array response
                let count = data.as_array().map_or(0, std::vec::Vec::len);
                serde_json::Value::Number(serde_json::Number::from(count))
            }
            Self::QueryComponentCount => {
                // Extract total component count from nested query results
                let total = data.as_array().map_or(0, |entities| {
                    entities
                        .iter()
                        .filter_map(|e| e.as_object())
                        .map(serde_json::Map::len)
                        .sum::<usize>()
                });
                serde_json::Value::Number(serde_json::Number::from(total))
            }
            Self::SplitContentIntoLines => {
                // Extract content field and split into array of lines
                data.get(JSON_FIELD_CONTENT)
                    .and_then(|content| content.as_str())
                    .map_or(serde_json::Value::Array(vec![]), |content_str| {
                        let lines: Vec<serde_json::Value> = content_str
                            .lines()
                            .map(|line| serde_json::Value::String(line.to_string()))
                            .collect();
                        serde_json::Value::Array(lines)
                    })
            }
        }
    }
}

impl ResponseField {
    /// Get the field name for this response field specification.
    pub const fn name(&self) -> &'static str {
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
            } => name,
            Self::DirectToResult => JSON_FIELD_RESULT,
            Self::DirectToMetadata => JSON_FIELD_METADATA,
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

        // Create the formatter config for structured responses
        let mut success_fields = Vec::new();

        // Add response fields
        for field in &self.response_fields {
            let (extractor, placement) = super::create_response_field_extractor(field);
            success_fields.push((field.name().to_string(), extractor, placement));
        }

        // Set the template if provided
        let success_template = if self.message_template.is_empty() {
            None
        } else {
            Some(self.message_template.to_string())
        };

        super::FormatterConfig {
            success_template,
            success_fields,
            large_response_config: LargeResponseConfig {
                file_prefix: "brp_response_".to_string(),
                ..Default::default()
            },
        }
    }
}
