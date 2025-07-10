/// New response field specification that separates request and response concerns.
///
/// This enum replaces the old `ResponseField` struct, providing clear separation
/// between request parameter referencing and response data extraction specifications.
#[derive(Clone, Debug)]
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
}

/// Extraction strategies for response data only.
#[derive(Clone, Debug)]
pub enum ResponseExtractorType {
    /// Pass through the entire BRP response data wrapped in the "metadata" field of the MCP
    /// response. Used for Structured responses where the BRP data should be included under
    /// "metadata". Example output: `{ "status": "success", "message": "...", "metadata": {
    /// ...brp_data... } }`
    PassThroughData,

    /// Extract a specific field from the response data structure
    Field(&'static str),
    /// Extract count field from response data
    Count,
    /// Count entities in response data
    EntityCount,
    /// Count components in response data
    ComponentCount,
    /// Extract total component count from nested query results
    QueryComponentCount,
    /// Extract entity ID from response data (for spawn operations)
    EntityId,
}

impl ResponseExtractorType {
    /// Extract data based on the extraction strategy
    pub fn extract(&self, data: &serde_json::Value) -> serde_json::Value {
        match self {
            Self::PassThroughData => data.clone(),
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
            Self::EntityCount => {
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
            Self::ComponentCount => {
                // Same as EntityCount for now
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
            } => name,
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
