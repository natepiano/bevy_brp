use crate::brp_tools::constants::{JSON_FIELD_APP_NAME, JSON_FIELD_METADATA, JSON_FIELD_RESULT};

/// Specifies where a response field should be placed in the output JSON
#[derive(Clone, Debug)]
pub enum FieldPlacement {
    /// Place field in the metadata object
    Metadata,
    /// Place field in the result object
    #[allow(dead_code)] // Will be used in upcoming migration
    Result,
}

/// New response field specification that separates request and response concerns.
///
/// This enum replaces the old `ResponseField` struct, providing clear separation
/// between request parameter referencing and response data extraction specifications.
#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)] // Existing variants follow established naming pattern
pub enum ResponseField {
    /// Reference a field from already-extracted request parameters.
    ///
    /// This variant references data that was already extracted and validated during
    /// the parameter extraction phase, avoiding redundant extraction and ensuring
    /// consistent validation.
    FromRequest {
        /// Name of the field to be output in the response
        response_field_name:  &'static str,
        /// Field name in the `ExtractedParams` structure, i.e, tool call request parameters
        parameter_field_name: &'static str,
    },
    /// Extract a field from response data.
    ///
    /// This variant specifies extraction of data from the handler or BRP response payload.
    FromResponse {
        /// Name of the field in the response
        response_field_name: &'static str,
        /// Extractor type for response data
        extractor:           ResponseExtractorType,
    },
    /// Reference a field from already-extracted request parameters with explicit placement.
    ///
    /// This variant references data that was already extracted and validated during
    /// the parameter extraction phase, with explicit control over where the field is placed.
    #[allow(dead_code)] // Will be used in upcoming migration
    FromRequestWithPlacement {
        /// Name of the field to be output in the response
        response_field_name:  &'static str,
        /// Field name in the `ExtractedParams` structure, i.e, tool call request parameters
        parameter_field_name: &'static str,
        /// Where to place this field in the response
        placement:            FieldPlacement,
    },
    /// Extract a field from response data with explicit placement.
    ///
    /// This variant specifies extraction of data from the handler or BRP response payload
    /// with explicit control over where the field is placed.
    #[allow(dead_code)] // Will be used in upcoming migration
    FromResponseWithPlacement {
        /// Name of the field in the response
        response_field_name: &'static str,
        /// Extractor type for response data
        extractor:           ResponseExtractorType,
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
        extractor:           ResponseExtractorType,
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
    /// Extract entity ID from response data (for spawn operations)
    EntityId,
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
                // Check if data is wrapped in a structure with a "count" field
                data.as_object()
                    .and_then(|obj| obj.get("count"))
                    .map_or_else(
                        || data.as_array().map_or(0, std::vec::Vec::len).into(),
                        std::clone::Clone::clone,
                    )
            }
            Self::ItemCount => {
                // Check if data is wrapped in a structure with a "metadata" field
                let count = data
                    .as_object()
                    .and_then(|obj| obj.get("metadata"))
                    .map_or_else(
                        || data.as_array().map_or(0, std::vec::Vec::len),
                        |inner_data| inner_data.as_array().map_or(0, std::vec::Vec::len),
                    );
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
            Self::EntityId => {
                // Extract entity ID from response data (for spawn operation)
                data.get("entity")
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::Number(serde_json::Number::from(0)))
            }
            Self::SplitContentIntoLines => {
                // Extract content field and split into array of lines
                data.get("content")
                    .and_then(|content| content.as_str())
                    .map(|content_str| {
                        let lines: Vec<serde_json::Value> = content_str
                            .lines()
                            .map(|line| serde_json::Value::String(line.to_string()))
                            .collect();
                        serde_json::Value::Array(lines)
                    })
                    .unwrap_or(serde_json::Value::Array(vec![]))
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
            | Self::FromRequestWithPlacement {
                response_field_name: name,
                ..
            }
            | Self::FromResponseWithPlacement {
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
/// This enum provides type-safe response patterns, making illegal states unrepresentable:
/// - `Structured`: Traditional response with extracted/processed fields under "metadata"
/// - `Raw`: Raw BRP response directly in "result" field (no other fields allowed)
#[derive(Clone)]
pub enum ResponseSpecification {
    /// Traditional response with extracted/processed fields under "metadata"
    Structured {
        /// Template for success messages
        message_template: &'static str,
        /// Fields to include in the response
        response_fields:  Vec<ResponseField>,
    },
    /// Raw BRP response directly in "result" field (no other fields allowed)
    Raw {
        /// Template for success messages
        template: &'static str,
    },
}
